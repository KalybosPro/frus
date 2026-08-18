//! The **relayout boundary** cache: holds, between frames, the computed rectangles
//! of each layout root, indexed by identity.
//!
//! Frus rebuilds the widget tree on every frame and, until now, re-ran taffy *from
//! scratch* at every layout root (`build_ui`, every scrollable, every screen, every
//! overlay…). Yet the geometry depends **only** on the tree's *style* and
//! *structure* and on the parent's *constraints* — not on colors or text, which
//! only affect painting. A hover, a blinking caret, an animating color → the same
//! layout.
//!
//! This cache records, per root (`WidgetId`), `(fingerprint, constraints,
//! rectangles)`. If the layout fingerprint and the constraints are unchanged, the
//! **rectangles are reused** and taffy is not called again. The output is
//! **bit-for-bit identical** to the full computation — only the performance
//! changes. It is also the foundation of the future phase system: "fingerprint
//! changed" *is* a root's "layout dirty" bit.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use frus_core::{Rect, Size};
use frus_layout::Overflowing;

use crate::interaction::WidgetId;
use crate::runtime::Runtime;
use crate::ui::{build_layout, child_id, effective_style};
use crate::widget::Widget;

/// The constraints passed to taffy for a layout root. `free_x`/`free_y` leave an
/// axis **free** (the content takes its natural size) — the case of scrollable
/// content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Constraints {
    pub w: f32,
    pub h: f32,
    pub free_x: bool,
    pub free_y: bool,
    /// **Fill** the box rather than hug the content: an unset (`Auto`) dimension of
    /// the root becomes the available size. For content that is *given* its box —
    /// a page of a [`crate::PageView`] — rather than asked how big it wants to be.
    pub fill: bool,
}

impl Constraints {
    /// Both axes constrained to `size` (ordinary layout).
    pub fn definite(size: Size) -> Self {
        Self {
            w: size.width,
            h: size.height,
            free_x: false,
            free_y: false,
            fill: false,
        }
    }

    /// A box the content is **handed**: constrained on both axes, and filled.
    pub fn filled(size: Size) -> Self {
        Self {
            fill: true,
            ..Self::definite(size)
        }
    }

    /// Scrollable content: each axis either constrained or free.
    pub fn scroll(w: f32, h: f32, free_x: bool, free_y: bool) -> Self {
        Self {
            w,
            h,
            free_x,
            free_y,
            fill: false,
        }
    }
}

/// One cache entry: the fingerprint of the last root computed under this identity,
/// its constraints, and the rectangles produced (in prefix order).
struct Entry {
    signature: u64,
    constraints: Constraints,
    rects: Vec<Rect>,
    /// Boxes in this root whose children did not fit. Cached alongside the rectangles
    /// because it is computed from the same taffy pass and would otherwise need a
    /// second one on every cache hit.
    overflows: Vec<Overflowing>,
}

/// The relayout cache, retained in the [`crate::Runtime`] from one frame to the next.
#[derive(Default)]
pub struct LayoutCache {
    entries: HashMap<WidgetId, Entry>,
    /// Roots touched during the current frame (to evict the vanished ones).
    touched: HashSet<WidgetId>,
    /// Diagnostic: reuses and recomputations of the **last** frame.
    last_hits: u32,
    last_misses: u32,
    hits: u32,
    misses: u32,
}

impl LayoutCache {
    /// The rectangles (prefix order) of root `key` under constraints `c`. Reuses
    /// the cache if the layout fingerprint **and** the constraints are unchanged;
    /// otherwise re-runs taffy and records the result.
    pub(crate) fn rects<Msg>(
        &mut self,
        key: WidgetId,
        root: &dyn Widget<Msg>,
        runtime: &Runtime,
        theme: &crate::theme::Theme,
        c: Constraints,
    ) -> (Vec<Rect>, Vec<Overflowing>) {
        self.touched.insert(key);
        let signature = layout_signature(root, key, runtime, theme);
        if let Some(entry) = self.entries.get(&key) {
            if entry.signature == signature && entry.constraints == c {
                self.hits += 1;
                return (entry.rects.clone(), entry.overflows.clone());
            }
        }
        self.misses += 1;
        let (rects, overflows) = compute_rects(root, key, runtime, theme, c);
        self.entries.insert(
            key,
            Entry {
                signature,
                constraints: c,
                rects: rects.clone(),
                overflows: overflows.clone(),
            },
        );
        (rects, overflows)
    }

    /// To be called at the end of a frame: forgets untouched roots (vanished
    /// widgets) and freezes the frame's diagnostic counters.
    pub(crate) fn end_frame(&mut self) {
        let touched = std::mem::take(&mut self.touched);
        self.entries.retain(|id, _| touched.contains(id));
        self.last_hits = self.hits;
        self.last_misses = self.misses;
        self.hits = 0;
        self.misses = 0;
    }

    /// Reuses and recomputations of the last completed frame (diagnostic).
    pub fn last_frame_stats(&self) -> (u32, u32) {
        (self.last_hits, self.last_misses)
    }
}

/// Computes a root's absolute rectangles — the "full" path (build + taffy +
/// collect), taken on every cache *miss*.
fn compute_rects<Msg>(
    root: &dyn Widget<Msg>,
    key: WidgetId,
    runtime: &Runtime,
    theme: &crate::theme::Theme,
    c: Constraints,
) -> (Vec<Rect>, Vec<Overflowing>) {
    let mut layout = frus_layout::Layout::new();
    let node = build_layout(root, key, runtime, theme, &mut layout);
    // `compute_scroll(_, _, false, false)` is equivalent to `compute` (both axes
    // `Definite`): a single path covers both cases.
    if c.fill {
        layout.compute_filled(node, c.w, c.h);
    } else {
        layout.compute_scroll(node, c.w, c.h, c.free_x, c.free_y);
    }
    let rects = layout
        .absolute_rects(node)
        .into_iter()
        .map(|(rect, _)| rect)
        .collect();
    (rects, layout.overflows(node))
}

/// A 64-bit fingerprint of a subtree's **layout**: styles + structure, following
/// **exactly** the branching of [`build_layout`] (scrollable/navigator/list/stack
/// = a leaf; portal = the anchor alone). Colors, texts and messages are excluded
/// (they only affect painting).
pub(crate) fn layout_signature<Msg>(
    root: &dyn Widget<Msg>,
    id: WidgetId,
    runtime: &Runtime,
    theme: &crate::theme::Theme,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_node(root, id, runtime, theme, &mut hasher);
    hasher.finish()
}

fn hash_node<Msg, H: Hasher>(
    widget: &dyn Widget<Msg>,
    id: WidgetId,
    runtime: &Runtime,
    theme: &crate::theme::Theme,
    hasher: &mut H,
) {
    // A themed subtree, exactly as `build_layout` sees it: its styles are resolved
    // against its own theme, so the fingerprint is taken against that one too. A
    // fingerprint that skipped the swap would hash one geometry and the cache would
    // store another.
    let scoped = widget.theme_override(theme);
    let theme = scoped.as_deref().unwrap_or(theme);
    // The cache exists to **skip** `build_layout`, so this walk can be the first one down
    // the tree: a deferred subtree has to be composed here too, or the fingerprint would
    // be taken of a node with no children and the cache would agree with itself forever.
    widget.build_themed(theme);
    // These branches must stay aligned with `build_layout`: the shape of the taffy
    // tree (and therefore the number and order of the rectangles) depends on it.
    // The **effective** style is hashed (animated size injected) — the same source
    // as `build_layout`, so the fingerprint changes while the size moves.
    // `RotatedBox`: its box depends on the child's **natural** size (swapped for an
    // odd quarter), so the fingerprint must include the child; otherwise a modified
    // child would leave a stale box in the cache.
    if let Some(q) = widget.rotated_quarter_turns() {
        4u8.hash(hasher);
        q.hash(hasher);
        effective_style(widget, id, runtime, theme).layout_hash(hasher);
        if let Some(child) = widget.children().first() {
            hash_node(
                child.as_ref(),
                child_id(id, 0, child.as_ref()),
                runtime,
                theme,
                hasher,
            );
        }
        return;
    }
    if widget.scroll_content().is_some()
        || widget.interactive().is_some()
        || widget.fitted().is_some()
        || widget.navigator().is_some()
        || widget.virtual_list().is_some()
        || widget.page_view().is_some()
        || widget.overflow_box().is_some()
        || widget.layout_builder().is_some()
        || widget.stack()
    {
        1u8.hash(hasher);
        effective_style(widget, id, runtime, theme).layout_hash(hasher);
        return;
    }
    if widget.overlay().is_some() {
        2u8.hash(hasher);
        effective_style(widget, id, runtime, theme).layout_hash(hasher);
        let anchor = widget.children()[0].as_ref();
        hash_node(anchor, child_id(id, 0, anchor), runtime, theme, hasher);
        return;
    }
    let children = widget.children();
    3u8.hash(hasher);
    effective_style(widget, id, runtime, theme).layout_hash(hasher);
    // A measured leaf: its **content** (text…) affects the geometry without going
    // through the style — without this fingerprint, two different contents would
    // be conflated and the cache would keep an old layout.
    widget.measure_key().hash(hasher);
    children.len().hash(hasher);
    for (i, child) in children.iter().enumerate() {
        hash_node(
            child.as_ref(),
            child_id(id, i, child.as_ref()),
            runtime,
            theme,
            hasher,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex};

    fn sig<Msg>(w: &dyn Widget<Msg>) -> u64 {
        layout_signature(
            w,
            WidgetId::ROOT,
            &Runtime::default(),
            &crate::theme::Theme::default(),
        )
    }

    #[test]
    fn identical_trees_share_a_signature() {
        let a: Container<()> = Container::new().width(100.0).height(40.0);
        let b: Container<()> = Container::new().width(100.0).height(40.0);
        assert_eq!(sig(&a), sig(&b));
    }

    #[test]
    fn a_size_change_changes_the_signature() {
        let a: Container<()> = Container::new().width(100.0).height(40.0);
        let b: Container<()> = Container::new().width(101.0).height(40.0);
        assert_ne!(sig(&a), sig(&b));
    }

    #[test]
    fn child_count_changes_the_signature() {
        let one: Flex<()> = Flex::row().child(Container::new());
        let two: Flex<()> = Flex::row().child(Container::new()).child(Container::new());
        assert_ne!(sig(&one), sig(&two));
    }

    #[test]
    fn cache_hits_on_repeated_identical_root() {
        let mut cache = LayoutCache::default();
        let rt = Runtime::default();
        let key = WidgetId::ROOT;
        let c = Constraints::definite(Size::new(200.0, 100.0));

        let tree: Container<()> = Container::new().width(200.0).height(100.0);
        let first = cache.rects(key, &tree, &rt, &crate::theme::Theme::default(), c);
        let second = cache.rects(key, &tree, &rt, &crate::theme::Theme::default(), c);
        assert_eq!(first, second, "the same rectangles");
        // 1 miss (computed), then 1 hit (reused).
        assert_eq!((cache.hits, cache.misses), (1, 1));
    }

    #[test]
    fn changed_constraints_miss() {
        let mut cache = LayoutCache::default();
        let rt = Runtime::default();
        let key = WidgetId::ROOT;
        let tree: Container<()> = Container::new();
        let theme = crate::theme::Theme::default();
        cache.rects(
            key,
            &tree,
            &rt,
            &theme,
            Constraints::definite(Size::new(200.0, 100.0)),
        );
        cache.rects(
            key,
            &tree,
            &rt,
            &theme,
            Constraints::definite(Size::new(300.0, 100.0)),
        );
        assert_eq!(
            (cache.hits, cache.misses),
            (0, 2),
            "size changed → recomputed"
        );
    }

    #[test]
    fn end_frame_evicts_untouched_roots() {
        let mut cache = LayoutCache::default();
        let rt = Runtime::default();
        let c = Constraints::definite(Size::new(10.0, 10.0));
        let tree: Container<()> = Container::new();
        cache.rects(
            WidgetId::ROOT,
            &tree,
            &rt,
            &crate::theme::Theme::default(),
            c,
        );
        cache.rects(
            WidgetId::ROOT.child(0),
            &tree,
            &rt,
            &crate::theme::Theme::default(),
            c,
        );
        cache.end_frame();
        assert_eq!(cache.entries.len(), 2);

        // Next frame: only one root touched → the other is evicted.
        cache.rects(
            WidgetId::ROOT,
            &tree,
            &rt,
            &crate::theme::Theme::default(),
            c,
        );
        cache.end_frame();
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key(&WidgetId::ROOT));
    }
}

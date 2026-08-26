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
    /// **Force** the constrained axes rather than merely fill them: a size the content
    /// chose for itself is overruled. What a stack layer pinned to two opposite edges
    /// asks for — see [`frus_layout::Layout::compute_tight`].
    pub tight: bool,
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
            tight: false,
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
            tight: false,
        }
    }

    /// A box the content is **forced** into on the axes that were given, with the others
    /// left free so the content's own size comes back on them.
    pub fn pinned(w: Option<f32>, h: Option<f32>, available: Size) -> Self {
        Self {
            w: w.unwrap_or(available.width),
            h: h.unwrap_or(available.height),
            free_x: w.is_none(),
            free_y: h.is_none(),
            fill: false,
            tight: true,
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
        let (signature, volatile) = signature_of(root, key, runtime, theme);
        if !volatile {
            if let Some(entry) = self.entries.get(&key) {
                if entry.signature == signature && entry.constraints == c {
                    self.hits += 1;
                    return (entry.rects.clone(), entry.overflows.clone());
                }
            }
        }
        self.misses += 1;
        let (rects, overflows) = compute_rects(root, key, runtime, theme, c);
        if volatile {
            // Nothing is stored either: an entry that can never be trusted would only
            // sit there being evicted and re-made.
            self.entries.remove(&key);
            return (rects, overflows);
        }
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
    if c.tight {
        layout.compute_tight(node, c.w, c.h, c.free_x, c.free_y);
    } else if c.fill {
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

/// A 64-bit fingerprint of a subtree's **layout** — styles + structure, following
/// **exactly** the branching of [`build_layout`] (scrollable/navigator/list/stack = a
/// leaf; portal = the anchor alone), with colours, texts and messages excluded because
/// they only affect painting — and whether that fingerprint can be **trusted between
/// frames**.
///
/// A `LayoutBuilder`'s box is the size of what its closure built, and a closure cannot be
/// fingerprinted: two frames whose styles and structure are identical can still want
/// different geometry, because the application changed what it builds. So a root holding
/// one is marked *volatile* and recomputed every frame — the cache's contract is that a
/// hit is **bit-for-bit** what the full computation would have produced, and the only
/// honest way to keep that here is to take the miss.
fn signature_of<Msg>(
    root: &dyn Widget<Msg>,
    id: WidgetId,
    runtime: &Runtime,
    theme: &crate::theme::Theme,
) -> (u64, bool) {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // The **reader's font size**, once, at the root. Every measured box in the tree is
    // resolved against it, so a reader who changes the system setting changes every
    // geometry below — and a fingerprint blind to it would answer the new frame with the
    // old layout and nothing would move.
    frus_core::text_scale().to_bits().hash(&mut hasher);
    let mut volatile = false;
    hash_node(root, id, runtime, theme, &mut hasher, &mut volatile);
    (hasher.finish(), volatile)
}

fn hash_node<Msg, H: Hasher>(
    widget: &dyn Widget<Msg>,
    id: WidgetId,
    runtime: &Runtime,
    theme: &crate::theme::Theme,
    hasher: &mut H,
    volatile: &mut bool,
) {
    // A themed subtree, exactly as `build_layout` sees it: its styles are resolved
    // against its own theme, so the fingerprint is taken against that one too. A
    // fingerprint that skipped the swap would hash one geometry and the cache would
    // store another.
    let scoped = widget.theme_override(theme);
    let theme = scoped.as_deref().unwrap_or(theme);
    // The same for the **surface**: a scoped description changes what the subtree measures
    // with, so it has to be in force here as well — and hashed, or two different surfaces
    // would share one fingerprint and the cache would keep the first one's geometry.
    let surface = widget.media_override(crate::MediaQuery::of());
    let _surface = surface.map(crate::MediaQuery::install);
    if let Some(mq) = surface {
        mq.measure_hash(hasher);
    }
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
                volatile,
            );
        }
        return;
    }
    if widget.scroll_content().is_some()
        || widget.interactive().is_some()
        || widget.fitted().is_some()
        || widget.navigator().is_some()
        // Only *whether*, never *what*: this asks if the widget windows its
        // children at all, and a size it will not read is the honest argument.
        || widget.virtual_list(frus_core::Size::ZERO).is_some()
        || widget.page_view().is_some()
        || widget.overflow_box().is_some()
        || widget.stack()
    {
        1u8.hash(hasher);
        effective_style(widget, id, runtime, theme).layout_hash(hasher);
        return;
    }
    // A `LayoutBuilder` hashes like a leaf and poisons the entry: its style is all there
    // is to hash, and since milestone 355 its style is no longer all there is to its box.
    if widget.layout_builder().is_some() {
        1u8.hash(hasher);
        effective_style(widget, id, runtime, theme).layout_hash(hasher);
        *volatile = true;
        return;
    }
    if widget.overlay().is_some() {
        2u8.hash(hasher);
        effective_style(widget, id, runtime, theme).layout_hash(hasher);
        let anchor = widget.children()[0].as_ref();
        hash_node(
            anchor,
            child_id(id, 0, anchor),
            runtime,
            theme,
            hasher,
            volatile,
        );
        return;
    }
    let children = widget.children();
    3u8.hash(hasher);
    effective_style(widget, id, runtime, theme).layout_hash(hasher);
    // A measured leaf: its **content** (text…) affects the geometry without going
    // through the style — without this fingerprint, two different contents would
    // be conflated and the cache would keep an old layout.
    widget.measure_key(theme).hash(hasher);
    // Filling the parent is resolved during the walk rather than written into the style,
    // so the style hash alone would conflate a run that fills with one that shrink-wraps.
    let fills = widget.fill_axes(theme);
    (fills.horizontal, fills.vertical).hash(hasher);
    widget.main_axis_floor(theme).map(f32::to_bits).hash(hasher);
    widget.tile_shape().map(f32::to_bits).hash(hasher);
    children.len().hash(hasher);
    for (i, child) in children.iter().enumerate() {
        hash_node(
            child.as_ref(),
            child_id(id, i, child.as_ref()),
            runtime,
            theme,
            hasher,
            volatile,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex};

    fn sig<Msg>(w: &dyn Widget<Msg>) -> u64 {
        signature_of(
            w,
            WidgetId::ROOT,
            &Runtime::default(),
            &crate::theme::Theme::default(),
        )
        .0
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

    /// Filling the parent is resolved during the walk and never reaches the style, so
    /// the fingerprint has to ask for it separately — otherwise a row that fills and one
    /// that shrink-wraps are the same tree as far as the cache is concerned, and the
    /// second one silently gets the first one's layout.
    #[test]
    fn a_change_of_main_axis_size_changes_the_signature() {
        let fills: crate::Row<()> = crate::Row::new().child(Container::new());
        let hugs: crate::Row<()> = crate::Row::new().shrink_wrap().child(Container::new());
        assert_ne!(sig(&fills), sig(&hugs));
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

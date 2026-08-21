//! The driver: walks the tree carrying a context (translation + clip), produces the
//! [`Scene`] and the hit-test maps (click, focus, scroll), and makes it possible to find a
//! widget by identity so that keyboard/edit events can be routed to it.

use std::hash::{Hash, Hasher};

use frus_core::{
    Affine, ClipShape, Color, FontWeight, LayerFilter, LayerTransform, Path, Point, Primitive,
    Rect, Scene, Size, TextStyle,
};
use frus_layout::{Layout, NodeId, Overflowing, Side};

use crate::shortcuts::{Intent, KeyStroke};

use crate::barrier::ModalBarrier;
use crate::dismiss::{DismissPhase, Dismissable};
use crate::dragdrop::{DragSource, DropZone};
use crate::hero::{lerp_rect, HeroSpot};
use crate::interaction::{Status, WidgetId};
use crate::pageview::PageSnap;
use crate::physics::{ScrollMetrics, ScrollPhysics};
use crate::portal::Placement;
use crate::positioned::Positioning;
use crate::refresh::Refreshable;
use crate::relayout::Constraints;
use crate::runtime::Runtime;
use crate::theme::Theme;
use crate::widget::Widget;

/// Parallax factor of the screen behind during a transition (0 = fixed, 1 = follows
/// exactly). It is what gives a native navigation its depth.
const NAV_PARALLAX: f32 = 0.3;

/// Thickness of a scrollbar, in pixels.
const BAR_SIZE: f32 = 10.0;
/// Minimum length of a thumb.
const MIN_THUMB: f32 = 28.0;

/// A scrollbar thumb (for hit-testing a drag).
#[derive(Copy, Clone, Debug)]
pub struct Scrollbar {
    pub id: WidgetId,
    pub vertical: bool,
    pub thumb: Rect,
    /// Start and length of the track, along the axis.
    pub track_start: f32,
    pub track_len: f32,
    pub thumb_len: f32,
    /// Offset maximal correspondant.
    pub max: f32,
    /// The offsets of a **reversed** axis run the other way along the track: offset 0
    /// is the far end, so the thumb rests there and a drag towards the start raises the
    /// number. See [`Scrollable::reverse_y`].
    pub reverse: bool,
}

/// A scrollable area of the frame: where it is, how far it may scroll, and how it
/// is meant to behave at its edges.
///
/// The registry the shell and the runtime both read. It carries the area's own
/// [`ScrollPhysics`] only when the widget asked for one — resolving the default is
/// the application's job, through [`Scrollable::physics_or`].
#[derive(Copy, Clone, Debug)]
pub struct Scrollable {
    pub id: WidgetId,
    /// The viewport, in absolute coordinates.
    pub viewport: Rect,
    /// Largest horizontal offset the content may rest at (0 = nothing to scroll).
    pub max_x: f32,
    /// Largest vertical offset the content may rest at.
    pub max_y: f32,
    /// The area's own physics, when it asked for one.
    pub physics: Option<ScrollPhysics>,
    /// The [`crate::RefreshIndicator`] area this scrollable sits inside, when there is one.
    /// Movement refused at its **top** edge feeds that area's pull instead of the
    /// overscroll glow.
    pub refresh: Option<WidgetId>,
    /// Set when this area rests **only on pages** ([`crate::PageView`]): the release
    /// springs to a page boundary instead of flinging.
    pub page: Option<PageSnap>,
    /// Whether the horizontal axis is **reversed** — offset 0 is the right-hand end of
    /// the content rather than the left.
    pub reverse_x: bool,
    /// Whether the vertical axis is **reversed** — offset 0 is the **bottom** of the
    /// content rather than the top.
    ///
    /// Offsets are measured from the end an axis starts at, which is what makes a
    /// reversed list stay where it is when content arrives: a conversation resting at
    /// offset 0 is resting at the newest message, and the newest message is wherever the
    /// end now happens to be. Measuring from the top instead would leave the view
    /// drifting away from the end every time something was appended, which is the one
    /// thing this exists to prevent.
    ///
    /// Everything the *user* does is unchanged by it. A finger pushes the content the
    /// way it moves, in either direction; only the arithmetic between that push and the
    /// number changes sign, in one place.
    pub reverse_y: bool,
}

impl Scrollable {
    /// This area's physics, falling back to the application's choice.
    pub fn physics_or(&self, default: ScrollPhysics) -> ScrollPhysics {
        self.physics.unwrap_or(default)
    }

    /// Whether a finger may move this area at all, resting at `offset`.
    ///
    /// The rule the reference states plainly: the user can change the offset **if, and
    /// only if, there is content outside the viewport to reveal** — or the content is
    /// already displaced, having shrunk under an offset it no longer reaches. An area that
    /// fails this takes no drag at all: not a short one, not a refused one, and so no
    /// end-of-content glow either. An edge effect where there is no edge to meet is a
    /// statement about the content that is not true.
    ///
    /// A [`crate::RefreshIndicator`] listening above is the exception, and the same one the
    /// reference makes: a list of two items must still pull down to reload.
    pub fn accepts_user_offset(&self, offset: (f32, f32)) -> bool {
        self.max_x > 0.0
            || self.max_y > 0.0
            || offset.0 != 0.0
            || offset.1 != 0.0
            || self.refresh.is_some()
    }

    /// The horizontal axis as the physics sees it, at `offset`.
    pub fn metrics_x(&self, offset: f32) -> ScrollMetrics {
        ScrollMetrics::new(offset, self.max_x, self.viewport.width)
    }

    /// The vertical axis as the physics sees it, at `offset`.
    pub fn metrics_y(&self, offset: f32) -> ScrollMetrics {
        ScrollMetrics::new(offset, self.max_y, self.viewport.height)
    }

    /// Turns a **screen** delta — how far the content was pushed, in pixels — into the
    /// change it makes to this area's offsets.
    ///
    /// Two sign changes, and both have a reason. The content moves *opposite* the
    /// number: dragging down reveals what is above, which is a smaller offset. And a
    /// reversed axis counts from the other end, so the same drag raises it instead.
    ///
    /// One function rather than a minus sign at each of the five places a delta becomes
    /// an offset — the wheel, the drag, the release fling, and both axes of each — so
    /// that a reversed scroll is right in all of them or in none.
    pub fn offset_delta(&self, screen: (f32, f32)) -> (f32, f32) {
        (
            if self.reverse_x { screen.0 } else { -screen.0 },
            if self.reverse_y { screen.1 } else { -screen.1 },
        )
    }

    /// The edge a refusal happened at, given the refused movement **in offset space**.
    ///
    /// A negative refusal means the content was pulled towards the axis's start, and
    /// where that is on screen is exactly what reversing changes: the start of a
    /// reversed vertical axis is the **bottom**. Without this the glow would flash at
    /// the far end of a conversation the moment it refused to go further back.
    pub fn refused_edge(&self, vertical: bool, refused: f32) -> crate::overscroll::GlowEdge {
        let reversed = if vertical {
            self.reverse_y
        } else {
            self.reverse_x
        };
        crate::overscroll::edge_for(vertical, if reversed { -refused } else { refused })
    }

    /// The edge an axis **starts** at, which is where a pull-to-refresh listens.
    pub fn start_edge(&self, vertical: bool) -> crate::overscroll::GlowEdge {
        self.refused_edge(vertical, -1.0)
    }

    /// Where the content's leading edge sits, relative to the viewport's, at `offset`.
    ///
    /// Normally minus the offset. On a reversed axis the content is anchored to the far
    /// end, so offset 0 puts the content's *end* against the viewport's — which is also
    /// the answer for content too short to scroll, where it sits at the bottom of the
    /// box rather than the top.
    pub fn content_origin(&self, offset: (f32, f32), content: Size) -> (f32, f32) {
        let axis = |reverse: bool, offset: f32, viewport: f32, content: f32| {
            if reverse {
                viewport - content + offset
            } else {
                -offset
            }
        };
        (
            axis(self.reverse_x, offset.0, self.viewport.width, content.width),
            axis(
                self.reverse_y,
                offset.1,
                self.viewport.height,
                content.height,
            ),
        )
    }
}

/// Direction of arrow-key focus navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone)]
struct Hit<Msg> {
    id: WidgetId,
    rect: Rect,
    /// The message the target emits, or `None` for a target that **swallows** input
    /// without doing anything — an [`crate::AbsorbPointer`]. Such a target still wins
    /// the hit test, which is exactly what stops the click reaching what is behind it.
    msg: Option<Msg>,
    /// **Inverse** transform (screen → the flat frame) of a transformed subtree
    /// (`Transform::scale`/`rotate`/a composition): the test point is passed through it to
    /// get back to the untransformed frame where `rect` lives. `None` = an axis-aligned,
    /// untransformed rect (the common case).
    xform: Option<Affine>,
}

impl<Msg> Hit<Msg> {
    /// `true` when `point` (in screen coordinates) falls inside the target, taking any
    /// transform on the subtree into account.
    fn contains(&self, point: Point) -> bool {
        let p = self.xform.map_or(point, |inv| inv.apply(point));
        self.rect.contains(p)
    }
}

/// The painted output of a repaint-boundary subtree, cached (see `paintcache.rs`) and
/// replayed as is on a *hit*. Erased behind a `Box<dyn Any>` in the cache; brought back down
/// to `Msg` here (one `Msg` instance per app → the `downcast` always succeeds).
#[derive(Clone)]
struct BoundaryData<Msg> {
    prims: Vec<frus_core::Primitive>,
    hits: Vec<Hit<Msg>>,
    long_presses: Vec<Hit<Msg>>,
    dismisses: Vec<Msg>,
    focusables: Vec<Focusable>,
    scrollables: Vec<Scrollable>,
    draggables: Vec<(WidgetId, Rect)>,
    drag_sources: Vec<DragSource>,
    drop_zones: Vec<DropZone>,
    inks: Vec<(WidgetId, Rect)>,
    semantics: Vec<(WidgetId, Rect, frus_core::Semantics)>,
}

/// Lengths of the builder's collections on entering a boundary: the lower bounds of the
/// slices to capture on the way out.
struct Snapshot {
    scene: usize,
    hits: usize,
    long_presses: usize,
    dismisses: usize,
    focusables: usize,
    scrollables: usize,
    draggables: usize,
    drag_sources: usize,
    drop_zones: usize,
    inks: usize,
    semantics: usize,
    overlays: usize,
    focus_scope_start: Option<usize>,
}

/// Lower bounds of the **interaction** registries at the start of a transformed composition
/// (a scale/rotation layer): only what the subtree has just added is re-mapped. Distinct from
/// [`Snapshot`] (the boundary cache) because it **includes `reorderables`** — those are never
/// cached but do have to be transformed. See [`transform_interaction_registries`].
/// Lengths of the registries a [`ModalBarrier`] withholds from, on entering its subtree: the
/// point each is truncated back to on the way out. Distinct from [`XformBase`] because a
/// barrier also covers the **scene** and the **scrollbars**, and distinct from [`Snapshot`]
/// because it covers the registries that are never cached.
struct BarrierBase {
    scene: usize,
    hits: usize,
    long_presses: usize,
    focusables: usize,
    scrollables: usize,
    scrollbars: usize,
    draggables: usize,
    drag_sources: usize,
    drop_zones: usize,
    inks: usize,
    reorderables: usize,
    interactives: usize,
    semantics: usize,
}

struct XformBase {
    hits: usize,
    long_presses: usize,
    focusables: usize,
    scrollables: usize,
    draggables: usize,
    drag_sources: usize,
    drop_zones: usize,
    inks: usize,
    reorderables: usize,
    semantics: usize,
}

/// Node count of the subtree **if** it is "plain" — that is, if the boundary and **all** its
/// descendants take the default walk branch (children in prefix order): not scrollable, not a
/// navigator, not a virtualised list, no `layout_builder`, not a stack, no overlay, no
/// continuous animation. The count then follows the walk's rect order **exactly**, which is
/// what makes the fingerprint and the bit-for-bit replay correct. `None` = a subtree that
/// cannot be cached (it is repainted in full — the safe fallback).
fn plain_subtree_len<Msg>(widget: &dyn Widget<Msg>) -> Option<usize> {
    if widget.continuous()
        || widget.scroll_content().is_some()
        || widget.interactive().is_some()
        || widget.fitted().is_some()
        || widget.rotated_quarter_turns().is_some()
        || widget.navigator().is_some()
        || widget.virtual_list().is_some()
        || widget.page_view().is_some()
        || widget.overflow_box().is_some()
        || widget.layout_builder().is_some()
        || widget.stack()
        || widget.overlay().is_some()
        // A reorderable (a Kanban card, a draggable header): its bounds feed the reorderables
        // registry, which is not cached — so its subtree is not put in the paint cache.
        || widget.reorder_index().is_some()
    {
        return None;
    }
    let mut n = 1;
    for child in widget.children() {
        n += plain_subtree_len(child.as_ref())?;
    }
    Some(n)
}

/// Quantises a `[0,1]` float for the fingerprint (independent of tiny binary differences: two
/// visually identical progresses ⇒ the same fingerprint).
fn quant(x: f32) -> i32 {
    (x * 4096.0).round() as i32
}

/// Adds to `h` everything the paint reads from a `Status`, **except** the time (excluded: a
/// cacheable boundary holds no `continuous` widget).
fn hash_status<H: Hasher>(s: &Status, h: &mut H) {
    (s.interaction as u8).hash(h);
    s.focused.hash(h);
    s.cursor.hash(h);
    s.selection.hash(h);
    s.composing.hash(h);
    quant(s.hover_progress).hash(h);
    quant(s.focus_progress).hash(h);
    quant(s.opacity).hash(h);
    quant(s.value).hash(h);
    // Sub-region hover (milestone 208): a change of position repaints the highlight.
    s.hover_cursor.map(|p| (quant(p.x), quant(p.y))).hash(h);
}

/// The result of building an interface for one frame.
/// A **shortcut / action scope**: a subtree that binds keystrokes, answers intents, or
/// both, together with the range of focus stops it contains.
///
/// The range is how "focus is inside this subtree" is answered without an ancestor test:
/// the walk is depth-first, so a subtree's focus stops are contiguous, and the focused
/// stop's index either falls inside the range or does not.
struct Scope<Msg> {
    strokes: Vec<(KeyStroke, Intent)>,
    callbacks: Vec<(KeyStroke, Msg)>,
    actions: Vec<(Intent, Msg)>,
    listeners: Vec<(Intent, Msg)>,
    /// The focus stops this subtree contains, as indices into `focusables`.
    range: std::ops::Range<usize>,
}

/// A **focus stop**: somewhere the keyboard can land, and what the traversal should make
/// of it.
#[derive(Clone, Copy, Debug)]
pub struct Focusable {
    /// The widget's identity.
    pub id: WidgetId,
    /// Its visible box this frame.
    pub rect: Rect,
    /// Focusable by a click, but **passed over by Tab** — the reference's
    /// `ExcludeFocusTraversal`. The two are separate questions.
    pub skip: bool,
    /// An explicit traversal position, smallest first. `None` means tree order, which is
    /// where everything sits until someone says otherwise.
    pub order: Option<f32>,
    /// The traversal group it belongs to, within which its order is resolved.
    pub group: Option<WidgetId>,
}

pub struct Ui<Msg> {
    scene: Scene,
    hits: Vec<Hit<Msg>>,
    /// Long-press targets (id, visible bounds, message).
    long_presses: Vec<Hit<Msg>>,
    /// Overlay dismissal messages, from the bottom to the **top**.
    dismisses: Vec<Msg>,
    focusables: Vec<Focusable>,
    /// **Focus scope**: index of the topmost modal overlay's first focusable —
    /// Tab/arrows/click-to-focus are trapped from there on (`None` = no modal, every
    /// focusable takes part).
    focus_scope_start: Option<usize>,
    /// (id, viewport, offset max x, offset max y)
    scrollables: Vec<Scrollable>,
    scrollbars: Vec<Scrollbar>,
    draggables: Vec<(WidgetId, Rect)>,
    drag_sources: Vec<DragSource>,
    drop_zones: Vec<DropZone>,
    /// **Inked surfaces**: (id, the box the ink is clipped to). Tracked separately from
    /// clicking because the splash needs the surface's **whole** box — where the finger
    /// landed inside it, and how far the circle has to travel to cover it — which a
    /// click target, recorded as its *visible* part, does not give.
    inks: Vec<(WidgetId, Rect)>,
    /// **Reorderables** (column headers, Kanban cards): (id, visible bounds). Tracked
    /// independently of clicking — a card is not clickable but can still be picked up and
    /// dropped onto.
    reorderables: Vec<(WidgetId, Rect)>,
    /// Interactive viewports (`InteractiveViewer`): (id, the screen viewport). The shell
    /// routes panning (dragging) and zooming (wheel / pinch) to them.
    interactives: Vec<(WidgetId, Rect)>,
    /// Pull-to-refresh areas of the frame, with their configuration.
    refreshes: Vec<Refreshable>,
    /// Swipe-to-dismiss items of the frame, with their configuration.
    dismissables: Vec<Dismissable>,
    wants_animation: bool,
    /// The accessibility tree: semantic nodes (id, bounds, annotation), in paint order. The
    /// shell maps it onto AccessKit.
    semantics: Vec<(WidgetId, Rect, frus_core::Semantics)>,
    /// Boxes whose children ran outside them, with the edge and the amount.
    overflows: Vec<Overflowing>,
    /// Shortcut and action scopes, in the order the walk closed them — innermost first
    /// among any that overlap, which is the order the resolution wants.
    scopes: Vec<Scope<Msg>>,
    /// Keystroke listeners, with the focus stops they cover.
    #[allow(clippy::type_complexity)]
    listeners: Vec<(
        std::ops::Range<usize>,
        std::rc::Rc<dyn Fn(KeyStroke) -> Option<Msg>>,
    )>,
}

impl<Msg: Clone> Ui<Msg> {
    /// The scene to hand to the renderer.
    /// Boxes in this frame whose children did not fit inside them.
    ///
    /// Empty is the normal answer. A non-empty one means content is being drawn outside
    /// its parent — invisible where it is clipped, unhittable where it leaves the window,
    /// and silent either way until something asks. That silence is what let a task row's
    /// delete button sit off-screen through three milestones.
    pub fn overflows(&self) -> &[Overflowing] {
        &self.overflows
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// The frame's **accessibility tree**: every meaningful node with its bounds on screen and
    /// its annotation ([`frus_core::Semantics`]). Paint order (= reading order). The shell
    /// pushes it to AccessKit.
    pub fn semantics(&self) -> &[(WidgetId, Rect, frus_core::Semantics)] {
        &self.semantics
    }

    /// `true` when a widget animates continuously (the framework must redraw).
    pub fn wants_animation(&self) -> bool {
        self.wants_animation
    }

    /// Identity of the topmost clickable widget containing `point`.
    pub fn hit(&self, point: Point) -> Option<WidgetId> {
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.contains(point))
            .map(|hit| hit.id)
    }

    /// The message tied to a given clickable widget. `None` for a target that swallows
    /// input without emitting anything.
    pub fn msg_for(&self, id: WidgetId) -> Option<Msg> {
        self.hits
            .iter()
            .find(|hit| hit.id == id)
            .and_then(|hit| hit.msg.clone())
    }

    /// Dismissal message of the **topmost** overlay (for Escape).
    pub fn top_dismiss(&self) -> Option<Msg> {
        self.dismisses.last().cloned()
    }

    /// **Long-press** message of the topmost target containing `point`.
    pub fn long_press_at(&self, point: Point) -> Option<Msg> {
        self.long_presses
            .iter()
            .rev()
            .find(|hit| hit.contains(point))
            .and_then(|hit| hit.msg.clone())
    }

    /// The frame's **pull-to-refresh areas**, with the configuration each was built
    /// with. The shell steps them and reads their `refreshing` flag from here.
    pub fn refresh_areas(&self) -> &[Refreshable] {
        &self.refreshes
    }

    /// The frame's **dismissible items**, with the configuration each was built with.
    pub fn dismissables(&self) -> &[Dismissable] {
        &self.dismissables
    }

    /// The topmost dismissible item containing `point` — the candidate a press has to
    /// weigh against the scrollable underneath it.
    pub fn dismissable_at(&self, point: Point) -> Option<Dismissable> {
        self.dismissables
            .iter()
            .rev()
            .find(|item| item.rect.contains(point))
            .copied()
    }

    /// Topmost focusable widget containing `point`: (id, its bounds).
    pub fn focus_hit(&self, point: Point) -> Option<(WidgetId, Rect)> {
        self.focus_pool()
            .iter()
            .rev()
            .find(|f| f.rect.contains(point))
            .map(|f| (f.id, f.rect))
    }

    /// The **participating** focusables: those of the topmost modal scope when there is one
    /// (a focus trap), otherwise all of them.
    fn focus_pool(&self) -> &[Focusable] {
        match self.focus_scope_start {
            Some(start) => &self.focusables[start.min(self.focusables.len())..],
            None => &self.focusables,
        }
    }

    /// Nearest focusable in the given **direction** (arrow navigation, a geometric policy):
    /// among the focusables whose centre lies on the right side, it minimises the distance
    /// along the main axis with a penalty on the cross-axis offset. `None` when there is
    /// nothing in that direction.
    pub fn focus_directional(
        &self,
        current: WidgetId,
        direction: FocusDirection,
    ) -> Option<WidgetId> {
        // The starting point may be outside the scope (focus from before it opened); the
        // **candidates**, though, are trapped inside it.
        let from = self
            .focusables
            .iter()
            .find(|f| f.id == current)
            .map(|f| &f.rect)?;
        let center = |r: &Rect| (r.x + r.width * 0.5, r.y + r.height * 0.5);
        let (fx, fy) = center(from);

        let mut best: Option<(WidgetId, f32)> = None;
        for candidate in self.focus_pool() {
            let (id, rect) = (candidate.id, &candidate.rect);
            if id == current {
                continue;
            }
            let (cx, cy) = center(rect);
            // (how far ahead in the direction, the cross-axis offset)
            let (ahead, cross) = match direction {
                FocusDirection::Right => (cx - fx, (cy - fy).abs()),
                FocusDirection::Left => (fx - cx, (cy - fy).abs()),
                FocusDirection::Down => (cy - fy, (cx - fx).abs()),
                FocusDirection::Up => (fy - cy, (cx - fx).abs()),
            };
            // Inside a **cone** around the direction (not a plain half-plane): a candidate
            // almost aligned on the cross axis but barely "ahead" (slightly different widths)
            // is not a directional target.
            if ahead <= 0.5 || cross > ahead * 3.0 {
                continue;
            }
            let score = ahead + cross * 3.0;
            if best.map(|(_, s)| score < s).unwrap_or(true) {
                best = Some((id, score));
            }
        }
        best.map(|(id, _)| id)
    }

    /// Next/previous focus stop for Tab, wrapping. With no current focus, the first (or
    /// the last).
    ///
    /// **Tree order, unless something said otherwise.** Stops carrying an explicit order
    /// are sorted ahead of those that do not, smallest first, and the sort is applied
    /// **within a traversal group** rather than across the whole frame — so a reordered
    /// dialog does not reshuffle the page behind it. Stops marked `skip` are passed over
    /// here while remaining reachable by a click: the keyboard's order and the pointer's
    /// reach are two different questions.
    pub fn focus_next(&self, current: Option<WidgetId>, forward: bool) -> Option<WidgetId> {
        let order = self.traversal_order();
        if order.is_empty() {
            return None;
        }
        let n = order.len();
        // A current focus **outside the scope** (taken before the modal opened) is treated
        // as "none": Tab enters the trap.
        match current.and_then(|c| order.iter().position(|id| *id == c)) {
            Some(i) => {
                let j = if forward {
                    (i + 1) % n
                } else {
                    (i + n - 1) % n
                };
                Some(order[j])
            }
            None => Some(order[if forward { 0 } else { n - 1 }]),
        }
    }

    /// The Tab order: the participating stops, minus those that asked to be skipped,
    /// with each traversal group's members sorted among themselves.
    ///
    /// A **stable** sort on the order key, so everything without one keeps tree order and
    /// ties do too — the property that makes an explicit order a local statement rather
    /// than a rearrangement of the whole frame.
    pub fn traversal_order(&self) -> Vec<WidgetId> {
        let pool: Vec<&Focusable> = self.focus_pool().iter().filter(|f| !f.skip).collect();
        if pool.iter().all(|f| f.order.is_none()) {
            return pool.iter().map(|f| f.id).collect();
        }
        let mut out: Vec<WidgetId> = Vec::with_capacity(pool.len());
        let mut i = 0;
        while i < pool.len() {
            // A group's members are contiguous: the walk is depth-first, so a subtree's
            // stops arrive together.
            let group = pool[i].group;
            let mut j = i;
            while j < pool.len() && pool[j].group == group {
                j += 1;
            }
            let mut run: Vec<&Focusable> = pool[i..j].to_vec();
            run.sort_by(|a, b| {
                let key = |f: &Focusable| f.order.unwrap_or(f32::INFINITY);
                key(a).total_cmp(&key(b))
            });
            out.extend(run.iter().map(|f| f.id));
            i = j;
        }
        out
    }

    /// What a keystroke means here: the messages it sends, in the order they should be
    /// applied.
    ///
    /// The resolution, in one place because it is the whole feature:
    ///
    /// 1. Only scopes **containing the focused stop** are candidates; with nothing
    ///    focused, all of them are. Innermost first — `scopes` is filled on the way *out*
    ///    of the walk, so that order is already the one wanted.
    /// 2. A [`crate::KeyboardListener`] gets the first refusal, and a `None` from it lets
    ///    the stroke carry on.
    /// 3. A direct binding ([`crate::CallbackShortcuts`]) sends its message.
    /// 4. Otherwise a [`crate::Shortcuts`] binding names an **intent**, and the innermost
    ///    [`crate::Actions`] that answers it supplies the message. An intent nobody
    ///    answers does nothing — deliberately: a key bound to a meaning the current screen
    ///    has no answer for should be inert, not an error.
    /// 5. Every [`crate::ActionListener`] watching that intent adds its message too.
    ///
    /// Empty means *nobody wanted it*, and the caller should carry on to whatever it would
    /// have done with the key.
    pub fn keystroke(&self, stroke: KeyStroke, focused: Option<WidgetId>) -> Vec<Msg> {
        let at = focused.and_then(|id| self.focusables.iter().position(|f| f.id == id));
        let covers = |range: &std::ops::Range<usize>| match at {
            Some(i) => range.contains(&i),
            None => true,
        };

        for (range, on_key) in self.listeners.iter().rev() {
            if covers(range) {
                if let Some(msg) = on_key(stroke) {
                    return vec![msg];
                }
            }
        }

        let mut out = Vec::new();
        for scope in self.scopes.iter().filter(|s| covers(&s.range)) {
            if let Some((_, msg)) = scope.callbacks.iter().find(|(s, _)| stroke.matches(s)) {
                out.push(msg.clone());
                return out;
            }
            let Some((_, intent)) = scope.strokes.iter().find(|(s, _)| stroke.matches(s)) else {
                continue;
            };
            let answer = self
                .scopes
                .iter()
                .filter(|s| covers(&s.range))
                .find_map(|s| {
                    s.actions
                        .iter()
                        .find(|(i, _)| i == intent)
                        .map(|(_, m)| m.clone())
                });
            if let Some(msg) = answer {
                out.push(msg);
                for watcher in self.scopes.iter().filter(|s| covers(&s.range)) {
                    for (i, m) in &watcher.listeners {
                        if i == intent {
                            out.push(m.clone());
                        }
                    }
                }
            }
            return out;
        }
        out
    }

    /// Topmost scrollable area containing `point`.
    pub fn scroll_hit(&self, point: Point) -> Option<Scrollable> {
        self.scrollables
            .iter()
            .rev()
            .find(|area| area.viewport.contains(point))
            .copied()
    }

    /// **Every** scrollable area containing `point`, innermost first — the whole stack a
    /// finger has under it.
    ///
    /// [`Self::scroll_hit`] answers *which one is on top*, which is the right question for
    /// a wheel or a scrollbar. A finger asks a different one: a strip that only slides
    /// across, sitting in a page that only scrolls down, cannot take a drag downwards, and
    /// the page behind it can. Deciding that needs the ones behind as well.
    pub fn scroll_chain(&self, point: Point) -> impl Iterator<Item = Scrollable> + '_ {
        self.scrollables
            .iter()
            .rev()
            .filter(move |area| area.viewport.contains(point))
            .copied()
    }

    /// Frame of a **focusable** widget `id`, when it is present this frame — so the shell can
    /// find the focused field's geometry (vertical caret movement).
    pub fn widget_rect(&self, id: WidgetId) -> Option<Rect> {
        self.focusables
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.rect)
            // Falls back to the **reorderables** registry. Kanban cards (and headers that are
            // reorderable but **not** sortable) are not focusable: without this fallback the
            // shell cannot find their bounds, and the whole **vertical** drag preview (ghost,
            // insertion line, reflow) as well as the *insert-after* routing stay dead.
            .or_else(|| {
                self.reorderables
                    .iter()
                    .find(|(rid, _)| *rid == id)
                    .map(|(_, rect)| *rect)
            })
    }

    /// The box of the inked surface `id`, when the frame has one — what the shell needs
    /// to start a splash: where inside it the finger landed, and how big it is.
    ///
    /// The **topmost** match wins, as the hit-test does: two inked surfaces can overlap,
    /// and the one that took the tap is the one drawn last.
    pub fn ink_box(&self, id: WidgetId) -> Option<Rect> {
        self.inks
            .iter()
            .rev()
            .find(|(iid, _)| *iid == id)
            .map(|(_, rect)| *rect)
    }

    /// The identities of **every** focusable widget of the frame (the scope included) — so
    /// the shell can detect the focus **disappearing** (an overlay closed) and restore it.
    pub fn focusable_ids(&self) -> impl Iterator<Item = WidgetId> + '_ {
        self.focusables.iter().map(|f| f.id)
    }

    /// Frame (viewport) of the scrollable area `id`, when there is one — so the shell can find
    /// a multi-line field's width/height (caret following).
    pub fn scrollable_viewport(&self, id: WidgetId) -> Option<Rect> {
        self.scrollables
            .iter()
            .find(|area| area.id == id)
            .map(|area| area.viewport)
    }

    /// Every scrollable area of the frame — geometry, bounds and physics — which is
    /// what the runtime needs to drive momentum and edges.
    pub fn scroll_regions(&self) -> &[Scrollable] {
        &self.scrollables
    }

    /// The area `id`, when it is present this frame.
    pub fn scroll_region(&self, id: WidgetId) -> Option<Scrollable> {
        self.scrollables.iter().find(|area| area.id == id).copied()
    }

    /// SingleChildScrollView bounds `(id, max_x, max_y)` of every scrollable area.
    pub fn scrollable_maxes(&self) -> Vec<(WidgetId, f32, f32)> {
        self.scrollables
            .iter()
            .map(|area| (area.id, area.max_x, area.max_y))
            .collect()
    }

    /// Scrollbar thumb under `point` (to start a drag).
    pub fn scrollbar_at(&self, point: Point) -> Option<Scrollbar> {
        self.scrollbars
            .iter()
            .rev()
            .find(|bar| bar.thumb.contains(point))
            .copied()
    }

    /// Topmost draggable widget under `point`: (id, its bounds).
    pub fn draggable_at(&self, point: Point) -> Option<(WidgetId, Rect)> {
        self.draggables
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(point))
            .map(|(id, rect)| (*id, *rect))
    }

    /// Topmost **drag source** under `point`, with what it carries.
    pub fn drag_source_at(&self, point: Point) -> Option<DragSource> {
        self.drag_sources
            .iter()
            .rev()
            .find(|source| source.rect.contains(point))
            .copied()
    }

    /// Topmost **drop target** under `point`.
    pub fn drop_zone_at(&self, point: Point) -> Option<DropZone> {
        self.drop_zones
            .iter()
            .rev()
            .find(|zone| zone.rect.contains(point))
            .copied()
    }

    /// Topmost **reorderable** widget under `point`: its id. The basis of reordering
    /// drag-and-drop (the source on press, the target on drop) — independent of clickability.
    pub fn reorderable_at(&self, point: Point) -> Option<WidgetId> {
        self.reorderables
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(point))
            .map(|(id, _)| *id)
    }

    /// Topmost **interactive** viewport (`InteractiveViewer`) under `point`: (id, its screen
    /// viewport). The shell routes panning and zooming to it.
    pub fn interactive_at(&self, point: Point) -> Option<(WidgetId, Rect)> {
        self.interactives
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(point))
            .map(|(id, rect)| (*id, *rect))
    }

    /// Interactive viewports `(id, screen viewport)`, to drive the inertia (fling) and the
    /// clamping of the pan on the framework's side.
    pub fn interactive_bounds(&self) -> Vec<(WidgetId, Rect)> {
        self.interactives.clone()
    }
}

/// Paints a **striped band** along the edge of every box whose children ran past it.
///
/// Milestone 335 taught the layout to notice an overflow and the shell to say so in the
/// console. That is the half a developer reads; this is the half a *screenshot* shows —
/// and a screenshot is what a bug report is made of. Three of this framework's own
/// milestones went looking for defects that a band would have pointed at: a delete button
/// laid out past the card (333, 334), a segmented control 221 px outside its parent (335).
///
/// The look is the reference's, because the point of it is to be recognised on sight:
/// black and yellow diagonal stripes, three quarters opaque, over a tenth of the box along
/// the offending edge, with the edge and the number written across it. Not the mechanism,
/// though — the reference paints one repeating gradient and this paints the stripes,
/// because a repeating diagonal gradient is a shader feature and a parallelogram is four
/// points.
///
/// **Debug builds only.** A band is a message to whoever is building the application, and
/// in a release build there is nobody to read it — the reference draws the same line.
fn paint_overflow_bands(scene: &mut Scene, overflows: &[Overflowing]) {
    if !cfg!(debug_assertions) || overflows.is_empty() {
        return;
    }
    // The reference's two colours and its 10 % band, exactly.
    const BLACK: Color = Color::rgba(0.0, 0.0, 0.0, 0.75);
    const YELLOW: Color = Color::rgba(1.0, 1.0, 0.0, 0.75);
    const FRACTION: f32 = 0.1;
    /// The stripes run at 45°; this is their period measured along the x axis.
    const PERIOD: f32 = 14.0;

    let (outer, outer_bounds) = (scene.current_clip(), scene.current_bounds());
    scene.set_clip(Rect::UNBOUNDED);
    for over in overflows {
        let r = over.rect;
        let band = match over.side {
            Side::Left => Rect::new(r.x, r.y, r.width * FRACTION, r.height),
            Side::Right => Rect::new(
                r.x + r.width * (1.0 - FRACTION),
                r.y,
                r.width * FRACTION,
                r.height,
            ),
            Side::Top => Rect::new(r.x, r.y, r.width, r.height * FRACTION),
            Side::Bottom => Rect::new(
                r.x,
                r.y + r.height * (1.0 - FRACTION),
                r.width,
                r.height * FRACTION,
            ),
        };
        if band.width <= 0.0 || band.height <= 0.0 {
            continue;
        }
        scene.set_clip(band);
        scene.fill_rect(band, BLACK);
        // One parallelogram per stripe, sheared by the band's height so that every one
        // of them crosses it at 45° whatever shape the band is.
        let shear = band.height;
        let mut x = band.x - shear;
        while x < band.x + band.width {
            let stripe = Path::new()
                .move_to(Point::new(x, band.y))
                .line_to(Point::new(x + PERIOD / 2.0, band.y))
                .line_to(Point::new(x + PERIOD / 2.0 + shear, band.y + band.height))
                .line_to(Point::new(x + shear, band.y + band.height))
                .close();
            scene.fill_path(&stripe, YELLOW);
            x += PERIOD;
        }
        // The label is **not** clipped to the band: a vertical one is usually longer than
        // the box is tall, and half a sentence is worse than a sentence written over
        // whatever is beside it. The reference does not clip it either.
        scene.set_clip(Rect::UNBOUNDED);
        scene.set_bounds(Rect::UNBOUNDED);
        paint_overflow_label(scene, band, over);
    }
    scene.set_clip(outer);
    scene.set_bounds(outer_bounds);
}

/// Writes the reference's sentence across a band: which edge overflowed, and by how much.
///
/// The console already says it — but the console is on the developer's machine and the
/// band is on the device, and a photograph of a phone is what half the bug reports in the
/// world are made of. A striped edge says *something* is too big; `RIGHT OVERFLOWED BY 86
/// PIXELS` says which edge, and by enough that you can tell a forgotten padding from a
/// missing wrap without running anything.
///
/// The words are the reference's, exactly, so that searching for them finds the same
/// answers. So are the numbers: 7.5 px, the heaviest weight there is, dark red on an
/// opaque white plate — a label that must stay readable over stripes cannot be subtle —
/// centred on the band's outer edge and turned a quarter turn on the vertical ones.
fn paint_overflow_label(scene: &mut Scene, band: Rect, over: &Overflowing) {
    /// The reference's label metrics.
    const SIZE: f32 = 7.5;
    /// The gap between the label and the outer edge of the band.
    const PAD: f32 = 1.0;
    /// `0xFF900000`.
    const INK: Color = Color::rgb(0.5647059, 0.0, 0.0);

    let side = match over.side {
        Side::Left => "LEFT",
        Side::Right => "RIGHT",
        Side::Top => "TOP",
        Side::Bottom => "BOTTOM",
    };
    let text = format!("{side} OVERFLOWED BY {} PIXELS", format_pixels(over.amount));
    // The reference asks for an 800 weight; the heaviest this framework has is bold.
    let style = TextStyle::new(SIZE).weight(FontWeight::Bold);
    let size = frus_text::measure_styled(&text, SIZE, FontWeight::Bold, false);

    // Where the middle of the label's leading edge goes, before any rotation, and by how
    // much the whole thing then turns about that point.
    let (anchor, angle) = match over.side {
        Side::Left => (
            Point::new(band.x + SIZE + PAD, band.y + band.height / 2.0),
            std::f32::consts::FRAC_PI_2,
        ),
        Side::Right => (
            Point::new(band.x + band.width - SIZE - PAD, band.y + band.height / 2.0),
            -std::f32::consts::FRAC_PI_2,
        ),
        Side::Top => (Point::new(band.x + band.width / 2.0, band.y + PAD), 0.0),
        Side::Bottom => (
            Point::new(band.x + band.width / 2.0, band.y + band.height - SIZE - PAD),
            0.0,
        ),
    };
    if angle == 0.0 {
        let at = Point::new(anchor.x - size.width / 2.0, anchor.y);
        scene.fill_rect(Rect::from_point_size(at, size), Color::WHITE);
        scene.text_styled(at, text, &style, INK);
        return;
    }
    // A rotation of a group, not of a glyph: the plate and the sentence turn together and
    // neither has to know it is turning.
    //
    // The group is painted **at the origin** rather than where it lands, because a layer
    // is rendered flat into a window-sized texture before it is composited: a vertical
    // label is written across a box near the right edge, so laid out flat it would run
    // off the texture and lose its last word before the rotation ever brought it back
    // inside. The transform is the one that carries the origin to where the label goes —
    // shift it half its width to the left, turn it, and land it on the anchor.
    let flat = Rect::from_point_size(Point::ZERO, size);
    let matrix = Affine::translation(-size.width / 2.0, 0.0)
        .then(Affine::rotation(angle))
        .then(Affine::translation(anchor.x, anchor.y));
    scene.transformed(LayerTransform::new(matrix), move |scene: &mut Scene| {
        scene.fill_rect(flat, Color::WHITE);
        scene.text_styled(flat.origin(), text, &style, INK);
    });
}

/// How the reference writes an overflow: whole pixels past ten, one decimal past one, and
/// three significant figures below that — because the difference between 0.5 px and
/// 0.0001 px is the difference between a layout bug and a rounding error.
fn format_pixels(value: f32) -> String {
    if value > 10.0 {
        format!("{value:.0}")
    } else if value > 1.0 {
        format!("{value:.1}")
    } else {
        // Three significant figures: the first one sits at the first non-zero decimal.
        let leading = if value > 0.0 {
            (-value.abs().log10().floor()) as i32
        } else {
            0
        };
        format!("{value:.*}", (leading + 2).clamp(0, 12) as usize)
    }
}

/// Identity of the `index`-th child: **by key** when the child declares one (stable whatever
/// its position), **positional** otherwise. It must be used everywhere a child identity is
/// derived (render, collection, lookup, animations) so they all stay consistent.
pub(crate) fn child_id<Msg>(parent: WidgetId, index: usize, child: &dyn Widget<Msg>) -> WidgetId {
    match child.key() {
        Some(key) => parent.keyed(key),
        None => parent.child(index),
    }
}

/// A widget's **effective** style for layout: its `style()`, whose size is **replaced by the
/// runtime's interpolated size** when the widget is animated (`Widget::anim_size` +
/// `Container::animated_size`). This is the only place where an animated size enters layout —
/// used **identically** by [`build_layout`] and by the cache fingerprint (`hash_node`), which
/// is what keeps them consistent (the cache stays invalid for as long as the size moves).
pub(crate) fn effective_style<Msg>(
    widget: &dyn Widget<Msg>,
    id: WidgetId,
    runtime: &Runtime,
    theme: &Theme,
) -> frus_layout::Style {
    // The theme's say on size and spacing (milestone 309), before the runtime's
    // animated overrides below — an animation in flight wins over a resting default.
    let mut style = widget.style_themed(theme);
    if widget.anim_size().is_some() {
        if let Some(size) = runtime.anim_size(id) {
            style.width = frus_layout::Dimension::Length(size.width);
            style.height = frus_layout::Dimension::Length(size.height);
        }
    }
    if widget.anim_padding().is_some() {
        if let Some(padding) = runtime.anim_padding(id) {
            style.padding = padding;
        }
    }
    // A dismissed item closing its gap. It collapses along the axis it was **not**
    // swiped along — a row swiped sideways loses its height — which is what makes the
    // neighbours slide up rather than the row narrow to a sliver. Only an explicit
    // length can shrink: an `Auto` box has no number to scale, which is why
    // `Dismissible` asks for a size (see its docs).
    if let Some(spec) = widget.dismissible() {
        if let Some(factor) = runtime.dismiss_extent_factor(id) {
            let axis = if spec.axis.is_horizontal() {
                &mut style.height
            } else {
                &mut style.width
            };
            if let frus_layout::Dimension::Length(extent) = *axis {
                *axis = frus_layout::Dimension::Length(extent * factor);
            }
        }
    }
    style
}

/// Builds the main layout tree (a scrollable is a **leaf**). `id`/`runtime` are what inject
/// the animated size through [`effective_style`], following the walk's identity scheme
/// (`child_id`) **exactly**.
pub(crate) fn build_layout<'a, Msg>(
    widget: &'a dyn Widget<Msg>,
    id: WidgetId,
    runtime: &'a Runtime,
    theme: &Theme,
    layout: &mut Layout<'a, BaselineData>,
) -> NodeId {
    let (node, fills) = build_layout_scoped(widget, id, runtime, theme, layout, true);
    // A root that wants to fill has no parent to fill: the flex machinery below needs one
    // on each side of the question. A percentage of the room the layout is being computed
    // in is the same answer without a parent, and it degrades to hugging the content when
    // that room is unbounded — a scrollable measuring its content, a natural size.
    if fills.horizontal {
        layout.fill_root(node, true);
    }
    if fills.vertical {
        layout.fill_root(node, false);
    }
    node
}

/// The axes a subtree asks to **fill** rather than shrink-wrap.
///
/// It is not a property of one widget. A column whose row fills the width is itself as
/// wide as the room it was given, because the row inside it took that room — so the
/// request travels up as the layout is built, and each container answers it with the one
/// thing a container can do about it: grow along its own axis, or stretch across it.
#[derive(Clone, Copy, Default)]
pub(crate) struct Fills {
    horizontal: bool,
    vertical: bool,
}

impl Fills {
    /// What a widget asks for on its own account.
    fn own<Msg>(widget: &dyn Widget<Msg>) -> Self {
        match widget.main_axis_fill() {
            Some(axis) if axis.is_horizontal() => Fills {
                horizontal: true,
                vertical: false,
            },
            Some(_) => Fills {
                horizontal: false,
                vertical: true,
            },
            None => Fills::default(),
        }
    }

    fn merge(self, other: Fills) -> Self {
        Fills {
            horizontal: self.horizontal || other.horizontal,
            vertical: self.vertical || other.vertical,
        }
    }

    /// The half of a child's request a container passes on to **its** parent, given
    /// which way the container runs. Along its own axis a container divides the room up
    /// and the request stops there; across it, the container's size *is* the child's, so
    /// the request has to keep going.
    /// Whether one axis is asked for.
    fn wants(self, horizontal: bool) -> bool {
        if horizontal {
            self.horizontal
        } else {
            self.vertical
        }
    }

    /// Drops any axis this box was **given a size on**: it has already been told how big
    /// to be, and a request to fill cannot reach past that answer.
    fn bounded_by(self, style: &frus_layout::Style) -> Self {
        let auto = |d| matches!(d, frus_layout::Dimension::Auto);
        Fills {
            horizontal: self.horizontal && auto(style.width),
            vertical: self.vertical && auto(style.height),
        }
    }

    fn across(self, direction: frus_layout::FlexDirection) -> Self {
        if direction.is_horizontal() {
            Fills {
                horizontal: false,
                vertical: self.vertical,
            }
        } else {
            Fills {
                horizontal: self.horizontal,
                vertical: false,
            }
        }
    }
}

/// A layout leaf's **text baseline**, when it has one: the distance from the top of its
/// box down to the line its letters sit on.
///
/// It rides on the layout tree rather than being asked of the widgets afterwards because
/// only the layout knows where a node ended up, and only the walk that built it knows
/// which branch it took — a scrollable, a stack and a page view are all *leaves* here,
/// with their contents laid out elsewhere, and a second walk guessing at that would be a
/// copy of this one waiting to drift out of step.
pub(crate) type BaselineData = Option<f32>;

/// The body of [`build_layout`], carrying whether baselines are still being collected.
/// An [`crate::IgnoreBaseline`] turns them off for its subtree — the widgets inside still
/// have baselines, and the point is that nothing above may see them.
fn build_layout_scoped<'a, Msg>(
    widget: &'a dyn Widget<Msg>,
    id: WidgetId,
    runtime: &'a Runtime,
    theme: &Theme,
    layout: &mut Layout<'a, BaselineData>,
    baselines: bool,
) -> (NodeId, Fills) {
    // A themed subtree lays out under **its** theme, not the frame's: a theme reaches
    // sizes and spacing (milestone 309), so this has to happen here and not only at paint
    // time. `hash_node`, which fingerprints this same walk for the relayout cache, makes
    // the same swap — the two staying in step is what keeps the cache honest.
    let scoped = widget.theme_override(theme);
    let theme = scoped.as_deref().unwrap_or(theme);
    // A subtree that could not be composed until the theme was known (`ThemeBuilder`).
    // It has to happen **before** anything reads `children()`, and under the subtree's
    // own theme, which is why it sits after the swap above rather than at the call site.
    widget.build_themed(theme);
    // Everything below an `IgnoreBaseline` keeps its baseline and loses the right to be
    // seen: the flag is narrowed once, here, so no branch below has to remember.
    let baselines = baselines && !widget.ignores_baseline();
    // A leaf's own baseline, or nothing when this subtree is being ignored.
    let own_baseline = if baselines {
        widget.text_baseline(theme)
    } else {
        None
    };
    // `Baseline`: the child is pushed down until **its** baseline lands `target` pixels
    // from the top of this box, and this box grows to contain it. If the child's own
    // baseline is already lower than that, there is nowhere to push it up to and the
    // child is top-aligned instead — the same answer the reference gives.
    if let Some(target) = widget.baseline_target() {
        let mut style = effective_style(widget, id, runtime, theme);
        if let Some(child) = widget.children().first() {
            let child = child.as_ref();
            let cid = child_id(id, 0, child);
            // A child with no baseline of its own is treated as if its baseline were its
            // bottom edge, which is what a box with no text amounts to.
            let child_baseline = natural_baseline(child, cid, runtime, theme)
                .unwrap_or_else(|| natural_size(child, cid, runtime, theme).height);
            style.padding.top += (target - child_baseline).max(0.0);
            let (node, fills) = build_layout_scoped(child, cid, runtime, theme, layout, baselines);
            return (layout.container(style, &[node]), fills);
        }
        return (layout.leaf(style, None), Fills::default());
    }
    // `RotatedBox`: a leaf whose box is the child's **natural** size, with its dimensions
    // **swapped** for an odd quarter turn (the rotation does affect layout). The child itself
    // is laid out separately (at render time).
    if let Some(q) = widget.rotated_quarter_turns() {
        let mut style = effective_style(widget, id, runtime, theme);
        if let Some(child) = widget.children().first() {
            let nat = natural_size(
                child.as_ref(),
                child_id(id, 0, child.as_ref()),
                runtime,
                theme,
            );
            let (w, h) = if q.rem_euclid(4) % 2 != 0 {
                (nat.height, nat.width)
            } else {
                (nat.width, nat.height)
            };
            style.width = frus_layout::Dimension::Length(w);
            style.height = frus_layout::Dimension::Length(h);
        }
        return (layout.leaf(style, own_baseline), Fills::default());
    }
    // `Intrinsic`: one axis is taken from what the content would **like** to be, not from
    // the space on offer. The content is measured once, unconstrained, and the answer is
    // written into this node's style as a length — the same trick `RotatedBox` uses, except
    // that here the child stays inside, laid out normally within the size it asked for.
    if let Some((axis, step)) = widget.intrinsic() {
        let mut style = effective_style(widget, id, runtime, theme);
        if let Some(child) = widget.children().first() {
            let child = child.as_ref();
            let cid = child_id(id, 0, child);
            let nat = natural_size(child, cid, runtime, theme);
            let quantise = |extent: f32| match step {
                Some(step) if step > 0.0 => (extent / step).ceil() * step,
                _ => extent,
            };
            match axis {
                crate::constraints::IntrinsicAxis::Width => {
                    style.width = frus_layout::Dimension::Length(quantise(nat.width));
                }
                crate::constraints::IntrinsicAxis::Height => {
                    style.height = frus_layout::Dimension::Length(quantise(nat.height));
                }
            }
            let (node, fills) = build_layout_scoped(child, cid, runtime, theme, layout, baselines);
            return (layout.container(style, &[node]), fills);
        }
        return (layout.leaf(style, own_baseline), Fills::default());
    }
    // `OverflowBox`: a leaf, because its child is laid out **separately**, to constraints of
    // its own. That separation is the whole feature — a child sharing this node's layout
    // could never be bigger than it.
    if widget.overflow_box().is_some() {
        return (
            layout.leaf(effective_style(widget, id, runtime, theme), own_baseline),
            Fills::default(),
        );
    }
    // A `LayoutBuilder` **is measured**: it builds its content from the space offered and
    // is then as big as what it built, which is the reference's
    // `size = constraints.constrain(child.size)`. It cannot be a plain leaf, because a
    // leaf answers before it has been asked and there is nothing to answer with until the
    // box is known; and it cannot be a container, because its child does not exist yet.
    //
    // Taffy calls the closure during the computation with the space actually available —
    // the same moment the reference runs its layout callback — and the closure builds the
    // subtree, lays it out in a `Layout` of its own, and hands back the size. An axis the
    // style pins is still the style's: taffy does not ask about a dimension it already
    // knows, so `LayoutBuilder::height(200.0)` behaves exactly as it did.
    if let Some(build) = widget.layout_builder() {
        let style = effective_style(widget, id, runtime, theme);
        // `Theme` is `Copy`, and it has to be **owned**: a themed subtree's theme is a
        // local in this function, so it cannot be borrowed for as long as the closure
        // lives. The runtime and the widget are borrowed, which is the whole reason
        // `Layout` carries a lifetime.
        let theme = *theme;
        let cid = id.child(0);
        let measure: frus_layout::MeasureFn<'a> = Box::new(move |w, h| {
            // What taffy offers. `None` is an *intrinsic* question — how big would you
            // be with no limit, or with none at all — and there is no honest answer to
            // it here: finding out means running the application's callback
            // speculatively. The reference refuses these outright (`computeDryLayout`
            // asserts); we build at the offered numbers, substituting zero for what is
            // not offered, and leave the unoffered axis free in the nested computation
            // so the content's own size comes back.
            let offered = Size::new(w.unwrap_or(0.0), h.unwrap_or(0.0));
            let child = build(offered);
            let mut inner: Layout<BaselineData> = Layout::new();
            let node = build_layout(child.as_ref(), cid, runtime, &theme, &mut inner);
            // The content is **handed** the offered box, not asked what it would like:
            // it was built from that box. An axis that was not offered is the one being
            // asked about, and stays free so the content's own size comes back.
            match (w, h) {
                (Some(w), Some(h)) => inner.compute_filled(node, w, h),
                _ => inner.compute_scroll(
                    node,
                    offered.width,
                    offered.height,
                    w.is_none(),
                    h.is_none(),
                ),
            }
            inner.size_of(node)
        });
        return (
            layout.measured_leaf(style, own_baseline, measure),
            Fills::own(widget),
        );
    }
    // Scrollables, interactive viewports, fitters (`FittedBox`), navigators, virtualised
    // lists and stacks: their content is laid out separately (independent layers / screens /
    // items, or a child laid out at its natural size).
    if widget.scroll_content().is_some()
        || widget.interactive().is_some()
        || widget.fitted().is_some()
        || widget.navigator().is_some()
        || widget.virtual_list().is_some()
        || widget.page_view().is_some()
        || widget.stack()
    {
        // Nothing bubbles out of these: what is inside is laid out somewhere else, to
        // constraints of its own, and cannot have an opinion about this box.
        return (
            layout.leaf(effective_style(widget, id, runtime, theme), own_baseline),
            Fills::own(widget),
        );
    }
    // A portal only lays out its anchor (child 0); the overlay is deferred.
    if widget.overlay().is_some() {
        let anchor_w = widget.children()[0].as_ref();
        let (anchor, fills) = build_layout_scoped(
            anchor_w,
            child_id(id, 0, anchor_w),
            runtime,
            theme,
            layout,
            baselines,
        );
        return (
            layout.container(effective_style(widget, id, runtime, theme), &[anchor]),
            fills,
        );
    }
    let children = widget.children();
    if children.is_empty() {
        // A leaf measured under constraints (a paragraph that wraps…): taffy calls the
        // closure during the computation.
        if let Some(measure) = widget.measure() {
            return (
                layout.measured_leaf(
                    effective_style(widget, id, runtime, theme),
                    own_baseline,
                    measure,
                ),
                Fills::own(widget),
            );
        }
        (
            layout.leaf(effective_style(widget, id, runtime, theme), own_baseline),
            Fills::own(widget),
        )
    } else {
        let style = effective_style(widget, id, runtime, theme);
        let built: Vec<(NodeId, Fills)> = children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                build_layout_scoped(
                    child.as_ref(),
                    child_id(id, i, child.as_ref()),
                    runtime,
                    theme,
                    layout,
                    baselines,
                )
            })
            .collect();
        let child_ids: Vec<NodeId> = built.iter().map(|(node, _)| *node).collect();
        // `MainAxisSize::Max`: a run that fills the room its parent leaves it. Only the
        // parent knows what that means — grow along an axis it shares, stretch across
        // one it does not — which is why the child asks rather than saying.
        //
        // The request is the child's **and its subtree's**: a column whose row fills the
        // width is itself as wide as the room it was given.
        let alone = children.len() == 1;
        for (node, fills) in &built {
            for horizontal in [true, false] {
                if !fills.wants(horizontal) {
                    continue;
                }
                let parallel = horizontal == style.flex_direction.is_horizontal();
                // Along an axis the parent runs too, the reference leaves the main axis
                // **unbounded** and `Max` quietly degrades to `Min`: there is no maximum
                // to take. The exception is a parent with a single child — a padding, an
                // alignment, a decorated box — which passes its own constraints straight
                // down, and there the run does fill.
                if parallel && !alone {
                    continue;
                }
                layout.fill_parent(*node, horizontal, parallel);
            }
        }
        // A **lone** child gives way; several do not.
        //
        // The reference hands an inflexible child of a row or a column an unbounded main
        // axis: it is never squeezed, and a line that does not fit overflows and says so.
        // That is the default here now, because a framework that quietly crushes a button
        // instead of reporting a layout that does not fit is one whose bugs are invisible
        // — milestone 333 spent three milestones on exactly that crushed button.
        //
        // But a box with one child is not dividing a line up: it is handing its own
        // constraints down, and the reference bounds a lone child by them. The same
        // exception the fill request and the floor below both make, for the same reason.
        if alone {
            for node in &child_ids {
                layout.allow_shrink(*node);
            }
        }
        // A grid's tiles are all the same shape. The ratio is the *container's*, since a
        // tile cannot know how wide its column came out, and the reference says the same:
        // a grid's delegate hands every child a tight box derived from the track, square
        // unless the application asked for another ratio.
        if let Some(ratio) = widget.tile_shape() {
            for node in &child_ids {
                layout.set_tile_shape(*node, ratio);
            }
        }
        // A child that refuses to be squeezed along the row. Only a row of **several**
        // children may say so. Across a column the same floor would refuse a width the
        // column was handing it, which is how a paragraph is told how wide to be; and a
        // box with a single child — a padding, an alignment, a decorated box — is handing
        // one down rather than dividing a line up, whichever way it nominally runs.
        if style.flex_direction.is_horizontal() && !alone {
            for (child, node) in children.iter().zip(&child_ids) {
                if let Some(floor) = child.main_axis_floor() {
                    layout.set_min_width(*node, floor);
                }
            }
        }
        // **Baseline** cross-alignment: the children are pushed down until their
        // baselines meet, which is the only alignment that makes two runs of different
        // sizes read as one line rather than as one row.
        //
        // It is resolved here rather than by taffy, which cannot: taffy asks a leaf for
        // a size and a leaf can only answer with one. The measurement a baseline needs
        // is the font's, so the answer has to come from up here, where the widgets are.
        //
        // The **natural** baseline is the right one to measure even though the children
        // will be laid out at some other width: the first line of a piece of text sits
        // in the same place however narrow the box gets, since narrowing it adds lines
        // below rather than moving the first one.
        if style.align == frus_layout::Align::Baseline && style.flex_direction.is_horizontal() {
            let own: Vec<Option<f32>> = children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    natural_baseline(
                        child.as_ref(),
                        child_id(id, i, child.as_ref()),
                        runtime,
                        theme,
                    )
                })
                .collect();
            if let Some(deepest) = own.iter().flatten().copied().reduce(f32::max) {
                for (node, b) in child_ids.iter().zip(&own) {
                    // A child with no baseline keeps its place: it is a box, and a box
                    // has nothing to line up.
                    if let Some(b) = b {
                        layout.add_margin_top(*node, deepest - b);
                    }
                }
            }
        }
        // What this container passes on to **its** parent. Across its own axis the
        // container's size is its child's, so the request keeps going; along it the
        // container divides the room up and the request stops. A container with a single
        // child divides nothing, and passes the whole request on.
        let mut fills = Fills::own(widget);
        for (_, child) in &built {
            fills = fills.merge(if alone {
                *child
            } else {
                child.across(style.flex_direction)
            });
        }
        (
            layout.container(style, &child_ids),
            fills.bounded_by(&style),
        )
    }
}

/// The distance from the top of `widget`'s **natural** box to the first baseline inside
/// it, or `None` when there is no text in there to have one.
///
/// It lays the subtree out on its own, exactly as [`natural_size`] does, and takes the
/// first baseline the layout collected. First in *layout* order, which is the order a
/// reader would find it in — and it is the leaves that carry them, so an intervening
/// padding or alignment is already counted in the rectangle it comes with.
pub(crate) fn natural_baseline<Msg>(
    widget: &dyn Widget<Msg>,
    id: WidgetId,
    runtime: &Runtime,
    theme: &Theme,
) -> Option<f32> {
    let mut layout = Layout::new();
    let node = build_layout(widget, id, runtime, theme, &mut layout);
    layout.compute_scroll(node, 0.0, 0.0, true, true);
    let rects = layout.absolute_rects(node);
    let top = rects.first()?.0.y;
    rects
        .iter()
        .find_map(|(rect, data)| data.copied().flatten().map(|b| rect.y + b - top))
}

/// **Natural** (intrinsic) size of a subtree: laid out under free axes (`MaxContent`), with
/// no imposed constraint. Used by `RotatedBox` (the dimensions to swap) and — at render time
/// — by `FittedBox` (the fit factor).
pub(crate) fn natural_size<Msg>(
    widget: &dyn Widget<Msg>,
    id: WidgetId,
    runtime: &Runtime,
    theme: &Theme,
) -> Size {
    let mut layout = Layout::new();
    let node = build_layout(widget, id, runtime, theme, &mut layout);
    layout.compute_scroll(node, 0.0, 0.0, true, true);
    layout
        .absolute_rects(node)
        .first()
        .map(|(r, _)| Size::new(r.width, r.height))
        .unwrap_or(Size::new(0.0, 0.0))
}

/// A deferred overlay: `(content, id, the anchor's bounds, placement, dismissal,
/// progress 0..=1, whether it takes the pointer, the theme it was declared under)`. The
/// progress animates the appearance — a drawer sliding in, a scrim fading — and is
/// `1.0` for overlays that are not animated.
///
/// The **theme travels with it** because an overlay is painted long after the walk has
/// left the node that declared it: a dialog opened from inside a [`crate::Themed`]
/// subtree would otherwise come out in the root's theme, which is the surprise the
/// reference had to grow a whole mechanism to avoid.
type Overlay<'a, Msg> = (
    &'a dyn Widget<Msg>,
    WidgetId,
    Rect,
    Placement,
    Option<Msg>,
    f32,
    bool,
    Theme,
);

struct Builder<'a, Msg> {
    scene: Scene,
    hits: Vec<Hit<Msg>>,
    long_presses: Vec<Hit<Msg>>,
    dismisses: Vec<Msg>,
    focusables: Vec<Focusable>,
    /// Start of the topmost modal overlay's focus scope.
    focus_scope_start: Option<usize>,
    /// Set while inside an `ExcludeFocus`: nothing in here registers a focus stop.
    focus_excluded: bool,
    /// The identity of the nearest enclosing backdrop group — the key a backdrop
    /// asking to be shared takes. Pushed and popped by the walk, like the focus flags.
    backdrop_group: Option<u64>,
    /// Set while inside an `ExcludeFocusTraversal`: stops register, Tab passes them by.
    focus_skipped: bool,
    /// The traversal order in force, from the nearest enclosing `FocusTraversalOrder`.
    focus_order: Option<f32>,
    /// The nearest enclosing `FocusTraversalGroup`, within which an order is resolved.
    focus_group: Option<WidgetId>,
    /// Shortcut and action scopes closed so far this frame.
    scopes: Vec<Scope<Msg>>,
    /// Keystroke listeners, with the focus stops they cover.
    #[allow(clippy::type_complexity)]
    listeners: Vec<(
        std::ops::Range<usize>,
        std::rc::Rc<dyn Fn(KeyStroke) -> Option<Msg>>,
    )>,
    scrollables: Vec<Scrollable>,
    scrollbars: Vec<Scrollbar>,
    draggables: Vec<(WidgetId, Rect)>,
    drag_sources: Vec<DragSource>,
    drop_zones: Vec<DropZone>,
    inks: Vec<(WidgetId, Rect)>,
    reorderables: Vec<(WidgetId, Rect)>,
    interactives: Vec<(WidgetId, Rect)>,
    /// Deferred overlays: (content, id, the anchor's bounds, placement, dismissal, progress
    /// `0..=1`). The progress animates the appearance (a drawer sliding in, a scrim fading);
    /// it is `1.0` for overlays that are not animated.
    overlays: Vec<Overlay<'a, Msg>>,
    /// A widget is asking for a continuous, time-driven animation.
    wants_animation: bool,
    available: Size,
    runtime: &'a Runtime,
    /// The theme **in force at this point of the walk** — the root's, unless a
    /// [`crate::Themed`] ancestor replaced it. Owned rather than borrowed because a
    /// subtree's theme is derived from the one above it and outlives nothing.
    theme: Theme,
    /// The inspector's collection (`Some` only while it is on): one node per painted widget,
    /// in paint order.
    inspector: Option<Vec<crate::inspector::InspectorNode>>,
    /// Current depth of the walk (for the dump's indentation and the palette of the
    /// inspector's outlines).
    depth: usize,
    /// The [`crate::RefreshIndicator`] area currently being walked, if any: every scrollable
    /// registered under it records it, so the shell knows where to send the movement
    /// its physics refuses. Saved and restored around the subtree, so sibling areas do
    /// not inherit one another's.
    refresh_host: Option<WidgetId>,
    /// Which screen of a route transition the walk is inside (`0` = leaving, `1` =
    /// entering); `None` outside one. Recorded on each hero so the two sides of a
    /// flight are told apart rather than guessed from paint order.
    hero_screen: Option<u8>,
    /// The shared elements seen so far, each with the widget that declared it (needed
    /// to lift its subtree's painting) and the transition screen it belongs to.
    heroes: Vec<(HeroSpot, &'a dyn Widget<Msg>)>,
    /// The frame's refresh areas, in paint order.
    refreshes: Vec<Refreshable>,
    /// The frame's dismissible items, in paint order.
    dismissables: Vec<Dismissable>,
    /// Accessibility nodes collected during the walk (paint order).
    semantics: Vec<(WidgetId, Rect, frus_core::Semantics)>,
    /// Boxes whose children did not fit, screen-positioned, from every sub-root walked
    /// this frame.
    overflows: std::cell::RefCell<Vec<Overflowing>>,
    /// The same, still in their sub-root's own coordinates, waiting for the walk to reach
    /// that sub-root and say where on screen it ended up.
    pending_overflows: std::cell::RefCell<std::collections::HashMap<WidgetId, Vec<Overflowing>>>,
}

impl<'a, Msg: Clone + 'static> Builder<'a, Msg> {
    /// A layout root's rects, through the relayout cache retained in the runtime (it only
    /// recomputes through taffy when the style/structure/constraints have changed). A brief
    /// mutable borrow: the `Vec` returned is owned.
    ///
    /// In **RTL**, taffy computes in LTR (canonical, and cached), then each rect is
    /// **mirrored** about the root's width (the 1st rect): the rows reverse, the alignment
    /// and the margins flip — without touching the widgets. The text itself is drawn
    /// normally inside its moved box (the *internal* bidi is handled by cosmic-text).
    fn cached_rects(&self, key: WidgetId, root: &dyn Widget<Msg>, c: Constraints) -> Vec<Rect> {
        let (mut rects, overflows) =
            self.runtime
                .layout_cache
                .borrow_mut()
                .rects(key, root, self.runtime, &self.theme, c);
        self.mirror(&mut rects);
        self.record_overflows(key, &rects, overflows);
        rects
    }

    /// Files this sub-root's overflowing boxes under its identity, mirrored with
    /// everything else in RTL — where an edge changes name as well as position, since the
    /// layout is the mirror image and "ran past the right" becomes "ran past the left".
    ///
    /// They wait here rather than going straight out because a sub-root's rectangles are
    /// in its **own** coordinates: a scrollable's content, a page, a list item and a
    /// stack's layer each get their own taffy pass, and where that pass lands on screen is
    /// only known when the walk reaches it. [`Self::claim_overflows`] is the other half.
    fn record_overflows(&self, key: WidgetId, mirrored: &[Rect], overflows: Vec<Overflowing>) {
        if overflows.is_empty() {
            return;
        }
        let rtl = self.rtl();
        let root = mirrored.first().copied();
        let list: Vec<Overflowing> = overflows
            .into_iter()
            .map(|mut o| {
                if let (true, Some(root)) = (rtl, root) {
                    o.rect.x = root.x + (root.width - (o.rect.x - root.x) - o.rect.width);
                    o.side = match o.side {
                        Side::Left => Side::Right,
                        Side::Right => Side::Left,
                        other => other,
                    };
                }
                o
            })
            .collect();
        self.pending_overflows.borrow_mut().insert(key, list);
    }

    /// The walk has reached sub-root `id` and knows where it sits: its overflowing boxes
    /// become screen rectangles.
    fn claim_overflows(&self, id: WidgetId, translation: (f32, f32)) {
        let taken = self.pending_overflows.borrow_mut().remove(&id);
        if let Some(list) = taken {
            self.overflows
                .borrow_mut()
                .extend(list.into_iter().map(|o| Overflowing {
                    rect: o.rect.translate(translation.0, translation.1),
                    ..o
                }));
        }
    }

    /// The layout direction **in force here** — read from the ambient theme rather than
    /// from a field set once at the root, so a [`crate::Themed`] subtree carries its own.
    ///
    /// What that reaches is everything decided during the walk: which edge a drawer
    /// slides from, which way an anchored overlay opens, the sign of a rotation, the
    /// mirroring of any **layout root** inside the subtree (a scrollable, an overlay).
    /// What it does not reach is ordinary flow inside the subtree: the frame's rects are
    /// computed once at the root and mirrored there as a whole, so a direction set part
    /// way down does not reverse the rows around it. Setting the direction on the theme
    /// handed to `build_ui` is still the way to flip an application.
    fn rtl(&self) -> bool {
        self.theme.direction.is_rtl()
    }

    /// Mirrors a root's rects horizontally (RTL). The first rect is the root itself: its
    /// width is the axis of symmetry.
    fn mirror(&self, rects: &mut [Rect]) {
        if !self.rtl() {
            return;
        }
        let Some(root) = rects.first().copied() else {
            return;
        };
        // The axis: the rects are relative to the root (origin at root.x).
        for r in rects.iter_mut() {
            r.x = root.x + (root.width - (r.x - root.x) - r.width);
        }
    }

    /// 64-bit fingerprint of everything painting a boundary subtree reads **without**
    /// rebuilding the `view`: each descendant's `Status` (in walk order) and the subtree's
    /// absolute rects. An unchanged fingerprint + generation ⇒ a bit-for-bit identical paint.
    /// The time is excluded (a cacheable boundary holds no `continuous` widget).
    fn boundary_fingerprint(
        &self,
        widget: &dyn Widget<Msg>,
        id: WidgetId,
        translation: (f32, f32),
        sub: &[Rect],
    ) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.rtl().hash(&mut h);
        translation.0.to_bits().hash(&mut h);
        translation.1.to_bits().hash(&mut h);
        for r in sub {
            r.x.to_bits().hash(&mut h);
            r.y.to_bits().hash(&mut h);
            r.width.to_bits().hash(&mut h);
            r.height.to_bits().hash(&mut h);
        }
        self.hash_statuses(widget, id, &mut h);
        h.finish()
    }

    /// Adds `widget`'s `Status` to `h`, then its children's, recursively — following the
    /// walk's identity scheme (`child_id`) **exactly**, so it lines up with the order in which
    /// `walk_node` paints them.
    fn hash_statuses<H: Hasher>(&self, widget: &dyn Widget<Msg>, id: WidgetId, h: &mut H) {
        hash_status(&self.full_status(widget, id), h);
        // Live ink is part of what the boundary paints, and it moves every frame while a
        // splash is alive. Without it here the cached primitives would be replayed and
        // the ripple would stand still — and once the ink dries the hash goes back to
        // what it was, so the boundary starts hitting again.
        if let Some(ripples) = self.runtime.ink.get(&id) {
            ripples.hash_state(h);
        }
        for (i, child) in widget.children().iter().enumerate() {
            self.hash_statuses(child.as_ref(), child_id(id, i, child.as_ref()), h);
        }
    }

    /// Current lengths of the builder's collections: the lower bound of the slices to capture
    /// for a boundary (see [`capture_since`](Self::capture_since)).
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            scene: self.scene.primitives().len(),
            hits: self.hits.len(),
            long_presses: self.long_presses.len(),
            dismisses: self.dismisses.len(),
            focusables: self.focusables.len(),
            scrollables: self.scrollables.len(),
            draggables: self.draggables.len(),
            drag_sources: self.drag_sources.len(),
            drop_zones: self.drop_zones.len(),
            inks: self.inks.len(),
            semantics: self.semantics.len(),
            overlays: self.overlays.len(),
            focus_scope_start: self.focus_scope_start,
        }
    }

    /// Captures the output produced since `snap` (the collections' *tail slices*) into a
    /// [`BoundaryData`]. Returns `None` — hence **not cacheable** — when the subtree pushed an
    /// overlay or touched the modal focus scope (global state that cannot be captured here).
    fn capture_since(&self, snap: &Snapshot) -> Option<BoundaryData<Msg>> {
        if self.overlays.len() != snap.overlays || self.focus_scope_start != snap.focus_scope_start
        {
            return None;
        }
        Some(BoundaryData {
            prims: self.scene.primitives()[snap.scene..].to_vec(),
            hits: self.hits[snap.hits..].to_vec(),
            long_presses: self.long_presses[snap.long_presses..].to_vec(),
            dismisses: self.dismisses[snap.dismisses..].to_vec(),
            focusables: self.focusables[snap.focusables..].to_vec(),
            scrollables: self.scrollables[snap.scrollables..].to_vec(),
            draggables: self.draggables[snap.draggables..].to_vec(),
            drag_sources: self.drag_sources[snap.drag_sources..].to_vec(),
            drop_zones: self.drop_zones[snap.drop_zones..].to_vec(),
            inks: self.inks[snap.inks..].to_vec(),
            semantics: self.semantics[snap.semantics..].to_vec(),
        })
    }

    /// Replays a boundary from the cache: primitives already formed (clip/owner baked in) and
    /// interaction maps, appended as is.
    fn splice_boundary(&mut self, data: BoundaryData<Msg>) {
        for p in data.prims {
            self.scene.push_primitive(p);
        }
        self.hits.extend(data.hits);
        self.long_presses.extend(data.long_presses);
        self.dismisses.extend(data.dismisses);
        self.focusables.extend(data.focusables);
        self.scrollables.extend(data.scrollables);
        self.draggables.extend(data.draggables);
        self.drag_sources.extend(data.drag_sources);
        self.drop_zones.extend(data.drop_zones);
        self.inks.extend(data.inks);
        self.semantics.extend(data.semantics);
    }

    /// Lengths of every registry a [`ModalBarrier`] can withhold from, taken before its subtree
    /// is walked.
    fn barrier_base(&self) -> BarrierBase {
        BarrierBase {
            scene: self.scene.primitives().len(),
            hits: self.hits.len(),
            long_presses: self.long_presses.len(),
            focusables: self.focusables.len(),
            scrollables: self.scrollables.len(),
            scrollbars: self.scrollbars.len(),
            draggables: self.draggables.len(),
            drag_sources: self.drag_sources.len(),
            drop_zones: self.drop_zones.len(),
            inks: self.inks.len(),
            reorderables: self.reorderables.len(),
            interactives: self.interactives.len(),
            semantics: self.semantics.len(),
        }
    }

    /// Drops what the subtree added since `base`, according to `barrier`, and — for an
    /// absorbing barrier — puts a message-less hit target in its place so that input stops
    /// at the barrier instead of reaching whatever is painted behind it.
    /// Resolves the **shared elements** of a route transition: every tag that appears
    /// on both screens flies from where it was to where it is going.
    ///
    /// Called once, after both screens have been drawn, because that is the first
    /// moment both boxes are known. `hero_base` and `scene_base` mark where those two
    /// screens started.
    ///
    /// What flies is the **destination**'s own painting, mapped onto the box it is
    /// travelling through. Both originals are taken out of the frame: a thing that is
    /// flying is not also sitting at either end, and leaving them in is the difference
    /// between a shared element and three copies of one.
    fn fly_heroes(&mut self, hero_base: usize, scene_base: usize, progress: f32) {
        let spots: Vec<(HeroSpot, &'a dyn Widget<Msg>)> = self.heroes.split_off(hero_base);
        if spots.len() < 2 {
            return;
        }
        // The pairs: one tag, one hero on each side. A tag on one side only has nothing
        // to fly to, and a tag used more than once per side is ambiguous — both are
        // left alone rather than guessed at.
        let mut flights: Vec<(Rect, Rect, &'a dyn Widget<Msg>, WidgetId, WidgetId)> = Vec::new();
        for (from, _) in spots.iter().filter(|(s, _)| s.screen == 0) {
            let mut matching = spots
                .iter()
                .filter(|(s, _)| s.screen == 1 && s.tag == from.tag);
            let Some((to, widget)) = matching.next() else {
                continue;
            };
            if matching.next().is_some() {
                continue;
            }
            flights.push((from.rect, to.rect, *widget, from.id, to.id));
        }
        if flights.is_empty() {
            return;
        }

        // Everything the two screens painted, so the originals can be taken out and the
        // travelling copies laid on top.
        let painted = self.scene.split_off(scene_base);
        let mut hidden: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for (_, _, widget, from_id, to_id) in &flights {
            for id in subtree_ids(*widget, *to_id) {
                hidden.insert(id.as_u64());
            }
            // The source's subtree is the *other* screen's widget, whose identity is
            // rooted at `from_id`; the same relative shape, so the ids follow.
            for id in subtree_ids(*widget, *from_id) {
                hidden.insert(id.as_u64());
            }
        }
        for primitive in &painted {
            if !hidden.contains(&primitive.owner()) {
                self.scene.push_primitive(primitive.clone());
            }
        }

        self.scene.set_clip(Rect::UNBOUNDED);
        for (from, to, _, _, to_id) in &flights {
            let travelling = lerp_rect(*from, *to, progress);
            if to.width <= 0.0 || to.height <= 0.0 {
                continue;
            }
            let (sx, sy) = (travelling.width / to.width, travelling.height / to.height);
            let pivot = Point::new(to.x, to.y);
            let owners: std::collections::HashSet<u64> = subtree_ids(
                flights
                    .iter()
                    .find(|(_, _, _, _, id)| id == to_id)
                    .map(|(_, _, w, _, _)| *w)
                    .expect("the flight just matched"),
                *to_id,
            )
            .iter()
            .map(|id| id.as_u64())
            .collect();
            for primitive in painted.iter().filter(|p| owners.contains(&p.owner())) {
                let flown = primitive
                    .scaled_about_xy(pivot, sx, sy)
                    .translated(travelling.x - to.x, travelling.y - to.y)
                    .with_clip(Rect::UNBOUNDED);
                self.scene.push_primitive(flown);
            }
        }
    }

    fn apply_barrier(
        &mut self,
        barrier: ModalBarrier,
        base: &BarrierBase,
        id: WidgetId,
        own: Rect,
    ) {
        if barrier.pointer {
            self.hits.truncate(base.hits);
            self.long_presses.truncate(base.long_presses);
            self.focusables.truncate(base.focusables);
            self.scrollables.truncate(base.scrollables);
            self.scrollbars.truncate(base.scrollbars);
            self.draggables.truncate(base.draggables);
            self.drag_sources.truncate(base.drag_sources);
            self.drop_zones.truncate(base.drop_zones);
            self.inks.truncate(base.inks);
            self.reorderables.truncate(base.reorderables);
            self.interactives.truncate(base.interactives);
            // The modal focus scope is an index **into** `focusables`. A barrier that cut
            // away the focusable it pointed at would leave it dangling past the end, and
            // every later slice of the pool would then be empty — no focusable at all,
            // rather than the ones the barrier meant to spare.
            if let Some(start) = self.focus_scope_start {
                self.focus_scope_start = Some(start.min(self.focusables.len()));
            }
        }
        if barrier.absorb && own.width > 0.0 && own.height > 0.0 {
            self.hits.push(Hit {
                id,
                rect: own,
                msg: None,
                xform: None,
            });
        }
        if barrier.paint {
            // The clip is baked into each primitive when it is pushed, so dropping the tail
            // leaves nothing for the next sibling to repair.
            let _hidden = self.scene.split_off(base.scene);
        }
        if barrier.semantics {
            self.semantics.truncate(base.semantics);
        }
    }

    /// Lower bounds of the interaction registries **before** a transformed composited subtree.
    fn xform_base(&self) -> XformBase {
        XformBase {
            hits: self.hits.len(),
            long_presses: self.long_presses.len(),
            focusables: self.focusables.len(),
            scrollables: self.scrollables.len(),
            draggables: self.draggables.len(),
            drag_sources: self.drag_sources.len(),
            drop_zones: self.drop_zones.len(),
            inks: self.inks.len(),
            reorderables: self.reorderables.len(),
            semantics: self.semantics.len(),
        }
    }

    /// Applies the layer transform `matrix` to the registry entries **added since** `base`:
    /// the test point of click / long-press targets goes through `M⁻¹` (without overwriting an
    /// inner transform already set); and when `matrix` preserves axis alignment
    /// (scale/translation, no rotation), the **rects** of focus / scrolling / dragging /
    /// **reordering** / accessibility are mapped exactly by `M` (otherwise they are left as
    /// they are — the click stays correct through `M⁻¹`). The **shared factor** of the two
    /// transformed-composition sites (`walk`'s boundary and `emit_transformed_child`): one
    /// place to keep up to date, which forgets no registry (milestone 250 had forgotten
    /// `reorderables` in exactly one of the two).
    fn transform_interaction_registries(&mut self, base: &XformBase, matrix: Affine) {
        let inverse = matrix.inverse();
        for h in &mut self.hits[base.hits..] {
            h.xform.get_or_insert(inverse);
        }
        for h in &mut self.long_presses[base.long_presses..] {
            h.xform.get_or_insert(inverse);
        }
        if matrix.is_axis_aligned() {
            for f in &mut self.focusables[base.focusables..] {
                f.rect = matrix.apply_rect(f.rect);
            }
            for area in &mut self.scrollables[base.scrollables..] {
                area.viewport = matrix.apply_rect(area.viewport);
            }
            for source in &mut self.drag_sources[base.drag_sources..] {
                source.rect = matrix.apply_rect(source.rect);
            }
            for zone in &mut self.drop_zones[base.drop_zones..] {
                zone.rect = matrix.apply_rect(zone.rect);
            }
            for (_, r) in &mut self.inks[base.inks..] {
                *r = matrix.apply_rect(*r);
            }
            for (_, r) in &mut self.draggables[base.draggables..] {
                *r = matrix.apply_rect(*r);
            }
            for (_, r) in &mut self.reorderables[base.reorderables..] {
                *r = matrix.apply_rect(*r);
            }
            for (_, r, _) in &mut self.semantics[base.semantics..] {
                *r = matrix.apply_rect(*r);
            }
        }
    }

    /// Entry point for walking a node. When the node is a cacheable **repaint boundary**, it
    /// tries to replay its subtree from the paint cache; otherwise (or on a *miss*) it
    /// delegates to the full walk [`walk_node`](Self::walk_node), capturing its output for the
    /// next frame. Every recursion goes through here → nested boundaries get cached too.
    /// The focus flags a subtree carries — exclusion, Tab-skipping, an order, a group —
    /// are **subtree-scoped**, so they are pushed here and popped on the way out rather
    /// than asked of each widget at the moment it registers. The body of the walk has
    /// early returns in a dozen branches; wrapping it is the only way to be sure the
    /// scope is closed on every one of them.
    fn walk(
        &mut self,
        widget: &'a dyn Widget<Msg>,
        id: WidgetId,
        translation: (f32, f32),
        clip: Rect,
        rects: &[Rect],
        index: &mut usize,
    ) {
        let outer = (
            self.focus_excluded,
            self.focus_skipped,
            self.focus_order,
            self.focus_group,
        );
        if !widget.descendants_focusable() {
            self.focus_excluded = true;
        }
        if widget.focus_skip_traversal() {
            self.focus_skipped = true;
        }
        if let Some(order) = widget.focus_order() {
            self.focus_order = Some(order);
        }
        if widget.focus_group() {
            self.focus_group = Some(id);
        }
        // The backdrop group a subtree sits in, scoped the same way and for the same
        // reason. Its own identity is the key: stable across frames, and unique
        // without anything having to hand one out.
        let outer_group = self.backdrop_group;
        if widget.backdrop_group() {
            self.backdrop_group = Some(id.as_u64());
        }
        // The focus stops this subtree contains start here; the walk is depth-first, so
        // they are contiguous and the range closes below.
        let stops_before = self.focusables.len();
        self.walk_scoped(widget, id, translation, clip, rects, index);
        (
            self.focus_excluded,
            self.focus_skipped,
            self.focus_order,
            self.focus_group,
        ) = outer;
        self.backdrop_group = outer_group;
        self.close_scope(widget, stops_before);
    }

    /// Files whatever tables this subtree carried, with the focus stops it turned out to
    /// contain. Recorded **on the way out**, because the range is not known on the way in;
    /// so `scopes` ends up innermost-first among any that overlap, which is the order the
    /// resolution wants and the reason it is not sorted afterwards.
    fn close_scope(&mut self, widget: &'a dyn Widget<Msg>, stops_before: usize) {
        let range = stops_before..self.focusables.len();
        if let Some(on_key) = widget.on_keystroke() {
            self.listeners.push((range.clone(), on_key));
        }
        let (strokes, callbacks) = (widget.shortcut_bindings(), widget.shortcut_callbacks());
        let (actions, listeners) = (widget.action_bindings(), widget.action_listeners());
        if strokes.is_empty() && callbacks.is_empty() && actions.is_empty() && listeners.is_empty()
        {
            return;
        }
        self.scopes.push(Scope {
            strokes: strokes.to_vec(),
            callbacks: callbacks.to_vec(),
            actions: actions.to_vec(),
            listeners: listeners.to_vec(),
            range,
        });
    }

    fn walk_scoped(
        &mut self,
        widget: &'a dyn Widget<Msg>,
        id: WidgetId,
        translation: (f32, f32),
        clip: Rect,
        rects: &[Rect],
        index: &mut usize,
    ) {
        self.claim_overflows(id, translation);
        // A **refresh area**: the subtree is walked with this widget named as the host, so
        // every scrollable inside records where to send the movement its physics refuses at
        // the top edge. The indicator is then painted **over** the subtree — painting it in
        // the widget's own `paint` would put it under the list it belongs to.
        if let Some(spec) = widget.refresh() {
            let viewport = rects[*index]
                .translate(translation.0, translation.1)
                .intersect(clip);
            let outer = self.refresh_host.replace(id);
            self.walk_node(widget, id, translation, clip, rects, index);
            self.refresh_host = outer;

            let area = Refreshable { id, viewport, spec };
            if let Some(pull) = self.runtime.refresh.get(&id) {
                crate::refresh::paint_refresh(&mut self.scene, &area, pull, &self.theme, clip);
                // A pull that is settling, spinning or fading away drives itself, so the
                // frame after this one has to happen.
                self.wants_animation = true;
            }
            self.refreshes.push(area);
            return;
        }

        // A **barrier** (`IgnorePointer`, `AbsorbPointer`, a hidden `Visibility`,
        // `ExcludeSemantics`): the subtree is walked exactly as usual, and then whatever it
        // added to the withheld registries is dropped again.
        //
        // Removing afterwards rather than skipping the walk is what makes it exact. A widget
        // deep inside registers its click target, focus stop, scrollable area or accessibility
        // node without knowing that something above is holding the subtree out of the frame;
        // truncating at the barrier catches every one of them, including those added by
        // widgets written after this code. It also keeps the walk's rect indexing untouched,
        // which a skipped subtree would break.
        if let Some(barrier) = widget.barrier().filter(|b| !b.is_none()) {
            let base = self.barrier_base();
            // The barrier's own box, before `walk_node` advances the index past it.
            let own = rects[*index]
                .translate(translation.0, translation.1)
                .intersect(clip);
            self.walk_node(widget, id, translation, clip, rects, index);
            self.apply_barrier(barrier, &base, id, own);
            return;
        }

        // An opacity group: the subtree is painted normally, then its range of primitives is
        // **drained** into a composited layer at the group's opacity — so overlaps do not
        // double-blend. The animated opacity is the value the runtime tweened; otherwise the
        // fixed target. Opaque (≈1): no layer at all (zero cost).
        if let Some(target) = widget.opacity_group() {
            let opacity = self.runtime.value_or(id, target).clamp(0.0, 1.0);
            if opacity < 0.999 {
                let start = self.scene.primitives().len();
                self.walk_node(widget, id, translation, clip, rects, index);
                let group = self.scene.split_off(start);
                self.scene.push_primitive(Primitive::Layer {
                    primitives: group,
                    opacity,
                    clip,
                    clip_shape: ClipShape::Rect,
                    transform: None,
                    filter: LayerFilter::NONE,
                    owner: id.as_u64(),
                });
                return;
            }
            // Fully opaque: an ordinary render (no pointless layer).
        }

        // A **pixel effect** over the subtree (`ColorFiltered`, `ImageFiltered`,
        // `ShaderMask`): painted normally, then drained into a composited layer that
        // carries the effect. The renderer applies it when it composites — the image
        // filter as a pre-pass over the layer texture, the colour filter and the mask
        // in the compositing fragment.
        //
        // The box is passed in because a mask is written in fractions of the box it
        // covers, and this is the first point at which that box is a place on screen.
        if let Some(filter) = widget.layer_filter(crate::widget::FilterContext {
            box_rect: rects[*index].translate(translation.0, translation.1),
            backdrop_group: self.backdrop_group,
        }) {
            // The layer keeps the **ambient** clip rather than its own box: an image
            // filter reaches past the pixels it came from, and cutting it at the box
            // would shave the edge off every blur.
            let start = self.scene.primitives().len();
            self.walk_node(widget, id, translation, clip, rects, index);
            let group = self.scene.split_off(start);
            let (group, filter, clip_box) = fold_filter(group, filter, clip);
            self.scene.push_primitive(Primitive::Layer {
                primitives: group,
                opacity: 1.0,
                clip: clip_box,
                clip_shape: ClipShape::Rect,
                transform: None,
                filter,
                owner: id.as_u64(),
            });
            return;
        }

        // **Shape** clipping: the subtree is painted normally, bounded to the widget's box,
        // then its primitives are **drained** into a composited layer whose shape (rounded
        // corners / ellipse) modulates the alpha — the overflowing corners are erased at
        // compositing time. The shape's box is the layer's clip rect. Two sources: `ClipPath`
        // (an arbitrary path, taking priority) or `ClipRRect`/`ClipOval` (an analytic shape).
        // The local path is **offset to the box's screen position**.
        if widget.clip_path().is_some() || widget.clip_shape().is_some() {
            let box_rect = rects[*index].translate(translation.0, translation.1);
            let shape = widget
                .clip_path()
                .map(|p| ClipShape::Path(p.translated(box_rect.x, box_rect.y)))
                .or_else(|| widget.clip_shape())
                .expect("clip_path or clip_shape is Some");
            let clip_box = clip.intersect(box_rect);
            let start = self.scene.primitives().len();
            self.walk_node(widget, id, translation, clip_box, rects, index);
            let group = self.scene.split_off(start);
            // A clip around a filter is **one** layer, not two. It matters most for a
            // backdrop: the reference's own advice is to wrap one in a clip to give it
            // its shape, and a backdrop pushed a level down would be filtering the
            // clip's contents rather than the frame.
            let (group, filter, clip_box) = fold_filter(group, LayerFilter::NONE, clip_box);
            self.scene.push_primitive(Primitive::Layer {
                primitives: group,
                opacity: 1.0,
                clip: clip_box,
                clip_shape: shape,
                transform: None,
                filter,
                owner: id.as_u64(),
            });
            return;
        }

        // Composable paint transforms (`Transform`: scale and/or rotation; translation goes
        // through the child offset instead). Scale and rotation are melted into **a single
        // affine matrix** `M`, the subtree is painted **flat**, then wrapped in a composited
        // layer transformed by `M`. The hit-test applies `M⁻¹` to the point. The translation,
        // applied upstream through `child_offset`, is the innermost one.
        let scale = widget
            .transform_scale()
            .filter(|(sx, sy, _)| (sx - 1.0).abs() > 1e-4 || (sy - 1.0).abs() > 1e-4);
        let rotate = widget.transform_rotate().filter(|(a, _)| a.abs() > 1e-4);
        if scale.is_some() || rotate.is_some() {
            // The pivots are taken on the **child's** box (the next node in prefix order): it
            // is the child that gets transformed, and its box hugs the content even when the
            // Transform's own box is stretched by the parent.
            let basis = rects.get(*index + 1).copied().unwrap_or(rects[*index]);
            let box_rect = basis.translate(translation.0, translation.1);
            let pivot_of = |align: frus_core::Alignment| {
                Point::new(
                    box_rect.x + box_rect.width * align.fraction_x(),
                    box_rect.y + box_rect.height * align.fraction_y(),
                )
            };
            // The composition: scale (about its own pivot) **then** rotation (about its own).
            // In RTL the world is mirrored → the rotation direction is inverted.
            let mut matrix = Affine::IDENTITY;
            if let Some((sx, sy, pivot_align)) = scale {
                matrix = matrix.then(Affine::scale(sx, sy).about(pivot_of(pivot_align)));
            }
            if let Some((angle, pivot_align)) = rotate {
                let angle = if self.rtl() { -angle } else { angle };
                matrix = matrix.then(Affine::rotation(angle).about(pivot_of(pivot_align)));
            }

            let p0 = self.scene.primitives().len();
            let base = self.xform_base();
            self.walk_node(widget, id, translation, clip, rects, index);
            // Wraps the range of primitives — painted flat — in a layer transformed by `M`
            // (scale/rotation applied at compositing time).
            let group = self.scene.split_off(p0);
            self.scene.push_primitive(Primitive::Layer {
                primitives: group,
                opacity: 1.0,
                clip,
                clip_shape: ClipShape::Rect,
                transform: Some(LayerTransform::new(matrix)),
                filter: LayerFilter::NONE,
                owner: id.as_u64(),
            });
            // Counter-transforms the hit-test and maps the subtree's interaction rects.
            self.transform_interaction_registries(&base, matrix);
            return;
        }

        // The inspector wants a full walk (it collects every node); caching only happens
        // outside inspection.
        if self.inspector.is_none() && widget.repaint_boundary() {
            if let Some(count) = plain_subtree_len(widget) {
                let start = *index;
                let sub = &rects[start..start + count];
                let fp = self.boundary_fingerprint(widget, id, translation, sub);

                // Hit: the same generation (config) + the same fingerprint (state+geometry)
                // ⇒ the paint would be identical. The output is cloned and replayed.
                let hit = {
                    let mut pc = self.runtime.paint_cache.borrow_mut();
                    pc.get(id, fp).and_then(|(rc, any)| {
                        any.downcast_ref::<BoundaryData<Msg>>()
                            .map(|d| (rc, d.clone()))
                    })
                };
                if let Some((rc, data)) = hit {
                    self.runtime.paint_cache.borrow_mut().note_hit();
                    self.splice_boundary(data);
                    *index += rc;
                    return;
                }

                // Miss: paints normally, capturing the subtree's output (primitives +
                // interaction maps) for the next frame.
                let snap = self.snapshot();
                self.walk_node(widget, id, translation, clip, rects, index);
                self.runtime.paint_cache.borrow_mut().note_miss();
                if let Some(data) = self.capture_since(&snap) {
                    self.runtime
                        .paint_cache
                        .borrow_mut()
                        .put(id, fp, count, Box::new(data));
                }
                return;
            }
        }
        self.walk_node(widget, id, translation, clip, rects, index);
    }

    /// Paints a child **flat** (laid out separately, at `translation`, with its own rects)
    /// then wraps it in a composited layer transformed by `matrix` and clipped to `clip`. The
    /// hit-test counter-transforms the point (`M⁻¹`); when `matrix` stays axis-aligned
    /// (scale/translation), the focus / scroll / drag / accessibility bounds are transformed
    /// too. The shared factor of `InteractiveViewer`, `RotatedBox` and `FittedBox`.
    // The child, its identity, and the four things that place it. Splitting them would
    // only mean passing the same values in two hops.
    #[allow(clippy::too_many_arguments)]
    fn emit_transformed_child(
        &mut self,
        child: &'a dyn Widget<Msg>,
        cid: WidgetId,
        translation: (f32, f32),
        clip: Rect,
        child_rects: &[Rect],
        matrix: Affine,
        owner: WidgetId,
    ) {
        let p0 = self.scene.primitives().len();
        let base = self.xform_base();
        let mut child_index = 0;
        self.walk(child, cid, translation, clip, child_rects, &mut child_index);
        let group = self.scene.split_off(p0);
        self.scene.push_primitive(Primitive::Layer {
            primitives: group,
            opacity: 1.0,
            clip,
            clip_shape: ClipShape::Rect,
            transform: Some(LayerTransform::new(matrix)),
            filter: LayerFilter::NONE,
            owner: owner.as_u64(),
        });
        self.transform_interaction_registries(&base, matrix);
    }

    /// Paints one node, **under the theme it asks for**. A [`crate::Themed`] replaces the
    /// ambient theme for itself and everything below it; it is put back on the way out, so
    /// the sibling that follows is untouched. Everything the subtree reads — its paint,
    /// its ink, its focus ring, its layout through `cached_rects`, and any overlay it
    /// defers — goes through this field, which is what makes one swap enough.
    fn walk_node(
        &mut self,
        widget: &'a dyn Widget<Msg>,
        id: WidgetId,
        translation: (f32, f32),
        clip: Rect,
        rects: &[Rect],
        index: &mut usize,
    ) {
        // The box this node is about to be given, read **before** the walk consumes the
        // index: a decoration painted over the children needs it once they are done, and
        // by then `*index` has moved past the whole subtree.
        let over = rects
            .get(*index)
            .copied()
            .map(|r| r.translate(translation.0, translation.1));
        let outer = widget
            .theme_override(&self.theme)
            .map(|theme| std::mem::replace(&mut self.theme, *theme));
        self.walk_node_themed(widget, id, translation, clip, rects, index);
        // Over its own children: the reference's `foregroundDecoration`, and the only
        // point in the walk where a widget paints after its subtree. Still under this
        // node's own theme, since it is this node's decoration.
        if let Some(bounds) = over {
            if let Some(decoration) = widget.foreground(&self.theme) {
                let status = self.full_status(widget, id);
                self.scene.set_clip(clip);
                self.scene.set_owner(id.as_u64());
                self.scene.set_bounds(bounds);
                decoration.paint_into(&mut self.scene, bounds, status.opacity);
                self.scene.set_clip(clip);
            }
        }
        if let Some(outer) = outer {
            self.theme = outer;
        }
    }

    /// The walk proper, with the theme already in place — split out so that no early
    /// return inside it can skip putting the outer theme back.
    fn walk_node_themed(
        &mut self,
        widget: &'a dyn Widget<Msg>,
        id: WidgetId,
        translation: (f32, f32),
        clip: Rect,
        rects: &[Rect],
        index: &mut usize,
    ) {
        let rect = rects[*index];
        *index += 1;
        let draw_rect = rect.translate(translation.0, translation.1);
        self.inspect_enter(widget, id, draw_rect);

        let status = self.full_status(widget, id);
        if widget.continuous() {
            self.wants_animation = true;
        }

        self.scene.set_clip(clip);
        self.scene.set_owner(id.as_u64());
        // The box this widget was given. Text primitives record it: a line of text
        // says only where it starts, and the renderer has to know what it covers to
        // order it against anything else (milestone 295).
        self.scene.set_bounds(draw_rect);
        widget.paint(draw_rect, status, &self.theme, &mut self.scene);
        // A widget may have tightened the clip (TextField, for one): it is restored here.
        self.scene.set_clip(clip);
        // The ink a tap left on this surface: over the surface's own paint, under its
        // children — where a material surface puts it. The box is recorded too, so the
        // shell can start the next splash at the right place, in the right size.
        if let Some(style) = widget.ink(&self.theme) {
            self.inks.push((id, draw_rect));
            if let Some(ripples) = self.runtime.ink.get(&id) {
                ripples.paint(
                    id.as_u64(),
                    draw_rect,
                    style.radius,
                    style.color,
                    &mut self.scene,
                );
                self.scene.set_clip(clip);
            }
        }
        self.draw_focus_ring(draw_rect, &status, widget);

        // A shared element is recorded **whether or not it is on screen**: half way
        // through a transition the one being left behind has usually slid off the edge,
        // and its box is exactly what the flight starts from. Every other registry here
        // is about what a pointer can reach, which is why they are inside the guard.
        if let Some(screen) = self.hero_screen {
            if let Some(tag) = widget.hero_tag() {
                self.heroes.push((
                    HeroSpot {
                        tag,
                        screen,
                        id,
                        rect: draw_rect,
                    },
                    widget,
                ));
            }
        }

        let visible = draw_rect.intersect(clip);
        if visible.width > 0.0 && visible.height > 0.0 {
            if let Some(msg) = widget.on_click() {
                self.hits.push(Hit {
                    id,
                    rect: visible,
                    msg: Some(msg),
                    xform: None,
                });
            }
            if let Some(msg) = widget.on_long_press() {
                self.long_presses.push(Hit {
                    id,
                    rect: visible,
                    msg: Some(msg),
                    xform: None,
                });
            }
            if widget.focusable() && !self.focus_excluded {
                self.focusables.push(Focusable {
                    id,
                    rect: visible,
                    skip: self.focus_skipped || widget.focus_skip_traversal(),
                    order: widget.focus_order().or(self.focus_order),
                    group: self.focus_group,
                });
            }
            if widget.draggable() {
                self.draggables.push((id, visible));
            }
            if let Some(payload) = widget.drag_payload() {
                self.drag_sources.push(DragSource {
                    id,
                    rect: visible,
                    payload,
                });
            }
            if widget.drop_zone() {
                self.drop_zones.push(DropZone { id, rect: visible });
            }
            if widget.reorder_index().is_some() {
                self.reorderables.push((id, visible));
            }
            // The accessibility tree: nodes that carry meaning (a role or a label).
            if let Some(sem) = widget.semantics().filter(|s| s.is_meaningful()) {
                self.semantics.push((id, visible, sem));
            }
            // A **multi-line** field whose content overflows: a scrollable region (wheel and
            // inertia through the generic machinery; the shell follows the caret) plus a
            // draggable scrollbar (mouse and touch).
            if let Some(vp) = widget.text_viewport(draw_rect) {
                if let Some((content_h, visible_h, _, _)) = widget.text_metrics(draw_rect.width, 0)
                {
                    let max_y = (content_h - visible_h).max(0.0);
                    if max_y > 0.0 {
                        self.scrollables.push(Scrollable {
                            id,
                            viewport: vp,
                            max_x: 0.0,
                            max_y,
                            physics: widget.scroll_physics(),
                            refresh: self.refresh_host,
                            page: None,
                            reverse_x: false,
                            reverse_y: false,
                        });
                        let offset_y = self
                            .runtime
                            .scroll
                            .get(&id)
                            .map(|s| s.1)
                            .unwrap_or(0.0)
                            .clamp(0.0, max_y);
                        self.add_scrollbar(id, vp, true, offset_y, max_y, false);
                    }
                }
            }
        }

        if let Some((progress, forward)) = widget.navigator() {
            let bounds = draw_rect;
            let children = widget.children();
            let w = bounds.width;
            if children.len() >= 2 {
                // A transition: two offset screens. The screen "behind" (a negative offset)
                // moves less (parallax) → the sense of depth.
                let dir = if forward { 1.0 } else { -1.0 };
                let raw = [-progress * w * dir, (1.0 - progress) * w * dir];
                let off = [
                    if raw[0] < 0.0 {
                        raw[0] * NAV_PARALLAX
                    } else {
                        raw[0]
                    },
                    if raw[1] < 0.0 {
                        raw[1] * NAV_PARALLAX
                    } else {
                        raw[1]
                    },
                ];
                // Depth order: the one offset furthest left (the back one) goes first.
                let (back, front) = if off[0] <= off[1] { (0, 1) } else { (1, 0) };
                // Where the shared elements of this transition start, in both the
                // registry and the scene: everything after this point belongs to the
                // two screens, and the flight is resolved from it once both are drawn.
                let hero_base = self.heroes.len();
                let scene_base = self.scene.primitives().len();
                let outer_screen = self.hero_screen.replace(back as u8);
                // A screen being left keeps its floating layers to itself.
                // `process_overlays` draws every overlay above the **whole window**, after
                // both screens, so a menu left open on the departing screen is painted on
                // top of the screen that replaced it — opaque, and anchored to a bar the
                // window no longer shows. A device found it; the parallax is why it is not
                // self-correcting, since the outgoing screen travels only 30 % of the width
                // and its anchor never actually leaves.
                //
                // `Navigator::from` inserts the screen being left at index 0, so
                // `children[1]` is always the destination — on a push, on a pop, and under
                // a back gesture alike. Whatever the other one defers is dropped.
                let overlay_base = self.overlays.len();
                self.render_screen(
                    children[back].as_ref(),
                    child_id(id, back, children[back].as_ref()),
                    bounds,
                    off[back],
                    clip,
                );
                if back != 1 {
                    self.overlays.truncate(overlay_base);
                }
                // Darkens the screen behind in proportion to how far it is covered.
                let coverage = (off[back].abs() / (w * NAV_PARALLAX)).min(1.0);
                if coverage > 0.0 {
                    let scrim =
                        Rect::new(bounds.x + off[back], bounds.y, bounds.width, bounds.height);
                    self.scene.set_owner(0);
                    self.scene.set_clip(clip);
                    self.scene
                        .fill_rect(scrim, self.theme.scheme.scrim.with_alpha(0.22 * coverage));
                }
                self.hero_screen = Some(front as u8);
                let overlay_base = self.overlays.len();
                self.render_screen(
                    children[front].as_ref(),
                    child_id(id, front, children[front].as_ref()),
                    bounds,
                    off[front],
                    clip,
                );
                if front != 1 {
                    self.overlays.truncate(overlay_base);
                }
                self.hero_screen = outer_screen;
                self.fly_heroes(hero_base, scene_base, progress);
            } else if let Some(screen) = children.first() {
                self.render_screen(
                    screen.as_ref(),
                    child_id(id, 0, screen.as_ref()),
                    bounds,
                    0.0,
                    clip,
                );
            }
        } else if widget.interactive().is_some() {
            // An **interactive** viewport (`InteractiveViewer`): the child fills the viewport
            // at scale 1, then the retained transform (scale + translation, from the shell's
            // gestures) is applied to it through **a single layer** carrying both the matrix
            // `M` and the clip to the viewport.
            let viewport = draw_rect;
            let content_clip = clip.intersect(viewport);
            self.interactives.push((id, viewport));
            if let Some(content) = widget.children().first() {
                let content = content.as_ref();
                let cid = child_id(id, 0, content);
                let content_rects = self.cached_rects(
                    cid,
                    content,
                    Constraints::definite(Size::new(viewport.width, viewport.height)),
                );
                let matrix = self
                    .runtime
                    .interactive
                    .get(&id)
                    .copied()
                    .unwrap_or_default()
                    .matrix();
                self.emit_transformed_child(
                    content,
                    cid,
                    (viewport.x, viewport.y),
                    content_clip,
                    &content_rects,
                    matrix,
                    id,
                );
            }
        } else if let Some(q) = widget.rotated_quarter_turns() {
            // `RotatedBox`: the child, measured at its **natural** size, is centred in the box
            // (with the dimensions swapped for an odd quarter, see `build_layout`) then rotated
            // about the centre. The rotation, applied at compositing time, does not stay
            // axis-aligned for an odd quarter (the focus bounds are left as they are — the
            // click itself stays correct through `M⁻¹`).
            let box_rect = draw_rect;
            if let Some(child) = widget.children().first() {
                let child = child.as_ref();
                let cid = child_id(id, 0, child);
                let child_rects =
                    self.cached_rects(cid, child, Constraints::scroll(0.0, 0.0, true, true));
                let nat = child_rects
                    .first()
                    .copied()
                    .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
                let center = Point::new(
                    box_rect.x + box_rect.width / 2.0,
                    box_rect.y + box_rect.height / 2.0,
                );
                let translation = (center.x - nat.width / 2.0, center.y - nat.height / 2.0);
                let angle = q.rem_euclid(4) as f32 * std::f32::consts::FRAC_PI_2;
                let angle = if self.rtl() { -angle } else { angle };
                let matrix = Affine::rotation(angle).about(center);
                self.emit_transformed_child(
                    child,
                    cid,
                    translation,
                    clip,
                    &child_rects,
                    matrix,
                    id,
                );
            }
        } else if let Some(fit) = widget.fitted() {
            // `FittedBox`: the child, measured at its **natural** size, is scaled according to
            // the `BoxFit` (uniformly, except for `Fill`), centred and clipped to the box. The
            // scale stays axis-aligned → the focus bounds follow.
            let box_rect = draw_rect;
            let content_clip = clip.intersect(box_rect);
            if let Some(child) = widget.children().first() {
                let child = child.as_ref();
                let cid = child_id(id, 0, child);
                let child_rects =
                    self.cached_rects(cid, child, Constraints::scroll(0.0, 0.0, true, true));
                let nat = child_rects
                    .first()
                    .copied()
                    .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
                let (sx, sy) = fit.scale(
                    Size::new(nat.width, nat.height),
                    Size::new(box_rect.width, box_rect.height),
                );
                let center = Point::new(
                    box_rect.x + box_rect.width / 2.0,
                    box_rect.y + box_rect.height / 2.0,
                );
                let translation = (center.x - nat.width / 2.0, center.y - nat.height / 2.0);
                let matrix = Affine::scale(sx, sy).about(center);
                self.emit_transformed_child(
                    child,
                    cid,
                    translation,
                    content_clip,
                    &child_rects,
                    matrix,
                    id,
                );
            }
        } else if let Some(content) = widget.scroll_content() {
            let axis = widget.scroll_axis();
            let viewport = draw_rect;
            let content_clip = clip.intersect(viewport);
            let (offset_x, offset_y) = self.runtime.scroll.get(&id).copied().unwrap_or((0.0, 0.0));
            // `reverse` applies to the axis this scrolls along; a two-dimensional scroll
            // takes it on the vertical, which is the one a reversed view is ever about.
            let reverse = if widget.scroll_reverse() {
                (axis.free_x() && !axis.free_y(), axis.free_y())
            } else {
                (false, false)
            };

            // The padding is **inside** the viewport and scrolls with the content:
            // room at the end of a feed is reachable by scrolling to it, room taken out
            // of the window would sit there for ever. The reference's `SliverPadding`.
            let pad = widget.scroll_padding();
            let content_rects = self.cached_rects(
                child_id(id, 0, content),
                content,
                Constraints::scroll(
                    (viewport.width - pad.left - pad.right).max(0.0),
                    (viewport.height - pad.top - pad.bottom).max(0.0),
                    axis.free_x(),
                    axis.free_y(),
                ),
            );

            let content_size = content_rects
                .first()
                .copied()
                .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
            // What scrolls is the content **and** its room, which is what makes the far
            // inset reachable rather than decorative.
            let scrolled = Size::new(
                content_size.width + pad.left + pad.right,
                content_size.height + pad.top + pad.bottom,
            );
            let max_x = (scrolled.width - viewport.width).max(0.0);
            let max_y = (scrolled.height - viewport.height).max(0.0);
            self.scrollables.push(Scrollable {
                id,
                viewport,
                max_x,
                max_y,
                physics: widget.scroll_physics(),
                refresh: self.refresh_host,
                page: None,
                reverse_x: reverse.0,
                reverse_y: reverse.1,
            });

            let origin = Scrollable {
                id,
                viewport,
                max_x,
                max_y,
                physics: None,
                refresh: None,
                page: None,
                reverse_x: reverse.0,
                reverse_y: reverse.1,
            }
            .content_origin((offset_x, offset_y), scrolled);
            let content_translation = (
                viewport.x + origin.0 + pad.left,
                viewport.y + origin.1 + pad.top,
            );
            let mut content_index = 0;
            self.walk(
                content,
                child_id(id, 0, content),
                content_translation,
                content_clip,
                &content_rects,
                &mut content_index,
            );

            // The overscroll glow, then the scrollbars, over the content (not
            // clipped by it).
            self.scene.set_clip(clip);
            self.add_overscroll_glow(id, viewport);
            if max_y > 0.0 {
                self.add_scrollbar(id, viewport, true, offset_y, max_y, reverse.1);
            }
            if max_x > 0.0 {
                self.add_scrollbar(id, viewport, false, offset_x, max_x, reverse.0);
            }
        } else if let Some(vlist) = widget.virtual_list() {
            // A virtualised list: only build/lay out/paint the visible window.
            let viewport = draw_rect;
            let content_clip = clip.intersect(viewport);
            let (_, offset_y) = self.runtime.scroll.get(&id).copied().unwrap_or((0.0, 0.0));
            let reverse = widget.scroll_reverse();
            // Room around the items, inside the viewport and scrolling with them. The
            // **leading** inset is the one at the end the items start from, so a
            // reversed list clears its bottom first and both keep their sides.
            let pad = widget.scroll_padding();
            let lead = if reverse { pad.bottom } else { pad.top };
            let content_h = pad.top + pad.bottom + vlist.count as f32 * vlist.item_height;
            let max_y = (content_h - viewport.height).max(0.0);
            self.scrollables.push(Scrollable {
                id,
                viewport,
                max_x: 0.0,
                max_y,
                physics: widget.scroll_physics(),
                refresh: self.refresh_host,
                page: None,
                reverse_x: false,
                reverse_y: reverse,
            });

            if vlist.item_height > 0.0 && vlist.count > 0 {
                // The **window** is the same arithmetic either way, and that is not a
                // coincidence: a reversed list counts its indices from the end, and a
                // reversed offset counts its pixels from the end, so index and offset
                // agree about which way is forward. Only where an item lands differs.
                let first = ((offset_y - lead) / vlist.item_height).floor().max(0.0) as usize;
                let last = ((((offset_y + viewport.height - lead) / vlist.item_height).ceil())
                    .max(0.0) as usize)
                    .min(vlist.count);
                for i in first..last {
                    let item = (vlist.build)(i);
                    let top = if reverse {
                        viewport.y + viewport.height - lead - (i + 1) as f32 * vlist.item_height
                            + offset_y
                    } else {
                        viewport.y + lead + i as f32 * vlist.item_height - offset_y
                    };

                    // **Filled**, not merely constrained: a list hands its children a
                    // box rather than asking them how big they would like to be. The
                    // reference gives a list's children a *tight* cross-axis extent, and
                    // for a fixed-extent list a tight main-axis one too — which is
                    // exactly the pair of numbers here.
                    //
                    // Constrained-but-not-filled, a row whose width nobody set hugged
                    // its own text: a list of coloured rows painted a column of chips
                    // down the left instead of rows across the list. Found on a device
                    // in milestone 349.
                    let item_rects = self.cached_rects(
                        id.child(i),
                        item.as_ref(),
                        Constraints::filled(Size::new(
                            (viewport.width - pad.left - pad.right).max(0.0),
                            vlist.item_height,
                        )),
                    );

                    let mut item_index = 0;
                    self.render_item(
                        item.as_ref(),
                        id.child(i),
                        (viewport.x + pad.left, top),
                        content_clip,
                        &item_rects,
                        &mut item_index,
                    );
                }
            }

            self.scene.set_clip(clip);
            self.add_overscroll_glow(id, viewport);
            if max_y > 0.0 {
                self.add_scrollbar(id, viewport, true, offset_y, max_y, false);
            }
        } else if let Some(pages) = widget.page_view() {
            // A paged view: like a virtualised list turned on its side, and with the
            // page extent — the whole geometry — derived from the viewport rather than
            // given. Only the pages the viewport touches are built.
            let viewport = draw_rect;
            let content_clip = clip.intersect(viewport);
            let snap = pages.snap(viewport);
            let (viewport_along, page_across) = if snap.horizontal {
                (viewport.width, viewport.height)
            } else {
                (viewport.height, viewport.width)
            };
            let total = pages.count as f32 * snap.extent;
            let max = (total - viewport_along).max(0.0);
            // With no retained offset this is the view's **first** frame, and it opens
            // on the page it was asked for. Reading the initial page here rather than
            // correcting it a frame later is what keeps page 0 from flashing past on
            // the way to page 3.
            let along = match self.runtime.scroll.get(&id).copied() {
                Some((x, y)) => {
                    if snap.horizontal {
                        x
                    } else {
                        y
                    }
                }
                None => snap
                    .offset_of(snap.requested.min(pages.count.saturating_sub(1)))
                    .clamp(0.0, max),
            };
            self.scrollables.push(Scrollable {
                id,
                viewport,
                max_x: if snap.horizontal { max } else { 0.0 },
                max_y: if snap.horizontal { 0.0 } else { max },
                physics: widget.scroll_physics(),
                refresh: self.refresh_host,
                page: Some(snap),
                reverse_x: false,
                reverse_y: false,
            });

            if pages.count > 0 {
                let first = (along / snap.extent).floor().max(0.0) as usize;
                let last = (((along + viewport_along) / snap.extent).ceil() as usize)
                    .min(pages.count)
                    .max(first + 1);
                for index in first..last.min(pages.count) {
                    let page = (pages.build)(index);
                    let start = index as f32 * snap.extent - along;
                    let (size, origin) = if snap.horizontal {
                        (
                            Size::new(snap.extent, page_across),
                            (viewport.x + start, viewport.y),
                        )
                    } else {
                        (
                            Size::new(page_across, snap.extent),
                            (viewport.x, viewport.y + start),
                        )
                    };

                    // A page is **given** its box, not asked for one: it fills the
                    // panel even when its content is a single centred line.
                    let page_rects = self.cached_rects(
                        id.child(index),
                        page.as_ref(),
                        Constraints::filled(size),
                    );

                    let mut page_index = 0;
                    self.render_item(
                        page.as_ref(),
                        id.child(index),
                        origin,
                        content_clip,
                        &page_rects,
                        &mut page_index,
                    );
                }
            }

            self.scene.set_clip(clip);
            self.add_overscroll_glow(id, viewport);
        } else if let Some(overflow) = widget.overflow_box() {
            // The child is laid out to constraints of its own — which is why it can
            // come out bigger than the box holding it — then anchored inside it and
            // rendered **without** the box's clip. A spill that got clipped here would
            // be no spill at all; a `ClipRect` above is how a caller asks for one.
            let own = draw_rect;
            if let Some(child) = widget.children().first() {
                let child = child.as_ref();
                let cid = child_id(id, 0, child);
                let asked = overflow.child_size(Size::new(own.width, own.height));
                let child_rects = if overflow.unconstrained {
                    // Unconstrained: the child is asked how big it wants to be, and
                    // gets exactly that.
                    self.cached_rects(cid, child, Constraints::scroll(0.0, 0.0, true, true))
                } else {
                    self.cached_rects(cid, child, Constraints::filled(asked))
                };
                let size = child_rects
                    .first()
                    .map(|r| Size::new(r.width, r.height))
                    .unwrap_or(asked);
                let origin = overflow.origin(own, size);

                let mut child_index = 0;
                self.render_item(child, cid, origin, clip, &child_rects, &mut child_index);
            }
        } else if let Some(build) = widget.layout_builder() {
            // Builds the content from the actual box, then lays it out and renders it inside
            // (like a list item: with no retained state).
            let bounds = draw_rect;
            let content_clip = clip.intersect(bounds);
            let child = build(Size::new(bounds.width, bounds.height));

            // **Filled**, not merely constrained: the content is handed this box, and it
            // was built from this box in the first place. Constraining it instead let a
            // root with no width of its own hug its content — a grid built here laid its
            // columns out at nothing, which is what `GridView::extent`'s first tests found
            // (milestone 356). The same distinction a list's items and a paged view's
            // pages already draw.
            let child_rects = self.cached_rects(
                id.child(0),
                child.as_ref(),
                Constraints::filled(Size::new(bounds.width, bounds.height)),
            );

            let mut child_index = 0;
            self.render_item(
                child.as_ref(),
                id.child(0),
                (bounds.x, bounds.y),
                content_clip,
                &child_rects,
                &mut child_index,
            );
        } else if let Some(spec) = widget.dismissible() {
            // A dismissible item is a stack whose **last** layer — the item itself — is
            // offset by however far it has been swiped, and whose earlier layers, the
            // backgrounds, show only through the strip the item has uncovered. Clipping
            // them is what makes a background read as something the item is sliding
            // *off*, rather than a block that was always there.
            let bounds = draw_rect;
            let state = self.runtime.dismiss.get(&id).copied();
            let progress = state.map(|s| s.progress()).unwrap_or(0.0);
            let layers = widget.children();
            let last = layers.len().saturating_sub(1);
            // Which background: the first covers a swipe towards the end, the second —
            // when there is one — the other way. With only one, it serves both.
            let shown_background = if progress < 0.0 && last >= 2 { 1 } else { 0 };
            let strip = crate::dismiss::revealed_strip(bounds, progress);

            for (i, layer) in layers.iter().enumerate() {
                let is_item = i == last;
                if !is_item && (i != shown_background || progress == 0.0) {
                    // Not the background this direction shows: skip it, and skip the
                    // rects it would have consumed — it is laid out separately, so
                    // there is no index to keep in step.
                    continue;
                }
                let offset = if is_item {
                    if spec.axis.is_horizontal() {
                        (bounds.width * progress, 0.0)
                    } else {
                        (0.0, bounds.height * progress)
                    }
                } else {
                    (0.0, 0.0)
                };
                let layer_clip = if is_item {
                    clip.intersect(bounds)
                } else {
                    clip.intersect(strip)
                };
                if layer_clip.width <= 0.0 || layer_clip.height <= 0.0 {
                    continue;
                }
                let cid = child_id(id, i, layer.as_ref());
                let layer_rects = self.cached_rects(
                    cid,
                    layer.as_ref(),
                    Constraints::filled(Size::new(bounds.width, bounds.height)),
                );
                let mut layer_index = 0;
                self.walk(
                    layer.as_ref(),
                    cid,
                    (bounds.x + offset.0, bounds.y + offset.1),
                    layer_clip,
                    &layer_rects,
                    &mut layer_index,
                );
            }

            self.dismissables.push(Dismissable {
                id,
                rect: bounds,
                spec,
            });
            // A swipe that is settling, flying or collapsing drives itself.
            if state.is_some_and(|s| s.phase() != DismissPhase::Drag) {
                self.wants_animation = true;
            }
        } else if widget.stack() {
            // A stack: the layers are rendered in order, bottom first.
            let bounds = draw_rect;
            let layer_clip = clip.intersect(bounds);
            let loose = widget.stack_loose();
            // Where a layer smaller than the stack sits. Resolved once, against the
            // reading direction, so a start-anchored badge follows the script.
            let align = widget
                .alignment_geometry()
                .unwrap_or(frus_core::AlignmentGeometry::Physical(
                    frus_core::Alignment::TOP_LEFT,
                ))
                .resolve(if self.rtl() {
                    frus_core::TextDirection::Rtl
                } else {
                    frus_core::TextDirection::Ltr
                });
            for (i, layer) in widget.children().iter().enumerate() {
                let cid = child_id(id, i, layer.as_ref());
                // What this layer asks for on each axis: a pinned pair gives a number, a
                // lone pin plus an explicit extent gives one, and neither leaves the axis
                // to the layer itself — or, under `Expand`, to the stack.
                let pins = layer.positioned();
                let (want_w, want_h) = match pins {
                    Some(p) => (
                        p.resolved_width(bounds.width),
                        p.resolved_height(bounds.height),
                    ),
                    None if loose => (None, None),
                    // The historical behaviour, and still the default: a layer is
                    // **given** the box rather than asked what size it would like. An
                    // unsized layer that hugged its content would collapse to nothing —
                    // invisibly, since a stack draws no box of its own.
                    None => (Some(bounds.width), Some(bounds.height)),
                };
                // A pinned layer is **forced** into what its edges say: two opposite
                // pins are not a suggestion, and a width of its own is a contradiction
                // rather than a second opinion. An unpinned layer under `Expand` is
                // merely handed the box, which leaves a layer that chose a size alone —
                // that is what a badge sitting in a filled stack has always relied on.
                // An axis nobody pinned is left free, so the layer's own size comes back.
                let layer_rects = if pins.is_some() {
                    self.cached_rects(
                        cid,
                        layer.as_ref(),
                        Constraints::pinned(want_w, want_h, Size::new(bounds.width, bounds.height)),
                    )
                } else {
                    match (want_w, want_h) {
                        (Some(w), Some(h)) => self.cached_rects(
                            cid,
                            layer.as_ref(),
                            Constraints::filled(Size::new(w, h)),
                        ),
                        _ => self.cached_rects(
                            cid,
                            layer.as_ref(),
                            Constraints::scroll(bounds.width, bounds.height, true, true),
                        ),
                    }
                };
                let own = layer_rects
                    .first()
                    .copied()
                    .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
                let (dx, dy) = match pins {
                    Some(p) => (
                        Positioning::place(
                            p.left,
                            p.right,
                            bounds.width,
                            own.width,
                            align.fraction_x(),
                        ),
                        Positioning::place(
                            p.top,
                            p.bottom,
                            bounds.height,
                            own.height,
                            align.fraction_y(),
                        ),
                    ),
                    None => (
                        (bounds.width - own.width).max(0.0) * align.fraction_x(),
                        (bounds.height - own.height).max(0.0) * align.fraction_y(),
                    ),
                };
                let mut layer_index = 0;
                self.walk(
                    layer.as_ref(),
                    cid,
                    (bounds.x + dx, bounds.y + dy),
                    layer_clip,
                    &layer_rects,
                    &mut layer_index,
                );
            }
        } else if let Some((content, placement)) = widget.overlay() {
            // The anchor (child 0) is rendered inline; the overlay (child 1) is deferred.
            self.walk(
                widget.children()[0].as_ref(),
                child_id(id, 0, widget.children()[0].as_ref()),
                translation,
                clip,
                rects,
                index,
            );
            // The appearance progress: for an animated overlay (a drawer), the value the
            // runtime interpolated; otherwise `1.0` (shown at once). With no value recorded
            // (an isolated render), the target is adopted immediately.
            let target = widget.anim_target().unwrap_or(1.0);
            let progress = self.runtime.value_or(id, target);
            // A tooltip only shows while the anchor is hovered; an animated overlay disappears
            // once its progress has fallen back to zero.
            let visible = match placement {
                Placement::Tooltip => self.runtime.input.hovered == Some(id.child(0)),
                _ => true,
            };
            if visible && progress > 0.001 {
                self.overlays.push((
                    content,
                    child_id(id, 1, content),
                    draw_rect,
                    placement,
                    widget.overlay_dismiss(),
                    progress,
                    widget.overlay_traps_focus(),
                    self.theme,
                ));
            }
        } else {
            let children = widget.children();
            // Fractional alignment (`Container::alignment`) + paint offset
            // (`Transform::translate`): both offset the subtree. `(0, 0)` otherwise → the
            // ordinary flex walk.
            let extra = self.child_offset(widget, id, rect, rects, *index, children);
            for (child_index, child) in children.iter().enumerate() {
                self.walk(
                    child.as_ref(),
                    child_id(id, child_index, child.as_ref()),
                    (translation.0 + extra.0, translation.1 + extra.1),
                    clip,
                    rects,
                    index,
                );
            }
        }
        self.depth -= 1;
    }

    /// Offset to apply to a child's subtree: the sum of the **fractional alignment**
    /// (`Widget::alignment`, a single child slid through the free space) and the **paint
    /// offset** (`Transform::translate`, the whole subtree offset without touching layout).
    /// `(0, 0)` by default. Since it is added to the walk's translation, this offset follows
    /// both the primitives **and** the hit-test.
    fn child_offset(
        &self,
        widget: &dyn Widget<Msg>,
        id: WidgetId,
        container: Rect,
        rects: &[Rect],
        child_index: usize,
        children: &[Box<dyn Widget<Msg>>],
    ) -> (f32, f32) {
        let mut off = (0.0, 0.0);

        // Fractional alignment: it targets a single child. taffy laid the child out at the
        // top left (Start/Start) at its natural size; it is then slid by `free × fraction`.
        // A **stack** answers this question too, and means something else by it: where a
        // layer smaller than the box sits, which the stack branch applies per layer. It
        // must not also be applied to the stack itself, or a stack with exactly one layer
        // would shift twice.
        if let (Some(geo), 1, false) = (widget.alignment_geometry(), children.len(), widget.stack())
        {
            // Resolves the alignment (physical or directional) against the reading direction;
            // `resolve` produces a physical `Alignment` that the rest (with its RTL correction)
            // handles uniformly.
            let direction = if self.rtl() {
                frus_core::TextDirection::Rtl
            } else {
                frus_core::TextDirection::Ltr
            };
            let align = geo.resolve(direction);
            let child = rects[child_index];
            let pad = effective_style(widget, id, self.runtime, &self.theme).padding;
            let free_w = (container.width - pad.left - pad.right - child.width).max(0.0);
            let free_h = (container.height - pad.top - pad.bottom - child.height).max(0.0);
            // In RTL, taffy lays the child out on the left and `mirror` has sent it back to
            // the right: the baseline is therefore already right-aligned. 1 is subtracted so
            // the fraction stays **physical** (x = +1 ⇒ the right in both directions).
            let fx = align.fraction_x() - if self.rtl() { 1.0 } else { 0.0 };
            off.0 += free_w * fx;
            off.1 += free_h * align.fraction_y();
        }

        // Paint offset (`Transform::translate`): in RTL the world's x axis is mirrored, so a
        // logical +x offset ("towards the end") points left — the sign is inverted to stay
        // consistent with the reading direction.
        if let Some((tx, ty)) = widget.transform_translate() {
            off.0 += if self.rtl() { -tx } else { tx };
            off.1 += ty;
        }

        off
    }

    /// Records the widget in the inspector's collection (when it is on) and opens a depth
    /// level — every render path closes it again at the end of the function
    /// (`self.depth -= 1`).
    fn inspect_enter(&mut self, widget: &dyn Widget<Msg>, id: WidgetId, draw_rect: Rect) {
        if let Some(nodes) = &mut self.inspector {
            nodes.push(crate::inspector::InspectorNode {
                id,
                rect: draw_rect,
                name: widget.debug_name(),
                depth: self.depth,
            });
        }
        self.depth += 1;
    }

    /// A widget's full status: pointer interaction + focus + animation progresses + the
    /// cursor/selection when there is one.
    fn full_status(&self, widget: &dyn Widget<Msg>, id: WidgetId) -> crate::interaction::Status {
        let mut status = self.runtime.input.status_for(id);
        status.hover_progress = self.runtime.hover_progress(id);
        status.focus_progress = self.runtime.focus_progress(id);
        status.opacity = self.runtime.opacity(id);
        // A widget's own animated value, or **its target** where the runtime has never
        // heard of it: the same rule the runtime applies on mount (adopt, do not animate
        // in from zero). Without the fallback an isolated render — a test, a frame built
        // before the loop has advanced anything — draws every such widget at zero: a
        // switch that is on drawn off, an indicator under the wrong tab.
        status.value = match widget.anim_target() {
            Some(target) => self.runtime.value_or(id, target),
            None => self.runtime.value(id),
        };
        status.anim_color = self.runtime.anim_color(id);
        status.anim_radius = self.runtime.anim_radius(id);
        status.time = self.runtime.time;
        status.drag_over = self.runtime.drag_over == Some(id);
        status.scroll_y = self.runtime.scroll.get(&id).map(|s| s.1).unwrap_or(0.0);
        if status.focused {
            if let Some(edit) = self.runtime.edits.get(&id) {
                status.cursor = Some(edit.cursor);
                status.selection = edit.selection_range();
                status.composing = edit.composing;
            }
        }
        status
    }

    /// The generic focus ring (for widgets that do not draw their own).
    fn draw_focus_ring(
        &mut self,
        draw_rect: Rect,
        status: &crate::interaction::Status,
        widget: &dyn Widget<Msg>,
    ) {
        // The generic ring only appears when the last interaction was a **keyboard** one
        // (`focus_visible`) — a click must not flash a ring.
        if status.focused
            && self.runtime.focus_visible
            && widget.focusable()
            && !widget.draws_own_focus()
        {
            let ring = Rect::new(
                draw_rect.x - 2.0,
                draw_rect.y - 2.0,
                draw_rect.width + 4.0,
                draw_rect.height + 4.0,
            );
            let alpha = 0.4 + 0.6 * status.focus_progress.clamp(0.0, 1.0);
            self.scene.draw_rect(
                ring,
                Color::TRANSPARENT,
                self.theme.radius + 2.0,
                2.0,
                self.theme.focus.fade(alpha),
            );
        }
    }

    /// Renders a **virtualised list item**: built on the fly, it cannot defer an overlay
    /// (hence a render of its own, without the special branches).
    fn render_item(
        &mut self,
        widget: &dyn Widget<Msg>,
        id: WidgetId,
        translation: (f32, f32),
        clip: Rect,
        rects: &[Rect],
        index: &mut usize,
    ) {
        let rect = rects[*index];
        *index += 1;
        let draw_rect = rect.translate(translation.0, translation.1);
        self.inspect_enter(widget, id, draw_rect);

        let status = self.full_status(widget, id);
        if widget.continuous() {
            self.wants_animation = true;
        }
        self.scene.set_clip(clip);
        self.scene.set_owner(id.as_u64());
        // The box this widget was given. Text primitives record it: a line of text
        // says only where it starts, and the renderer has to know what it covers to
        // order it against anything else (milestone 295).
        self.scene.set_bounds(draw_rect);
        widget.paint(draw_rect, status, &self.theme, &mut self.scene);
        self.scene.set_clip(clip);
        self.draw_focus_ring(draw_rect, &status, widget);

        let visible = draw_rect.intersect(clip);
        if visible.width > 0.0 && visible.height > 0.0 {
            if let Some(msg) = widget.on_click() {
                self.hits.push(Hit {
                    id,
                    rect: visible,
                    msg: Some(msg),
                    xform: None,
                });
            }
            if let Some(msg) = widget.on_long_press() {
                self.long_presses.push(Hit {
                    id,
                    rect: visible,
                    msg: Some(msg),
                    xform: None,
                });
            }
            if widget.focusable() && !self.focus_excluded {
                self.focusables.push(Focusable {
                    id,
                    rect: visible,
                    skip: self.focus_skipped || widget.focus_skip_traversal(),
                    order: widget.focus_order().or(self.focus_order),
                    group: self.focus_group,
                });
            }
            if widget.draggable() {
                self.draggables.push((id, visible));
            }
            if widget.reorder_index().is_some() {
                self.reorderables.push((id, visible));
            }
            // The accessibility tree: nodes that carry meaning (a role or a label).
            if let Some(sem) = widget.semantics().filter(|s| s.is_meaningful()) {
                self.semantics.push((id, visible, sem));
            }
        }

        let children = widget.children();
        // Fractional alignment + paint offset, as in the main walk (a virtualised-list /
        // `layout_builder` child may itself be an aligned or transformed container).
        let extra = self.child_offset(widget, id, rect, rects, *index, children);
        for (child_index, child) in children.iter().enumerate() {
            self.render_item(
                child.as_ref(),
                child_id(id, child_index, child.as_ref()),
                (translation.0 + extra.0, translation.1 + extra.1),
                clip,
                rects,
                index,
            );
        }
        self.depth -= 1;
    }

    /// Lays out a full-window screen and renders it offset by `off_x`.
    fn render_screen(
        &mut self,
        screen: &'a dyn Widget<Msg>,
        id: WidgetId,
        bounds: Rect,
        off_x: f32,
        clip: Rect,
    ) {
        let rects = self.cached_rects(
            id,
            screen,
            Constraints::definite(Size::new(bounds.width, bounds.height)),
        );
        let screen_clip = clip.intersect(bounds);
        let mut index = 0;
        self.walk(
            screen,
            id,
            (bounds.x + off_x, bounds.y),
            screen_clip,
            &rects,
            &mut index,
        );
    }

    /// Processes the deferred overlays: sub-layout, positioning and rendering **above**
    /// everything (their clickable areas win). May spawn further overlays (nested portals).
    fn process_overlays(&mut self) {
        let window = Rect::new(0.0, 0.0, self.available.width, self.available.height);
        while let Some((content, oid, anchor, placement, dismiss, progress, traps, theme)) =
            self.overlays.pop()
        {
            // The theme this overlay was declared under, for the whole of its layout and
            // painting; whatever was in force when the walk ended is put back after it,
            // so two overlays declared under two themes do not bleed into each other.
            let outer = std::mem::replace(&mut self.theme, theme);
            // Drawers slide along a **spring curve** (a gentle arrival), not linearly; the
            // other overlays keep their raw progress.
            let progress = if matches!(
                placement,
                Placement::Left | Placement::Right | Placement::Bottom
            ) {
                crate::runtime::spring_ease(progress)
            } else {
                progress
            };
            // The content's natural size. A drawer (`Left`) is constrained in height to the
            // window (so its `Percent(1.0)` panel unfolds) with a free width; the other
            // overlays take their natural size.
            let (free_x, free_y) = match placement {
                Placement::Left | Placement::Right => (true, false),
                // The sheet is full-width (constrained to the window) with its natural height:
                // its `Percent(1.0)` panel unfolds across the width.
                Placement::Bottom => (false, true),
                _ => (true, true),
            };
            let rects = self.cached_rects(
                oid,
                content,
                Constraints::scroll(self.available.width, self.available.height, free_x, free_y),
            );
            let size = rects
                .first()
                .copied()
                .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));

            // A drawer's slide-in from the left / right edge.
            let from_left = -(1.0 - progress) * size.width;
            let from_right = self.available.width - progress * size.width;
            // An anchored menu/tooltip: aligned to the anchor's **start** edge — in RTL that
            // edge is the right one (the menu opens leftwards).
            let anchor_x = if self.rtl() {
                anchor.x + anchor.width - size.width
            } else {
                anchor.x
            };
            let mut pos = match placement {
                Placement::Below => (anchor_x, anchor.y + anchor.height + 4.0),
                Placement::Center => (
                    (self.available.width - size.width) * 0.5,
                    (self.available.height - size.height) * 0.5,
                ),
                Placement::Tooltip => (anchor_x, anchor.y - size.height - 6.0),
                // `Left` = a drawer on the **start** side; in RTL, start = the right.
                Placement::Left if self.rtl() => (from_right, 0.0),
                Placement::Left => (from_left, 0.0),
                // `Right` = the **end** side; in RTL, end = the left.
                Placement::Right if self.rtl() => (from_left, 0.0),
                Placement::Right => (from_right, 0.0),
                // The sheet slides up from the bottom: its bottom edge stays flush with the
                // window, offset downwards by `(1-progress)·height`.
                Placement::Bottom => (0.0, self.available.height - progress * size.height),
            };

            // An **anchored** overlay follows a widget; the others (a modal, a drawer, a
            // sheet) are positioned against the window and have no anchor worth the name.
            // Only the first kind can find its anchor gone from the window: a screen
            // sliding out under a `Navigator`, a row scrolled sideways past the edge.
            let anchored = matches!(placement, Placement::Below | Placement::Tooltip);
            let anchor_on_screen = !anchored
                || (anchor.x < self.available.width
                    && anchor.x + anchor.width > 0.0
                    && anchor.y < self.available.height
                    && anchor.y + anchor.height > 0.0);

            // Auto-flip: when an anchored overlay overflows an edge, it is flipped / nudged
            // back inside the window.
            //
            // This is for a menu opened near the right margin, and it assumes the anchor is
            // something the window is showing. When the anchor has **left** the window the
            // nudge does the opposite of its job: it drags a departing screen's menu back
            // into view, fully opaque, over the screen that replaced it. An overlay whose
            // anchor is off screen goes off screen with it.
            if anchored && anchor_on_screen {
                // A vertical overflow → flip to the other side of the anchor.
                if placement == Placement::Below
                    && pos.1 + size.height > self.available.height
                    && anchor.y - size.height - 4.0 >= 0.0
                {
                    pos.1 = anchor.y - size.height - 4.0;
                } else if placement == Placement::Tooltip
                    && pos.1 < 0.0
                    && anchor.y + anchor.height + size.height + 6.0 <= self.available.height
                {
                    pos.1 = anchor.y + anchor.height + 6.0;
                }
                // A horizontal overflow → nudge back inside the window.
                if pos.0 + size.width > self.available.width {
                    pos.0 = (self.available.width - size.width).max(0.0);
                }
                if pos.0 < 0.0 {
                    pos.0 = 0.0;
                }
            }

            if matches!(
                placement,
                Placement::Center | Placement::Left | Placement::Right | Placement::Bottom
            ) {
                // The scrim behind the modal / the drawer (the `scrim` role), modulated by the
                // progress (a fade synchronised with the slide).
                self.scene.set_owner(0);
                self.scene.set_clip(window);
                self.scene
                    .fill_rect(window, self.theme.scheme.scrim.with_alpha(0.5 * progress));
            }

            // Dismissal on a click **outside** the content (a modal, a menu…): a full-screen
            // hit added **before** the content, so the content beats it where they overlap.
            // The dismissal is also remembered for Escape (the last one rendered = the
            // topmost).
            //
            // Gated on the same condition as the nudge, and for the same reason: a
            // window-wide barrier belonging to an overlay nobody can see would swallow the
            // next press anywhere on the screen that replaced it.
            if let Some(msg) = dismiss.filter(|_| anchor_on_screen) {
                self.dismisses.push(msg.clone());
                self.hits.push(Hit {
                    id: oid,
                    rect: window,
                    msg: Some(msg),
                    xform: None,
                });
            }

            // A **modal** (scrimmed) or **trapping anchored** (menu) overlay: its focusables
            // form a **scope** that traps Tab/arrows/click-to-focus. The last one rendered (the
            // topmost) wins; anchored overlays that do not trap (a tooltip, an autocomplete
            // list) leave the focus alone.
            let modal = matches!(
                placement,
                Placement::Center | Placement::Left | Placement::Right | Placement::Bottom
            );
            if modal || traps {
                self.focus_scope_start = Some(self.focusables.len());
            }

            let mut index = 0;
            self.walk(content, oid, pos, window, &rects, &mut index);
            self.theme = outer;
        }
    }
}

impl<Msg: Clone> Builder<'_, Msg> {
    /// Draws a scrollbar (track + thumb) and registers it for hit-testing a drag.
    /// Paints the region's overscroll glows over its viewport.
    ///
    /// The colour is the scheme's **secondary**: the edge feedback is an
    /// acknowledgement, not a call to action, so it must read as part of the surface
    /// rather than compete with whatever primary-coloured control sits nearby.
    fn add_overscroll_glow(&mut self, id: WidgetId, viewport: Rect) {
        if let Some(glows) = self.runtime.scroll_glow.get(&id) {
            glows.paint(viewport, self.theme.scheme.secondary, &mut self.scene);
        }
    }

    fn add_scrollbar(
        &mut self,
        id: WidgetId,
        viewport: Rect,
        vertical: bool,
        offset: f32,
        max: f32,
        reverse: bool,
    ) {
        let (track_start, track_len, content_len) = if vertical {
            (viewport.y, viewport.height, viewport.height + max)
        } else {
            (viewport.x, viewport.width, viewport.width + max)
        };
        let thumb_len = (track_len * track_len / content_len)
            .max(MIN_THUMB)
            .min(track_len);
        let travel = track_len - thumb_len;
        // Along a reversed axis the numbers run the other way, so the thumb does too:
        // offset 0 is the far end, and that is where it rests.
        let fraction = if max > 0.0 { offset / max } else { 0.0 };
        let fraction = if reverse { 1.0 - fraction } else { fraction };
        let thumb_pos = track_start + fraction * travel;

        let (track, thumb) = if vertical {
            let x = viewport.x + viewport.width - BAR_SIZE;
            (
                Rect::new(x, viewport.y, BAR_SIZE, viewport.height),
                Rect::new(x + 1.0, thumb_pos, BAR_SIZE - 2.0, thumb_len),
            )
        } else {
            let y = viewport.y + viewport.height - BAR_SIZE;
            (
                Rect::new(viewport.x, y, viewport.width, BAR_SIZE),
                Rect::new(thumb_pos, y + 1.0, thumb_len, BAR_SIZE - 2.0),
            )
        };

        let track_color = self.theme.muted.fade(0.18);
        let thumb_color = self.theme.muted.fade(0.55);
        self.scene
            .draw_rect(track, track_color, BAR_SIZE * 0.5, 0.0, Color::TRANSPARENT);
        self.scene.draw_rect(
            thumb,
            thumb_color,
            (BAR_SIZE - 2.0) * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        self.scrollbars.push(Scrollbar {
            id,
            vertical,
            thumb,
            track_start,
            track_len,
            thumb_len,
            reverse,
            max,
        });
    }
}

/// Turns a widget tree into a [`Ui`] for a given size, runtime state and theme.
pub fn build_ui<'a, Msg: Clone + 'static>(
    root: &'a dyn Widget<Msg>,
    available: Size,
    runtime: &'a Runtime,
    theme: &'a Theme,
) -> Ui<Msg> {
    build_ui_impl(root, available, runtime, theme, false).0
}

/// Like [`build_ui`], but it also collects the **inspection nodes** (one per painted widget:
/// name, box, depth) — the raw material of the runtime inspector and of the diagnostic dump.
/// Reserve it for the frames where the inspector is on.
pub fn build_ui_inspected<'a, Msg: Clone + 'static>(
    root: &'a dyn Widget<Msg>,
    available: Size,
    runtime: &'a Runtime,
    theme: &'a Theme,
) -> (Ui<Msg>, Vec<crate::inspector::InspectorNode>) {
    let (ui, nodes) = build_ui_impl(root, available, runtime, theme, true);
    (ui, nodes.unwrap_or_default())
}

fn build_ui_impl<'a, Msg: Clone + 'static>(
    root: &'a dyn Widget<Msg>,
    available: Size,
    runtime: &'a Runtime,
    theme: &'a Theme,
    inspect: bool,
) -> (Ui<Msg>, Option<Vec<crate::inspector::InspectorNode>>) {
    let (mut rects, overflows) = runtime.layout_cache.borrow_mut().rects(
        WidgetId::ROOT,
        root,
        runtime,
        theme,
        Constraints::definite(available),
    );

    let mut builder = Builder {
        scene: Scene::new(),
        hits: Vec::new(),
        long_presses: Vec::new(),
        dismisses: Vec::new(),
        focusables: Vec::new(),
        focus_scope_start: None,
        scrollables: Vec::new(),
        scrollbars: Vec::new(),
        draggables: Vec::new(),
        drag_sources: Vec::new(),
        drop_zones: Vec::new(),
        inks: Vec::new(),
        reorderables: Vec::new(),
        interactives: Vec::new(),
        overlays: Vec::new(),
        wants_animation: false,
        available,
        runtime,
        theme: *theme,
        inspector: inspect.then(Vec::new),
        depth: 0,
        refresh_host: None,
        hero_screen: None,
        heroes: Vec::new(),
        refreshes: Vec::new(),
        dismissables: Vec::new(),
        semantics: Vec::new(),
        focus_excluded: false,
        backdrop_group: None,
        focus_skipped: false,
        focus_order: None,
        focus_group: None,
        scopes: Vec::new(),
        listeners: Vec::new(),
        overflows: std::cell::RefCell::new(Vec::new()),
        pending_overflows: std::cell::RefCell::new(std::collections::HashMap::new()),
    };
    // The root is mirrored in RTL (like every layout root).
    builder.mirror(&mut rects);
    builder.record_overflows(WidgetId::ROOT, &rects, overflows);
    let mut index = 0;
    builder.walk(
        root,
        WidgetId::ROOT,
        (0.0, 0.0),
        Rect::UNBOUNDED,
        &rects,
        &mut index,
    );

    // Overlays (floating menus, modals, tooltips) above everything else. (Their walk restarts
    // from depth 0: roots as far as the inspector is concerned.)
    builder.process_overlays();

    // End of frame: forget the layout roots and repaint boundaries of widgets that have gone,
    // and freeze the caches' diagnostic counters.
    runtime.layout_cache.borrow_mut().end_frame();
    runtime.paint_cache.borrow_mut().end_frame();

    // Replays the outgoing subtrees, fading out, over the current scene.
    builder.scene.set_clip(Rect::UNBOUNDED);
    for (primitives, opacity) in runtime.leaving.values() {
        for primitive in primitives {
            builder.scene.push_faded(primitive, *opacity);
        }
    }

    // A box whose children ran past it now **says so on the screen**, not only in the
    // console — see `paint_overflow_bands`. Last, so nothing paints over it.
    paint_overflow_bands(&mut builder.scene, &builder.overflows.borrow());

    let ui = Ui {
        scene: builder.scene,
        hits: builder.hits,
        long_presses: builder.long_presses,
        dismisses: builder.dismisses,
        focusables: builder.focusables,
        focus_scope_start: builder.focus_scope_start,
        scrollables: builder.scrollables,
        scrollbars: builder.scrollbars,
        draggables: builder.draggables,
        drag_sources: builder.drag_sources,
        drop_zones: builder.drop_zones,
        inks: builder.inks,
        reorderables: builder.reorderables,
        interactives: builder.interactives,
        refreshes: builder.refreshes,
        dismissables: builder.dismissables,
        // An image is still on its way: keep drawing, or the frame that would show it
        // never happens.
        //
        // A **count**, asked once here, and not a flag on the widget. Showing a
        // placeholder means taking the image out of the tree, and the fetch is still
        // going on when the widget that started it is gone -- so a hook the walk reads
        // off `Image` would go quiet at exactly the moment it is needed.
        wants_animation: builder.wants_animation || frus_core::images_in_flight() > 0,
        semantics: builder.semantics,
        overflows: builder.overflows.into_inner(),
        scopes: builder.scopes,
        listeners: builder.listeners,
    };
    (ui, builder.inspector)
}

/// Collects the identities of every widget in the tree (prefix order), following the same
/// positional scheme as [`build_ui`]. Used to detect mounts/unmounts between two frames.
pub fn collect_ids<Msg>(root: &dyn Widget<Msg>) -> Vec<WidgetId> {
    fn walk<Msg>(widget: &dyn Widget<Msg>, id: WidgetId, out: &mut Vec<WidgetId>) {
        out.push(id);
        for (index, child) in widget.children().iter().enumerate() {
            walk(child.as_ref(), child_id(id, index, child.as_ref()), out);
        }
    }
    let mut out = Vec::new();
    walk(root, WidgetId::ROOT, &mut out);
    out
}

/// Identities of the **subtree** rooted at `root_id` — `widget` being the widget with that
/// identity — that is `[root_id, …descendants]`, derived by the same positional scheme as
/// [`collect_ids`].
///
/// Used by the **drag ghost**: a **rich** card's primitives are painted by its children (owners
/// other than the card itself). To capture all of its visuals, the shell gathers the primitives
/// of the **whole** subtree, not only the card's own.
pub fn subtree_ids<Msg>(widget: &dyn Widget<Msg>, root_id: WidgetId) -> Vec<WidgetId> {
    fn walk<Msg>(widget: &dyn Widget<Msg>, id: WidgetId, out: &mut Vec<WidgetId>) {
        out.push(id);
        for (index, child) in widget.children().iter().enumerate() {
            walk(child.as_ref(), child_id(id, index, child.as_ref()), out);
        }
    }
    let mut out = Vec::new();
    walk(widget, root_id, &mut out);
    out
}

/// Identity of the **first** widget declaring the key `key` (a hash), or `None`. It is how the
/// shell resolves a focus-by-key request (`Command::focus`): the application wraps a field in
/// `keyed(k, …)`, then focuses by `k`.
pub fn find_by_key<Msg>(root: &dyn Widget<Msg>, key: u64) -> Option<WidgetId> {
    fn walk<Msg>(widget: &dyn Widget<Msg>, id: WidgetId, key: u64) -> Option<WidgetId> {
        if widget.key() == Some(key) {
            return Some(id);
        }
        for (index, child) in widget.children().iter().enumerate() {
            if let Some(found) = walk(child.as_ref(), child_id(id, index, child.as_ref()), key) {
                return Some(found);
            }
        }
        None
    }
    walk(root, WidgetId::ROOT, key)
}

/// Path from the **root down to the widget** with identity `target` (`[root, …, target]`), for
/// the leaf→root bubbling of keys. Empty when it cannot be found.
pub fn find_path<Msg>(root: &dyn Widget<Msg>, target: WidgetId) -> Vec<&dyn Widget<Msg>> {
    fn walk<'a, Msg>(
        widget: &'a dyn Widget<Msg>,
        id: WidgetId,
        target: WidgetId,
        path: &mut Vec<&'a dyn Widget<Msg>>,
    ) -> bool {
        path.push(widget);
        if id == target {
            return true;
        }
        for (index, child) in widget.children().iter().enumerate() {
            if walk(
                child.as_ref(),
                child_id(id, index, child.as_ref()),
                target,
                path,
            ) {
                return true;
            }
        }
        path.pop();
        false
    }
    let mut path = Vec::new();
    if !walk(root, WidgetId::ROOT, target, &mut path) {
        path.clear();
    }
    path
}

pub fn find_widget<Msg>(root: &dyn Widget<Msg>, target: WidgetId) -> Option<&dyn Widget<Msg>> {
    fn walk<Msg>(
        widget: &dyn Widget<Msg>,
        id: WidgetId,
        target: WidgetId,
    ) -> Option<&dyn Widget<Msg>> {
        if id == target {
            return Some(widget);
        }
        for (index, child) in widget.children().iter().enumerate() {
            if let Some(found) = walk(child.as_ref(), child_id(id, index, child.as_ref()), target) {
                return Some(found);
            }
        }
        None
    }
    walk(root, WidgetId::ROOT, target)
}

/// Folds a filter into the single filtered layer its subtree turned out to be, so two
/// filter widgets wrapped one inside the other cost one layer rather than two.
///
/// It is not only a saving. Compositing renders a layer into a texture and composites
/// that texture; a layer *inside* that texture is not composited again, so a nested
/// filter would simply not be applied. Folding is what makes `ColorFiltered` around
/// `ImageFiltered` mean what it reads as.
///
/// Everything about the inner layer must be neutral — full opacity, a plain
/// rectangular clip, no transform — because those are applied at compositing time and
/// there is only one composite to apply them at. And the two filters must not want
/// the same slot: greyscale of an inverted picture is not the inversion of a
/// greyscale one, so there is no single layer that means both, and the two nest.
fn fold_filter(
    group: Vec<Primitive>,
    filter: LayerFilter,
    clip: Rect,
) -> (Vec<Primitive>, LayerFilter, Rect) {
    if group.len() != 1 {
        return (group, filter, clip);
    }
    let Primitive::Layer {
        opacity,
        clip: inner_clip,
        clip_shape,
        transform,
        filter: inner,
        ..
    } = &group[0]
    else {
        return (group, filter, clip);
    };
    if *opacity < 0.999 || !matches!(clip_shape, ClipShape::Rect) || transform.is_some() {
        return (group, filter, clip);
    }
    let (inner, inner_clip) = (*inner, *inner_clip);
    let Some(merged) = filter.merge(inner) else {
        return (group, filter, clip);
    };
    let Some(Primitive::Layer { primitives, .. }) = group.into_iter().next() else {
        unreachable!("checked just above")
    };
    (primitives, merged, clip.intersect(inner_clip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Edit;
    use crate::{
        Button, Container, Flex, Key, Keyed, OverlayPortal, Placement, SingleChildScrollView,
        TextField,
    };
    use frus_core::{Color, Point, Primitive, Rect, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        A,
        B,
        C,
        D,
        Edited(String),
    }

    #[test]
    fn semantics_tree_carries_roles_and_labels() {
        use crate::{Button, Checkbox, Role, Text};
        let rt = Runtime::default();
        let tree = crate::Flex::column()
            .child(Text::new("Titre"))
            .child(Button::new("Submit").on_press(Msg::A))
            .child(Checkbox::new(true).on_toggle(|_| Msg::B));
        let ui = build_ui(&tree, Size::new(300.0, 200.0), &rt, &Theme::default());
        let sem = ui.semantics();
        // One node per meaningful widget (the containing Flex is skipped).
        let roles: Vec<Role> = sem.iter().map(|(_, _, s)| s.role).collect();
        assert!(roles.contains(&Role::Label));
        assert!(roles.contains(&Role::Button));
        assert!(roles.contains(&Role::CheckBox));
        // The button carries its label and is actionable.
        let button = sem.iter().find(|(_, _, s)| s.role == Role::Button).unwrap();
        assert_eq!(button.2.label.as_deref(), Some("Submit"));
        assert!(button.2.clickable);
        // The checked box mirrors its state.
        let check = sem
            .iter()
            .find(|(_, _, s)| s.role == Role::CheckBox)
            .unwrap();
        assert_eq!(check.2.toggled, crate::Toggled::True);
    }

    fn clickable_sample() -> Flex<Msg> {
        Flex::row()
            .width(400.0)
            .height(100.0)
            .padding(10.0)
            .gap(8.0)
            .child(
                Container::new()
                    .width(120.0)
                    .color(Color::rgb(1.0, 0.0, 0.0))
                    .hover_color(Color::rgb(0.0, 1.0, 0.0))
                    .on_click(Msg::A),
            )
            .child(
                Container::new()
                    .flex(1.0)
                    .color(Color::rgb(0.0, 0.0, 1.0))
                    .on_click(Msg::B),
            )
    }

    /// A box whose children ran past it is **marked on the screen**, over a tenth of it,
    /// on the edge they ran past — and a box whose children fit is left alone.
    #[test]
    fn an_overflow_paints_a_band_on_the_edge_it_ran_past() {
        use crate::{Container, Flex};
        // Two children, because a row with one is not dividing a line up — it hands its
        // constraints down, and a lone child is bounded by them rather than overflowing.
        let row = |child_width: f32| {
            let root: Flex<()> = Flex::row()
                .width(100.0)
                .height(40.0)
                .child(
                    Container::new()
                        .width(child_width)
                        .height(20.0)
                        .color(Color::WHITE),
                )
                .child(Container::new().width(20.0).height(20.0));
            let rt = crate::runtime::Runtime::default();
            build_ui(
                &root,
                Size::new(200.0, 100.0),
                &rt,
                &crate::theme::Theme::dark(),
            )
        };
        // The stripes are paths; nothing else in this tree draws one.
        let stripes = |ui: &Ui<()>| {
            ui.scene()
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Path { .. }))
                .count()
        };
        let over = row(300.0);
        assert!(!over.overflows().is_empty(), "the fixture has to overflow");
        assert!(stripes(&over) > 2, "striped: {}", stripes(&over));
        // The band is on the right-hand tenth: 90..100 of a 100-wide row.
        let band = over
            .scene()
            .primitives()
            .iter()
            .rev()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.a > 0.7 && color.r == 0.0 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the band under the stripes");
        assert!((band.x - 90.0).abs() < 0.5, "at the right edge: {band:?}");
        assert!((band.width - 10.0).abs() < 0.5, "a tenth of it: {band:?}");

        let fits = row(40.0);
        assert!(fits.overflows().is_empty(), "the fixture has to fit");
        assert!(fits.overflows().is_empty());
        assert_eq!(stripes(&fits), 0, "nothing to mark");
    }

    /// The band says **which edge and by how much**, in the reference's words. A striped
    /// edge tells you something is too big; the sentence tells you what to look for, and
    /// it is the only half of the report that survives being photographed off a device.
    #[test]
    fn a_band_writes_which_edge_overflowed_and_by_how_much() {
        use crate::{Container, Flex};
        let root: Flex<()> = Flex::row()
            .width(100.0)
            .height(40.0)
            .child(
                Container::new()
                    .width(300.0)
                    .height(20.0)
                    .color(Color::WHITE),
            )
            .child(Container::new().width(20.0).height(20.0));
        let ui = build_ui(
            &root,
            Size::new(200.0, 100.0),
            &Runtime::default(),
            &Theme::dark(),
        );
        // The label is inside the rotated layer, which is why the search is recursive.
        fn labels(primitives: &[Primitive], out: &mut Vec<String>) {
            for p in primitives {
                match p {
                    Primitive::Text { text, .. } => out.push(text.clone()),
                    Primitive::Layer { primitives, .. } => labels(primitives, out),
                    _ => {}
                }
            }
        }
        let mut found = Vec::new();
        labels(ui.scene().primitives(), &mut found);
        assert!(
            found.iter().any(|t| t == "RIGHT OVERFLOWED BY 220 PIXELS"),
            "the sentence, in the reference's words: {found:?}"
        );
    }

    /// The reference's precision rule: whole pixels when there are many, a decimal when
    /// there are few, three significant figures when there is almost nothing — which is
    /// the difference between a layout bug and a rounding error.
    #[test]
    fn an_overflow_is_written_to_the_precision_it_deserves() {
        assert_eq!(format_pixels(86.0), "86");
        // Past ten pixels the decimal is noise; at ten exactly it is not yet.
        assert_eq!(format_pixels(10.4), "10");
        assert_eq!(format_pixels(10.0), "10.0");
        assert_eq!(format_pixels(4.5), "4.5");
        assert_eq!(format_pixels(1.0), "1.00");
        assert_eq!(format_pixels(0.5), "0.500");
        assert_eq!(format_pixels(0.0123), "0.0123");
    }

    #[test]
    fn rtl_mirrors_row_horizontally() {
        let size = Size::new(400.0, 100.0);
        // A = a fixed 120 px button (on the left in LTR), B = the flexible remainder.
        let ltr = build_ui(
            &clickable_sample(),
            size,
            &Runtime::default(),
            &Theme::default(),
        );
        let rtl = build_ui(
            &clickable_sample(),
            size,
            &Runtime::default(),
            &Theme::default().rtl(),
        );
        let hit = |ui: &Ui<Msg>, x: f32| ui.hit(Point::new(x, 50.0)).and_then(|id| ui.msg_for(id));

        // LTR: button A takes the left edge.
        assert_eq!(hit(&ltr, 40.0), Some(Msg::A));
        assert_eq!(hit(&ltr, 360.0), Some(Msg::B));
        // RTL: everything is mirrored — A moves to the right, B takes the left.
        assert_eq!(hit(&rtl, 360.0), Some(Msg::A), "A on the right in RTL");
        assert_eq!(hit(&rtl, 40.0), Some(Msg::B), "B on the left in RTL");
    }

    #[test]
    fn wrapped_text_wraps_in_layout_and_invalidates_the_cache() {
        let tree = |text: &str| {
            crate::Flex::column()
                .width(120.0)
                .child(crate::Text::new(text).wrap())
                .child(Container::new().height(10.0).on_click(Msg::A))
        };
        // Position of the clickable follower: the first y hit while sweeping.
        let follower_y = |ui: &Ui<Msg>| {
            (0..600)
                .map(|y| y as f32)
                .find(|&y| ui.hit(Point::new(60.0, y)).is_some())
                .expect("suiveur cliquable")
        };

        let rt = Runtime::default();
        let long = "a rather long paragraph that will wrap onto several lines";
        let ui = build_ui(&tree(long), Size::new(120.0, 600.0), &rt, &Theme::default());

        // The paragraph's render carries its wrap width (≤ the column).
        let max_w = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Text { max_width, .. } => *max_width,
                _ => None,
            })
            .expect("a wrapped paragraph");
        assert!(max_w <= 120.5, "wrapped to the column width: {max_w}");

        // The wrapped text takes several lines: the follower is pushed down.
        let y_long = follower_y(&ui);
        assert!(
            y_long > 30.0,
            "the follower is pushed down by the wrap: {y_long}"
        );

        // The SAME structure/styles, different content, the SAME runtime (a warm cache): the
        // measure key must invalidate the cache — otherwise the rects would be stale.
        let ui2 = build_ui(
            &tree("short"),
            Size::new(120.0, 600.0),
            &rt,
            &Theme::default(),
        );
        let y_short = follower_y(&ui2);
        assert!(
            y_short < y_long,
            "shorter content → a higher follower (the cache was invalidated): {y_short} vs {y_long}"
        );
    }

    #[test]
    fn relayout_cache_reuses_the_root_layout_across_frames() {
        let rt = Runtime::default();
        let size = Size::new(400.0, 100.0);
        // Frame 1: nothing cached → a recomputation (at least the root).
        let _ = build_ui(&clickable_sample(), size, &rt, &Theme::default());
        let (hits1, misses1) = rt.layout_cache.borrow().last_frame_stats();
        assert_eq!(hits1, 0, "1st frame: nothing reused");
        assert!(misses1 >= 1, "1st frame: at least one computation");

        // Frame 2: the same tree, the same constraints → the root is reused.
        let _ = build_ui(&clickable_sample(), size, &rt, &Theme::default());
        let (hits2, misses2) = rt.layout_cache.borrow().last_frame_stats();
        assert_eq!(hits2, 1, "2nd frame: the root is reused");
        assert_eq!(misses2, 0, "2nd frame: nothing recomputed");

        // Frame 3: the window is resized → the constraints change → a recomputation.
        let _ = build_ui(
            &clickable_sample(),
            Size::new(500.0, 100.0),
            &rt,
            &Theme::default(),
        );
        let (hits3, misses3) = rt.layout_cache.borrow().last_frame_stats();
        assert_eq!((hits3, misses3), (0, 1), "redimensionnement → recalcul");
    }

    #[test]
    fn modal_traps_tab_arrows_and_pointer_focus() {
        use crate::portal::{OverlayPortal, Placement};
        // The background: two focusable buttons; the open modal: two buttons as well.
        let dialog = Flex::<Msg>::row()
            .child(Button::new("ok").on_press(Msg::C))
            .child(Button::new("no").on_press(Msg::D));
        let tree: Flex<Msg> = Flex::column()
            .child(Button::new("bg1").on_press(Msg::A))
            .child(Button::new("bg2").on_press(Msg::B))
            .child(
                OverlayPortal::new(Container::new())
                    .overlay(dialog, Placement::Center)
                    .dismiss(Msg::A),
            );
        let ui = build_ui(
            &tree,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );

        // Tab enters the trap (the modal's first focusable) and cycles inside it.
        let first = ui.focus_next(None, true).expect("the scope's first");
        assert_eq!(ui.msg_for(first), Some(Msg::C));
        let second = ui.focus_next(Some(first), true).expect("suivant");
        assert_eq!(ui.msg_for(second), Some(Msg::D));
        let wrapped = ui.focus_next(Some(second), true).expect("boucle");
        assert_eq!(
            ui.msg_for(wrapped),
            Some(Msg::C),
            "Tab cycles inside the modal"
        );

        // The arrows stay inside the scope: there is nothing above the dialog.
        assert_eq!(ui.focus_directional(first, FocusDirection::Up), None);
        let right = ui
            .focus_directional(first, FocusDirection::Right)
            .expect("to the right");
        assert_eq!(ui.msg_for(right), Some(Msg::D));

        // Click-to-focus ignores the background (the bg1 button is at the top left).
        assert_eq!(
            ui.focus_hit(Point::new(10.0, 10.0)),
            None,
            "fond hors scope"
        );

        // Without a modal: no trap, Tab starts at the background.
        let open_less: Flex<Msg> = Flex::column()
            .child(Button::new("bg1").on_press(Msg::A))
            .child(Button::new("bg2").on_press(Msg::B));
        let ui2 = build_ui(
            &open_less,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let f = ui2.focus_next(None, true).expect("premier");
        assert_eq!(ui2.msg_for(f), Some(Msg::A));
    }

    #[test]
    fn open_menu_traps_focus_in_its_items() {
        use crate::PopupMenuButton;
        // A focusable background plus an **open** menu (anchor + two items).
        let menu = PopupMenuButton::new(Button::new("open").on_press(Msg::A), true, Msg::D)
            .item("one", Msg::B)
            .item("two", Msg::C);
        let tree: Flex<Msg> = Flex::column()
            .child(Button::new("bg").on_press(Msg::A))
            .child(menu);
        let ui = build_ui(
            &tree,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );

        // Tab enters the menu's items and **cycles** inside (background and anchor are out of scope).
        let first = ui.focus_next(None, true).expect("the scope's first");
        assert_eq!(ui.msg_for(first), Some(Msg::B));
        let second = ui.focus_next(Some(first), true).expect("suivant");
        assert_eq!(ui.msg_for(second), Some(Msg::C));
        let wrapped = ui.focus_next(Some(second), true).expect("boucle");
        assert_eq!(
            ui.msg_for(wrapped),
            Some(Msg::B),
            "Tab cycles inside the open menu"
        );
        // The background (top left) is out of scope while the menu is open.
        assert_eq!(
            ui.focus_hit(Point::new(10.0, 10.0)),
            None,
            "fond hors scope"
        );

        // The menu **closed**: no trap, Tab starts at the background.
        let closed: Flex<Msg> = Flex::column()
            .child(Button::new("bg").on_press(Msg::A))
            .child(PopupMenuButton::new(
                Button::new("open").on_press(Msg::B),
                false,
                Msg::D,
            ));
        let ui2 = build_ui(
            &closed,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let f = ui2.focus_next(None, true).expect("premier");
        assert_eq!(ui2.msg_for(f), Some(Msg::A), "with no open menu, no trap");
    }

    #[test]
    fn escape_infrastructure_finds_path_and_topmost_dismiss() {
        use crate::portal::{OverlayPortal, Placement};
        // An open portal (a modal with a dismissal) around a clickable anchor.
        let anchor = Container::<Msg>::new()
            .width(50.0)
            .height(30.0)
            .on_click(Msg::A);
        let content = Container::<Msg>::new()
            .width(80.0)
            .height(40.0)
            .on_click(Msg::B);
        let portal = OverlayPortal::new(anchor)
            .overlay(content, Placement::Center)
            .dismiss(Msg::C);
        let tree: Flex<Msg> = Flex::column().child(portal);

        let ui = build_ui(
            &tree,
            Size::new(300.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The topmost dismissal is the portal's.
        assert_eq!(ui.top_dismiss(), Some(Msg::C));

        // Focus "inside the dialog": the root→content path goes through the portal, which
        // consumes Escape while bubbling. (The Center content, 80×40, is centred in 300×200 →
        // its centre is at (150, 100).)
        let inner = ui
            .hit(Point::new(150.0, 100.0))
            .expect("contenu de la modale");
        let path = find_path(&tree, inner);
        assert!(path.len() >= 3, "root, portal, content: {}", path.len());
        assert_eq!(
            path.last().unwrap().on_click(),
            Some(Msg::B),
            "the target closes the path"
        );
        let consumed = path
            .iter()
            .rev()
            .find_map(|w| match w.on_key(&crate::Key::Escape) {
                crate::KeyResponse::Handled(msg) => Some(msg),
                _ => None,
            });
        assert_eq!(
            consumed,
            Some(Some(Msg::C)),
            "the portal consumes Escape while bubbling"
        );

        // Chemin introuvable → vide ; pas d'overlay → pas de fermeture.
        assert!(find_path(&tree, WidgetId::ROOT.child(99)).is_empty());
        let closed: Flex<Msg> = Flex::column().child(Container::new().on_click(Msg::A));
        let ui2 = build_ui(
            &closed,
            Size::new(300.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(ui2.top_dismiss(), None);
    }

    #[test]
    fn long_press_targets_are_collected_topmost_first() {
        // A long-press container holding a long-press child: a point inside the child returns
        // the child's message (the topmost one).
        let tree: Container<Msg> = Container::new()
            .width(200.0)
            .height(100.0)
            .on_long_press(Msg::A)
            .child(
                Container::new()
                    .width(50.0)
                    .height(50.0)
                    .on_long_press(Msg::B),
            );
        let ui = build_ui(
            &tree,
            Size::new(200.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(ui.long_press_at(Point::new(25.0, 25.0)), Some(Msg::B));
        assert_eq!(ui.long_press_at(Point::new(150.0, 80.0)), Some(Msg::A));
        assert_eq!(ui.long_press_at(Point::new(500.0, 500.0)), None);
    }

    #[test]
    fn hit_and_msg_for_route_correctly() {
        let rt = Runtime::default();
        let ui = build_ui(
            &clickable_sample(),
            Size::new(400.0, 100.0),
            &rt,
            &Theme::default(),
        );
        let id_a = ui.hit(Point::new(50.0, 50.0)).expect("A");
        let id_b = ui.hit(Point::new(300.0, 50.0)).expect("B");
        assert_ne!(id_a, id_b);
        assert_eq!(ui.msg_for(id_a), Some(Msg::A));
        assert_eq!(ui.msg_for(id_b), Some(Msg::B));
        assert_eq!(ui.hit(Point::new(3.0, 3.0)), None);
    }

    #[test]
    fn hover_progress_interpolates_color() {
        let rt = Runtime::default();
        let base = build_ui(
            &clickable_sample(),
            Size::new(400.0, 100.0),
            &rt,
            &Theme::default(),
        );
        let id_a = base.hit(Point::new(50.0, 50.0)).unwrap();

        // Without any progress: the base colour (red).
        if let Primitive::Rect { color, .. } = base.scene().primitives()[0] {
            assert_eq!(color, Color::rgb(1.0, 0.0, 0.0));
        } else {
            panic!("expected a rect");
        }

        // Full progress: the hover colour (green).
        let mut rt = Runtime::default();
        rt.input.hovered = Some(id_a);
        rt.anims.insert(
            id_a,
            crate::Anim {
                hover: 1.0,
                ..Default::default()
            },
        );
        let ui = build_ui(
            &clickable_sample(),
            Size::new(400.0, 100.0),
            &rt,
            &Theme::default(),
        );
        if let Primitive::Rect { color, .. } = ui.scene().primitives()[0] {
            assert_eq!(color, Color::rgb(0.0, 1.0, 0.0));
        } else {
            panic!("expected a rect");
        }
    }

    #[test]
    fn multiline_field_registers_as_scrollable_when_overflowing() {
        // A multi-line field whose content exceeds `rows` registers itself as a scrollable
        // area (with `max_y > 0`) — which is what the wheel and the scrollbar target. A short
        // field does not register.
        let tall = TextField::<Msg>::new("a\nb\nc\nd\ne\nf")
            .on_input(Msg::Edited)
            .rows(2)
            .width(200.0);
        let tree: Flex<Msg> = Flex::column().child(tall);
        let rt = Runtime::default();
        let ui = build_ui(&tree, Size::new(220.0, 240.0), &rt, &Theme::default());
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1, "the overflowing field registers");
        assert!(maxes[0].2 > 0.0, "max_y > 0 (overflowing content)");

        let short = TextField::<Msg>::new("a\nb")
            .on_input(Msg::Edited)
            .rows(4)
            .width(200.0);
        let tree: Flex<Msg> = Flex::column().child(short);
        let ui = build_ui(&tree, Size::new(220.0, 240.0), &rt, &Theme::default());
        assert!(
            ui.scrollable_maxes().is_empty(),
            "a short field does not scroll"
        );
    }

    #[test]
    fn only_text_inputs_place_a_cursor() {
        // The invariant of the click fix (milestone 39): a focusable button returns NO cursor
        // (`cursor_at` = None), so the shell does not start a text selection on it and does not
        // capture the click. Only text fields place one.
        let button = Button::<Msg>::new("x").on_press(Msg::A);
        assert_eq!(Widget::<Msg>::cursor_at(&button, 10.0, 5.0, 200.0, 0), None);
        let input = TextField::<Msg>::new("hi").on_input(Msg::Edited);
        assert!(Widget::<Msg>::cursor_at(&input, 10.0, 5.0, 200.0, 0).is_some());
    }

    #[test]
    fn tab_cycles_focusables_in_order() {
        let tree = Flex::<Msg>::column()
            .child(Button::new("un").on_press(Msg::A))
            .child(Button::new("deux").on_press(Msg::B));
        let ui = build_ui(
            &tree,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(ui.focusables.len(), 2, "both buttons are focusable");
        let first = ui.focusables[0].id;
        let second = ui.focusables[1].id;

        // With no focus: Tab -> first, Shift+Tab -> last.
        assert_eq!(ui.focus_next(None, true), Some(first));
        assert_eq!(ui.focus_next(None, false), Some(second));
        // Wrap-around.
        assert_eq!(ui.focus_next(Some(first), true), Some(second));
        assert_eq!(ui.focus_next(Some(second), true), Some(first));
        assert_eq!(ui.focus_next(Some(first), false), Some(second));
    }

    #[test]
    fn focus_ring_for_button_not_for_textinput() {
        let theme = Theme::default();
        let ring = theme.focus.fade(0.4); // focus_progress = 0 → alpha 0.4
        let count_ring = |tree: &dyn Widget<Msg>, keyboard: bool| -> usize {
            let mut rt = Runtime::default();
            let probe = build_ui(tree, Size::new(200.0, 200.0), &rt, &theme);
            rt.input.focused = probe.focusables.first().map(|f| f.id);
            rt.focus_visible = keyboard;
            let ui = build_ui(tree, Size::new(200.0, 200.0), &rt, &theme);
            ui.scene()
                .primitives()
                .iter()
                .filter(
                    |p| matches!(p, Primitive::Rect { border_color, .. } if *border_color == ring),
                )
                .count()
        };
        let with_button = Flex::<Msg>::column().child(Button::new("x").on_press(Msg::A));
        assert!(
            count_ring(&with_button, true) >= 1,
            "a button focused with the keyboard has a generic ring"
        );
        // Focus taken with the pointer: no ring (FocusHighlightMode).
        assert_eq!(
            count_ring(&with_button, false),
            0,
            "a click does not flash a ring"
        );

        let with_input = Flex::<Msg>::column().child(TextField::new("hi").on_input(Msg::Edited));
        assert_eq!(
            count_ring(&with_input, true),
            0,
            "the field draws its own focus"
        );
    }

    #[test]
    fn arrow_focus_navigates_geometrically() {
        // A 2×2 grid of buttons; each target is identified by its message.
        let grid: Flex<Msg> = Flex::column()
            .child(
                Flex::row()
                    .child(Button::new("a").on_press(Msg::A))
                    .child(Button::new("b").on_press(Msg::B)),
            )
            .child(
                Flex::row()
                    .child(Button::new("c").on_press(Msg::C))
                    .child(Button::new("d").on_press(Msg::D)),
            );
        let ui = build_ui(
            &grid,
            Size::new(300.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let top_left = ui.focus_next(None, true).expect("premier focusable");
        assert_eq!(ui.msg_for(top_left), Some(Msg::A));

        // Right: a → b; down: a → c; and nothing to the left of a.
        let right = ui
            .focus_directional(top_left, FocusDirection::Right)
            .expect("to the right");
        assert_eq!(ui.msg_for(right), Some(Msg::B));
        let down = ui
            .focus_directional(top_left, FocusDirection::Down)
            .expect("downwards");
        assert_eq!(ui.msg_for(down), Some(Msg::C));
        assert_eq!(ui.focus_directional(top_left, FocusDirection::Left), None);
        // The diagonal is kept in check: from b, down → d (aligned), not c.
        let down_right = ui
            .focus_directional(right, FocusDirection::Down)
            .expect("down from b");
        assert_eq!(ui.msg_for(down_right), Some(Msg::D));
    }

    #[test]
    fn keyed_identity_survives_middle_removal() {
        let colored = |c: Color| Container::<Msg>::new().width(50.0).height(20.0).color(c);
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);

        let owner_of = |ui: &Ui<Msg>, c: Color| -> u64 {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, owner, .. } if *color == c => Some(*owner),
                    _ => None,
                })
                .expect("the primitive is there")
        };

        // The list [red(key 1), green(key 2), blue(key 3)].
        let full = Flex::<Msg>::column()
            .child(Keyed::new(1u64, colored(red)))
            .child(Keyed::new(2u64, colored(green)))
            .child(Keyed::new(3u64, colored(blue)));
        let ui_full = build_ui(
            &full,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );

        // The list [red(1), blue(3)]: the green in the middle is removed → blue goes from index 2 to 1.
        let removed = Flex::<Msg>::column()
            .child(Keyed::new(1u64, colored(red)))
            .child(Keyed::new(3u64, colored(blue)));
        let ui_removed = build_ui(
            &removed,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );

        // Blue's identity (owner, key 3) is UNCHANGED despite the shift in position.
        assert_eq!(owner_of(&ui_full, blue), owner_of(&ui_removed, blue));

        // Without a key, the 2nd child's positional identity DOES change (index 2 vs 1).
        let unkeyed_full = Flex::<Msg>::column()
            .child(colored(red))
            .child(colored(green))
            .child(colored(blue));
        let unkeyed_removed = Flex::<Msg>::column()
            .child(colored(red))
            .child(colored(blue));
        let u1 = build_ui(
            &unkeyed_full,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let u2 = build_ui(
            &unkeyed_removed,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_ne!(owner_of(&u1, blue), owner_of(&u2, blue));
    }

    #[test]
    fn center_overlay_scrim_click_dismisses() {
        // A Center modal with `.dismiss`: clicking the scrim (outside the content) returns the
        // dismissal message; clicking the content does not.
        let modal = Container::<Msg>::new()
            .width(100.0)
            .height(60.0)
            .color(Color::WHITE);
        let portal: OverlayPortal<Msg> =
            OverlayPortal::new(Container::<Msg>::new().width(20.0).height(20.0))
                .overlay(modal, Placement::Center)
                .dismiss(Msg::A);
        let ui = build_ui(
            &portal,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );

        // The top-left corner: on the scrim → it dismisses.
        let corner = ui.hit(Point::new(5.0, 5.0)).expect("a clickable scrim");
        assert_eq!(ui.msg_for(corner), Some(Msg::A));
    }

    #[test]
    fn find_widget_and_edit_types() {
        let tree = Flex::column()
            .width(300.0)
            .height(80.0)
            .child(TextField::new("hi").width(200.0).on_input(Msg::Edited));
        let rt = Runtime::default();
        let ui = build_ui(&tree, Size::new(300.0, 80.0), &rt, &Theme::default());
        let (id, _rect) = ui.focus_hit(Point::new(10.0, 10.0)).expect("the field");

        let widget = find_widget(&tree, id).expect("the widget was found");
        let mut edit = Edit {
            cursor: 2,
            anchor: None,
            composing: None,
        };
        assert_eq!(
            widget.on_edit(&mut edit, &Key::Text("!".to_string())),
            Some(Msg::Edited("hi!".to_string()))
        );
    }

    #[test]
    fn find_by_key_resolves_a_keyed_field_to_its_focus_id() {
        // Two named fields: `find_by_key` finds each one's focus identity (the one the
        // hit-test would assign), and it tells the keys apart.
        fn hash(k: &str) -> u64 {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            k.hash(&mut h);
            h.finish()
        }
        let tree: Flex<Msg> = Flex::column()
            .child(Keyed::new(
                "email",
                TextField::new("").on_input(Msg::Edited),
            ))
            .child(Keyed::new(
                "password",
                TextField::new("").on_input(Msg::Edited),
            ));

        let email = find_by_key(&tree, hash("email")).expect("email was found");
        let password = find_by_key(&tree, hash("password")).expect("password was found");
        assert_ne!(email, password, "distinct keys → distinct identities");
        // The identity resolved is indeed the by-key identity (stable, independent of the
        // position).
        assert_eq!(email, WidgetId::ROOT.keyed(hash("email")));
        assert!(
            find_by_key(&tree, hash("absent")).is_none(),
            "an unknown key → None"
        );

        // And above all: it is **exactly** the identity the focus hit-test would assign to
        // that field — so setting this focus really does route to it.
        let rt = Runtime::default();
        let ui = build_ui(&tree, Size::new(300.0, 200.0), &rt, &Theme::default());
        let (hit_id, _) = ui
            .focus_hit(Point::new(10.0, 10.0))
            .expect("the email field under the cursor");
        assert_eq!(email, hit_id, "find_by_key == the field's focus identity");
    }

    #[test]
    fn ink_is_painted_over_the_surface_and_under_its_child() {
        use crate::ink::InkWell;
        let tree = Container::<Msg>::new().width(120.0).height(60.0).child(
            InkWell::<Msg>::new().radius(8.0).on_click(Msg::A).child(
                Container::<Msg>::new()
                    .width(120.0)
                    .height(60.0)
                    .color(frus_core::Color::WHITE),
            ),
        );
        let size = Size::new(120.0, 60.0);

        // Dry: the well paints nothing of its own.
        let rt = Runtime::default();
        let dry = build_ui(&tree, size, &rt, &Theme::default());
        let plain = dry.scene.len();
        let id = dry.inks.first().expect("the well registered its box").0;
        assert_eq!(
            dry.ink_box(id).map(|r| (r.width, r.height)),
            Some((120.0, 60.0)),
            "the registry carries the **whole** box, which is what sizes the splash"
        );

        // A finger lands in the top-left corner, and is still down.
        let mut rt = Runtime::default();
        rt.input.pressed = Some(id);
        rt.ink_press(id, Point::new(10.0, 10.0), Size::new(120.0, 60.0));
        rt.advance_ink(1.0 / 60.0);
        let ui = build_ui(&tree, size, &rt, &Theme::default());
        assert_eq!(ui.scene.len(), plain + 1, "the splash adds one layer");

        // It is a shape-clipped layer, so the ink cannot escape the rounded corners…
        let (index, clip_shape) = ui
            .scene
            .primitives()
            .iter()
            .enumerate()
            .find_map(|(i, p)| match p {
                frus_core::Primitive::Layer { clip_shape, .. } => Some((i, clip_shape.clone())),
                _ => None,
            })
            .expect("the ink is a composited layer");
        assert_eq!(clip_shape, frus_core::ClipShape::RRect(8.0.into()));

        // …and it is painted **before** the child that sits on top of it: the ink is
        // under the content, the way a material surface holds it.
        let child_index = ui
            .scene
            .primitives()
            .iter()
            .rposition(|p| matches!(p, frus_core::Primitive::Rect { .. }))
            .expect("the child painted a rectangle");
        assert!(
            index < child_index,
            "ink at {index} must come before the content at {child_index}"
        );
    }

    #[test]
    fn a_splash_left_unconfirmed_does_not_wait_for_ever() {
        // The finger came down and never came back up on this widget — it slid off, or
        // the widget went away mid-press. Nothing will ever confirm the splash, and
        // without the sweep in `advance_ink` the ink would sit there for good.
        let mut rt = Runtime::default();
        let id = WidgetId::from_u64(42);
        rt.input.pressed = Some(id);
        rt.ink_press(id, Point::new(5.0, 5.0), Size::new(100.0, 40.0));
        // Held for a fifth of a second: the ink is up, and it stays up.
        for _ in 0..12 {
            rt.advance_ink(1.0 / 60.0);
        }
        assert!(rt.ink.contains_key(&id), "the splash waits for the finger");

        // The finger is gone and nothing confirmed the tap.
        rt.input.pressed = None;
        let mut frames = 0;
        while rt.advance_ink(1.0 / 60.0) && frames < 200 {
            frames += 1;
        }
        assert!(rt.ink.is_empty(), "the ink is gone after {frames} frames");
        assert!(frames <= 6, "and quickly: a cancel is 75 ms, took {frames}");
    }

    #[test]
    fn an_overscroll_glow_is_painted_over_its_scroll_area() {
        let tree = SingleChildScrollView::<Msg>::new()
            .width(200.0)
            .height(100.0)
            .child(Container::<Msg>::new().width(100.0).height(400.0));
        let size = Size::new(200.0, 100.0);

        // Quiet: nothing extra is drawn.
        let rt = Runtime::default();
        let plain = build_ui(&tree, size, &rt, &Theme::default()).scene.len();

        // A fling has just landed on the bottom edge.
        let mut rt = Runtime::default();
        let id = build_ui(&tree, size, &rt, &Theme::default()).scroll_regions()[0].id;
        rt.glow_absorb(id, crate::overscroll::GlowEdge::Bottom, 4000.0);
        rt.advance_glow(0.02);
        let ui = build_ui(&tree, size, &rt, &Theme::default());
        assert_eq!(
            ui.scene.len(),
            plain + 1,
            "the glow adds exactly one shape to the frame"
        );
        // The glow sits under the scrollbars, so it is not the last primitive: it is
        // the only filled path in the frame.
        let clip = ui
            .scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Path { clip, .. } => Some(*clip),
                _ => None,
            })
            .expect("the glow is a filled path");
        // Drawn against the bottom of the viewport, and inside it.
        assert!(
            (clip.y + clip.height - 100.0).abs() < 1.0,
            "clip = {clip:?}"
        );
        assert!(clip.width <= 200.0 + 1e-3);
    }

    #[test]
    fn a_scroll_area_carries_its_physics_into_the_registry() {
        let content = Container::<Msg>::new().width(100.0).height(400.0);
        // Unset: the region says nothing and the application decides.
        let plain = SingleChildScrollView::new()
            .width(200.0)
            .height(100.0)
            .child(content);
        let rt = Runtime::default();
        let ui = build_ui(&plain, Size::new(200.0, 100.0), &rt, &Theme::default());
        assert_eq!(ui.scroll_regions()[0].physics, None);
        assert_eq!(
            ui.scroll_regions()[0].physics_or(ScrollPhysics::Clamping),
            ScrollPhysics::Clamping,
            "an area with no opinion follows the application"
        );

        // Set: the area's own choice wins over the application's.
        let bouncy = SingleChildScrollView::new()
            .width(200.0)
            .height(100.0)
            .physics(ScrollPhysics::Bouncing)
            .child(Container::<Msg>::new().width(100.0).height(400.0));
        let ui = build_ui(&bouncy, Size::new(200.0, 100.0), &rt, &Theme::default());
        let area = ui.scroll_regions()[0];
        assert_eq!(area.physics, Some(ScrollPhysics::Bouncing));
        assert_eq!(
            area.physics_or(ScrollPhysics::Clamping),
            ScrollPhysics::Bouncing
        );
        // And the metrics it hands the physics describe the right axis.
        let metrics = area.metrics_y(10.0);
        assert_eq!(metrics.pixels, 10.0);
        assert_eq!(metrics.max, 300.0); // 400 of content in a 100 viewport
        assert_eq!(metrics.viewport, 100.0);
    }

    #[test]
    fn scroll_translates_and_clips_content() {
        let content = Flex::<Msg>::column()
            .gap(0.0)
            .child(
                Container::new()
                    .height(60.0)
                    .color(Color::rgb(1.0, 0.0, 0.0)),
            )
            .child(
                Container::new()
                    .height(60.0)
                    .color(Color::rgb(0.0, 1.0, 0.0)),
            )
            .child(
                Container::new()
                    .height(60.0)
                    .color(Color::rgb(0.0, 0.0, 1.0)),
            );
        let tree = SingleChildScrollView::new()
            .width(200.0)
            .height(100.0)
            .child(content);

        let rt = Runtime::default();
        let ui = build_ui(&tree, Size::new(200.0, 100.0), &rt, &Theme::default());
        let area = ui.scrollables[0];
        let sid = area.id;
        assert_eq!(area.max_y, 80.0); // 180 - 100
        assert_eq!(area.max_x, 0.0);
        assert_eq!(ui.first_rect().0.y, 0.0);
        assert_eq!(ui.first_rect().1, Rect::new(0.0, 0.0, 200.0, 100.0));

        let mut rt = Runtime::default();
        rt.scroll.insert(sid, (0.0, 50.0));
        let ui2 = build_ui(&tree, Size::new(200.0, 100.0), &rt, &Theme::default());
        assert_eq!(ui2.first_rect().0.y, -50.0);
    }

    #[test]
    fn kanban_cards_are_reorderable_without_being_clickable() {
        use crate::Kanban;
        let board = Kanban::new(|_, _, _, _| Msg::A).column("To do", ["Card A"]);
        let rt = Runtime::default();
        let ui = build_ui(&board, Size::new(400.0, 300.0), &rt, &Theme::default());
        // The card **and** the drop zone are registered as reorderables.
        assert!(
            ui.reorderables.len() >= 2,
            "the card + the drop zone are in the reorderables registry"
        );
        // A point on the card can be **picked up** (it is reorderable) but is **not clickable**
        // — which is the whole point of the registry: without it, `ui.hit` alone would never
        // find the card.
        let (card_id, card) = ui.reorderables[0];
        let p = Point::new(card.x + 5.0, card.y + 5.0);
        assert!(
            ui.reorderable_at(p).is_some(),
            "the card can be picked up at that point"
        );
        assert!(
            ui.hit(p).is_none(),
            "…but it is not clickable (absent from the hit registry)"
        );
        // `widget_rect` must find the card **through the reorderable fallback** (it is not
        // focusable): otherwise the shell's vertical drag preview never starts.
        assert_eq!(
            ui.widget_rect(card_id),
            Some(card),
            "widget_rect falls back to the reorderables registry"
        );
    }

    #[test]
    fn reorderables_inside_a_scroll_are_still_registered() {
        // The milestone 258/260 scenario: the board (with its reorderable cards) is **wrapped
        // in a `SingleChildScrollView`**. The reorderables must stay registered — otherwise dragging a card
        // stops engaging as soon as the board scrolls.
        use crate::{Axis, Kanban, SingleChildScrollView};
        let board = Kanban::new(|_, _, _, _| Msg::A).column("To do", ["Card A"]);
        let scrolled = SingleChildScrollView::new()
            .axis(Axis::Horizontal)
            .width(400.0)
            .height(300.0)
            .child(board);
        let rt = Runtime::default();
        let ui = build_ui(&scrolled, Size::new(400.0, 300.0), &rt, &Theme::default());
        assert!(
            ui.reorderables.len() >= 2,
            "the card + the drop zone stay reorderable even inside a SingleChildScrollView (got {})",
            ui.reorderables.len()
        );
    }

    #[test]
    fn reorderables_inside_a_per_column_card_scroll_are_still_registered() {
        // Milestone 264: `card_area_height` puts each column's cards inside a **vertical
        // `SingleChildScrollView` with an explicit height**. This is exactly the case that *collapsed* at
        // milestone 263 (a flex scroll with no defined ancestor height → cards clipped to zero
        // and no longer reorderable). With a **defined** height the visible cards must stay
        // registered as reorderables — a guard against a regression of per-column dragging.
        use crate::Kanban;
        let board = Kanban::new(|_, _, _, _| Msg::A)
            .card_area_height(220.0)
            .column("To do", ["Card A", "Card B"]);
        let rt = Runtime::default();
        let ui = build_ui(&board, Size::new(400.0, 300.0), &rt, &Theme::default());
        assert!(
            ui.reorderables.len() >= 3,
            "2 cards + the drop zone stay reorderable inside a vertical scroll with a defined height \
             (got {})",
            ui.reorderables.len()
        );
    }

    #[test]
    fn scrollable_columns_fill_the_board_height_then_scroll() {
        // Milestone 266: `Kanban::scrollable_columns()` makes the columns **fill** the board's
        // height (laid out in an ancestor with a defined height), and each column scrolls its
        // cards vertically **with no explicit height** (the flex does the arithmetic, so the
        // milestone 264 stopgap is gone). This checks that at least one column has a vertical
        // `SingleChildScrollView` whose viewport **fills** the height (far more than the default 200) and
        // **scrolls** (max_y > 0), which is the proof of fill-then-scroll.
        use crate::{Axis, Container, Flex, Kanban, SingleChildScrollView};
        // Enough cards to **overflow** the filled viewport (otherwise max_y = 0: nothing to scroll).
        let long: Vec<String> = (0..24).map(|i| format!("card {i}")).collect();
        let board = Kanban::new(|_, _, _, _| Msg::A)
            .scrollable_columns()
            .column("To do", long);
        // `board_screen`'s nesting: the board inside a **plain `Container` with padding** (the
        // visual margin), itself inside a horizontal `flex(1)` SingleChildScrollView, inside a screen (a Flex
        // column) of bounded height. That `Auto` `Container` **collapsed** at milestone 266
        // (hence the `Flex` `flex(1)` workaround); since `compute_scroll` **fills the
        // constrained axis**, it fills the viewport's height and the board follows — no filler
        // container needed any more.
        let padded = Container::<Msg>::new().padding(24.0).child(board);
        let scroll_h = SingleChildScrollView::new()
            .axis(Axis::Horizontal)
            .width(360.0)
            .flex(1.0)
            .child(padded);
        let root: Flex<Msg> = Flex::column().width(400.0).height(600.0).child(scroll_h);
        let ui = build_ui(
            &root,
            Size::new(400.0, 600.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // A column's **vertical** scroll: a tall viewport (it fills) and it scrolls.
        let filled = ui
            .scrollables
            .iter()
            .any(|area| area.viewport.height > 300.0 && area.max_y > 0.0);
        assert!(
            filled,
            "a column fills the board's height then scrolls (scrollables: {:?})",
            ui.scrollables
        );
    }

    #[test]
    fn subtree_ids_covers_a_widget_and_its_descendants() {
        use crate::Text;
        // Root (Container) > Flex(column) > [Text, Text].
        let tree: Container<Msg> = Container::new().child(
            Flex::<Msg>::column()
                .child(Text::new("a"))
                .child(Text::new("b")),
        );
        let all = collect_ids(&tree);
        // From the root: identical to `collect_ids` (the same positional walk).
        assert_eq!(subtree_ids(&tree, WidgetId::ROOT), all);
        // A child's subtree starts with its own identity and stays a **strict** subset of the
        // tree (which is how a rich card's ghost captures all of its content).
        let flex_id = all[1];
        let sub = subtree_ids(Widget::children(&tree)[0].as_ref(), flex_id);
        assert_eq!(
            sub[0], flex_id,
            "the subtree starts with the identity supplied"
        );
        assert!(
            sub.len() < all.len() && sub.iter().all(|i| all.contains(i)),
            "a subset of the tree including the descendants"
        );
    }

    impl<Msg: Clone> Ui<Msg> {
        fn first_rect(&self) -> (Rect, Rect) {
            for primitive in self.scene.primitives() {
                if let Primitive::Rect { rect, clip, .. } = primitive {
                    return (*rect, *clip);
                }
            }
            panic!("no rect at all");
        }
    }

    /// A static subtree under `RepaintBoundary` (mixed content: text + boxes) — enough to
    /// produce several primitives.
    fn boundary_tree() -> Container<Msg> {
        use crate::Text;
        Container::new().repaint_boundary().child(
            Flex::<Msg>::column()
                .child(Text::new("Statique"))
                .child(Container::new().width(30.0).height(20.0).on_click(Msg::A))
                .child(Button::new("ok").on_press(Msg::B)),
        )
    }

    #[test]
    fn repaint_boundary_reuses_a_static_subtree_bit_identical() {
        let tree = boundary_tree();
        let size = Size::new(200.0, 200.0);
        let theme = Theme::default();
        let rt = Runtime::default();

        // Frame 1: nothing cached → 1 miss (a full paint), the subtree is captured.
        let ui1 = build_ui(&tree, size, &rt, &theme);
        // The **primitives** are what is compared (the scene's transient ambient state — the
        // current clip/owner — is not rendered).
        let dbg1 = format!("{:?}", ui1.scene().primitives());
        assert_eq!(rt.paint_cache.borrow().last_frame_stats(), (0, 1));

        // Frame 2: the same generation + the same state → the boundary is reused.
        let ui2 = build_ui(&tree, size, &rt, &theme);
        let dbg2 = format!("{:?}", ui2.scene().primitives());
        assert_eq!(
            rt.paint_cache.borrow().last_frame_stats(),
            (1, 0),
            "the boundary is reused"
        );
        assert_eq!(
            dbg1, dbg2,
            "the replayed scene is bit-for-bit identical to the full repaint"
        );
        // The interaction maps are replayed too (a click stays routable).
        assert_eq!(ui1.hits.len(), ui2.hits.len());
        assert!(!ui2.hits.is_empty(), "the subtree's hit is indeed replayed");
    }

    #[test]
    fn repaint_boundary_invalidated_by_generation_bump() {
        let tree = boundary_tree();
        let size = Size::new(200.0, 200.0);
        let theme = Theme::default();
        let rt = Runtime::default();

        build_ui(&tree, size, &rt, &theme); // frame 1 : miss + capture

        // The `view` is rebuilt (the config may have changed).
        rt.paint_cache.borrow_mut().bump_generation();
        build_ui(&tree, size, &rt, &theme); // frame 2
        assert_eq!(
            rt.paint_cache.borrow().last_frame_stats(),
            (0, 1),
            "a stale generation → a full repaint"
        );
    }

    #[test]
    fn repaint_boundary_invalidated_by_interaction_change() {
        let tree = boundary_tree();
        let size = Size::new(200.0, 200.0);
        let theme = Theme::default();
        let mut rt = Runtime::default();

        build_ui(&tree, size, &rt, &theme); // frame 1 : miss + capture

        // No rebuild, but a descendant's interaction state changes (the boundary's animated
        // hover) → the fingerprint differs → a repaint.
        rt.anims.insert(
            WidgetId::ROOT,
            crate::Anim {
                hover: 0.5,
                focus: 0.0,
                opacity: 1.0,
            },
        );
        build_ui(&tree, size, &rt, &theme); // frame 2
        assert_eq!(
            rt.paint_cache.borrow().last_frame_stats(),
            (0, 1),
            "the interaction state changed → a full repaint"
        );

        // Once the state has settled, the boundary is reused again.
        build_ui(&tree, size, &rt, &theme); // frame 3
        assert_eq!(rt.paint_cache.borrow().last_frame_stats(), (1, 0));
    }
}

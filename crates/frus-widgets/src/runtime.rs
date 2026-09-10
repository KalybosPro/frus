//! Runtime state retained between frames, **keyed by widget identity**.
//!
//! A field's *value* stays controlled (application state); what lives here is the
//! widgets' own **interaction/edit** state: hover/focus, scroll offsets, and the
//! cursor/selection position of fields. This is the foundation of reconciliation
//! by identity (laid down at Milestone 6).

use std::cell::RefCell;
use std::collections::HashMap;

use frus_core::{
    BorderRadius, Color, Curve, Insets, Point, Primitive, Rect, Simulation, Size, TextStyle,
};

use crate::interaction::{InputState, WidgetId};
use crate::overscroll::{edge_for, GlowEdge, ScrollGlows};
use crate::physics::{Ballistic, ScrollPhysics};
use crate::relayout::LayoutCache;
use crate::ui::Scrollable;

/// SingleChildScrollView offsets `(x, y)`, per scrollable region.
pub type ScrollState = HashMap<WidgetId, (f32, f32)>;

/// Edit state of an input field: cursor + selection anchor.
///
/// Indices are in **characters**. They may exceed the value's length (e.g.
/// `usize::MAX` for "the end"): widgets clamp them at use.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Edit {
    /// Cursor position.
    pub cursor: usize,
    /// Selection anchor (`None` = no selection).
    pub anchor: Option<usize>,
    /// Range `(start, end)` **being composed** by the IME (provisional text,
    /// underlined on screen); `None` outside composition. In character indices.
    pub composing: Option<(usize, usize)>,
}

impl Edit {
    /// Selected range `(start, end)`, non-empty, otherwise `None`.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.anchor
            .map(|anchor| (anchor.min(self.cursor), anchor.max(self.cursor)))
            .filter(|(start, end)| start < end)
    }
}

/// **Default** transition duration, in seconds. A widget can set its own through
/// [`crate::widget::Widget::anim_duration`].
pub(crate) const ANIM_DURATION: f32 = 0.12;

/// The **press** transition's duration, in seconds (`ink_well.dart:998`).
///
/// Its own, and longer than [`ANIM_DURATION`], because the reference gives the press
/// highlight 200 ms against the 50 ms it gives hover and focus (`ink_well.dart:995`).
/// A press is the one interaction the pointer commits to, and it is drawn slowly enough
/// to be seen arriving and leaving.
pub(crate) const PRESS_DURATION: f32 = 0.2;

/// The fling in flight on one scroll region: one simulation per axis, and the time
/// elapsed since the finger let go.
///
/// The two axes share a clock — they were launched by the same gesture — and either
/// may be `None`, since a fling usually runs on one axis only. The whole thing is
/// `Copy`, so sampling it costs no allocation.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScrollBallistic {
    pub x: Option<Ballistic>,
    pub y: Option<Ballistic>,
    /// Seconds since the release.
    pub elapsed: f32,
}

impl ScrollBallistic {
    /// A fling on both axes at once (either may be `None`), starting now.
    pub fn new(x: Option<Ballistic>, y: Option<Ballistic>) -> Self {
        Self { x, y, elapsed: 0.0 }
    }

    /// Is there anything left to run?
    pub fn is_empty(&self) -> bool {
        self.x.is_none() && self.y.is_none()
    }
}

/// How long a scrollbar waits, after its area stops moving, before it starts to go
/// (`scrollbar.dart:18`).
const BAR_TIME_TO_FADE: f32 = 0.6;
/// How long a scrollbar takes to arrive, and to leave (`scrollbar.dart:17`).
const BAR_FADE: f32 = 0.3;
/// How long the thumb takes to reach its hovered colour (`scrollbar.dart:319`).
const BAR_HOVER: f32 = 0.2;

/// **The life of one area's scrollbar over time**: whether it is on screen at all, and
/// how far the thumb has gone towards the colour it takes under a pointer.
///
/// A bar is not a permanent fixture. It arrives when the area moves, waits, and goes
/// again — which is the whole of what makes it bearable to have one at all.
#[derive(Copy, Clone, Debug, Default)]
pub struct ScrollbarFade {
    /// The offset this area was last seen at. The bar answers **movement**, and the
    /// offset is written from the drag, the fling and the spring alike; watching the
    /// value catches all three, and anything added later, without asking any of them
    /// to remember to say so.
    pub(crate) seen: (f32, f32),
    /// Seconds since it last moved.
    pub(crate) since: f32,
    /// `0.0..=1.0` — how present the bar is.
    pub opacity: f32,
    /// `0.0..=1.0` — how far the thumb has gone towards its hovered colour.
    pub hover: f32,
}

/// Stiffness of the scroll spring (px·s⁻²).
const SCROLL_K: f32 = 200.0;
/// Damping of the scroll spring.
const SCROLL_C: f32 = 28.0;
/// Elastic pull of the target back towards the valid bounds (per second) — the bounce.
const SCROLL_RETRACT: f32 = 14.0;

/// One scroll axis: elastic pull of the target back into `[0, max]`, then a spring
/// from the current offset towards that target. Returns
/// `(offset, velocity, target, moving)`.
fn scroll_axis(current: f32, vel: f32, target: f32, max: f32, dt: f32) -> (f32, f32, f32, bool) {
    let clamp_t = target.clamp(0.0, max);
    // The target is pulled back towards the valid bound (overshoot → bounce).
    let target = target + (clamp_t - target) * (1.0 - (-SCROLL_RETRACT * dt).exp());
    let (offset, vel, _) = spring_step(current, vel, target, dt, SCROLL_K, SCROLL_C);
    // Thresholds in pixels (spring_step is calibrated in fractions).
    let moving = (offset - target).abs() > 0.5 || vel.abs() > 2.0 || (target - clamp_t).abs() > 0.5;
    if moving {
        (offset, vel, target, true)
    } else {
        (clamp_t, 0.0, clamp_t, false)
    }
}

/// A widget's animation progresses (`0.0..=1.0`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Anim {
    pub hover: f32,
    pub focus: f32,
    /// The press (0 at rest), on its own clock: 200 ms against the 120 the other two
    /// run on, because the reference gives the press highlight four times the fade it
    /// gives hover and focus (`ink_well.dart:995`).
    pub press: f32,
    /// Opacity (1 at rest; started at 0 on mount for the fade-in).
    pub opacity: f32,
}

impl Default for Anim {
    fn default() -> Self {
        Self {
            hover: 0.0,
            focus: 0.0,
            press: 0.0,
            opacity: 1.0,
        }
    }
}

/// **Timeline** of an implicitly animated value (`Widget::anim_target`):
/// interpolates `from → to` according to the widget's curve and duration.
/// `current` is the value handed to the paint. A change of target **rebases** the
/// timeline from the current value (a clean, continuous restart).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ValueAnim {
    /// Current interpolated value (what the paint reads).
    pub current: f32,
    /// Starting value of the transition in progress.
    from: f32,
    /// Target of the transition in progress.
    to: f32,
    /// Time elapsed (s) since the transition started.
    elapsed: f32,
}

impl ValueAnim {
    /// A value **at rest** at `v` (no transition in progress).
    fn settled(v: f32) -> Self {
        Self {
            current: v,
            from: v,
            to: v,
            elapsed: 0.0,
        }
    }
}

/// Timeline of an animated **colour** (`Container::animated_color`): interpolates
/// `from → to` per channel, according to the widget's curve and duration. Same
/// rebase model as [`ValueAnim`], applied to a colour.
#[derive(Copy, Clone, Debug, PartialEq)]
struct ColorAnim {
    current: Color,
    from: Color,
    to: Color,
    elapsed: f32,
}

impl ColorAnim {
    fn settled(c: Color) -> Self {
        Self {
            current: c,
            from: c,
            to: c,
            elapsed: 0.0,
        }
    }
}

/// Timeline of an animated **text style** (`AnimatedDefaultTextStyle`): interpolates
/// `from → to` field by field, according to the widget's curve and duration.
///
/// Unlike every other family here, what it drives is not the node's own box or paint but
/// **the theme its subtree inherits** — so it is consumed by the theme swap, at every
/// point of the walk that makes one. See `crate::ui::scoped_theme`.
#[derive(Copy, Clone, Debug, PartialEq)]
struct TextStyleAnim {
    current: TextStyle,
    from: TextStyle,
    to: TextStyle,
    elapsed: f32,
}

impl TextStyleAnim {
    fn settled(style: TextStyle) -> Self {
        Self {
            current: style,
            from: style,
            to: style,
            elapsed: 0.0,
        }
    }
}

/// Timeline of an animated **size** (`Container::animated_size`): interpolates
/// `from → to` (width/height) according to the widget's curve and duration. The
/// interpolated size is injected **at layout** (not at paint) through `effective_style`.
#[derive(Copy, Clone, Debug, PartialEq)]
struct SizeAnim {
    current: Size,
    from: Size,
    to: Size,
    elapsed: f32,
}

impl SizeAnim {
    fn settled(s: Size) -> Self {
        Self {
            current: s,
            from: s,
            to: s,
            elapsed: 0.0,
        }
    }
}

/// Linear interpolation of two sizes (component-wise).
fn lerp_size(a: Size, b: Size, t: f32) -> Size {
    Size::new(
        a.width + (b.width - a.width) * t,
        a.height + (b.height - a.height) * t,
    )
}

/// Timeline of an animated **corner radius** (`Container::animated_radius`):
/// interpolates `from → to` (all four corners) according to the widget's curve and
/// duration. A **paint** property: delivered to the paint through `Status::anim_radius`.
#[derive(Copy, Clone, Debug, PartialEq)]
struct RadiusAnim {
    current: BorderRadius,
    from: BorderRadius,
    to: BorderRadius,
    elapsed: f32,
}

impl RadiusAnim {
    fn settled(r: BorderRadius) -> Self {
        Self {
            current: r,
            from: r,
            to: r,
            elapsed: 0.0,
        }
    }
}

/// Timeline of an animated **padding** (`Container::animated_padding`): interpolates
/// `from → to` (all four sides) according to the widget's curve and duration. The
/// interpolated padding is injected **at layout** (`effective_style`), like the size.
#[derive(Copy, Clone, Debug, PartialEq)]
struct PaddingAnim {
    current: Insets,
    from: Insets,
    to: Insets,
    elapsed: f32,
}

impl PaddingAnim {
    fn settled(p: Insets) -> Self {
        Self {
            current: p,
            from: p,
            to: p,
            elapsed: 0.0,
        }
    }
}

/// Timeline of an animated **offset pair** (`Container::animated_alignment`,
/// `FractionalTranslation::animated`): the two fractions a node's offset rule is made of,
/// interpolated together so a child moving diagonally arrives on both axes at once.
#[derive(Copy, Clone, Debug, PartialEq)]
struct OffsetAnim {
    current: (f32, f32),
    from: (f32, f32),
    to: (f32, f32),
    elapsed: f32,
}

impl OffsetAnim {
    fn settled(o: (f32, f32)) -> Self {
        Self {
            current: o,
            from: o,
            to: o,
            elapsed: 0.0,
        }
    }
}

/// Timeline of a bundle of **optional** numbers a node hands to whatever lays it out —
/// the four edges and two extents of a [`crate::Positioned`], the two factors of a
/// [`crate::FractionallySizedBox`]. One timeline for the whole bundle, so a box that
/// changes two of its edges arrives on both at once, the way a scale and a turn do.
///
/// **A slot that is unset is not a number, and so is not animated.** `None` on an edge
/// does not mean nought there — it means the box is not pinned on that side at all, and
/// the two are different arrangements rather than two values of one. So a slot arriving
/// takes effect at once, a slot leaving goes at once, and only a slot set at both ends
/// travels. The reference does exactly this and for the same reason: its tween for such
/// a value is dropped the moment the value is null.
#[derive(Copy, Clone, Debug, PartialEq)]
struct SlotsAnim<const N: usize> {
    current: [Option<f32>; N],
    from: [Option<f32>; N],
    to: [Option<f32>; N],
    elapsed: f32,
}

impl<const N: usize> SlotsAnim<N> {
    fn settled(v: [Option<f32>; N]) -> Self {
        Self {
            current: v,
            from: v,
            to: v,
            elapsed: 0.0,
        }
    }
}

/// Walks the tree collecting one family of slot bundles, following the walk's own
/// identity scheme (`child_id`) exactly — an id computed any other way is an animation
/// the paint never finds.
fn collect_slots<Msg, const N: usize>(
    widget: &dyn crate::widget::Widget<Msg>,
    id: WidgetId,
    still: bool,
    out: &mut Vec<(WidgetId, [Option<f32>; N], f32, Curve)>,
    read: impl Fn(&dyn crate::widget::Widget<Msg>) -> Option<[Option<f32>; N]> + Copy,
) {
    if let Some(target) = read(widget) {
        out.push((
            id,
            target,
            if still {
                0.0
            } else {
                widget.anim_duration().max(0.0)
            },
            widget.anim_curve(),
        ));
    }
    for (index, child) in widget.children().iter().enumerate() {
        collect_slots(
            child.as_ref(),
            crate::ui::child_id(id, index, child.as_ref()),
            still,
            out,
            read,
        );
    }
}

/// Steps one family of slot bundles towards the targets their widgets declare. Shared by
/// every quantity made of optional numbers, so the rule about an unset slot is written
/// once and cannot come out differently in two places.
fn advance_slot_family<const N: usize>(
    map: &mut HashMap<WidgetId, SlotsAnim<N>>,
    targets: Vec<(WidgetId, [Option<f32>; N], f32, Curve)>,
    dt: f32,
) -> bool {
    let present: std::collections::HashSet<WidgetId> = targets.iter().map(|(id, ..)| *id).collect();
    map.retain(|id, _| present.contains(id));

    let mut animating = false;
    for (id, target, duration, curve) in targets {
        match map.entry(id) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let s = e.get_mut();
                if s.to != target {
                    s.from = s.current;
                    s.to = target;
                    s.elapsed = 0.0;
                    // A slot with nothing at one end has nowhere to travel from or to, so
                    // it is already where it is going.
                    for i in 0..N {
                        if s.from[i].is_none() || s.to[i].is_none() {
                            s.from[i] = s.to[i];
                        }
                    }
                }
                if s.from == s.to {
                    s.current = s.to;
                } else {
                    s.elapsed += dt;
                    let t = if duration > 0.0 {
                        (s.elapsed / duration).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let e = curve.transform(t);
                    for i in 0..N {
                        s.current[i] = match (s.from[i], s.to[i]) {
                            (Some(a), Some(b)) => Some(a + (b - a) * e),
                            _ => s.to[i],
                        };
                    }
                    if t < 1.0 {
                        animating = true;
                    }
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(SlotsAnim::settled(target));
            }
        }
    }
    animating
}

/// **The numbers a paint-time transform is made of** that can sensibly be interpolated:
/// a scale on each axis and a turn.
///
/// The **pivot** is not here, and that is the decision worth stating: it is a choice of
/// origin, not a quantity. Interpolating it would move a shape across the screen while
/// every number describing the shape stayed still, which is a thing no caller ever asks
/// for and a thing nobody would be able to debug. A widget that wants the pivot to change
/// has changed what it is doing, not how much of it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TransformValues {
    /// Horizontal scale; `1.0` is neutral.
    pub scale_x: f32,
    /// Vertical scale; `1.0` is neutral.
    pub scale_y: f32,
    /// Clockwise turn, in radians.
    pub rotation: f32,
}

impl Default for TransformValues {
    /// The identity — **not** zeroes. A scale of nought is a shape with no area, so a
    /// transform mounting at `Default` would flash the subtree out of existence.
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
        }
    }
}

/// Timeline of an animated **transform** (`Transform::animated`): interpolates
/// `from → to` on both scales and the turn at once, so a widget that scales and rotates
/// together arrives on both at the same moment.
#[derive(Copy, Clone, Debug, PartialEq)]
struct TransformAnim {
    current: TransformValues,
    from: TransformValues,
    to: TransformValues,
    elapsed: f32,
}

impl TransformAnim {
    fn settled(v: TransformValues) -> Self {
        Self {
            current: v,
            from: v,
            to: v,
            elapsed: 0.0,
        }
    }
}

/// Linear interpolation of two transforms.
///
/// A scale is interpolated **linearly**, not geometrically. Halfway between 1× and 4× is
/// 2.5× here rather than 2×, which is what the reference's own `Tween<double>` gives and
/// what a caller reading the two numbers expects; a geometric mean would be defensible
/// and would disagree with every other implicit animation in the framework.
fn lerp_transform(a: TransformValues, b: TransformValues, t: f32) -> TransformValues {
    let mix = |x: f32, y: f32| x + (y - x) * t;
    TransformValues {
        scale_x: mix(a.scale_x, b.scale_x),
        scale_y: mix(a.scale_y, b.scale_y),
        rotation: mix(a.rotation, b.rotation),
    }
}

/// Linear interpolation of two insets (per side).
fn lerp_insets(a: Insets, b: Insets, t: f32) -> Insets {
    let mix = |x: f32, y: f32| x + (y - x) * t;
    Insets::new(
        mix(a.top, b.top),
        mix(a.right, b.right),
        mix(a.bottom, b.bottom),
        mix(a.left, b.left),
    )
}

/// Linear interpolation of two radii (per corner) — the one rule, `BorderRadius::lerp`,
/// so that an animated container and a decoration transition cannot round differently.
fn lerp_radius(a: BorderRadius, b: BorderRadius, t: f32) -> BorderRadius {
    a.lerp(b, t)
}

/// One damped-spring step (semi-implicit Euler) driving `progress` towards
/// `target`, primed by `velocity`. `stiffness`/`damping` set the stiffness and the
/// damping (≈ `2·√stiffness` = critical damping, no overshoot).
/// Returns `(progress, velocity, at_rest)`.
///
/// A general-purpose motion helper: used by screen transitions and by gestures
/// (a settle primed by the finger's velocity).
pub fn spring_step(
    progress: f32,
    velocity: f32,
    target: f32,
    dt: f32,
    stiffness: f32,
    damping: f32,
) -> (f32, f32, bool) {
    let accel = stiffness * (target - progress) - damping * velocity;
    let velocity = velocity + accel * dt;
    let progress = progress + velocity * dt;
    let at_rest = (progress - target).abs() < 0.004 && velocity.abs() < 0.06;
    (progress, velocity, at_rest)
}

/// A **spring** curve (the step response of a **critically** damped spring)
/// remapping a linear progress `t ∈ [0,1]`: starts at rest (zero slope), rises
/// briskly, arrives gently **without overshoot** — the same feel as the screen
/// transitions, but in closed form (no velocity state). `f(0) = 0`, `f(1) = 1`,
/// monotonically increasing.
pub fn spring_ease(t: f32) -> f32 {
    // The critical response (`omega = 8`), now provided by `frus-core`'s shared
    // animation layer: a single source of truth for this curve.
    frus_core::Curve::critical_spring().transform(t)
}

/// Drives `value` towards `target` in steps of `step`; records whether it is still moving.
fn approach(value: &mut f32, target: f32, step: f32, animating: &mut bool) {
    if *value < target {
        *value = (*value + step).min(target);
    } else if *value > target {
        *value = (*value - step).max(target);
    }
    if (*value - target).abs() > 1e-3 {
        *animating = true;
    }
}

/// Runtime context handed to `build_ui`: all the state retained between frames.
#[derive(Default)]
pub struct Runtime {
    /// **Motion is off**: the user has asked for animations to be reduced or removed,
    /// and the shell has passed that on from the platform.
    ///
    /// The **implicit** animations — the ones a widget starts by itself when a value it
    /// was given changes — complete at once instead of over time. The change still
    /// happens; it stops moving, which is what the setting asks for. Skipping the change
    /// as well would leave the interface **wrong** rather than still.
    ///
    /// It does not touch scrolling. A fling is physics answering a finger, not a
    /// decoration, and a list that jumped to a stop would be harder to use rather than
    /// calmer — the reference draws the same line.
    pub still: bool,
    /// **Whether scroll areas draw a scrollbar**, as the platform answers it and the
    /// application may override it. Set by the shell every frame, like `still` above.
    ///
    /// It lives here rather than on the theme because it is a platform behaviour and not
    /// an appearance: the reference resolves it through `ScrollBehavior`, beside the
    /// physics, and not through `ThemeData`.
    pub scrollbars: crate::physics::Scrollbars,
    /// **How present each area's scrollbar is**, and how close a pointer has come to
    /// it. Advanced by [`Runtime::advance_scroll`], which is where movement is seen.
    pub scrollbar_fade: HashMap<WidgetId, ScrollbarFade>,
    /// The area whose scrollbar a **mouse is near**, set by the shell from the previous
    /// frame's registry. A bar that has gone comes back for a pointer that reaches for
    /// it — otherwise a faded bar could never be grabbed again (`scrollbar.dart:2132`).
    pub scrollbar_hovered: Option<WidgetId>,
    /// The area whose scrollbar is **being dragged**. It stays while the thumb is held,
    /// however long the hand pauses (`scrollbar.dart:1970`).
    pub scrollbar_dragged: Option<WidgetId>,
    /// Hover / press / focus.
    pub input: InputState,
    /// **Current** scroll offsets (the rendered ones), per region.
    pub scroll: ScrollState,
    /// **Target** scroll offsets (what the spring drives towards), per region.
    pub scroll_target: ScrollState,
    /// SingleChildScrollView velocity (for the spring), per region.
    pub scroll_velocity: ScrollState,
    /// The **ballistic** motion still running after a fling, per region — the
    /// simulation the platform's physics handed us, sampled frame by frame by
    /// [`Runtime::advance_scroll`]. Absent = no fling in flight.
    pub scroll_ballistic: HashMap<WidgetId, ScrollBallistic>,
    /// The overscroll glows of each region — the edge feedback a platform that
    /// clamps needs, since it has no bounce to speak with. Absent = all quiet.
    pub scroll_glow: HashMap<WidgetId, ScrollGlows>,
    /// The **ink** splashed on each surface that takes it, keyed by the surface. A
    /// ripple outlives the frame that started it — a tap is 450 ms of motion — so it
    /// is retained here rather than in the widget, which is rebuilt every frame.
    /// Absent = dry.
    pub ink: HashMap<WidgetId, crate::ink::Ripples>,
    /// The retained pull of each [`crate::RefreshIndicator`] area, keyed by the area. Absent =
    /// nothing pulled and nothing spinning.
    pub refresh: HashMap<WidgetId, crate::refresh::RefreshPull>,
    /// The retained swipe of each [`crate::Dismissible`] item. Absent = at rest.
    pub dismiss: HashMap<WidgetId, crate::dismiss::DismissState>,
    /// The page each [`crate::PageView`] was last **told to show**, so that a request
    /// is acted on when it *changes* rather than re-asserted every frame — which
    /// would leave the offset unswipeable. Absent = never seen, so the next request
    /// is the initial page and arrives without an animation.
    pub page_requested: HashMap<WidgetId, usize>,
    /// The box each scroll region was last asked to **keep in view**, by the key the
    /// content named it with, so that the request is acted on when it *changes* rather
    /// than re-asserted every frame — which would pin the content and leave it
    /// undraggable. Absent = never seen, and the walk has already opened there.
    pub kept_visible: HashMap<WidgetId, u64>,
    /// The page each paged view was last **reported as showing**, so that
    /// `on_page_changed` fires on a change and not on every frame of the motion.
    pub page_shown: HashMap<WidgetId, usize>,
    /// The offset each scroll region was last **reported at**, so that
    /// [`crate::Widget::on_scroll`] fires on a change and not on every frame, and so
    /// that a region's grain is measured from what the application was actually told
    /// rather than from the previous frame — otherwise a slow drag would never travel
    /// a whole grain in one frame and would never be reported at all.
    pub scroll_reported: HashMap<WidgetId, (f32, f32)>,
    /// The drop target a drag is currently **over**, when it would accept it. The
    /// target paints its own "drop it here" state from this, through
    /// [`crate::interaction::Status::drag_over`] — the shell decides *which* target,
    /// the widget decides what that looks like.
    pub drag_over: Option<WidgetId>,
    /// The region a finger is currently holding, if any.
    ///
    /// A scroll offset has **one owner at a time**. While a drag owns it, nothing
    /// else may move it: without this, the edge spring keeps retracting the offset
    /// between two moves of the finger, and a rubber band pulled against a bouncing
    /// edge is dragged back as fast as it is stretched — it never appears at all.
    /// At most one, because at most one gesture is live.
    pub scroll_held: Option<WidgetId>,
    /// Retained transform (scale + translation) of each
    /// [`InteractiveViewer`](crate::InteractiveViewer), per viewport. Absent =
    /// identity (scale 1, no translation).
    pub interactive: HashMap<WidgetId, crate::interactive::InteractiveView>,
    /// Pan velocity (px/s) of a *fling* still running after release, per viewport —
    /// decelerated every frame by [`Runtime::advance_interactive`]. Absent = at rest.
    pub interactive_velocity: HashMap<WidgetId, (f32, f32)>,
    /// Edit state, per input field.
    pub edits: HashMap<WidgetId, Edit>,
    /// **Undo history**, per input field: what it was, and what it was about to be.
    ///
    /// Beside `edits` rather than inside it, because they are different things kept for
    /// different reasons — a caret is where the field is now, a history is everywhere it
    /// has been — and because `Edit` is `Copy` and a history is not.
    pub undo: HashMap<WidgetId, crate::undo::UndoHistory>,
    /// Animation progresses (hover/focus/opacity), per widget.
    pub anims: HashMap<WidgetId, Anim>,
    /// The widgets' own animated values (`Widget::anim_target`), per widget — each
    /// one a curved **timeline** (see [`ValueAnim`]).
    pub values: HashMap<WidgetId, ValueAnim>,
    /// Animated background colours (`Container::animated_color`), per widget.
    colors: HashMap<WidgetId, ColorAnim>,
    /// Animated inherited text styles (`AnimatedDefaultTextStyle`), per widget — read by
    /// the **theme swap**, which is why this one is consumed nowhere near the others.
    text_styles: HashMap<WidgetId, TextStyleAnim>,
    /// Animated sizes (`Container::animated_size`), per widget — injected at layout.
    sizes: HashMap<WidgetId, SizeAnim>,
    /// Animated corner radii (`Container::animated_radius`), per widget.
    radii: HashMap<WidgetId, RadiusAnim>,
    /// Animated paddings (`Container::animated_padding`), per widget — injected at layout.
    paddings: HashMap<WidgetId, PaddingAnim>,
    /// Animated offset pairs (an alignment, a slide), per widget — read at paint.
    offsets: HashMap<WidgetId, OffsetAnim>,
    /// Animated transforms (`Transform::animated`), per widget — read at paint.
    transforms: HashMap<WidgetId, TransformAnim>,
    /// Animated stack pins (`AnimatedPositioned`), per widget — read where the stack
    /// lays its layers out. Six slots: left, top, right, bottom, width, height.
    pins: HashMap<WidgetId, SlotsAnim<6>>,
    /// Animated fractions of the parent (`FractionallySizedBox::animated`), per widget —
    /// injected at layout. Two slots: the width factor and the height factor.
    fractions: HashMap<WidgetId, SlotsAnim<2>>,
    /// Widgets present at the previous frame (to detect mounts).
    pub mounted: std::collections::HashSet<WidgetId>,
    /// Snapshots of outgoing subtrees, fading out: event key → (captured
    /// primitives, remaining opacity `1 → 0`).
    pub leaving: HashMap<u64, (Vec<Primitive>, f32)>,
    /// What each [`AnimatedSwitcher`](crate::AnimatedSwitcher) is showing and letting go
    /// of, per widget. Interior mutability, like the caches below: a switcher meets a new
    /// value while the tree is being **built**, which is the only time it has one.
    pub(crate) switchers: RefCell<HashMap<WidgetId, crate::switcher::Switch>>,
    /// Time elapsed (seconds) since start-up, for continuous animations.
    pub time: f32,
    /// Was the last interaction a **keyboard** one? The generic focus ring is only
    /// painted in that case (`FocusHighlightMode`: a click must not flash a ring).
    /// The focus itself stays active.
    pub focus_visible: bool,
    /// Relayout-boundary cache (rects retained per layout root, from one frame to
    /// the next). Interior mutability: `build_ui` updates it while holding only a
    /// shared reference to the `Runtime`.
    pub layout_cache: RefCell<LayoutCache>,
    /// **Repaint**-boundary cache (primitives + interactions retained per boundary,
    /// from one frame to the next). Same interior mutability.
    pub paint_cache: RefCell<crate::paintcache::PaintCache>,
}

impl Runtime {
    /// A runtime that answers a given **scrollbar policy** — what the shell sets from the
    /// application's `scrollbars()`, and what an isolated render has to say for itself,
    /// there being no shell to set it.
    pub fn with_scrollbars(scrollbars: crate::physics::Scrollbars) -> Self {
        Self {
            scrollbars,
            ..Self::default()
        }
    }

    /// A widget's animated hover progress.
    pub fn hover_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.hover).unwrap_or(0.0)
    }

    /// A widget's animated focus progress.
    pub fn focus_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.focus).unwrap_or(0.0)
    }

    /// A widget's animated **press** progress, or `default` where the runtime has never
    /// heard of it.
    ///
    /// The fallback is the rule [`Runtime::value_or`] applies: a frame built before the
    /// loop has advanced anything — a test, an isolated render — must draw a widget that
    /// *is* being held as held, not as untouched on its way there.
    pub fn press_progress_or(&self, id: WidgetId, default: f32) -> f32 {
        self.anims.get(&id).map(|a| a.press).unwrap_or(default)
    }

    /// A widget's animated press progress (0 by default).
    pub fn press_progress(&self, id: WidgetId) -> f32 {
        self.press_progress_or(id, 0.0)
    }

    /// A widget's animated opacity (1 by default).
    pub fn opacity(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.opacity).unwrap_or(1.0)
    }

    /// A widget's animated value (0 by default).
    pub fn value(&self, id: WidgetId) -> f32 {
        self.values.get(&id).map(|v| v.current).unwrap_or(0.0)
    }

    /// A widget's animated value, or `default` if **no** value has been recorded yet
    /// (a widget never animated — e.g. an isolated render with no loop). Lets the
    /// target be adopted immediately, as on mount.
    pub fn value_or(&self, id: WidgetId, default: f32) -> f32 {
        self.values.get(&id).map(|v| v.current).unwrap_or(default)
    }

    /// Sets a widget's animated value to `v` (at rest, no transition in progress) —
    /// for isolated renders/tests that want a precise progress without running the
    /// animation.
    pub fn set_value(&mut self, id: WidgetId, v: f32) {
        self.values.insert(id, ValueAnim::settled(v));
    }

    /// Drives every animated value towards the target its widget declares
    /// (`Widget::anim_target`). A widget seen for the **first** time adopts its
    /// target with no transition (no animation on mount). Returns `true` if a value
    /// is still moving.
    pub fn advance_values<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            still: bool,
            out: &mut Vec<(WidgetId, f32, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_target() {
                out.push((
                    id,
                    target,
                    if still {
                        0.0
                    } else {
                        widget.anim_duration().max(0.0)
                    },
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    still,
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, f32, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, self.still, &mut targets);

        // Forget the values of widgets that have gone.
        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.values.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.values.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let v = e.get_mut();
                    // New target: rebase the timeline from the current value.
                    if v.to != target {
                        v.from = v.current;
                        v.to = target;
                        v.elapsed = 0.0;
                    }
                    if v.from == v.to {
                        v.current = v.to;
                    } else {
                        v.elapsed += dt;
                        let t = if duration > 0.0 {
                            (v.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        v.current = v.from + (v.to - v.from) * curve.transform(t);
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    // Mount: adopts the target with no transition.
                    e.insert(ValueAnim::settled(target));
                }
            }
        }
        animating
    }

    /// A widget's animated background colour, if in transition (`None` otherwise).
    pub fn anim_color(&self, id: WidgetId) -> Option<Color> {
        self.colors.get(&id).map(|c| c.current)
    }

    /// Drives every animated background colour towards the target its widget
    /// declares (`Widget::anim_color`), following its duration/curve
    /// (`anim_duration`/`anim_curve`). On mount: adopts the target with no
    /// transition. Returns `true` if a colour is still moving. Same model as
    /// [`Self::advance_values`].
    pub fn advance_colors<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            still: bool,
            out: &mut Vec<(WidgetId, Color, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_color() {
                out.push((
                    id,
                    target,
                    if still {
                        0.0
                    } else {
                        widget.anim_duration().max(0.0)
                    },
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    still,
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, Color, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, self.still, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.colors.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.colors.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let c = e.get_mut();
                    if c.to != target {
                        c.from = c.current;
                        c.to = target;
                        c.elapsed = 0.0;
                    }
                    if c.from == c.to {
                        c.current = c.to;
                    } else {
                        c.elapsed += dt;
                        let t = if duration > 0.0 {
                            (c.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        c.current = c.from.lerp(c.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(ColorAnim::settled(target));
                }
            }
        }
        animating
    }

    /// A widget's animated inherited text style, if in transition (`None` otherwise).
    ///
    /// Read by the theme swap rather than by the node itself: see
    /// `crate::ui::scoped_theme`, which is the one place the value is consumed and the
    /// reason there is one place at all.
    pub fn anim_text_style(&self, id: WidgetId) -> Option<TextStyle> {
        self.text_styles.get(&id).map(|s| s.current)
    }

    /// Drives every animated inherited text style towards the target its widget declares
    /// (`Widget::anim_text_style`), following its duration/curve. On mount: adopts the
    /// target with no transition. Returns `true` if a style is still moving.
    ///
    /// Same model as [`Self::advance_colors`], over [`TextStyle::lerp`] — which is where
    /// the rules live for what travels, what holds and what swaps at the halfway point.
    pub fn advance_text_styles<Msg>(
        &mut self,
        root: &dyn crate::widget::Widget<Msg>,
        dt: f32,
    ) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            still: bool,
            out: &mut Vec<(WidgetId, TextStyle, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_text_style() {
                out.push((
                    id,
                    target,
                    if still {
                        0.0
                    } else {
                        widget.anim_duration().max(0.0)
                    },
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    still,
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, TextStyle, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, self.still, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.text_styles.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.text_styles.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let s = e.get_mut();
                    if s.to != target {
                        s.from = s.current;
                        s.to = target;
                        s.elapsed = 0.0;
                    }
                    if s.from == s.to {
                        s.current = s.to;
                    } else {
                        s.elapsed += dt;
                        let t = if duration > 0.0 {
                            (s.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        s.current = s.from.lerp(s.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(TextStyleAnim::settled(target));
                }
            }
        }
        animating
    }

    /// A widget's animated size, if in transition (`None` otherwise).
    pub fn anim_size(&self, id: WidgetId) -> Option<Size> {
        self.sizes.get(&id).map(|s| s.current)
    }

    /// Drives every animated size towards the target its widget declares
    /// (`Widget::anim_size`), following its duration/curve. On mount: adopts the
    /// target with no transition. Returns `true` if a size is still moving. Same
    /// model as [`Self::advance_values`], except the output is **consumed at
    /// layout** (`effective_style`), not at paint.
    pub fn advance_sizes<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            still: bool,
            out: &mut Vec<(WidgetId, Size, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_size() {
                out.push((
                    id,
                    target,
                    if still {
                        0.0
                    } else {
                        widget.anim_duration().max(0.0)
                    },
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    still,
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, Size, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, self.still, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.sizes.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.sizes.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let s = e.get_mut();
                    if s.to != target {
                        s.from = s.current;
                        s.to = target;
                        s.elapsed = 0.0;
                    }
                    if s.from == s.to {
                        s.current = s.to;
                    } else {
                        s.elapsed += dt;
                        let t = if duration > 0.0 {
                            (s.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        s.current = lerp_size(s.from, s.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(SizeAnim::settled(target));
                }
            }
        }
        animating
    }

    /// A widget's animated corner radius, if in transition (`None` otherwise).
    pub fn anim_radius(&self, id: WidgetId) -> Option<BorderRadius> {
        self.radii.get(&id).map(|r| r.current)
    }

    /// Drives every animated corner radius towards the target its widget declares
    /// (`Widget::anim_radius`), following its duration/curve. On mount: adopts the
    /// target with no transition. Returns `true` if a radius is still moving. Same
    /// model as [`Self::advance_colors`], applied to a [`BorderRadius`].
    pub fn advance_radii<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            still: bool,
            out: &mut Vec<(WidgetId, BorderRadius, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_radius() {
                out.push((
                    id,
                    target,
                    if still {
                        0.0
                    } else {
                        widget.anim_duration().max(0.0)
                    },
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    still,
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, BorderRadius, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, self.still, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.radii.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.radii.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let r = e.get_mut();
                    if r.to != target {
                        r.from = r.current;
                        r.to = target;
                        r.elapsed = 0.0;
                    }
                    if r.from == r.to {
                        r.current = r.to;
                    } else {
                        r.elapsed += dt;
                        let t = if duration > 0.0 {
                            (r.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        r.current = lerp_radius(r.from, r.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(RadiusAnim::settled(target));
                }
            }
        }
        animating
    }

    /// A widget's animated transform, if in transition (`None` otherwise).
    pub fn anim_transform(&self, id: WidgetId) -> Option<TransformValues> {
        self.transforms.get(&id).map(|t| t.current)
    }

    /// Drives every animated transform towards the target its widget declares
    /// (`Widget::anim_transform`), following its duration/curve. On mount: adopts the
    /// target with no transition — a widget that appears already scaled does not scale
    /// in from nothing. Returns `true` if a transform is still moving. The output is
    /// consumed **at paint**, unlike the size and the padding.
    pub fn advance_transforms<Msg>(
        &mut self,
        root: &dyn crate::widget::Widget<Msg>,
        dt: f32,
    ) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            still: bool,
            out: &mut Vec<(WidgetId, TransformValues, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_transform() {
                out.push((
                    id,
                    target,
                    if still {
                        0.0
                    } else {
                        widget.anim_duration().max(0.0)
                    },
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    still,
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, TransformValues, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, self.still, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.transforms.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.transforms.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let v = e.get_mut();
                    if v.to != target {
                        v.from = v.current;
                        v.to = target;
                        v.elapsed = 0.0;
                    }
                    if v.from == v.to {
                        v.current = v.to;
                    } else {
                        v.elapsed += dt;
                        let t = if duration > 0.0 {
                            (v.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        v.current = lerp_transform(v.from, v.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(TransformAnim::settled(target));
                }
            }
        }
        animating
    }

    /// A widget's animated padding, if in transition (`None` otherwise).
    pub fn anim_padding(&self, id: WidgetId) -> Option<Insets> {
        self.paddings.get(&id).map(|p| p.current)
    }

    /// A widget's animated offset pair, if in transition (`None` otherwise).
    pub fn anim_offset(&self, id: WidgetId) -> Option<(f32, f32)> {
        self.offsets.get(&id).map(|o| o.current)
    }

    /// Drives every animated offset pair towards the target its widget declares
    /// (`Widget::anim_offset`), following its duration/curve. On mount: adopts the target
    /// with no transition. Returns `true` if one is still moving.
    ///
    /// The pair belongs to whichever **offset rule** the node declares — its alignment,
    /// or its slide — which is why one quantity serves both: a node has one of those
    /// rules, and the wrappers in `animated.rs` each give it a node of its own.
    pub fn advance_offsets<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            still: bool,
            out: &mut Vec<(WidgetId, (f32, f32), f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_offset() {
                out.push((
                    id,
                    target,
                    if still {
                        0.0
                    } else {
                        widget.anim_duration().max(0.0)
                    },
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    still,
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, (f32, f32), f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, self.still, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.offsets.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.offsets.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let o = e.get_mut();
                    if o.to != target {
                        o.from = o.current;
                        o.to = target;
                        o.elapsed = 0.0;
                    }
                    if o.from == o.to {
                        o.current = o.to;
                    } else {
                        o.elapsed += dt;
                        let t = if duration > 0.0 {
                            (o.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        let e = curve.transform(t);
                        o.current = (
                            o.from.0 + (o.to.0 - o.from.0) * e,
                            o.from.1 + (o.to.1 - o.from.1) * e,
                        );
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(OffsetAnim::settled(target));
                }
            }
        }
        animating
    }

    /// Drives every animated padding towards the target its widget declares
    /// (`Widget::anim_padding`), following its duration/curve. On mount: adopts the
    /// target with no transition. Returns `true` if a padding is still moving. Like
    /// the size, the output is **consumed at layout** (`effective_style`).
    pub fn advance_paddings<Msg>(
        &mut self,
        root: &dyn crate::widget::Widget<Msg>,
        dt: f32,
    ) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            still: bool,
            out: &mut Vec<(WidgetId, Insets, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_padding() {
                out.push((
                    id,
                    target,
                    if still {
                        0.0
                    } else {
                        widget.anim_duration().max(0.0)
                    },
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    still,
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, Insets, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, self.still, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.paddings.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.paddings.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let p = e.get_mut();
                    if p.to != target {
                        p.from = p.current;
                        p.to = target;
                        p.elapsed = 0.0;
                    }
                    if p.from == p.to {
                        p.current = p.to;
                    } else {
                        p.elapsed += dt;
                        let t = if duration > 0.0 {
                            (p.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        p.current = lerp_insets(p.from, p.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(PaddingAnim::settled(target));
                }
            }
        }
        animating
    }

    /// A widget's animated stack pins, if it has any (`None` otherwise).
    pub fn anim_pins(&self, id: WidgetId) -> Option<crate::positioned::Positioning> {
        self.pins.get(&id).map(|p| {
            let [left, top, right, bottom, width, height] = p.current;
            crate::positioned::Positioning {
                left,
                top,
                right,
                bottom,
                width,
                height,
            }
        })
    }

    /// A widget's animated fractions of its parent, if it has any (`None` otherwise), as
    /// `(width factor, height factor)`.
    pub fn anim_fractions(&self, id: WidgetId) -> Option<(Option<f32>, Option<f32>)> {
        self.fractions
            .get(&id)
            .map(|f| (f.current[0], f.current[1]))
    }

    /// Drives every animated set of stack pins towards the target its widget declares
    /// (`Widget::anim_pins`), following its duration/curve. On mount: adopts the target
    /// with no transition. Returns `true` if a layer is still moving.
    ///
    /// The output is consumed where the **stack** lays its layers out, which is the only
    /// place pins mean anything — a pin is a request to a parent of one particular kind,
    /// not a property of the box.
    pub fn advance_pins<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        let mut targets = Vec::new();
        collect_slots(root, WidgetId::ROOT, self.still, &mut targets, |w| {
            w.anim_pins()
                .map(|p| [p.left, p.top, p.right, p.bottom, p.width, p.height])
        });
        advance_slot_family(&mut self.pins, targets, dt)
    }

    /// Drives every animated pair of fractions towards the target its widget declares
    /// (`Widget::anim_fractions`), following its duration/curve. On mount: adopts the
    /// target with no transition. Returns `true` if a box is still growing or shrinking.
    ///
    /// Like the size and the padding, the output is **consumed at layout**
    /// (`effective_style`): a share of the parent is a claim on room, and a claim on room
    /// is settled before anything is drawn.
    pub fn advance_fractions<Msg>(
        &mut self,
        root: &dyn crate::widget::Widget<Msg>,
        dt: f32,
    ) -> bool {
        let mut targets = Vec::new();
        collect_slots(root, WidgetId::ROOT, self.still, &mut targets, |w| {
            w.anim_fractions().map(|(width, height)| [width, height])
        });
        advance_slot_family(&mut self.fractions, targets, dt)
    }

    /// Advances the transitions (hover/focus/press) by `dt` seconds towards their
    /// targets. Returns `true` if at least one animation is still running.
    pub fn advance(&mut self, dt: f32) -> bool {
        let hovered = self.input.hovered;
        let focused = self.input.focused;
        // Held **and** still under the pointer, which is what makes it a press: the same
        // rule `InputState::status_for` applies, so the progression and the flag can
        // never disagree about what is happening.
        let held = self.input.pressed.filter(|id| Some(*id) == hovered);
        if let Some(id) = hovered {
            self.anims.entry(id).or_default();
        }
        if let Some(id) = focused {
            self.anims.entry(id).or_default();
        }
        if let Some(id) = held {
            self.anims.entry(id).or_default();
        }

        let step = if ANIM_DURATION > 0.0 {
            dt / ANIM_DURATION
        } else {
            1.0
        };
        // The press runs on its own, slower clock (`ink_well.dart:995`).
        let press_step = if PRESS_DURATION > 0.0 {
            dt / PRESS_DURATION
        } else {
            1.0
        };
        let mut animating = false;

        self.anims.retain(|id, anim| {
            let hover_target = if Some(*id) == hovered { 1.0 } else { 0.0 };
            let focus_target = if Some(*id) == focused { 1.0 } else { 0.0 };
            let press_target = if Some(*id) == held { 1.0 } else { 0.0 };
            approach(&mut anim.hover, hover_target, step, &mut animating);
            approach(&mut anim.focus, focus_target, step, &mut animating);
            approach(&mut anim.press, press_target, press_step, &mut animating);
            // Opacity always tends towards 1 (the fade-in).
            approach(&mut anim.opacity, 1.0, step, &mut animating);
            // Entries that are entirely at rest are forgotten (nothing to animate). A
            // press still fading **out** is not at rest: forgetting it here would be the
            // jump this progression exists to remove, on the way back.
            !(hover_target == 0.0
                && focus_target == 0.0
                && press_target == 0.0
                && anim.hover <= 0.0
                && anim.focus <= 0.0
                && anim.press <= 0.0
                && anim.opacity >= 1.0)
        });

        animating
    }

    /// Advances every scroll region by `dt`, and returns `true` if any is still
    /// moving.
    ///
    /// Two motions live here, and they are not the same thing:
    ///
    /// - a **fling**, when the finger let go with speed: the region's physics built
    ///   a simulation ([`ScrollBallistic`]), and we sample it. This is where the
    ///   platform's feel lives — spline deceleration that stops at the edge, or
    ///   friction that hands over to a spring and bounces.
    /// - a **glide to a target**, for the discrete inputs (the wheel, a programmatic
    ///   scroll): a spring eases the offset across to where it was asked to go.
    ///
    /// A fling wins while it runs: it drives the offset directly and keeps the
    /// target in step, so the spring has nothing to pull against.
    ///
    /// `regions` describes the scrollables of the last frame; `default` is the
    /// application's physics, used by every region that did not ask for its own.
    pub fn advance_scroll(
        &mut self,
        regions: &[Scrollable],
        default: ScrollPhysics,
        dt: f32,
    ) -> bool {
        let mut animating = false;
        let ballistic_ids: Vec<WidgetId> = self.scroll_ballistic.keys().copied().collect();
        for id in ballistic_ids {
            if self.advance_ballistic(id, regions, default, dt) {
                animating = true;
            }
        }

        let ids: Vec<WidgetId> = self.scroll_target.keys().copied().collect();
        for id in ids {
            // Under a fling this region's offset is already settled for the frame.
            if self.scroll_ballistic.contains_key(&id) {
                continue;
            }
            // Under a finger it belongs to the finger, full stop.
            if self.scroll_held == Some(id) {
                continue;
            }
            let area = regions.iter().find(|area| area.id == id);
            let (max_x, max_y) = area.map(|a| (a.max_x, a.max_y)).unwrap_or((0.0, 0.0));
            let current = self.scroll.get(&id).copied().unwrap_or((0.0, 0.0));
            let target = self.scroll_target.get(&id).copied().unwrap_or(current);
            let vel = self.scroll_velocity.get(&id).copied().unwrap_or((0.0, 0.0));

            let (cx, vx, tx, ax) = scroll_axis(current.0, vel.0, target.0, max_x, dt);
            let (cy, vy, ty, ay) = scroll_axis(current.1, vel.1, target.1, max_y, dt);

            self.scroll.insert(id, (cx, cy));
            if ax || ay {
                self.scroll_target.insert(id, (tx, ty));
                self.scroll_velocity.insert(id, (vx, vy));
                animating = true;
            } else {
                // At rest: the animation state is cleared (the current offset stays).
                self.scroll_target.remove(&id);
                self.scroll_velocity.remove(&id);
            }
        }
        animating | self.advance_scrollbars(regions, dt)
    }

    /// Advances every area's **scrollbar fade** by `dt`, and returns `true` while any of
    /// them has something left to do — including simply *waiting* to start going.
    ///
    /// The bar arrives on movement and leaves `BAR_TIME_TO_FADE` after it stops. Nothing
    /// reports that movement: the offset is written from the drag handler, from the
    /// fling and from the spring, and none of the three leaves word. So this compares
    /// the offset against the one it saw last frame, which catches every writer there
    /// is, and every writer there will be.
    fn advance_scrollbars(&mut self, regions: &[Scrollable], dt: f32) -> bool {
        self.scrollbar_fade
            .retain(|id, _| regions.iter().any(|area| area.id == *id));
        let mut animating = false;
        for area in regions {
            let now = self.scroll.get(&area.id).copied().unwrap_or((0.0, 0.0));
            let hovered = self.scrollbar_hovered == Some(area.id);
            let held = hovered || self.scrollbar_dragged == Some(area.id);
            let fade = self.scrollbar_fade.entry(area.id).or_insert(ScrollbarFade {
                seen: now,
                // An area **nobody has moved yet** has no bar: the reference's fade
                // starts closed and is only ever opened by a scroll notification. A
                // list that has just appeared shows its content, not its furniture.
                since: BAR_TIME_TO_FADE,
                ..ScrollbarFade::default()
            });
            if fade.seen != now {
                fade.seen = now;
                fade.since = 0.0;
            } else if held {
                fade.since = 0.0;
            } else {
                fade.since += dt;
            }
            let waiting = fade.since < BAR_TIME_TO_FADE;
            let target = if waiting { 1.0 } else { 0.0 };
            approach(&mut fade.opacity, target, dt / BAR_FADE, &mut animating);
            let warm = if hovered { 1.0 } else { 0.0 };
            approach(&mut fade.hover, warm, dt / BAR_HOVER, &mut animating);
            // The wait is not a movement, but it still needs a frame at the end of it,
            // or the bar would sit there until something else happened to ask for one.
            if waiting && !held {
                animating = true;
            }
        }
        animating
    }

    /// What `id`'s scrollbar looks like this frame. An area nothing has touched, or one
    /// rendered in isolation with no runtime behind it, answers a closed fade.
    pub fn scrollbar_fade_of(&self, id: WidgetId) -> ScrollbarFade {
        self.scrollbar_fade.get(&id).copied().unwrap_or_default()
    }

    /// Samples one region's fling for this frame. Returns `true` while it runs.
    ///
    /// An axis ends either because its simulation says so, or because it hit an edge
    /// the physics does not let it cross — under clamping physics the simulation
    /// knows nothing of the bounds, so reaching one **stops that axis dead**, which
    /// is exactly the behaviour a platform without a bounce wants.
    fn advance_ballistic(
        &mut self,
        id: WidgetId,
        regions: &[Scrollable],
        default: ScrollPhysics,
        dt: f32,
    ) -> bool {
        let Some(mut fling) = self.scroll_ballistic.get(&id).copied() else {
            return false;
        };
        // A region that vanished from the frame (a route changed under the fling)
        // has nothing left to move.
        let Some(area) = regions.iter().find(|area| area.id == id).copied() else {
            self.scroll_ballistic.remove(&id);
            return false;
        };
        let physics = area.physics_or(default);
        fling.elapsed += dt;
        let t = fling.elapsed;
        let current = self.scroll.get(&id).copied().unwrap_or((0.0, 0.0));

        // An axis that slams into an edge reports the speed it lost there, so the
        // glow can show what the clamp swallowed.
        let mut absorbed: Vec<(GlowEdge, f32)> = Vec::new();
        let mut axis = |sim: &mut Option<Ballistic>, previous: f32, max: f32, vertical: bool| {
            let Some(active) = sim.as_ref() else {
                return previous;
            };
            let mut position = active.x(t);
            let mut finished = active.is_done(t);
            if !physics.allows_overscroll() {
                let pinned = position.clamp(0.0, max);
                if pinned != position {
                    // Hitting the edge is the end of the motion, not a pause at it.
                    finished = true;
                    absorbed.push((edge_for(vertical, position - pinned), active.dx(t)));
                }
                position = pinned;
            }
            if finished {
                *sim = None;
            }
            position
        };

        let x = axis(&mut fling.x, current.0, area.max_x, false);
        let y = axis(&mut fling.y, current.1, area.max_y, true);
        for (edge, velocity) in absorbed {
            self.glow_absorb(id, edge, velocity);
        }
        self.scroll.insert(id, (x, y));
        // The glide-to-target path must not fight the fling, nor yank the offset
        // back when the fling ends.
        self.scroll_target.insert(id, (x, y));
        self.scroll_velocity.remove(&id);

        if fling.is_empty() {
            self.scroll_ballistic.remove(&id);
            self.scroll_target.remove(&id);
            false
        } else {
            self.scroll_ballistic.insert(id, fling);
            true
        }
    }

    /// Launches a fling on `area` from the release `velocity` (px/s, in scroll
    /// space), under `physics`. A release too slow to fling leaves any overscroll to
    /// be sprung back, and otherwise does nothing.
    pub fn fling_scroll(
        &mut self,
        area: Scrollable,
        physics: ScrollPhysics,
        velocity: (f32, f32),
    ) -> bool {
        let current = self.scroll.get(&area.id).copied().unwrap_or((0.0, 0.0));
        // A paged area does not fling: it goes to **a** page, and the release speed
        // only says which one. The raw velocity is passed on rather than the
        // fling-filtered one, because the question a page view asks of a release is
        // "which way did it go", to which 60 px/s is as clear an answer as 2000.
        //
        // Unless it has been told not to snap, in which case it is a scrollable that
        // happens to know where its pages are: it still reports them and still glides
        // to the one it is asked for, and only the release falls through to the
        // ordinary fling below.
        if let Some(snap) = area.page.filter(|snap| snap.snapping) {
            let (metrics, velocity) = if snap.horizontal {
                (area.metrics_x(current.0), velocity.0)
            } else {
                (area.metrics_y(current.1), velocity.1)
            };
            let motion = physics.page_ballistic(metrics, velocity, snap.extent);
            if motion.is_none() {
                return false;
            }
            let ballistic = if snap.horizontal {
                ScrollBallistic::new(motion, None)
            } else {
                ScrollBallistic::new(None, motion)
            };
            self.scroll_ballistic.insert(area.id, ballistic);
            self.scroll_velocity.remove(&area.id);
            return true;
        }
        let min_velocity = physics.min_fling_velocity();
        // Below the threshold the gesture was not a fling — but an offset left out
        // of range still has to come home, so the physics is asked either way, with
        // a velocity of zero.
        let vx = if velocity.0.abs() >= min_velocity {
            velocity.0
        } else {
            0.0
        };
        let vy = if velocity.1.abs() >= min_velocity {
            velocity.1
        } else {
            0.0
        };
        let x = physics.ballistic(area.metrics_x(current.0), vx);
        let y = physics.ballistic(area.metrics_y(current.1), vy);
        if x.is_none() && y.is_none() {
            return false;
        }
        self.scroll_ballistic
            .insert(area.id, ScrollBallistic::new(x, y));
        self.scroll_velocity.remove(&area.id);
        true
    }

    /// Acts on the page each paged view is **asking** for.
    ///
    /// A request is honoured when it *changes*, never re-asserted: the widget is
    /// rebuilt every frame and carries the same number each time, so a view that
    /// obeyed it on every frame could not be swiped at all — the finger would move
    /// the offset and the next frame would put it straight back.
    ///
    /// The **first** sighting of a region is its initial page, and arrives without
    /// an animation: an application opening on page 3 wants to be on page 3, not to
    /// watch it fly there.
    pub fn sync_pages(&mut self, regions: &[Scrollable]) {
        for area in regions {
            let Some(snap) = area.page else { continue };
            let previous = self.page_requested.insert(area.id, snap.requested);
            if previous == Some(snap.requested) {
                continue;
            }
            let max = if snap.horizontal {
                area.max_x
            } else {
                area.max_y
            };
            let offset = snap
                .offset_of(snap.requested.min(snap.count.saturating_sub(1)))
                .clamp(0.0, max);
            let current = self.scroll.get(&area.id).copied().unwrap_or((0.0, 0.0));
            let target = if snap.horizontal {
                (offset, current.1)
            } else {
                (current.0, offset)
            };
            match previous {
                // Never seen: this is where the view opens.
                None => {
                    self.scroll.insert(area.id, target);
                    self.scroll_target.remove(&area.id);
                    self.scroll_velocity.remove(&area.id);
                }
                // A change under way: glide, and let go of anything already moving
                // the offset — the application has just overruled it.
                Some(_) => {
                    self.scroll_ballistic.remove(&area.id);
                    self.scroll_target.insert(area.id, target);
                }
            }
        }
    }

    /// Brings each region's **kept** box into view when it has changed.
    ///
    /// A tab bar wider than its window is what this is for: the application selects the
    /// eighth tab and the bar is still showing the first three, with nothing to say the
    /// selection went anywhere.
    ///
    /// Acted on **when the key changes**, never re-asserted. The box moves as the region
    /// scrolls, so a region that chased the box itself would pin the strip in place and
    /// no finger could move it — the same trap [`Runtime::sync_pages`] avoids, and the
    /// same answer.
    ///
    /// The **first** sighting arrives without an animation — a bar restored on its
    /// eighth tab wants to be on the eighth tab, not to watch it slide there — and the
    /// walk has already drawn that first frame at the same offset, so the jump is
    /// invisible. It is retained here all the same: without it the region would have no
    /// offset of its own, and the frame after would find nothing to keep it there.
    pub fn sync_visible(&mut self, regions: &[Scrollable]) {
        for area in regions {
            let Some(keep) = area.keep_visible else {
                continue;
            };
            let previous = self.kept_visible.insert(area.id, keep.key);
            if previous == Some(keep.key) {
                continue;
            }
            let (offset, _) = area.keep_offset(keep);
            match previous {
                // Never seen: this is where the region opens.
                None => {
                    self.scroll.insert(area.id, offset);
                    self.scroll_target.remove(&area.id);
                    self.scroll_velocity.remove(&area.id);
                }
                // A change: glide, and let go of anything already moving the offset,
                // since the selection has just overruled it.
                Some(_) => {
                    self.scroll_ballistic.remove(&area.id);
                    self.scroll_target.insert(area.id, offset);
                }
            }
        }
    }

    /// The paged views whose page **on screen** has just changed, and the page each
    /// now shows.
    ///
    /// Reported as soon as the rounding tips — mid-drag, not once the spring has
    /// settled — because that is when a reader would say they are on the next page.
    /// A caller turns each into a message; the runtime holds no messages of its own.
    ///
    /// A view **appearing** is not a page change: the first sighting of a region is
    /// recorded silently, whatever page it opens on. An application that opens on
    /// page 3 already knows it is on page 3, and being told so on the first frame
    /// would only invite it to answer.
    pub fn page_changes(&mut self, regions: &[Scrollable]) -> Vec<(WidgetId, usize)> {
        let mut changed = Vec::new();
        for area in regions {
            let Some(snap) = area.page else { continue };
            let offset = self.scroll.get(&area.id).copied().unwrap_or_else(|| {
                let start = snap.offset_of(snap.requested);
                if snap.horizontal {
                    (start, 0.0)
                } else {
                    (0.0, start)
                }
            });
            let page = snap.page_at(if snap.horizontal { offset.0 } else { offset.1 });
            match self.page_shown.insert(area.id, page) {
                Some(previous) if previous != page => changed.push((area.id, page)),
                _ => {}
            }
        }
        changed
    }

    /// Is region `id` **still**? Nothing is driving it: no finger, no fling, and no
    /// spring left to run.
    ///
    /// The three writers of an offset, asked in one place. A caller that asked only two
    /// of them would call a region at rest in the middle of a fling.
    pub fn scroll_at_rest(&self, id: WidgetId) -> bool {
        self.scroll_held != Some(id)
            && !self.scroll_ballistic.contains_key(&id)
            && !self.scroll_target.contains_key(&id)
    }

    /// The scroll regions whose offset is worth reporting this frame, and where each is
    /// — the regions' half of [`crate::Widget::on_scroll`]. `grain` answers
    /// [`crate::Widget::scroll_grain`] for a region; a caller turns each result into a
    /// message, the runtime holding none of its own.
    ///
    /// Three rules, and each of them is a decision:
    ///
    /// - **A region appearing is not a movement** — as long as it appears where an
    ///   application would assume it is. A region seen for the first time at rest at
    ///   `(0, 0)` is recorded silently: telling an application on the first frame of
    ///   every screen that its list is at the top would only invite it to answer, and it
    ///   already knew. A region that appears **somewhere else** — restored, or opened at
    ///   a box it was asked to keep in view — is reported, because that one is news.
    /// - **A grain suppresses the reports in between, never the last one.** While the
    ///   region is moving, nothing under `grain` pixels from the last report is sent; the
    ///   moment it comes to rest, a position different from the last reported one is sent
    ///   whatever the distance. A coarse grain therefore costs frames, never accuracy.
    /// - **The baseline is what was reported**, not the previous frame. Measuring
    ///   against the previous frame would let a slow drag creep the whole way down a
    ///   list, a fraction of a grain at a time, without ever saying so.
    pub fn scroll_changes(
        &mut self,
        regions: &[Scrollable],
        grain: impl Fn(WidgetId) -> f32,
    ) -> Vec<(WidgetId, crate::scrollposition::ScrollPosition)> {
        let mut changed = Vec::new();
        for area in regions {
            let offset = self.scroll.get(&area.id).copied().unwrap_or((0.0, 0.0));
            let previous = self.scroll_reported.get(&area.id).copied();
            let report = match previous {
                None => offset != (0.0, 0.0),
                Some(previous) if previous == offset => false,
                Some(previous) => {
                    let grain = grain(area.id).max(0.0);
                    let moved = (offset.0 - previous.0)
                        .abs()
                        .max((offset.1 - previous.1).abs());
                    moved >= grain || self.scroll_at_rest(area.id)
                }
            };
            if report {
                changed.push((
                    area.id,
                    crate::scrollposition::ScrollPosition::of(area, offset),
                ));
            }
            if report || previous.is_none() {
                self.scroll_reported.insert(area.id, offset);
            }
        }
        self.scroll_reported
            .retain(|id, _| regions.iter().any(|area| area.id == *id));
        changed
    }

    /// Moves region `area` where `request` asks. `true` when it was acted on.
    ///
    /// **A finger refuses it.** An offset has one owner at a time, and while a hand is
    /// on the content that owner is the hand — a list that jumped out from under a
    /// finger because a timer fired would be the framework overruling the reader.
    pub fn scroll_to(
        &mut self,
        area: &Scrollable,
        request: crate::scrollposition::ScrollTo,
    ) -> bool {
        if request.is_empty() || self.scroll_held == Some(area.id) {
            return false;
        }
        let current = self.scroll.get(&area.id).copied().unwrap_or((0.0, 0.0));
        let to = request.resolve(area, current);
        // A fling in flight has just been overruled, animated or not: the request is
        // the more recent statement of where this list should be.
        self.scroll_ballistic.remove(&area.id);
        if request.animate {
            self.scroll_target.insert(area.id, to);
        } else {
            self.scroll.insert(area.id, to);
            self.scroll_target.remove(&area.id);
            self.scroll_velocity.remove(&area.id);
        }
        true
    }

    /// A finger takes hold of `id`: from now until [`Runtime::release_scroll`],
    /// this region's offset moves only when the finger says so. Any fling in
    /// flight is caught, since the finger has just overruled it.
    /// Records a change to field `id`, which **was** `before`, `since` seconds after the
    /// last one — see [`crate::UndoHistory::record`] for what counts as one step.
    pub fn record_edit(
        &mut self,
        id: WidgetId,
        before: crate::undo::EditSnapshot,
        kind: crate::undo::EditKind,
        since: f32,
    ) {
        self.undo.entry(id).or_default().record(before, kind, since);
    }

    /// Ends the run in progress in field `id`: the caret moved, so what is typed next is a
    /// step of its own.
    pub fn close_edit_run(&mut self, id: WidgetId) {
        if let Some(history) = self.undo.get_mut(&id) {
            history.close_run();
        }
    }

    /// Steps field `id` back one change, `current` being where it is now. `None` when
    /// there is nothing to undo.
    pub fn undo_edit(
        &mut self,
        id: WidgetId,
        current: crate::undo::EditSnapshot,
    ) -> Option<crate::undo::EditSnapshot> {
        self.undo.get_mut(&id)?.undo(current)
    }

    /// Steps field `id` forward again. `None` when nothing was undone.
    pub fn redo_edit(
        &mut self,
        id: WidgetId,
        current: crate::undo::EditSnapshot,
    ) -> Option<crate::undo::EditSnapshot> {
        self.undo.get_mut(&id)?.redo(current)
    }

    pub fn hold_scroll(&mut self, id: WidgetId) {
        self.scroll_held = Some(id);
    }

    /// The finger lets go: the offset is up for grabs again — a fling, a spring
    /// back from an overscroll, or nothing.
    pub fn release_scroll(&mut self, id: WidgetId) {
        if self.scroll_held == Some(id) {
            self.scroll_held = None;
        }
    }

    /// Feeds `overscroll` px of refused movement into the pull of the refresh area
    /// `id`, over a scrollable of `extent` px.
    ///
    /// This is the same measurement the glow is fed — what
    /// `apply_boundary_conditions` refused — routed to a different consumer. Where a
    /// refresh area is listening, the glow on that edge stands down: two answers to
    /// one gesture would say the same thing twice.
    pub fn refresh_pull(&mut self, id: WidgetId, overscroll: f32, extent: f32) {
        crate::refresh::pull_into(&mut self.refresh, id, overscroll, extent);
    }

    /// Moves the dismissible `id` by `delta` px along its axis, over an item of
    /// `extent` px.
    pub fn dismiss_drag(
        &mut self,
        id: WidgetId,
        delta: f32,
        extent: f32,
        axis: crate::dismiss::DismissAxis,
    ) {
        crate::dismiss::drag_into(&mut self.dismiss, id, delta, extent, axis);
    }

    /// The finger lets go of the dismissible `id`. Returns the direction it is being
    /// dismissed in, or `None` when it slides back.
    pub fn dismiss_release(
        &mut self,
        id: WidgetId,
        velocity: f32,
        cross: f32,
        axis: crate::dismiss::DismissAxis,
        threshold: f32,
    ) -> Option<crate::dismiss::DismissDirection> {
        crate::dismiss::release_of(&mut self.dismiss, id, velocity, cross, axis, threshold)
    }

    /// How much of a dismissed item's box is left, while its gap closes. `None` = not
    /// collapsing, so the item keeps the size its style asks for.
    pub fn dismiss_extent_factor(&self, id: WidgetId) -> Option<f32> {
        self.dismiss
            .get(&id)
            .filter(|s| s.phase() == crate::dismiss::DismissPhase::Collapse)
            .map(|s| s.extent_factor())
    }

    /// Advances every dismissible of the frame by `dt`. Returns `(still animating, the
    /// items whose gap has just closed)` — the second being what the shell turns into
    /// messages.
    pub fn advance_dismiss(
        &mut self,
        items: &[crate::dismiss::Dismissable],
        dt: f32,
    ) -> (bool, Vec<(WidgetId, crate::dismiss::DismissDirection)>) {
        crate::dismiss::advance_all(&mut self.dismiss, items, dt)
    }

    /// Calls off the pull of `id` without asking for anything — the list scrolled away
    /// from its top edge, or the gesture was cancelled.
    pub fn refresh_cancel(&mut self, id: WidgetId) {
        crate::refresh::cancel_of(&mut self.refresh, id);
    }

    /// Ends the pull of `id`. Returns `true` when it was armed, and so when the
    /// application should be asked to refresh.
    pub fn refresh_release(&mut self, id: WidgetId) -> bool {
        crate::refresh::release_of(&mut self.refresh, id)
    }

    /// Advances every refresh area of the frame by `dt`, reading each one's current
    /// `refreshing` flag from `areas`, and drops those that have gone quiet. Returns
    /// `true` while any is still moving.
    pub fn advance_refresh(&mut self, areas: &[crate::refresh::Refreshable], dt: f32) -> bool {
        crate::refresh::advance_all(&mut self.refresh, areas, dt)
    }

    /// Tells the glow on one edge of `id` that a finger is dragging past it.
    ///
    /// `overscroll` is the movement the physics **refused** — which is exactly the
    /// distance the user asked for and did not get, and so exactly what the glow is
    /// there to acknowledge.
    pub fn glow_pull(
        &mut self,
        id: WidgetId,
        edge: GlowEdge,
        overscroll: f32,
        extent: f32,
        cross_offset: f32,
        cross_extent: f32,
    ) {
        if overscroll.abs() < 1e-3 {
            return;
        }
        self.scroll_glow.entry(id).or_default().edge_mut(edge).pull(
            overscroll,
            extent,
            cross_offset,
            cross_extent,
        );
    }

    /// Tells the glow on one edge of `id` that a fling just landed on it.
    pub fn glow_absorb(&mut self, id: WidgetId, edge: GlowEdge, velocity: f32) {
        self.scroll_glow
            .entry(id)
            .or_default()
            .edge_mut(edge)
            .absorb_impact(velocity);
    }

    /// Tells every glow of `id` that the gesture is over, so a held pull can fade.
    pub fn glow_scroll_end(&mut self, id: WidgetId) {
        if let Some(glows) = self.scroll_glow.get_mut(&id) {
            glows.scroll_end();
        }
    }

    /// Advances every glow by `dt`, dropping those that have gone quiet. Returns
    /// `true` while any is still animating.
    pub fn advance_glow(&mut self, dt: f32) -> bool {
        if self.scroll_glow.is_empty() {
            return false;
        }
        let mut animating = false;
        self.scroll_glow.retain(|_, glows| {
            animating |= glows.advance(dt);
            !glows.is_idle()
        });
        animating
    }

    /// A finger landed on the inked surface `id`, at `origin` in the surface's own
    /// coordinates, on a box of `size`. Starts a splash.
    pub fn ink_press(&mut self, id: WidgetId, origin: Point, size: Size) {
        self.ink.entry(id).or_default().press(origin, size);
    }

    /// The tap on `id` completed: its splash finishes growing and begins to fade.
    pub fn ink_confirm(&mut self, id: WidgetId) {
        if let Some(ripples) = self.ink.get_mut(&id) {
            ripples.confirm();
        }
    }

    /// Advances every splash by `dt`, dropping those that have gone. Returns `true`
    /// while any is still moving.
    ///
    /// A splash still waiting for a finger that is no longer down is cancelled here
    /// rather than left hanging. A release that lands on the widget confirms it first
    /// (the shell calls [`Runtime::ink_confirm`] before the next frame); anything else
    /// — the finger slid off, the widget was unmounted mid-press, a gesture was taken
    /// over by a scroll — leaves nothing to confirm it, and without this sweep the ink
    /// would sit there for the life of the application.
    pub fn advance_ink(&mut self, dt: f32) -> bool {
        if self.ink.is_empty() {
            return false;
        }
        let pressed = self.input.pressed;
        let mut animating = false;
        self.ink.retain(|id, ripples| {
            if Some(*id) != pressed {
                ripples.cancel();
            }
            animating |= ripples.advance(dt);
            !ripples.is_idle()
        });
        animating
    }

    /// Cancels any fling on `id` — a new gesture, a wheel notch or a programmatic
    /// scroll takes precedence over momentum from the last one.
    pub fn stop_scroll_fling(&mut self, id: WidgetId) -> Option<ScrollBallistic> {
        self.scroll_ballistic.remove(&id)
    }
    /// Catches a fling under a new press: stops it, and returns the momentum the
    /// next release inherits from it, per axis, in px/s.
    ///
    /// It is what makes repeated swipes accelerate on the platforms that do that;
    /// where they do not, [`ScrollPhysics::carried_momentum`] returns zero and this
    /// is simply a stop.
    pub fn catch_scroll_fling(&mut self, id: WidgetId, physics: ScrollPhysics) -> (f32, f32) {
        let Some(fling) = self.scroll_ballistic.remove(&id) else {
            return (0.0, 0.0);
        };
        let carried = |sim: Option<Ballistic>| {
            sim.map(|s| physics.carried_momentum(s.dx(fling.elapsed)))
                .unwrap_or(0.0)
        };
        (carried(fling.x), carried(fling.y))
    }

    /// Advances the pan *fling* of the interactive viewports: each velocity moves the
    /// translation (decelerated by exponential friction) and is then **clamped** to its
    /// viewport — hitting an edge cancels the velocity on that axis. `viewports` supplies
    /// the viewport of each interactive viewer (from the current frame). Returns `true`
    /// if a fling is still running.
    pub fn advance_interactive(&mut self, viewports: &[(WidgetId, Rect)], dt: f32) -> bool {
        use crate::interactive::{PAN_FRICTION, PAN_MIN_VELOCITY};
        if self.interactive_velocity.is_empty() {
            return false;
        }
        let ids: Vec<WidgetId> = self.interactive_velocity.keys().copied().collect();
        let decay = (-PAN_FRICTION * dt).exp();
        let mut animating = false;
        for id in ids {
            let mut v = self
                .interactive_velocity
                .get(&id)
                .copied()
                .unwrap_or((0.0, 0.0));
            let mut view = self.interactive.get(&id).copied().unwrap_or_default();
            let moved = view.pan(v.0 * dt, v.1 * dt);
            // Clamping: an axis that hits a bound cancels its velocity (no bounce).
            if let Some((_, vp)) = viewports.iter().find(|(i, _)| *i == id) {
                let clamped = moved.clamped(*vp);
                if (clamped.tx - moved.tx).abs() > 1e-3 {
                    v.0 = 0.0;
                }
                if (clamped.ty - moved.ty).abs() > 1e-3 {
                    v.1 = 0.0;
                }
                view = clamped;
            } else {
                view = moved;
            }
            self.interactive.insert(id, view);
            v.0 *= decay;
            v.1 *= decay;
            if v.0.hypot(v.1) < PAN_MIN_VELOCITY {
                self.interactive_velocity.remove(&id);
            } else {
                self.interactive_velocity.insert(id, v);
                animating = true;
            }
        }
        animating
    }

    /// Fades out the opacity of outgoing subtrees; forgets those that have reached
    /// 0. Returns `true` if an exit is still running.
    pub fn advance_leaving(&mut self, dt: f32) -> bool {
        let step = if ANIM_DURATION > 0.0 {
            dt / ANIM_DURATION
        } else {
            1.0
        };
        let mut animating = false;
        self.leaving.retain(|_, (_, opacity)| {
            *opacity -= step;
            if *opacity > 0.0 {
                animating = true;
                true
            } else {
                false
            }
        });
        animating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vertical scroll region `max` px long, in a 400 px viewport, with no
    /// physics of its own.
    fn region(id: WidgetId, max: f32) -> Scrollable {
        Scrollable {
            id,
            viewport: Rect::new(0.0, 0.0, 300.0, 400.0),
            max_x: 0.0,
            max_y: max,
            physics: None,
            refresh: None,
            page: None,
            reverse_x: false,
            reverse_y: false,
            host: None,
            keep_visible: None,
        }
    }

    /// A three-page horizontal view, 300 px wide, asking for `requested`.
    fn paged_region(id: WidgetId, requested: usize) -> Scrollable {
        Scrollable {
            id,
            viewport: Rect::new(0.0, 0.0, 300.0, 400.0),
            max_x: 600.0,
            max_y: 0.0,
            physics: None,
            refresh: None,
            page: Some(crate::PageSnap {
                snapping: true,
                extent: 300.0,
                count: 3,
                requested,
                horizontal: true,
            }),
            reverse_x: false,
            reverse_y: false,
            host: None,
            keep_visible: None,
        }
    }

    #[test]
    fn a_paged_release_springs_to_a_page_and_never_coasts() {
        let id = WidgetId::ROOT.child(0);
        let mut rt = Runtime::default();
        let area = paged_region(id, 0);
        rt.scroll.insert(id, (40.0, 0.0));
        // A flick far too slow to fling an ordinary list still turns the page.
        assert!(rt.fling_scroll(area, ScrollPhysics::Clamping, (60.0, 0.0)));
        for _ in 0..200 {
            rt.advance_scroll(&[area], ScrollPhysics::Clamping, 1.0 / 60.0);
        }
        let (x, _) = rt.scroll[&id];
        assert!((x - 300.0).abs() < 1.0, "settled at {x}");
    }

    #[test]
    fn a_hard_fling_on_a_paged_view_crosses_one_page() {
        let id = WidgetId::ROOT.child(0);
        let mut rt = Runtime::default();
        let area = paged_region(id, 0);
        rt.scroll.insert(id, (10.0, 0.0));
        rt.fling_scroll(area, ScrollPhysics::Clamping, (7000.0, 0.0));
        for _ in 0..300 {
            rt.advance_scroll(&[area], ScrollPhysics::Clamping, 1.0 / 60.0);
        }
        let (x, _) = rt.scroll[&id];
        assert!((x - 300.0).abs() < 1.0, "a fling must not skip pages: {x}");
    }

    #[test]
    fn the_page_asked_for_is_taken_on_the_first_sighting_and_on_a_change() {
        let id = WidgetId::ROOT.child(0);
        let mut rt = Runtime::default();
        // First sighting: the view opens there, with no animation to watch.
        rt.sync_pages(&[paged_region(id, 2)]);
        assert_eq!(rt.scroll[&id], (600.0, 0.0));
        assert!(!rt.scroll_target.contains_key(&id));

        // Asked again for the same page while the finger has moved it: left alone,
        // or the view could not be swiped at all.
        rt.scroll.insert(id, (450.0, 0.0));
        rt.sync_pages(&[paged_region(id, 2)]);
        assert_eq!(rt.scroll[&id], (450.0, 0.0));

        // A new page: a glide, not a jump.
        rt.sync_pages(&[paged_region(id, 0)]);
        assert_eq!(rt.scroll[&id], (450.0, 0.0));
        assert_eq!(rt.scroll_target[&id], (0.0, 0.0));
    }

    #[test]
    fn a_page_change_is_reported_once_and_never_on_arrival() {
        let id = WidgetId::ROOT.child(0);
        let mut rt = Runtime::default();
        let area = paged_region(id, 1);
        // The view appearing is not a page change, whatever page it opens on.
        assert!(rt.page_changes(&[area]).is_empty());

        // Half a page across: the rounding has not tipped yet.
        rt.scroll.insert(id, (440.0, 0.0));
        assert!(rt.page_changes(&[area]).is_empty());
        // Past the middle: reported, and only once.
        rt.scroll.insert(id, (460.0, 0.0));
        assert_eq!(rt.page_changes(&[area]), vec![(id, 2)]);
        rt.scroll.insert(id, (600.0, 0.0));
        assert!(rt.page_changes(&[area]).is_empty());
    }

    #[test]
    fn hover_rises_then_falls_and_clears() {
        let id = WidgetId::ROOT.child(0);
        let mut rt = Runtime::default();
        rt.input.hovered = Some(id);

        // Hovered: small steps → the progress rises without reaching 1.
        assert!(rt.advance(0.03)); // ~0.25, still running
        assert!(rt.advance(0.03)); // ~0.5, still running
        let p = rt.hover_progress(id);
        assert!(p > 0.4 && p < 0.6, "progress = {p}");

        // A large step: reaches 1.0 and stays there (no more animation).
        rt.advance(1.0);
        assert_eq!(rt.hover_progress(id), 1.0);
        assert!(!rt.advance(0.03));

        // End of hover: falls back (still running), then reaches 0 and the entry disappears.
        rt.input.hovered = None;
        assert!(rt.advance(0.03));
        rt.advance(1.0);
        assert_eq!(rt.hover_progress(id), 0.0);
        assert!(rt.anims.is_empty());
    }

    /// **A press rises and falls** (milestone 441).
    ///
    /// It was a flag and nothing else, so a state layer reached full the instant a finger
    /// landed and vanished the instant it left. The reference fades its press highlight
    /// in and out instead (`ink_highlight.dart:62`), and this is that fade in the shape
    /// hover and focus already had here.
    #[test]
    fn a_press_rises_then_falls_back() {
        let id = WidgetId::ROOT.child(3);
        let mut rt = Runtime::default();
        rt.input.hovered = Some(id);
        rt.input.pressed = Some(id);

        // Under the finger: part way there, still running.
        assert!(rt.advance(0.05));
        let p = rt.press_progress(id);
        assert!(p > 0.0 && p < 1.0, "on the way in: {p}");
        rt.advance(1.0);
        assert_eq!(rt.press_progress(id), 1.0);

        // Let go: it comes back down rather than disappearing, which is the same jump
        // the other way round.
        rt.input.pressed = None;
        assert!(rt.advance(0.05));
        let p = rt.press_progress(id);
        assert!(p > 0.0 && p < 1.0, "on the way back: {p}");
        rt.advance(1.0);
        assert_eq!(rt.press_progress(id), 0.0);
    }

    /// And it is the **slower** of the two clocks: the reference gives the press 200 ms
    /// against the 50 it gives hover and focus (`ink_well.dart:995`). A press is the one
    /// interaction the pointer commits to, and it is drawn slowly enough to be seen.
    #[test]
    fn a_press_arrives_more_slowly_than_a_hover() {
        let id = WidgetId::ROOT.child(4);
        let mut rt = Runtime::default();
        rt.input.hovered = Some(id);
        rt.input.pressed = Some(id);
        rt.advance(0.06);
        let (press, hover) = (rt.press_progress(id), rt.hover_progress(id));
        assert!(press < hover, "press {press} should trail hover {hover}");
        assert!(press > 0.0, "but it is under way");
    }

    /// A press counts only while the pointer is **still on the widget** — the rule
    /// `InputState::status_for` applies, so the progression and the flag can never
    /// disagree about what is happening.
    #[test]
    fn a_press_dragged_off_the_widget_comes_back_down() {
        let id = WidgetId::ROOT.child(5);
        let elsewhere = WidgetId::ROOT.child(6);
        let mut rt = Runtime::default();
        rt.input.hovered = Some(id);
        rt.input.pressed = Some(id);
        rt.advance(1.0);
        assert_eq!(rt.press_progress(id), 1.0);

        // Still held, but the finger has slid away: not a press any more. Long enough
        // for the hover to have finished falling and the press not to have — which is
        // the case the forget-predicate has to survive, since an entry with nothing left
        // *but* a press is exactly what a release looks like on the way out.
        rt.input.hovered = Some(elsewhere);
        assert!(rt.advance(0.13));
        assert_eq!(rt.hover_progress(id), 0.0, "the hover is done");
        let p = rt.press_progress(id);
        assert!(
            p > 0.0 && p < 1.0,
            "and the press is still coming down: {p}"
        );

        rt.advance(1.0);
        assert_eq!(rt.press_progress(id), 0.0);
    }

    #[test]
    fn focus_animates_independently() {
        let id = WidgetId::ROOT.child(1);
        let mut rt = Runtime::default();
        rt.input.focused = Some(id);
        rt.advance(1.0);
        assert_eq!(rt.focus_progress(id), 1.0);
        assert_eq!(rt.hover_progress(id), 0.0);
    }

    #[test]
    fn opacity_rises_to_one() {
        let id = WidgetId::ROOT.child(2);
        let mut rt = Runtime::default();
        // Mount: starts transparent.
        rt.anims.insert(
            id,
            Anim {
                opacity: 0.0,
                ..Default::default()
            },
        );
        assert!(rt.advance(0.03));
        let o = rt.opacity(id);
        assert!(o > 0.0 && o < 1.0, "opacity = {o}");
        rt.advance(1.0);
        assert_eq!(rt.opacity(id), 1.0);
        // Default without an entry: opaque.
        assert_eq!(rt.opacity(WidgetId::ROOT), 1.0);
    }

    /// **Motion off means the change lands at once, not that it does not land.**
    ///
    /// The user asked for animations to be reduced or removed, so an implicit animation
    /// completes in the frame it starts rather than over its duration. Skipping the
    /// change instead would leave the interface *wrong* rather than *still*, which is
    /// the failure this setting is most often given.
    ///
    /// The check is both halves: the value has arrived, **and** nothing is still asking
    /// for another frame.
    #[test]
    fn motion_off_lands_the_change_at_once() {
        // A switch mounts at its target, so the animation is what happens on the
        // **change** — which is exactly what this setting is about.
        let off: crate::Switch<()> = crate::Switch::new(false);
        let on: crate::Switch<()> = crate::Switch::new(true);

        let mut moving = Runtime::default();
        assert!(!moving.advance_values(&off, 1.0));
        assert!(
            moving.advance_values(&on, 0.03),
            "a live animation owes the next frame to itself"
        );
        let part_way = moving.value(WidgetId::ROOT);
        assert!(part_way > 0.0 && part_way < 1.0, "part way: {part_way}");

        let mut stopped = Runtime {
            still: true,
            ..Default::default()
        };
        assert!(!stopped.advance_values(&off, 1.0));
        let more = stopped.advance_values(&on, 0.03);
        assert!(!more, "motion off: nothing is left to animate");
        assert_eq!(
            stopped.value(WidgetId::ROOT),
            1.0,
            "and the value the widget asked for has actually arrived"
        );
    }

    #[test]
    fn value_snaps_on_mount_then_animates() {
        let mut rt = Runtime::default();
        // Mounting a switch that is off: adopts the target (0) with no animation.
        let off: crate::Switch<()> = crate::Switch::new(false);
        assert!(!rt.advance_values(&off, 1.0));
        assert_eq!(rt.value(WidgetId::ROOT), 0.0);

        // Toggled on: the value rises towards 1 in small steps.
        let on: crate::Switch<()> = crate::Switch::new(true);
        assert!(rt.advance_values(&on, 0.03));
        let v = rt.value(WidgetId::ROOT);
        assert!(v > 0.0 && v < 1.0, "value = {v}");
        rt.advance_values(&on, 1.0);
        assert_eq!(rt.value(WidgetId::ROOT), 1.0);

        // Widget gone: the value is forgotten.
        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_values(&empty, 1.0);
        assert!(rt.values.is_empty());
    }

    /// Minimal widget exposing a tunable animated value (target, duration, curve) —
    /// to test the timeline without depending on a concrete widget.
    struct Mock {
        target: f32,
        duration: f32,
        curve: Curve,
    }

    impl crate::widget::Widget<()> for Mock {
        fn style(&self) -> frus_layout::Style {
            frus_layout::Style::default()
        }
        fn children(&self) -> &[Box<dyn crate::widget::Widget<()>>] {
            &[]
        }
        fn paint(
            &self,
            _bounds: frus_core::Rect,
            _status: crate::interaction::Status,
            _theme: &crate::theme::Theme,
            _scene: &mut frus_core::Scene,
        ) {
        }
        fn on_click(&self) -> Option<()> {
            None
        }
        fn anim_target(&self) -> Option<f32> {
            Some(self.target)
        }
        fn anim_duration(&self) -> f32 {
            self.duration
        }
        fn anim_curve(&self) -> Curve {
            self.curve.clone()
        }
    }

    /// The **curve** shapes the trajectory: at t=0.25 an *ease-in* lags behind the
    /// linear progress and an *ease-out* runs ahead; all of them converge on the
    /// target.
    #[test]
    fn curve_shapes_the_value_timeline() {
        let id = WidgetId::ROOT;
        let dt = 0.03; // t = 0.25 over a duration of 0.12
        let dur = 0.12;
        let sample = |curve: Curve| {
            let mut rt = Runtime::default();
            rt.set_value(id, 0.0);
            rt.advance_values(
                &Mock {
                    target: 1.0,
                    duration: dur,
                    curve,
                },
                dt,
            );
            (rt.value(id), rt)
        };
        let (ein, mut rt_in) = sample(Curve::ease_in());
        let (eout, mut rt_out) = sample(Curve::ease_out());
        let (lin, mut rt_lin) = sample(Curve::Linear);

        assert!((lin - 0.25).abs() < 1e-3, "linear = t: {lin}");
        assert!(ein < 0.25, "ease-in lags: {ein}");
        assert!(eout > 0.25, "ease-out leads: {eout}");

        // A large step: all of them reach the target (the curves end at 1).
        for rt in [&mut rt_in, &mut rt_out, &mut rt_lin] {
            rt.advance_values(
                &Mock {
                    target: 1.0,
                    duration: dur,
                    curve: Curve::Linear,
                },
                1.0,
            );
        }
        assert_eq!(rt_in.value(id), 1.0);
        assert_eq!(rt_out.value(id), 1.0);
        assert_eq!(rt_lin.value(id), 1.0);
    }

    /// The animated colour **snaps** on mount then **tweens** on a change of target,
    /// channel by channel; a widget that has gone is forgotten.
    #[test]
    fn animated_color_tweens_between_frames() {
        let id = WidgetId::ROOT;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let mut rt = Runtime::default();

        // Mounting on red: adopts the target with no transition.
        let start: crate::Container<()> =
            crate::Container::new().animated_color(red, 0.10, Curve::Linear);
        assert!(!rt.advance_colors(&start, 1.0));
        assert_eq!(rt.anim_color(id), Some(red));

        // Blue target: linear tween, halfway ≈ (0.5, 0, 0.5).
        let to_blue: crate::Container<()> =
            crate::Container::new().animated_color(blue, 0.10, Curve::Linear);
        assert!(rt.advance_colors(&to_blue, 0.05));
        let mid = rt.anim_color(id).unwrap();
        assert!(
            (mid.r - 0.5).abs() < 0.05 && (mid.b - 0.5).abs() < 0.05,
            "mi-parcours = {mid:?}"
        );

        // The end: blue reached.
        rt.advance_colors(&to_blue, 1.0);
        assert_eq!(rt.anim_color(id), Some(blue));

        // Widget gone: the colour is forgotten.
        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_colors(&empty, 1.0);
        assert_eq!(rt.anim_color(id), None);
    }

    /// The animated size **snaps** on mount then **tweens** on a change of target
    /// (width/height); a widget that has gone is forgotten.
    #[test]
    fn animated_size_tweens_between_frames() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();

        let small: crate::Container<()> =
            crate::Container::new().animated_size(20.0, 20.0, 0.10, Curve::Linear);
        assert!(!rt.advance_sizes(&small, 1.0));
        assert_eq!(rt.anim_size(id), Some(Size::new(20.0, 20.0)));

        // 40×40 target: halfway through a linear tween ≈ 30×30.
        let big: crate::Container<()> =
            crate::Container::new().animated_size(40.0, 40.0, 0.10, Curve::Linear);
        assert!(rt.advance_sizes(&big, 0.05));
        let mid = rt.anim_size(id).unwrap();
        assert!(
            (mid.width - 30.0).abs() < 0.5 && (mid.height - 30.0).abs() < 0.5,
            "mi-parcours = {mid:?}"
        );

        rt.advance_sizes(&big, 1.0);
        assert_eq!(rt.anim_size(id), Some(Size::new(40.0, 40.0)));

        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_sizes(&empty, 1.0);
        assert_eq!(rt.anim_size(id), None);
    }

    /// The animated corner radius **snaps** on mount then **tweens** on a change of
    /// target (per corner); a widget that has gone is forgotten.
    #[test]
    fn animated_radius_tweens_between_frames() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();

        let sharp: crate::Container<()> =
            crate::Container::new().animated_radius(0.0, 0.10, Curve::Linear);
        assert!(!rt.advance_radii(&sharp, 1.0));
        assert_eq!(rt.anim_radius(id), Some(BorderRadius::from(0.0)));

        // Target 20: halfway through a linear tween ≈ 10.
        let round: crate::Container<()> =
            crate::Container::new().animated_radius(20.0, 0.10, Curve::Linear);
        assert!(rt.advance_radii(&round, 0.05));
        let mid = rt.anim_radius(id).unwrap();
        assert!(
            (mid.top_left - 10.0).abs() < 0.5,
            "mi-parcours = {}",
            mid.top_left
        );

        rt.advance_radii(&round, 1.0);
        assert_eq!(rt.anim_radius(id), Some(BorderRadius::from(20.0)));

        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_radii(&empty, 1.0);
        assert_eq!(rt.anim_radius(id), None);
    }

    /// The animated padding **snaps** on mount then **tweens** on a change of target
    /// (per side); a widget that has gone is forgotten.
    #[test]
    fn animated_padding_tweens_between_frames() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();

        let p0: crate::Container<()> =
            crate::Container::new().animated_padding(0.0, 0.10, Curve::Linear);
        assert!(!rt.advance_paddings(&p0, 1.0));
        assert_eq!(rt.anim_padding(id), Some(Insets::uniform(0.0)));

        let p20: crate::Container<()> =
            crate::Container::new().animated_padding(20.0, 0.10, Curve::Linear);
        assert!(rt.advance_paddings(&p20, 0.05)); // t = 0.5 → 10
        let mid = rt.anim_padding(id).unwrap();
        assert!((mid.left - 10.0).abs() < 0.5, "mi-parcours = {}", mid.left);

        rt.advance_paddings(&p20, 1.0);
        assert_eq!(rt.anim_padding(id), Some(Insets::uniform(20.0)));

        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_paddings(&empty, 1.0);
        assert_eq!(rt.anim_padding(id), None);
    }

    /// The **duration** sets the speed: for the same `dt`, a shorter transition is
    /// further along.
    #[test]
    fn shorter_duration_animates_faster() {
        let id = WidgetId::ROOT;
        let advance = |duration: f32| {
            let mut rt = Runtime::default();
            rt.set_value(id, 0.0);
            rt.advance_values(
                &Mock {
                    target: 1.0,
                    duration,
                    curve: Curve::Linear,
                },
                0.025,
            );
            rt.value(id)
        };
        let fast = advance(0.05); // t = 0.5
        let slow = advance(0.20); // t = 0.125
        assert!(
            fast > slow,
            "shorter duration further along: {fast} vs {slow}"
        );
        assert!((fast - 0.5).abs() < 1e-3, "fast = {fast}");
        assert!((slow - 0.125).abs() < 1e-3, "slow = {slow}");
    }

    #[test]
    fn spring_ease_is_monotonic_no_overshoot() {
        assert!((spring_ease(0.0) - 0.0).abs() < 1e-6);
        assert!((spring_ease(1.0) - 1.0).abs() < 1e-6);
        // Increasing and bounded to [0,1] (no overshoot beyond 1).
        let mut prev = 0.0;
        for i in 0..=100 {
            let v = spring_ease(i as f32 / 100.0);
            assert!(v >= prev - 1e-6, "decreases at {i}");
            assert!(v <= 1.0 + 1e-6, "exceeds 1 at {i}");
            prev = v;
        }
        // Already well advanced at halfway (a gentle arrival at the end).
        assert!(spring_ease(0.5) > 0.7);
        // Clamped outside the domain.
        assert_eq!(spring_ease(-1.0), 0.0);
        assert_eq!(spring_ease(2.0), 1.0);
    }

    #[test]
    fn scroll_springs_to_target_and_settles() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        rt.scroll_target.insert(id, (0.0, 100.0));
        rt.scroll_velocity.insert(id, (0.0, 0.0));
        let regions = [region(id, 200.0)];
        for _ in 0..600 {
            if !rt.advance_scroll(&regions, ScrollPhysics::Clamping, 0.016) {
                break;
            }
        }
        let (_, y) = rt.scroll.get(&id).copied().unwrap();
        assert!((y - 100.0).abs() < 1.0, "reached the target: {y}");
        assert!(
            !rt.scroll_target.contains_key(&id),
            "animation state cleared at rest"
        );
    }

    /// **A scrollbar arrives when its area moves, and leaves once it has been still**
    /// (`scrollbar.dart:17`, `:18`): 600 ms of stillness, then 300 ms of going.
    ///
    /// Nothing announces that movement. The offset is written by the drag handler, by
    /// the fling and by the spring, and none of the three leaves word — so this watches
    /// the value itself, which catches all three and whatever is added next.
    #[test]
    fn a_bar_arrives_on_movement_and_goes_again() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        let regions = [region(id, 200.0)];
        let run = |rt: &mut Runtime, frames: usize| {
            for _ in 0..frames {
                rt.advance_scroll(&regions, ScrollPhysics::Clamping, 0.016);
            }
        };

        // An area nobody has scrolled has no bar at all.
        run(&mut rt, 10);
        assert_eq!(
            rt.scrollbar_fade_of(id).opacity,
            0.0,
            "untouched: no bar to see"
        );

        // Moved by something — anything — and it comes, over `BAR_FADE`.
        rt.scroll.insert(id, (0.0, 40.0));
        run(&mut rt, 1);
        let arriving = rt.scrollbar_fade_of(id).opacity;
        assert!(
            arriving > 0.0 && arriving < 1.0,
            "on its way in, not snapped: {arriving}"
        );
        run(&mut rt, 20);
        assert!(rt.scrollbar_fade_of(id).opacity > 0.99, "there");

        // Still for less than `BAR_TIME_TO_FADE` all told: it stays put.
        run(&mut rt, 10);
        assert!(
            rt.scrollbar_fade_of(id).opacity > 0.99,
            "half a second of stillness is not enough to send it away"
        );

        // And then it goes.
        run(&mut rt, 80);
        assert_eq!(rt.scrollbar_fade_of(id).opacity, 0.0, "gone");
    }

    /// **A held thumb keeps its bar**, however long the hand pauses on it
    /// (`scrollbar.dart:1970`), and so does a bar a pointer is resting near — which is
    /// what lets a faded bar be found again at all (`scrollbar.dart:2132`).
    #[test]
    fn a_bar_stays_for_the_pointer_that_wants_it() {
        let id = WidgetId::ROOT;
        let regions = [region(id, 200.0)];
        let run = |rt: &mut Runtime, frames: usize| {
            for _ in 0..frames {
                rt.advance_scroll(&regions, ScrollPhysics::Clamping, 0.016);
            }
        };

        let mut held = Runtime::default();
        held.scroll.insert(id, (0.0, 40.0));
        held.scrollbar_dragged = Some(id);
        run(&mut held, 140); // well past 600 ms of stillness
        assert!(
            held.scrollbar_fade_of(id).opacity > 0.99,
            "a thumb in a hand does not fade out from under it"
        );

        // A bar that has gone entirely comes back for a pointer that reaches for it,
        // and the thumb warms towards its hovered colour while it is there.
        let mut faded = Runtime::default();
        run(&mut faded, 10);
        assert_eq!(faded.scrollbar_fade_of(id).opacity, 0.0);
        faded.scrollbar_hovered = Some(id);
        run(&mut faded, 25);
        assert!(faded.scrollbar_fade_of(id).opacity > 0.99, "back");
        assert!(faded.scrollbar_fade_of(id).hover > 0.99, "and warmed");
    }

    #[test]
    fn interactive_fling_decelerates_settles_and_stays_bounded() {
        use crate::interactive::InteractiveView;
        let id = WidgetId::ROOT;
        let vp = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut rt = Runtime::default();
        // Content zoomed ×2, flung leftwards at high speed.
        rt.interactive.insert(
            id,
            InteractiveView {
                scale: 2.0,
                tx: 0.0,
                ty: 0.0,
            },
        );
        rt.interactive_velocity.insert(id, (-2000.0, 0.0));
        let viewports = [(id, vp)];
        let mut frames = 0;
        while rt.advance_interactive(&viewports, 0.016) {
            frames += 1;
            assert!(frames < 1000, "the fling must eventually stop");
        }
        let view = rt.interactive.get(&id).copied().unwrap();
        // Stays clamped (content ×2 covers the viewport → tx ∈ [-200, 0]).
        assert!(
            view.tx >= -200.0 - 1e-3 && view.tx <= 0.0 + 1e-3,
            "clamped: {}",
            view.tx
        );
        assert!(
            !rt.interactive_velocity.contains_key(&id),
            "velocity cleared at rest"
        );
    }

    #[test]
    fn scroll_overshoot_rubber_bands_back_to_max() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        // Target beyond the bound (overshoot) → must come back to max.
        rt.scroll_target.insert(id, (0.0, 240.0));
        rt.scroll_velocity.insert(id, (0.0, 0.0));
        let regions = [region(id, 200.0)];
        for _ in 0..1000 {
            if !rt.advance_scroll(&regions, ScrollPhysics::Clamping, 0.016) {
                break;
            }
        }
        let (_, y) = rt.scroll.get(&id).copied().unwrap();
        assert!((y - 200.0).abs() < 1.0, "back at the max bound: {y}");
    }

    /// Runs a fling to completion and answers where the content came to rest.
    fn settle(physics: ScrollPhysics, max: f32, from: f32, velocity: f32) -> f32 {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        let area = region(id, max);
        rt.scroll.insert(id, (0.0, from));
        rt.fling_scroll(area, physics, (0.0, velocity));
        for _ in 0..1200 {
            if !rt.advance_scroll(&[area], physics, 1.0 / 60.0) {
                break;
            }
        }
        assert!(
            rt.scroll_ballistic.is_empty(),
            "the fling should have finished"
        );
        rt.scroll.get(&id).copied().unwrap().1
    }

    #[test]
    fn a_clamping_fling_stops_at_the_edge() {
        // Far more momentum than there is content: it must stop exactly at the end,
        // with no overshoot to spring back from.
        let rest = settle(ScrollPhysics::Clamping, 400.0, 0.0, 6000.0);
        assert_eq!(rest, 400.0, "stopped dead at the end, at {rest}");
        // And symmetrically at the start.
        let rest = settle(ScrollPhysics::Clamping, 400.0, 400.0, -6000.0);
        assert_eq!(rest, 0.0, "stopped dead at the start, at {rest}");
    }

    #[test]
    fn a_clamping_fling_that_fits_lands_where_the_platform_says() {
        // 1000 px/s covers about 194 px under the platform's spline.
        let rest = settle(ScrollPhysics::Clamping, 4000.0, 0.0, 1000.0);
        assert!((rest - 194.0).abs() < 3.0, "landed at {rest}");
    }

    #[test]
    fn a_bouncing_fling_overshoots_then_settles_back_on_the_edge() {
        let id = WidgetId::ROOT;
        let physics = ScrollPhysics::BOUNCING;
        let mut rt = Runtime::default();
        let area = region(id, 400.0);
        rt.scroll.insert(id, (0.0, 300.0));
        rt.fling_scroll(area, physics, (0.0, 4000.0));
        let mut peak: f32 = 0.0;
        for _ in 0..1200 {
            let moving = rt.advance_scroll(&[area], physics, 1.0 / 60.0);
            peak = peak.max(rt.scroll.get(&id).copied().unwrap().1);
            if !moving {
                break;
            }
        }
        assert!(
            peak > 400.0,
            "it should have gone past the end, peak {peak}"
        );
        let rest = rt.scroll.get(&id).copied().unwrap().1;
        assert!(
            (rest - 400.0).abs() < 1.0,
            "came back to the end, at {rest}"
        );
    }

    #[test]
    fn an_overscrolled_offset_comes_home_even_without_a_fling() {
        // A release too slow to fling still owes the content its edge back.
        for physics in [ScrollPhysics::BOUNCING, ScrollPhysics::Clamping] {
            let rest = settle(physics, 400.0, 460.0, 5.0);
            assert!(
                (rest - 400.0).abs() < 1.0,
                "{physics:?} left the content at {rest}"
            );
        }
    }

    #[test]
    fn a_release_with_nothing_to_do_starts_no_fling() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        rt.scroll.insert(id, (0.0, 100.0));
        assert!(!rt.fling_scroll(region(id, 400.0), ScrollPhysics::Clamping, (0.0, 5.0)));
        assert!(rt.scroll_ballistic.is_empty());
    }

    #[test]
    fn a_held_region_is_not_dragged_back_by_the_spring() {
        // The bug the device found: while a finger holds an overscrolled offset,
        // the edge spring kept retracting it between two moves, so a rubber band
        // was pulled back as fast as it was stretched and never appeared.
        let id = WidgetId::ROOT;
        let physics = ScrollPhysics::BOUNCING;
        let area = region(id, 400.0);
        let pulled = -60.0;

        let mut held = Runtime::default();
        held.hold_scroll(id);
        held.scroll.insert(id, (0.0, pulled));
        held.scroll_target.insert(id, (0.0, pulled));
        for _ in 0..10 {
            held.advance_scroll(&[area], physics, 1.0 / 60.0);
        }
        assert_eq!(
            held.scroll.get(&id).copied().unwrap().1,
            pulled,
            "a held offset must not move on its own"
        );

        // The same offset, let go: now it must come home.
        let mut free = Runtime::default();
        free.scroll.insert(id, (0.0, pulled));
        free.scroll_target.insert(id, (0.0, pulled));
        while free.advance_scroll(&[area], physics, 1.0 / 60.0) {}
        assert!(
            free.scroll.get(&id).copied().unwrap().1.abs() < 1.0,
            "a released overscroll springs back"
        );
    }

    #[test]
    fn releasing_a_region_that_was_never_held_changes_nothing() {
        let mut rt = Runtime::default();
        rt.hold_scroll(WidgetId::ROOT);
        rt.release_scroll(WidgetId::ROOT.child(1));
        assert_eq!(
            rt.scroll_held,
            Some(WidgetId::ROOT),
            "another region's release must not steal the hold"
        );
        rt.release_scroll(WidgetId::ROOT);
        assert_eq!(rt.scroll_held, None);
    }

    #[test]
    fn a_clamping_fling_lights_the_edge_it_slams_into() {
        let id = WidgetId::ROOT;
        let physics = ScrollPhysics::Clamping;
        let mut rt = Runtime::default();
        let area = region(id, 400.0);
        // Far more momentum than there is content: it will reach the end.
        rt.fling_scroll(area, physics, (0.0, 6000.0));
        while rt.advance_scroll(&[area], physics, 1.0 / 60.0) {}
        let glows = rt
            .scroll_glow
            .get(&id)
            .expect("the edge should have lit up");
        assert!(!glows.edge(GlowEdge::Bottom).is_idle(), "the end glows");
        assert!(
            glows.edge(GlowEdge::Top).is_idle(),
            "and only the end — the fling never touched the start"
        );
        // It is a flash, not a permanent mark.
        while rt.advance_glow(1.0 / 60.0) {}
        assert!(rt.scroll_glow.is_empty(), "the glow is dropped once quiet");
    }

    #[test]
    fn a_bouncing_fling_lights_nothing() {
        // The bounce *is* the feedback: a glow on top of it would say the same
        // thing twice.
        let id = WidgetId::ROOT;
        let physics = ScrollPhysics::BOUNCING;
        let mut rt = Runtime::default();
        let area = region(id, 400.0);
        rt.fling_scroll(area, physics, (0.0, 6000.0));
        while rt.advance_scroll(&[area], physics, 1.0 / 60.0) {}
        assert!(rt.scroll_glow.is_empty());
    }

    #[test]
    fn a_pull_that_was_refused_lights_the_edge_and_fades() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        rt.glow_pull(id, GlowEdge::Top, -30.0, 600.0, 150.0, 300.0);
        assert!(!rt.scroll_glow.get(&id).unwrap().is_idle());
        rt.glow_scroll_end(id);
        let mut frames = 0;
        while rt.advance_glow(1.0 / 60.0) {
            frames += 1;
            assert!(frames < 200, "the glow never faded");
        }
        assert!(rt.scroll_glow.is_empty());
    }

    #[test]
    fn a_refusal_of_nothing_lights_nothing() {
        let mut rt = Runtime::default();
        rt.glow_pull(WidgetId::ROOT, GlowEdge::Top, 0.0, 600.0, 150.0, 300.0);
        assert!(
            rt.scroll_glow.is_empty(),
            "a zero pull is not an overscroll"
        );
        assert!(!rt.advance_glow(1.0 / 60.0));
    }

    #[test]
    fn a_fling_on_a_region_that_vanished_is_dropped() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        rt.fling_scroll(region(id, 400.0), ScrollPhysics::Clamping, (0.0, 2000.0));
        assert!(!rt.scroll_ballistic.is_empty());
        // The route changed: the scrollable is no longer part of the frame.
        assert!(!rt.advance_scroll(&[], ScrollPhysics::Clamping, 1.0 / 60.0));
        assert!(rt.scroll_ballistic.is_empty());
    }

    #[test]
    fn leaving_fades_out_and_clears() {
        let mut rt = Runtime::default();
        rt.leaving.insert(0, (Vec::new(), 1.0));
        assert!(rt.advance_leaving(0.06)); // ~0.5, still running
        assert!(!rt.advance_leaving(0.06)); // reaches 0 → cleared
        assert!(rt.leaving.is_empty());
    }
}

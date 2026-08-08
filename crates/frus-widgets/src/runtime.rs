//! Runtime state retained between frames, **keyed by widget identity**.
//!
//! A field's *value* stays controlled (application state); what lives here is the
//! widgets' own **interaction/edit** state: hover/focus, scroll offsets, and the
//! cursor/selection position of fields. This is the foundation of reconciliation
//! by identity (laid down at Milestone 6).

use std::cell::RefCell;
use std::collections::HashMap;

use frus_core::{BorderRadius, Color, Curve, Insets, Primitive, Rect, Size};

use crate::interaction::{InputState, WidgetId};
use crate::relayout::LayoutCache;

/// Scroll offsets `(x, y)`, per scrollable region.
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

/// Progressions d'animation d'un widget (`0.0..=1.0`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Anim {
    pub hover: f32,
    pub focus: f32,
    /// Opacity (1 at rest; started at 0 on mount for the fade-in).
    pub opacity: f32,
}

impl Default for Anim {
    fn default() -> Self {
        Self {
            hover: 0.0,
            focus: 0.0,
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

/// Linear interpolation of two radii (per corner).
fn lerp_radius(a: BorderRadius, b: BorderRadius, t: f32) -> BorderRadius {
    let mix = |x: f32, y: f32| x + (y - x) * t;
    BorderRadius {
        top_left: mix(a.top_left, b.top_left),
        top_right: mix(a.top_right, b.top_right),
        bottom_right: mix(a.bottom_right, b.bottom_right),
        bottom_left: mix(a.bottom_left, b.bottom_left),
    }
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
    /// Hover / press / focus.
    pub input: InputState,
    /// **Current** scroll offsets (the rendered ones), per region.
    pub scroll: ScrollState,
    /// **Target** scroll offsets (what the spring drives towards), per region.
    pub scroll_target: ScrollState,
    /// Scroll velocity (for the spring), per region.
    pub scroll_velocity: ScrollState,
    /// Retained transform (scale + translation) of each
    /// [`InteractiveViewer`](crate::InteractiveViewer), per viewport. Absent =
    /// identity (scale 1, no translation).
    pub interactive: HashMap<WidgetId, crate::interactive::InteractiveView>,
    /// Pan velocity (px/s) of a *fling* still running after release, per viewport —
    /// decelerated every frame by [`Runtime::advance_interactive`]. Absent = at rest.
    pub interactive_velocity: HashMap<WidgetId, (f32, f32)>,
    /// Edit state, per input field.
    pub edits: HashMap<WidgetId, Edit>,
    /// Animation progresses (hover/focus/opacity), per widget.
    pub anims: HashMap<WidgetId, Anim>,
    /// The widgets' own animated values (`Widget::anim_target`), per widget — each
    /// one a curved **timeline** (see [`ValueAnim`]).
    pub values: HashMap<WidgetId, ValueAnim>,
    /// Animated background colours (`Container::animated_color`), per widget.
    colors: HashMap<WidgetId, ColorAnim>,
    /// Animated sizes (`Container::animated_size`), per widget — injected at layout.
    sizes: HashMap<WidgetId, SizeAnim>,
    /// Animated corner radii (`Container::animated_radius`), per widget.
    radii: HashMap<WidgetId, RadiusAnim>,
    /// Animated paddings (`Container::animated_padding`), per widget — injected at layout.
    paddings: HashMap<WidgetId, PaddingAnim>,
    /// Widgets present at the previous frame (to detect mounts).
    pub mounted: std::collections::HashSet<WidgetId>,
    /// Snapshots of outgoing subtrees, fading out: event key → (captured
    /// primitives, remaining opacity `1 → 0`).
    pub leaving: HashMap<u64, (Vec<Primitive>, f32)>,
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
    /// A widget's animated hover progress.
    pub fn hover_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.hover).unwrap_or(0.0)
    }

    /// A widget's animated focus progress.
    pub fn focus_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.focus).unwrap_or(0.0)
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
            out: &mut Vec<(WidgetId, f32, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_target() {
                out.push((
                    id,
                    target,
                    widget.anim_duration().max(0.0),
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, f32, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

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
            out: &mut Vec<(WidgetId, Color, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_color() {
                out.push((
                    id,
                    target,
                    widget.anim_duration().max(0.0),
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, Color, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

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
            out: &mut Vec<(WidgetId, Size, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_size() {
                out.push((
                    id,
                    target,
                    widget.anim_duration().max(0.0),
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, Size, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

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
            out: &mut Vec<(WidgetId, BorderRadius, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_radius() {
                out.push((
                    id,
                    target,
                    widget.anim_duration().max(0.0),
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, BorderRadius, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

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

    /// A widget's animated padding, if in transition (`None` otherwise).
    pub fn anim_padding(&self, id: WidgetId) -> Option<Insets> {
        self.paddings.get(&id).map(|p| p.current)
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
            out: &mut Vec<(WidgetId, Insets, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_padding() {
                out.push((
                    id,
                    target,
                    widget.anim_duration().max(0.0),
                    widget.anim_curve(),
                ));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    out,
                );
            }
        }
        let mut targets: Vec<(WidgetId, Insets, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

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

    /// Advances the transitions (hover/focus) by `dt` seconds towards their targets.
    /// Returns `true` if at least one animation is still running.
    pub fn advance(&mut self, dt: f32) -> bool {
        let hovered = self.input.hovered;
        let focused = self.input.focused;
        if let Some(id) = hovered {
            self.anims.entry(id).or_default();
        }
        if let Some(id) = focused {
            self.anims.entry(id).or_default();
        }

        let step = if ANIM_DURATION > 0.0 {
            dt / ANIM_DURATION
        } else {
            1.0
        };
        let mut animating = false;

        self.anims.retain(|id, anim| {
            let hover_target = if Some(*id) == hovered { 1.0 } else { 0.0 };
            let focus_target = if Some(*id) == focused { 1.0 } else { 0.0 };
            approach(&mut anim.hover, hover_target, step, &mut animating);
            approach(&mut anim.focus, focus_target, step, &mut animating);
            // Opacity always tends towards 1 (the fade-in).
            approach(&mut anim.opacity, 1.0, step, &mut animating);
            // Entries that are entirely at rest are forgotten (nothing to animate).
            !(hover_target == 0.0
                && focus_target == 0.0
                && anim.hover <= 0.0
                && anim.focus <= 0.0
                && anim.opacity >= 1.0)
        });

        animating
    }

    /// Drives every **current** scroll offset towards its **target** through a
    /// spring (smooth scrolling), with an elastic pull at the edges (the bounce).
    /// `maxes` supplies `(max_x, max_y)` per region (from the last frame).
    /// Returns `true` if a scroll is still moving.
    pub fn advance_scroll(&mut self, maxes: &[(WidgetId, f32, f32)], dt: f32) -> bool {
        let ids: Vec<WidgetId> = self.scroll_target.keys().copied().collect();
        let mut animating = false;
        for id in ids {
            let (max_x, max_y) = maxes
                .iter()
                .find(|(i, _, _)| *i == id)
                .map(|(_, x, y)| (*x, *y))
                .unwrap_or((0.0, 0.0));
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
        animating
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

        // Fin : atteint le bleu.
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
        let maxes = [(id, 0.0, 200.0)];
        for _ in 0..600 {
            if !rt.advance_scroll(&maxes, 0.016) {
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
        let maxes = [(id, 0.0, 200.0)];
        for _ in 0..1000 {
            if !rt.advance_scroll(&maxes, 0.016) {
                break;
            }
        }
        let (_, y) = rt.scroll.get(&id).copied().unwrap();
        assert!((y - 200.0).abs() < 1.0, "back at the max bound: {y}");
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

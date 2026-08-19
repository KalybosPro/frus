//! The **ink ripple**: the touch feedback of a material surface.
//!
//! A circle grows from where the finger landed, drifts towards the middle of the box
//! and fades out. Everything about its motion is the reference's, transcribed rather
//! than approximated: the five durations, the starting radius of 30 %, the five extra
//! pixels the final radius overshoots by, the `ease` curve driving both radius and
//! drift, and the fade-out that only *begins* three fifths of the way through its own
//! timeline.
//!
//! Two pieces are needed to see one. A widget declares that it takes ink, through
//! [`Widget::ink`](crate::Widget::ink) — the shape to clip to and the colour to splash
//! in. The runtime keeps the ripples themselves ([`Runtime::ink`](crate::Runtime::ink)),
//! since a ripple outlives the frame it was born in: it is the shell that knows a
//! finger went down, and the paint walk that knows where the box is.
//!
//! [`InkWell`] is the ready-made wrapper — a transparent box that splashes and clicks,
//! like the reference's widget of the same name.

use frus_core::{
    BorderRadius, ClipShape, Color, Curve, LayerFilter, Path, Point, Primitive, Rect, Scene, Size,
};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// How long the ink takes to reach full opacity.
const FADE_IN: f32 = 0.075;
/// How long the radius takes to travel while the finger is **still down** — the slow
/// swell of a press being held.
const RADIUS_HELD: f32 = 1.0;
/// How long the radius has left once the tap is confirmed: the swell speeds up.
const RADIUS_CONFIRMED: f32 = 0.225;
/// How long the fade-out timeline runs for a confirmed tap.
const FADE_OUT: f32 = 0.375;
/// How long it runs for a cancelled one — a finger that slid off.
const CANCEL: f32 = 0.075;
/// The fade-out does nothing for the first 225 ms of its 375 ms: the ink holds its
/// colour while the circle finishes growing, and only then disappears.
const FADE_OUT_START: f32 = 225.0 / 375.0;
/// The radius starts at 30 % of its target (a diameter of 60 %).
const START_FRACTION: f32 = 0.30;
/// And ends 5 px past it, so the ink clears the corners of the box it is clipped to.
const OVERSHOOT: f32 = 5.0;

/// The default splash opacity — the state-layer weight the reference gives a pressed
/// surface. The colour itself comes from the theme, or from the caller.
const DEFAULT_ALPHA: f32 = 0.12;

/// The splash colour a surface gets when it has not been told otherwise: the theme's
/// `on_surface`, at the weight the reference gives a pressed state layer.
///
/// It is a **default**, not a rule — every widget that takes ink can hand back a colour
/// of its own, and every one that builds on [`InkWell`] can be given one by the caller.
pub fn default_splash(theme: &Theme) -> Color {
    theme
        .widgets
        .ink
        .color
        .unwrap_or_else(|| theme.scheme.on_surface.fade(DEFAULT_ALPHA))
}

/// What a widget declares when it takes ink: what colour to splash in, and the shape to
/// keep the splash inside.
///
/// A widget returns this from [`Widget::ink`](crate::Widget::ink) — with the theme in
/// hand, so a coloured surface can splash in its own `on` colour instead of the one a
/// plain surface would use. The walk then paints the runtime's ripples directly over
/// that widget's own paint and under its children, which is where a material surface
/// puts them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InkStyle {
    /// The splash colour, **alpha included**: how present the ink is, not just its hue.
    pub color: Color,
    /// The corner radii the ink is clipped to. Ink never escapes its box; this says how
    /// that box is shaped.
    pub radius: BorderRadius,
}

impl InkStyle {
    /// The ink a plain surface takes: the theme's splash, square corners.
    pub fn of(theme: &Theme) -> Self {
        Self {
            color: default_splash(theme),
            radius: BorderRadius::ZERO,
        }
    }

    /// Splashes in a colour of its own, **alpha included**.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Clips the ink to this rounding — whatever draws the surface underneath.
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = radius.into();
        self
    }
}

/// One splash: a single tap's worth of ink.
///
/// Its three timelines run independently, exactly as the reference's three animation
/// controllers do — which is why the alpha can still be rising while the radius is
/// already most of the way out.
#[derive(Clone, Copy, Debug)]
struct Ripple {
    /// Where the finger landed, in the box's own coordinates.
    origin: Point,
    /// The middle of the box — where the circle drifts to.
    centre: Point,
    /// The radius it grows towards.
    target: f32,
    /// Fade-in progress, `0..=1`.
    fade_in: f32,
    /// Is the fade-in still running? A cancel **stops** it where it stands, and the
    /// alpha switches over to the fade-out from that moment.
    fade_in_running: bool,
    /// Radius progress, `0..=1`, before the curve.
    radius_t: f32,
    /// How long the whole radius timeline lasts. Starts at [`RADIUS_HELD`] and drops
    /// to [`RADIUS_CONFIRMED`] when the tap is confirmed, which is how a release
    /// makes the ink hurry.
    radius_seconds: f32,
    /// Fade-out progress, `0..=1`.
    fade_out: f32,
    /// Its rate, per second. Zero while the finger is down: the ink waits.
    fade_out_rate: f32,
}

impl Ripple {
    fn new(origin: Point, size: Size) -> Self {
        Self {
            origin,
            centre: Point::new(size.width / 2.0, size.height / 2.0),
            target: target_radius(size),
            fade_in: 0.0,
            fade_in_running: true,
            radius_t: 0.0,
            radius_seconds: RADIUS_HELD,
            fade_out: 0.0,
            fade_out_rate: 0.0,
        }
    }

    /// Is the finger still down on this one?
    fn held(&self) -> bool {
        self.fade_out_rate == 0.0
    }

    /// The tap completed: the radius finishes over [`RADIUS_CONFIRMED`] and the ink
    /// begins its long fade.
    fn confirm(&mut self) {
        if !self.held() {
            return;
        }
        self.radius_seconds = RADIUS_CONFIRMED;
        self.fade_out_rate = 1.0 / FADE_OUT;
    }

    /// The tap did not complete — the finger slid off, or the widget went away. The
    /// fade-in stops where it is and the ink leaves quickly.
    ///
    /// The seeding of `fade_out` is the reference's, and it has a visible consequence
    /// worth knowing: a ripple cancelled *mid* fade-in briefly jumps to full opacity,
    /// because the fade-out timeline it is dropped into does nothing before
    /// [`FADE_OUT_START`]. Over 75 ms it reads as a flick rather than a flash.
    fn cancel(&mut self) {
        if !self.held() {
            return;
        }
        self.fade_in_running = false;
        self.fade_out = 1.0 - self.fade_in;
        let left = 1.0 - self.fade_out;
        self.fade_out_rate = if left > 0.0 { left / CANCEL } else { f32::MAX };
    }

    /// Advances every timeline by `dt`. Returns `true` while there is still something
    /// to show.
    fn advance(&mut self, dt: f32) -> bool {
        if self.fade_in_running && self.fade_in < 1.0 {
            self.fade_in = (self.fade_in + dt / FADE_IN).min(1.0);
        }
        if self.radius_t < 1.0 {
            self.radius_t = (self.radius_t + dt / self.radius_seconds).min(1.0);
        }
        if self.fade_out_rate > 0.0 && self.fade_out < 1.0 {
            self.fade_out = (self.fade_out + dt * self.fade_out_rate).min(1.0);
        }
        !self.finished()
    }

    /// A held ripple is never finished: it is waiting for a finger to lift.
    fn finished(&self) -> bool {
        !self.held() && self.fade_out >= 1.0
    }

    /// The opacity multiplier for this frame, `0..=1`.
    fn alpha(&self) -> f32 {
        if self.fade_in_running && self.fade_in < 1.0 {
            self.fade_in
        } else {
            1.0 - ((self.fade_out - FADE_OUT_START) / (1.0 - FADE_OUT_START)).clamp(0.0, 1.0)
        }
    }

    /// The circle to paint this frame, in the box's own coordinates.
    fn circle(&self) -> (Point, f32) {
        let t = Curve::ease().transform(self.radius_t);
        let radius = self.target * START_FRACTION
            + (self.target + OVERSHOOT - self.target * START_FRACTION) * t;
        let centre = Point::new(
            self.origin.x + (self.centre.x - self.origin.x) * t,
            self.origin.y + (self.centre.y - self.origin.y) * t,
        );
        (centre, radius)
    }
}

/// The radius a splash grows to: **half the box's diagonal**, so that a tap in any
/// corner still covers the opposite one.
///
/// The reference writes this as `max(d1, d2) / 2` over the box's two diagonals, which
/// are the same length for a rectangle; the maximum is left over from a shape that
/// could once be something else.
fn target_radius(size: Size) -> f32 {
    (size.width * size.width + size.height * size.height).sqrt() / 2.0
}

/// Every splash currently alive on one widget.
///
/// There can be several: a second tap does not cancel the first one's fade, it starts
/// its own circle beside it — which is what makes drumming on a surface look like
/// water rather than a switch.
#[derive(Clone, Debug, Default)]
pub struct Ripples {
    list: Vec<Ripple>,
}

/// Beyond this many live splashes on one widget, the oldest is dropped. A finger
/// cannot produce them this fast; a stuck event source can.
const MAX_RIPPLES: usize = 8;

impl Ripples {
    /// A finger landed at `origin`, in the coordinates of a box of `size`.
    pub fn press(&mut self, origin: Point, size: Size) {
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        if self.list.len() >= MAX_RIPPLES {
            self.list.remove(0);
        }
        self.list.push(Ripple::new(origin, size));
    }

    /// The tap completed. Only the splash still waiting for it is confirmed.
    pub fn confirm(&mut self) {
        if let Some(last) = self.list.iter_mut().rev().find(|r| r.held()) {
            last.confirm();
        }
    }

    /// The gesture ended without a tap: every waiting splash leaves.
    pub fn cancel(&mut self) {
        for ripple in self.list.iter_mut().filter(|r| r.held()) {
            ripple.cancel();
        }
    }

    /// Advances every splash, dropping the ones that have gone. Returns `true` while
    /// any is still moving.
    pub fn advance(&mut self, dt: f32) -> bool {
        let mut animating = false;
        self.list.retain_mut(|ripple| {
            animating |= ripple.advance(dt);
            !ripple.finished()
        });
        animating
    }

    /// Is there nothing left to draw?
    pub fn is_idle(&self) -> bool {
        self.list.is_empty()
    }

    /// Folds the splashes' motion into a hash — what the paint-boundary cache compares
    /// to notice that a cached surface no longer paints what it did last frame.
    pub fn hash_state<H: std::hash::Hasher>(&self, h: &mut H) {
        use std::hash::Hash;
        self.list.len().hash(h);
        for ripple in &self.list {
            ripple.radius_t.to_bits().hash(h);
            ripple.fade_in.to_bits().hash(h);
            ripple.fade_out.to_bits().hash(h);
        }
    }

    /// Paints the splashes into `bounds`, clipped to the box and its rounding.
    ///
    /// The circles go into a composited layer whose shape erases whatever runs past
    /// the corners — the same machinery [`crate::ClipRRect`] uses, and the reason ink
    /// hugs a rounded button instead of squaring it off.
    pub fn paint(
        &self,
        owner: u64,
        bounds: Rect,
        radius: BorderRadius,
        color: Color,
        scene: &mut Scene,
    ) {
        let start = scene.primitives().len();
        for ripple in &self.list {
            let alpha = ripple.alpha();
            if alpha <= 0.002 {
                continue;
            }
            let (centre, r) = ripple.circle();
            let circle = Rect::new(
                bounds.x + centre.x - r,
                bounds.y + centre.y - r,
                r * 2.0,
                r * 2.0,
            );
            scene.fill_path(&Path::oval(circle), color.fade(alpha));
        }
        let group = scene.split_off(start);
        if group.is_empty() {
            return;
        }
        scene.push_primitive(Primitive::Layer {
            primitives: group,
            opacity: 1.0,
            clip: bounds,
            clip_shape: if radius == BorderRadius::ZERO {
                ClipShape::Rect
            } else {
                ClipShape::RRect(radius)
            },
            transform: None,
            filter: LayerFilter::NONE,
            owner,
        });
    }
}

/// A box that **splashes when tapped** — the reference's `InkWell`.
///
/// A pass-through in layout, like [`crate::ClipRRect`]: it takes the size its parent
/// gives it and its child fills it. What it adds is the ink, and the tap that starts
/// it.
///
/// ```ignore
/// InkWell::new()
///     .radius(12.0)                          // the ink is clipped to this rounding
///     .on_click(Msg::Open(id))
///     .child(row)
///
/// InkWell::new().color(theme.scheme.primary.fade(0.16)).child(tile)   // its own colour
/// ```
///
/// The splash sits **over this widget and under its child**, so a row of text tapped
/// this way keeps its text on top of the ink, the way a material surface does.
pub struct InkWell<Msg> {
    /// `None` = the theme's splash, resolved at paint time so a theme swap is followed.
    color: Option<Color>,
    radius: BorderRadius,
    on_click: Option<Msg>,
    on_long_press: Option<Msg>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Default for InkWell<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> InkWell<Msg> {
    /// An ink well with the theme's splash colour and square corners.
    pub fn new() -> Self {
        Self {
            color: None,
            radius: BorderRadius::ZERO,
            on_click: None,
            on_long_press: None,
            children: Vec::new(),
        }
    }

    /// Overrides the splash colour, **alpha included** — the caller decides how
    /// present the ink is, not just what hue it takes.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The rounding the ink is clipped to (uniform via `f32`, per corner via
    /// [`BorderRadius`]). It should match whatever draws the surface underneath.
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = radius.into();
        self
    }

    /// The message a tap emits.
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }

    /// The message a long press emits.
    pub fn on_long_press(mut self, msg: Msg) -> Self {
        self.on_long_press = Some(msg);
        self
    }

    /// Sets the child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for InkWell<Msg> {
    fn style(&self) -> Style {
        // A pass-through: the box is sized by the context, like the child.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Nothing of its own: the ink is painted by the walk, from `ink()`.
    }

    fn ink(&self, theme: &Theme) -> Option<InkStyle> {
        Some(
            InkStyle::of(theme)
                .color(self.color.unwrap_or_else(|| default_splash(theme)))
                .radius(self.radius),
        )
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_click.clone()
    }

    fn on_long_press(&self) -> Option<Msg> {
        self.on_long_press.clone()
    }

    fn debug_name(&self) -> &'static str {
        "InkWell"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: Size = Size {
        width: 100.0,
        height: 60.0,
    };

    fn pressed(origin: Point) -> Ripples {
        let mut ripples = Ripples::default();
        ripples.press(origin, BOX);
        ripples
    }

    #[test]
    fn a_splash_starts_where_the_finger_landed_and_at_thirty_percent() {
        let ripples = pressed(Point::new(10.0, 10.0));
        let (centre, radius) = ripples.list[0].circle();
        assert!(
            (centre.x - 10.0).abs() < 0.01 && (centre.y - 10.0).abs() < 0.01,
            "it starts under the finger: {centre:?}"
        );
        let target = target_radius(BOX);
        assert!(
            (radius - target * 0.30).abs() < 1e-3,
            "the first frame is 30% of the target radius, got {radius} of {target}"
        );
    }

    #[test]
    fn the_target_radius_covers_the_far_corner() {
        // A tap in one corner has to reach the opposite one, which is the whole
        // diagonal away — hence half of it from the middle the splash drifts to.
        let target = target_radius(BOX);
        let half_diagonal = (100.0f32.powi(2) + 60.0f32.powi(2)).sqrt() / 2.0;
        assert!((target - half_diagonal).abs() < 1e-3);
    }

    #[test]
    fn a_held_splash_swells_slowly_and_never_leaves() {
        let mut ripples = pressed(Point::new(50.0, 30.0));
        // A quarter of a second in, a *confirmed* splash would be done growing. Held,
        // it is only a quarter of the way through its one-second timeline.
        for _ in 0..15 {
            ripples.advance(1.0 / 60.0);
        }
        let ripple = ripples.list[0];
        assert!(
            ripple.radius_t > 0.2 && ripple.radius_t < 0.3,
            "held, it is only a quarter of the way out: {}",
            ripple.radius_t
        );
        assert!(!ripple.finished(), "and it waits for the finger");
        assert_eq!(
            ripple.alpha(),
            1.0,
            "though it reached full opacity long ago"
        );
    }

    #[test]
    fn a_confirmed_splash_fades_out_and_is_dropped() {
        let mut ripples = pressed(Point::new(50.0, 30.0));
        ripples.advance(1.0 / 60.0);
        ripples.confirm();
        let mut frames = 0;
        while ripples.advance(1.0 / 60.0) && frames < 200 {
            frames += 1;
        }
        assert!(ripples.is_idle(), "the splash is gone");
        // 375 ms of fade-out, minus the frame already spent: around 22 frames at 60 Hz.
        assert!(
            (20..=24).contains(&frames),
            "it takes the reference's 375 ms, got {frames} frames"
        );
    }

    #[test]
    fn the_ink_holds_its_colour_before_it_starts_leaving() {
        let mut ripples = pressed(Point::new(50.0, 30.0));
        for _ in 0..6 {
            ripples.advance(1.0 / 60.0);
        }
        ripples.confirm();
        // 150 ms into the 375 ms fade-out — still inside the 225 ms the reference
        // spends holding, so the ink is at full strength while the circle grows.
        for _ in 0..9 {
            ripples.advance(1.0 / 60.0);
        }
        assert_eq!(ripples.list[0].alpha(), 1.0);
        // 300 ms in — past the hold, and 150 ms of the 150 ms that are left to fade have
        // run half way: the ink drops from full to nothing in the last two fifths.
        for _ in 0..9 {
            ripples.advance(1.0 / 60.0);
        }
        let alpha = ripples.list[0].alpha();
        assert!(
            (alpha - 0.5).abs() < 0.05,
            "past the hold it drops fast: {alpha}"
        );
    }

    #[test]
    fn a_cancelled_splash_leaves_five_times_faster_than_a_confirmed_one() {
        let mut held = pressed(Point::new(50.0, 30.0));
        for _ in 0..6 {
            held.advance(1.0 / 60.0); // past the fade-in
        }
        let mut cancelled = held.clone();
        held.confirm();
        cancelled.cancel();

        let count = |mut r: Ripples| {
            let mut frames = 0;
            while r.advance(1.0 / 60.0) && frames < 200 {
                frames += 1;
            }
            frames
        };
        let (confirmed_frames, cancelled_frames) = (count(held), count(cancelled));
        assert!(
            confirmed_frames > cancelled_frames * 3,
            "a cancel is 75 ms against 375: {cancelled_frames} frames against {confirmed_frames}"
        );
    }

    #[test]
    fn the_splash_drifts_to_the_middle_of_the_box() {
        let mut ripples = pressed(Point::new(5.0, 5.0));
        ripples.confirm();
        // 250 ms: the radius timeline (225 ms) is over, and the ink has not yet faded
        // out (375 ms) — the one moment where the splash is fully grown and still there.
        for _ in 0..15 {
            ripples.advance(1.0 / 60.0);
        }
        let (centre, _) = ripples.list[0].circle();
        assert!(
            (centre.x - 50.0).abs() < 0.5 && (centre.y - 30.0).abs() < 0.5,
            "it ends up centred: {centre:?}"
        );
    }

    #[test]
    fn two_taps_splash_twice() {
        let mut ripples = pressed(Point::new(10.0, 10.0));
        ripples.confirm();
        ripples.advance(1.0 / 60.0);
        ripples.press(Point::new(90.0, 50.0), BOX);
        assert_eq!(ripples.list.len(), 2, "the first fade is not interrupted");
        // The confirm belongs to the second tap: the first is already leaving.
        ripples.confirm();
        assert!(!ripples.list[1].held());
    }

    #[test]
    fn a_press_on_an_empty_box_splashes_nothing() {
        let mut ripples = Ripples::default();
        ripples.press(Point::new(0.0, 0.0), Size::new(0.0, 0.0));
        assert!(ripples.is_idle());
    }

    #[test]
    fn the_ink_is_clipped_to_the_rounding_it_was_given() {
        let mut ripples = pressed(Point::new(50.0, 30.0));
        ripples.advance(1.0 / 60.0);
        let mut scene = Scene::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 60.0);
        ripples.paint(
            7,
            bounds,
            BorderRadius::uniform(12.0),
            Color::WHITE,
            &mut scene,
        );
        assert_eq!(scene.primitives().len(), 1, "one composited layer");
        match &scene.primitives()[0] {
            Primitive::Layer {
                clip_shape,
                clip,
                owner,
                ..
            } => {
                assert_eq!(*clip_shape, ClipShape::RRect(BorderRadius::uniform(12.0)));
                assert_eq!(*clip, bounds, "ink never escapes its box");
                assert_eq!(*owner, 7);
            }
            other => panic!("expected a layer, got {other:?}"),
        }
    }
}

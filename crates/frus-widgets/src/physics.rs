//! SingleChildScrollView physics: **how a scrollable behaves at its edges and after the finger
//! lifts**.
//!
//! Two families, because the platforms disagree and users notice:
//!
//! - [`ScrollPhysics::BOUNCING`] — dragging past the end is allowed but grows
//!   progressively harder, and letting go springs the content back. A fling rolls
//!   on friction and hands over to that spring at the edge.
//! - [`ScrollPhysics::Clamping`] — the offset never leaves the content at all. A
//!   fling follows the platform's spline deceleration and stops dead at the edge.
//!
//! [`ScrollPhysics::platform_default`] picks one at compile time, so an app that
//! says nothing already feels native on each target. A [`crate::scroll::SingleChildScrollView`]
//! can override it, and so can the application (`Application::scroll_physics`),
//! for the cases where the content wants a particular feel.
//!
//! Everything here is a **pure function of the metrics**: no retained state, no
//! clock. The runtime samples the returned [`Ballistic`] frame by frame; the shell
//! calls [`ScrollPhysics::apply_user_offset`] while a finger is down. That split is
//! what keeps the policy unit-testable without a window.

use frus_core::{
    BouncingScrollSimulation, ClampingScrollSimulation, Simulation, SpringDescription,
    SpringSimulation, Tolerance,
};

/// The state of one scroll axis, as the physics sees it. All in logical pixels.
///
/// `pixels` is the current offset, `min`/`max` the offsets the content may rest
/// at (`min` is 0 in practice, `max` the overflow), and `viewport` the visible
/// extent along that axis — the yardstick the rubber band is measured against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollMetrics {
    pub pixels: f32,
    pub min: f32,
    pub max: f32,
    pub viewport: f32,
}

impl ScrollMetrics {
    /// Metrics for an axis scrolling from 0 to `max` in a `viewport`-long window.
    pub fn new(pixels: f32, max: f32, viewport: f32) -> Self {
        Self {
            pixels,
            min: 0.0,
            max: max.max(0.0),
            viewport: viewport.max(1.0),
        }
    }

    /// Is the offset outside the range the content may rest at?
    pub fn out_of_range(&self) -> bool {
        self.pixels < self.min || self.pixels > self.max
    }

    /// How far past an edge the offset has been dragged; 0 while inside.
    pub fn overscroll_past(&self) -> f32 {
        (self.min - self.pixels)
            .max(self.pixels - self.max)
            .max(0.0)
    }
}

/// The default minimum release speed, in px/s, below which nothing is flung.
pub const MIN_FLING_VELOCITY: f32 = 50.0;
/// The cap applied to a release speed, in px/s.
pub const MAX_FLING_VELOCITY: f32 = 8000.0;

/// **When a scroll area draws a scrollbar** (`app.dart:857`).
///
/// The reference asks this question per platform and answers it *no* on every touch
/// screen: a finger already knows where it is on the page, and a permanent bar over the
/// content is a desktop affordance for a pointer that cannot feel the edges. Along the
/// **horizontal** axis the answer is no everywhere, on every platform.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Scrollbars {
    /// Never — what a touch screen gets (`app.dart:870`).
    Never,
    /// One bar down the inner edge (`app.dart:865`), for the platforms whose own scroll
    /// views have one.
    Always,
}

impl Default for Scrollbars {
    fn default() -> Self {
        Self::platform_default()
    }
}

impl Scrollbars {
    /// What the running platform expects: nothing on a touch screen, a bar on a desktop.
    ///
    /// Resolved at **compile time** from the target, like
    /// [`ScrollPhysics::platform_default`] beside it — a build is for one platform, and a
    /// constant keeps the choice out of the frame loop.
    pub const fn platform_default() -> Self {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            Scrollbars::Never
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            Scrollbars::Always
        }
    }
}

/// **How fast a fling's momentum is taken away** — the bouncing family's second
/// deceleration profile.
///
/// It is chosen by **what is doing the scrolling**, not by how fast. A finger
/// throws a surface and lets go of it; a trackpad or a wheel is a hand resting on a
/// device that reports motion, and the two want different endings. So this is a
/// property of the physics a scrollable is built with — settled once, from the
/// platform — and not something recomputed per fling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ScrollDecelerationRate {
    /// A finger's. What a touch screen expects, and the default.
    #[default]
    Normal,
    /// A trackpad's or a wheel's — what desktop software expects from a pointing
    /// device more precise than a fingertip.
    ///
    /// Four things differ, not one: the fling carries a constant deceleration so
    /// that it actually stops rather than coasting to a halt at infinity, the
    /// rubber band is twice as stiff, a fling may be twice again as fast, and
    /// **easing back out of an overscroll is not resisted at all** — which is a
    /// behaviour rather than a number, and the one a reader notices.
    Fast,
}

/// The behaviour of a scrollable at its edges and after a fling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollPhysics {
    /// Overscroll is allowed, resisted, and springs back — at the given rate.
    Bouncing(ScrollDecelerationRate),
    /// Overscroll is refused; a fling stops at the edge.
    Clamping,
}

impl ScrollPhysics {
    /// Bouncing physics at a finger's deceleration — the common case, and what
    /// `ScrollPhysics::BOUNCING` meant before there were two.
    pub const BOUNCING: Self = Self::Bouncing(ScrollDecelerationRate::Normal);

    /// The deceleration profile in force, or `None` for a family that has none.
    pub fn deceleration_rate(self) -> Option<ScrollDecelerationRate> {
        match self {
            ScrollPhysics::Bouncing(rate) => Some(rate),
            ScrollPhysics::Clamping => None,
        }
    }
}

impl Default for ScrollPhysics {
    fn default() -> Self {
        Self::platform_default()
    }
}

impl ScrollPhysics {
    /// What the running platform expects. Bouncing where the system scroll views
    /// bounce, clamping everywhere else.
    ///
    /// This is resolved at **compile time** from the target, not at runtime: a
    /// build is for one platform, and a constant keeps the choice out of the frame
    /// loop.
    pub const fn platform_default() -> Self {
        // A hand-held screen is scrolled with a finger; a desktop one with a
        // trackpad or a wheel. That, and not the operating system's name, is what
        // the two profiles are about — which is why the two bouncing platforms
        // answer differently here.
        #[cfg(target_os = "ios")]
        {
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Normal)
        }
        #[cfg(target_os = "macos")]
        {
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast)
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            ScrollPhysics::Clamping
        }
    }

    /// The spring that returns overscrolled content to its edge.
    pub fn spring(self) -> SpringDescription {
        SpringDescription::with_damping_ratio(0.5, 100.0, 1.1)
    }

    /// The thresholds under which the motion counts as finished.
    pub fn tolerance(self) -> Tolerance {
        Tolerance::PIXELS
    }

    /// The release speed under which a drag does not fling at all.
    ///
    /// Bouncing physics decelerates far more gently, so it asks for a more
    /// deliberate gesture before it commits to a fling.
    pub fn min_fling_velocity(self) -> f32 {
        match self {
            ScrollPhysics::Bouncing(_) => MIN_FLING_VELOCITY * 2.0,
            ScrollPhysics::Clamping => MIN_FLING_VELOCITY,
        }
    }

    /// The cap on a release speed.
    ///
    /// A pointing device can be flicked far harder than a finger can — a wheel
    /// notch or a two-finger flick on a trackpad reports speeds no fingertip
    /// reaches — so the fast profile lifts the ceiling eightfold rather than
    /// clipping what the hardware actually said.
    pub fn max_fling_velocity(self) -> f32 {
        match self {
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast) => MAX_FLING_VELOCITY * 8.0,
            _ => MAX_FLING_VELOCITY,
        }
    }

    /// The distance a finger must cover before the content starts to move, in px.
    ///
    /// Bouncing physics asks for a little more, to swallow the small unintended
    /// motion of a finger lifting off.
    pub fn drag_start_distance_threshold(self) -> f32 {
        match self {
            ScrollPhysics::Bouncing(_) => 3.5,
            ScrollPhysics::Clamping => 0.0,
        }
    }

    /// The speed carried over when a fling is thrown on top of one already
    /// running — the repeated-swipe speed-up.
    ///
    /// Only the bouncing family builds momentum this way; clamping starts each
    /// fling from the gesture alone.
    pub fn carried_momentum(self, existing_velocity: f32) -> f32 {
        match self {
            ScrollPhysics::Bouncing(_) => {
                existing_velocity.signum()
                    * (0.000_816 * existing_velocity.abs().powf(1.967)).min(40_000.0)
            }
            ScrollPhysics::Clamping => 0.0,
        }
    }

    /// How much of a finger's `delta` actually reaches the offset.
    ///
    /// Inside the content it is all of it. Past an edge, the bouncing family
    /// applies a friction that falls off quadratically with how far out we already
    /// are: the further you pull, the less each pixel of finger buys — the rubber
    /// band. Easing back towards the content is measured at the position the
    /// gesture is heading *to*, so releasing tension is easier than adding it.
    pub fn apply_user_offset(self, metrics: ScrollMetrics, delta: f32) -> f32 {
        let Some(rate) = self.deceleration_rate() else {
            return delta;
        };
        if delta == 0.0 || !metrics.out_of_range() {
            return delta;
        }
        let past_start = (metrics.min - metrics.pixels).max(0.0);
        let past_end = (metrics.pixels - metrics.max).max(0.0);
        let past = past_start.max(past_end);
        // `delta` is a change **to the offset**: past the start (`pixels < min`) a
        // positive delta walks back towards the content, and past the end a
        // negative one does. Those are the directions that release the band.
        let easing = (past_start > 0.0 && delta > 0.0) || (past_end > 0.0 && delta < 0.0);
        // **Easing out of an overscroll is unresisted on the fast profile**, and
        // that is the difference a reader actually notices. A finger that pulled a
        // list past its end is still holding the band and lets it back gradually; a
        // trackpad has nothing to hold, so making the return sticky reads as the
        // content refusing to come back rather than as tension.
        if easing && rate == ScrollDecelerationRate::Fast {
            return delta;
        }
        let fraction = if easing {
            (past - delta.abs()) / metrics.viewport
        } else {
            past / metrics.viewport
        };
        delta.signum() * apply_friction(past, delta.abs(), friction_factor(rate, fraction))
    }

    /// The part of a proposed move that must be **refused**, so that the caller
    /// can apply `value - refused`.
    ///
    /// Clamping physics refuses everything that would leave the content; bouncing
    /// physics refuses nothing, and lets the spring deal with it.
    pub fn apply_boundary_conditions(self, metrics: ScrollMetrics, value: f32) -> f32 {
        if self != ScrollPhysics::Clamping {
            return 0.0;
        }
        let p = metrics.pixels;
        if value < p && p <= metrics.min {
            return value - p; // already under the start: refuse the whole move
        }
        if metrics.max <= p && p < value {
            return value - p; // already past the end
        }
        if value < metrics.min && metrics.min < p {
            return value - metrics.min; // this move hits the start
        }
        if p < metrics.max && metrics.max < value {
            return value - metrics.max; // this move hits the end
        }
        0.0
    }

    /// May the content be dragged past its edges at all?
    pub fn allows_overscroll(self) -> bool {
        matches!(self, ScrollPhysics::Bouncing(_))
    }

    /// The motion that follows the finger lifting off at `velocity` (px/s, positive
    /// meaning the offset grows). `None` when nothing should move.
    pub fn ballistic(self, metrics: ScrollMetrics, velocity: f32) -> Option<Ballistic> {
        let tolerance = self.tolerance();
        let velocity = velocity.clamp(-self.max_fling_velocity(), self.max_fling_velocity());
        match self {
            ScrollPhysics::Bouncing(rate) => {
                if velocity.abs() >= tolerance.velocity || metrics.out_of_range() {
                    Some(Ballistic::Bouncing(
                        BouncingScrollSimulation::with_constant_deceleration(
                            metrics.pixels,
                            velocity,
                            metrics.min,
                            metrics.max,
                            self.spring(),
                            tolerance,
                            match rate {
                                ScrollDecelerationRate::Normal => 0.0,
                                ScrollDecelerationRate::Fast => CONSTANT_DECELERATION_FAST,
                            },
                        ),
                    ))
                } else {
                    None
                }
            }
            ScrollPhysics::Clamping => {
                // Out of range at all — after a resize, say — is a return to the
                // edge, never a fling. The velocity may only help it home.
                if metrics.out_of_range() {
                    let end = if metrics.pixels > metrics.max {
                        metrics.max
                    } else {
                        metrics.min
                    };
                    return Some(Ballistic::Spring(SpringSimulation::new(
                        self.spring(),
                        metrics.pixels,
                        end,
                        velocity.min(0.0),
                        tolerance,
                    )));
                }
                if velocity.abs() < tolerance.velocity
                    || (velocity > 0.0 && metrics.pixels >= metrics.max)
                    || (velocity < 0.0 && metrics.pixels <= metrics.min)
                {
                    return None;
                }
                Some(Ballistic::Clamping(ClampingScrollSimulation::new(
                    metrics.pixels,
                    velocity,
                    tolerance,
                )))
            }
        }
    }

    /// The motion that follows the finger lifting off a **paged** area, where the
    /// content may not come to rest between two pages. `extent` is the distance
    /// from one page to the next.
    ///
    /// This replaces the fling rather than correcting it afterwards: a paged view
    /// never coasts. Whatever the release speed, the content springs to **one**
    /// page — the next one on a flick, the nearer one otherwise. A fling that
    /// crossed three pages and then had to be caught and pulled back would be a
    /// different, worse gesture.
    ///
    /// Past an edge and still heading out, there is no page to go to, so the
    /// ordinary physics takes over — which is what brings the overscroll home,
    /// with the platform's own bounce or clamp.
    pub fn page_ballistic(
        self,
        metrics: ScrollMetrics,
        velocity: f32,
        extent: f32,
    ) -> Option<Ballistic> {
        if (velocity <= 0.0 && metrics.pixels <= metrics.min)
            || (velocity >= 0.0 && metrics.pixels >= metrics.max)
        {
            return self.ballistic(metrics, velocity);
        }
        let tolerance = self.tolerance();
        let target = page_target(metrics, extent, velocity, tolerance.velocity);
        if (target - metrics.pixels).abs() < tolerance.distance {
            return None;
        }
        Some(Ballistic::Spring(SpringSimulation::new(
            self.spring(),
            metrics.pixels,
            target,
            velocity,
            tolerance,
        )))
    }
}

/// Which page an offset sits on, as a **fraction**: `1.5` is halfway between the
/// second and the third. `extent` is the distance from one page to the next.
pub fn page_of(pixels: f32, extent: f32) -> f32 {
    pixels / extent.max(1.0)
}

/// The offset the content must come to rest at after a release at `velocity`.
///
/// The rule is the one every paged view uses, and it is not "nearest page":
/// **any** release with speed above the tolerance counts as a flick and moves the
/// page half a unit *the way the finger went* before rounding. So a flick that
/// covered a tenth of the viewport still turns the page, while letting go slowly
/// falls back to whichever page is nearer. A gesture is an intention, not a
/// measurement.
pub fn page_target(metrics: ScrollMetrics, extent: f32, velocity: f32, tolerance: f32) -> f32 {
    let mut page = page_of(metrics.pixels, extent);
    if velocity < -tolerance {
        page -= 0.5;
    } else if velocity > tolerance {
        page += 0.5;
    }
    // A view whose content is not a whole number of pages — every one of them once
    // the pages are narrower than the viewport — has a last page that is not at a
    // page boundary. Resting past the end is not a page, it is an overscroll.
    (page.round() * extent.max(1.0)).clamp(metrics.min, metrics.max)
}

/// The constant deceleration the **fast** profile adds on top of the drag, in
/// px·s⁻² — the reference's own figure.
///
/// Drag alone is geometric: the velocity is multiplied down each second and reaches
/// nought only at infinity, which is exactly right for a surface that was thrown.
/// A trackpad gesture is not a throw, and a motion that never quite settles under
/// one reads as the content drifting. This term is what gives it an end.
pub const CONSTANT_DECELERATION_FAST: f32 = 1400.0;

/// The friction applied to the first pixels of overscroll, before the band
/// stiffens. It falls off quadratically with how far out the drag already is.
///
/// The **fast** profile halves the starting figure, which is to say it doubles the
/// stiffness: a trackpad reports a great deal of motion for a small gesture, and a
/// band as slack as a finger's would let a flick throw the content a long way out.
fn friction_factor(rate: ScrollDecelerationRate, overscroll_fraction: f32) -> f32 {
    let base = match rate {
        ScrollDecelerationRate::Normal => 0.52,
        ScrollDecelerationRate::Fast => 0.26,
    };
    (1.0 - overscroll_fraction).powi(2) * base
}

/// Applies `gamma` to the part of `delta` that is spent outside the content, and
/// none of it to the part that is spent inside — a drag that crosses the edge
/// mid-way is resisted only for the half that is out.
fn apply_friction(extent_outside: f32, mut delta: f32, gamma: f32) -> f32 {
    let mut total = 0.0;
    if extent_outside > 0.0 && gamma > 0.0 {
        let delta_to_limit = extent_outside / gamma;
        if delta < delta_to_limit {
            return delta * gamma;
        }
        total += extent_outside;
        delta -= delta_to_limit;
    }
    total + delta
}

/// The motion a scrollable follows once the finger has let go.
///
/// One enum rather than a boxed trait object: it stays `Copy`, so the runtime can
/// keep one per axis in a plain map and sample it without an allocation per frame.
#[derive(Clone, Copy, Debug)]
pub enum Ballistic {
    /// Friction then spring-back — the bouncing family.
    Bouncing(BouncingScrollSimulation),
    /// The platform spline deceleration — the clamping family.
    Clamping(ClampingScrollSimulation),
    /// A plain return to an edge, with no fling.
    Spring(SpringSimulation),
}

impl Simulation for Ballistic {
    fn x(&self, time: f32) -> f32 {
        match self {
            Ballistic::Bouncing(s) => s.x(time),
            Ballistic::Clamping(s) => s.x(time),
            Ballistic::Spring(s) => s.x(time),
        }
    }

    fn dx(&self, time: f32) -> f32 {
        match self {
            Ballistic::Bouncing(s) => s.dx(time),
            Ballistic::Clamping(s) => s.dx(time),
            Ballistic::Spring(s) => s.dx(time),
        }
    }

    fn is_done(&self, time: f32) -> bool {
        match self {
            Ballistic::Bouncing(s) => s.is_done(time),
            Ballistic::Clamping(s) => s.is_done(time),
            Ballistic::Spring(s) => s.is_done(time),
        }
    }

    fn tolerance(&self) -> Tolerance {
        match self {
            Ballistic::Bouncing(s) => s.tolerance(),
            Ballistic::Clamping(s) => s.tolerance(),
            Ballistic::Spring(s) => s.tolerance(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_the_content_every_pixel_of_finger_reaches_the_offset() {
        let metrics = ScrollMetrics::new(120.0, 400.0, 600.0);
        for physics in [ScrollPhysics::BOUNCING, ScrollPhysics::Clamping] {
            assert_eq!(physics.apply_user_offset(metrics, 25.0), 25.0);
            assert_eq!(physics.apply_user_offset(metrics, -25.0), -25.0);
        }
    }

    #[test]
    fn bouncing_resists_more_the_further_out_you_pull() {
        let viewport = 600.0;
        let near = ScrollMetrics::new(-10.0, 400.0, viewport);
        let far = ScrollMetrics::new(-200.0, 400.0, viewport);
        // Pulling further out (a negative delta past the start) is resisted, and
        // resisted harder the further out we already are.
        let a = ScrollPhysics::BOUNCING.apply_user_offset(near, -20.0);
        let b = ScrollPhysics::BOUNCING.apply_user_offset(far, -20.0);
        assert!(a < 0.0 && b < 0.0);
        assert!(a.abs() < 20.0, "the band already resists: {a}");
        assert!(b.abs() < a.abs(), "further out must be harder: {b} vs {a}");
        // Clamping physics has no band at all.
        assert_eq!(ScrollPhysics::Clamping.apply_user_offset(far, -20.0), -20.0);
    }

    #[test]
    fn bouncing_lets_you_ease_back_more_easily_than_you_pulled() {
        let metrics = ScrollMetrics::new(-100.0, 400.0, 600.0);
        let out = ScrollPhysics::BOUNCING.apply_user_offset(metrics, -20.0);
        let back = ScrollPhysics::BOUNCING.apply_user_offset(metrics, 20.0);
        assert!(
            back.abs() > out.abs(),
            "releasing the band ({back}) should be easier than loading it ({out})"
        );
    }

    #[test]
    fn clamping_refuses_exactly_what_would_leave_the_content() {
        let physics = ScrollPhysics::Clamping;
        let metrics = ScrollMetrics::new(390.0, 400.0, 600.0);
        // A move that stays inside is untouched.
        assert_eq!(physics.apply_boundary_conditions(metrics, 395.0), 0.0);
        // One that crosses the end is trimmed to the end.
        assert_eq!(physics.apply_boundary_conditions(metrics, 430.0), 30.0);
        // Already at the end, pushing further: the whole move is refused.
        let at_end = ScrollMetrics::new(400.0, 400.0, 600.0);
        assert_eq!(physics.apply_boundary_conditions(at_end, 420.0), 20.0);
        // The start behaves symmetrically.
        let near_start = ScrollMetrics::new(5.0, 400.0, 600.0);
        assert_eq!(physics.apply_boundary_conditions(near_start, -15.0), -15.0);
        // Bouncing physics refuses nothing.
        assert_eq!(
            ScrollPhysics::BOUNCING.apply_boundary_conditions(at_end, 420.0),
            0.0
        );
    }

    #[test]
    fn a_slow_release_flings_nothing() {
        let metrics = ScrollMetrics::new(100.0, 400.0, 600.0);
        assert!(ScrollPhysics::Clamping.ballistic(metrics, 1.0).is_none());
        assert!(ScrollPhysics::BOUNCING.ballistic(metrics, 1.0).is_none());
    }

    #[test]
    fn clamping_fling_stops_dead_inside_the_content() {
        let metrics = ScrollMetrics::new(100.0, 400.0, 600.0);
        let sim = ScrollPhysics::Clamping.ballistic(metrics, 800.0).unwrap();
        assert!(matches!(sim, Ballistic::Clamping(_)));
        // It travels, then finishes; the runtime clamps the tail into the content.
        assert!(sim.x(0.1) > 100.0);
        assert!(sim.is_done(2.0));
        assert!(sim.dx(2.0).abs() < 1.0, "no residual speed");
    }

    #[test]
    fn clamping_at_the_edge_pushing_outwards_does_nothing() {
        let at_end = ScrollMetrics::new(400.0, 400.0, 600.0);
        assert!(ScrollPhysics::Clamping.ballistic(at_end, 900.0).is_none());
        let at_start = ScrollMetrics::new(0.0, 400.0, 600.0);
        assert!(ScrollPhysics::Clamping
            .ballistic(at_start, -900.0)
            .is_none());
    }

    #[test]
    fn clamping_out_of_range_returns_to_the_edge() {
        // Overscroll can still happen without a drag — the content shrank under a
        // resting offset. Clamping physics springs it home rather than flinging.
        let past = ScrollMetrics::new(460.0, 400.0, 600.0);
        let sim = ScrollPhysics::Clamping.ballistic(past, 0.0).unwrap();
        assert!(matches!(sim, Ballistic::Spring(_)));
        assert!(
            (sim.x(2.0) - 400.0).abs() < 0.5,
            "settles at {}",
            sim.x(2.0)
        );
    }

    #[test]
    fn bouncing_overscroll_springs_back_even_with_no_velocity() {
        let past = ScrollMetrics::new(-40.0, 400.0, 600.0);
        let sim = ScrollPhysics::BOUNCING.ballistic(past, 0.0).unwrap();
        assert!((sim.x(2.0) - 0.0).abs() < 0.5, "settles at {}", sim.x(2.0));
        assert!(sim.is_done(2.0));
    }

    #[test]
    fn a_fling_never_exceeds_the_velocity_cap() {
        let metrics = ScrollMetrics::new(0.0, 100_000.0, 600.0);
        let sim = ScrollPhysics::Clamping
            .ballistic(metrics, 50_000.0)
            .unwrap();
        assert!(
            sim.dx(0.0) <= MAX_FLING_VELOCITY + 1.0,
            "clipped to the cap, got {}",
            sim.dx(0.0)
        );
    }

    #[test]
    fn bouncing_asks_for_a_more_deliberate_gesture() {
        assert!(
            ScrollPhysics::BOUNCING.min_fling_velocity()
                > ScrollPhysics::Clamping.min_fling_velocity()
        );
        // Repeated swipes build speed only where the platform does.
        assert!(ScrollPhysics::BOUNCING.carried_momentum(1000.0) > 0.0);
        assert_eq!(ScrollPhysics::Clamping.carried_momentum(1000.0), 0.0);
        // …and the carry is signed like the motion it continues, and capped.
        assert!(ScrollPhysics::BOUNCING.carried_momentum(-1000.0) < 0.0);
        assert!(ScrollPhysics::BOUNCING.carried_momentum(100_000.0) <= 40_000.0);
    }

    #[test]
    fn metrics_report_how_far_out_of_range_they_are() {
        let inside = ScrollMetrics::new(50.0, 400.0, 600.0);
        assert!(!inside.out_of_range());
        assert_eq!(inside.overscroll_past(), 0.0);
        let before = ScrollMetrics::new(-12.0, 400.0, 600.0);
        assert!(before.out_of_range());
        assert_eq!(before.overscroll_past(), 12.0);
        let after = ScrollMetrics::new(430.0, 400.0, 600.0);
        assert!(after.out_of_range());
        assert_eq!(after.overscroll_past(), 30.0);
    }

    /// A three-page view, 300 px per page: offsets 0, 300, 600.
    fn paged(pixels: f32) -> ScrollMetrics {
        ScrollMetrics::new(pixels, 600.0, 300.0)
    }

    #[test]
    fn letting_go_slowly_falls_back_to_the_nearer_page() {
        assert_eq!(page_target(paged(130.0), 300.0, 0.0, 2.0), 0.0);
        assert_eq!(page_target(paged(170.0), 300.0, 0.0, 2.0), 300.0);
    }

    #[test]
    fn a_flick_turns_the_page_however_short_it_was() {
        // A tenth of a page, thrown forward: the page turns.
        assert_eq!(page_target(paged(30.0), 300.0, 900.0, 2.0), 300.0);
        // And the same distance thrown back stays put rather than going forward.
        assert_eq!(page_target(paged(30.0), 300.0, -900.0, 2.0), 0.0);
        // Dragged most of the way and flicked back: the flick wins over the drag.
        assert_eq!(page_target(paged(260.0), 300.0, -900.0, 2.0), 0.0);
    }

    #[test]
    fn a_page_target_never_lands_outside_the_content() {
        // Pages narrower than the viewport: the last one is not on a boundary.
        let metrics = ScrollMetrics::new(430.0, 450.0, 300.0);
        assert_eq!(page_target(metrics, 240.0, 900.0, 2.0), 450.0);
        assert_eq!(
            page_target(ScrollMetrics::new(5.0, 600.0, 300.0), 300.0, -900.0, 2.0),
            0.0
        );
    }

    #[test]
    fn a_paged_release_always_settles_on_a_page() {
        for physics in [ScrollPhysics::BOUNCING, ScrollPhysics::Clamping] {
            // Released mid-page with no speed at all: it still has to go somewhere.
            let sim = physics.page_ballistic(paged(190.0), 0.0, 300.0).unwrap();
            assert!(
                (sim.x(3.0) - 300.0).abs() < 1.0,
                "settled at {}",
                sim.x(3.0)
            );
            // A hard fling crosses exactly one page, not three.
            let sim = physics.page_ballistic(paged(10.0), 6000.0, 300.0).unwrap();
            assert!(
                (sim.x(3.0) - 300.0).abs() < 1.0,
                "settled at {}",
                sim.x(3.0)
            );
        }
    }

    #[test]
    fn resting_on_a_page_starts_nothing() {
        assert!(ScrollPhysics::Clamping
            .page_ballistic(paged(300.0), 0.0, 300.0)
            .is_none());
    }

    #[test]
    fn past_an_edge_the_ordinary_physics_takes_the_content_home() {
        // No page out there: bouncing springs back, clamping has nothing to do.
        let out = ScrollMetrics::new(-40.0, 600.0, 300.0);
        let sim = ScrollPhysics::BOUNCING
            .page_ballistic(out, -50.0, 300.0)
            .unwrap();
        assert!((sim.x(3.0) - 0.0).abs() < 1.0, "settled at {}", sim.x(3.0));
        assert!(ScrollPhysics::Clamping
            .page_ballistic(ScrollMetrics::new(0.0, 600.0, 300.0), -900.0, 300.0)
            .is_none());
    }

    #[test]
    fn the_platform_default_is_the_one_this_build_targets() {
        // The two bouncing platforms have not bounced alike since milestone 499 — a finger
        // on the one, a trackpad on the other — and this said one `BOUNCING` for both. It
        // was left behind, and only a macOS runner could see it: every build here is for
        // Windows, Android or Linux, where the branch taken is the last one.
        let expected = if cfg!(target_os = "ios") {
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Normal)
        } else if cfg!(target_os = "macos") {
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast)
        } else {
            ScrollPhysics::Clamping
        };
        assert_eq!(ScrollPhysics::platform_default(), expected);
        assert_eq!(ScrollPhysics::default(), expected);
    }

    // --- The second deceleration profile (milestone 499) ---

    /// **The profile is a property of the device, not of the speed** — which is
    /// the decision this milestone turns on, and the opposite of what #55 asked for.
    ///
    /// A finger's fling coasts: drag multiplies the velocity down each second and
    /// only reaches nought at infinity, which is exactly what a thrown surface does.
    /// A trackpad's stops. If the two were chosen by velocity, a hard fling and a
    /// gentle one on the same device would end differently — so this asks the same
    /// physics at two speeds three orders of magnitude apart and requires the same
    /// kind of ending from both.
    #[test]
    fn the_profile_is_the_devices_and_never_the_speeds() {
        // Content long enough that no fling here reaches an edge: what is being
        // compared is the deceleration, and a spring taking over would be a
        // different motion answering a different question.
        let metrics = ScrollMetrics::new(200.0, 200_000.0, 600.0);
        // Five seconds: long past where the fast profile has come to rest, and
        // short of where `0.135^t` underflows a 32-bit float — which it does around
        // twenty, and which would make a coasting fling look stopped for a reason
        // that is about the arithmetic rather than about the physics.
        let still_moving = |physics: ScrollPhysics, v: f32| {
            let sim = physics.ballistic(metrics, v).expect("a fling");
            sim.dx(5.0).abs() > 0.0
        };
        // Both within the finger cap, so the two speeds differ by nothing but
        // themselves — the cap is one of the things the profiles disagree about.
        for velocity in [300.0, 7000.0] {
            assert!(
                still_moving(ScrollPhysics::BOUNCING, velocity),
                "a finger coasts, at {velocity} as at any other speed"
            );
            assert!(
                !still_moving(
                    ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast),
                    velocity
                ),
                "and a trackpad stops, at {velocity} as at any other speed"
            );
        }
    }

    /// And the stop is not merely eventual: the fast profile comes to rest **short
    /// of** where coasting would have carried it, which is the whole reason a
    /// desktop list does not drift on after the hand has left the trackpad.
    #[test]
    fn a_trackpad_fling_stops_short_of_where_a_finger_would_coast() {
        let metrics = ScrollMetrics::new(200.0, 40_000.0, 600.0);
        let travel = |physics: ScrollPhysics| {
            let sim = physics.ballistic(metrics, 3000.0).expect("a fling");
            sim.x(30.0) - 200.0
        };
        let finger = travel(ScrollPhysics::BOUNCING);
        let trackpad = travel(ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast));
        assert!(finger > 0.0 && trackpad > 0.0, "both travel forwards");
        assert!(
            trackpad < finger,
            "the trackpad stops short: {trackpad} vs {finger}"
        );
    }

    /// **The fast band is twice as stiff.** A trackpad reports a great deal of
    /// motion for a small gesture, and a band as slack as a finger's would let one
    /// flick throw the content a long way out of its content.
    #[test]
    fn the_fast_band_resists_twice_as_hard() {
        let metrics = ScrollMetrics::new(-10.0, 400.0, 600.0);
        let finger = ScrollPhysics::BOUNCING.apply_user_offset(metrics, -20.0);
        let trackpad =
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast).apply_user_offset(metrics, -20.0);
        assert!(finger < 0.0 && trackpad < 0.0, "both pull further out");
        assert!(
            (trackpad / finger - 0.5).abs() < 0.05,
            "half as much movement for the same gesture: {trackpad} vs {finger}"
        );
    }

    /// **Easing back out of an overscroll is free on a trackpad**, and this is the
    /// one difference that is a behaviour rather than a number.
    ///
    /// A finger that pulled a list past its end is still holding the band, and
    /// letting it back gradually is what tension feels like. A trackpad is holding
    /// nothing; the same stickiness reads as the content refusing to come back.
    #[test]
    fn easing_out_of_an_overscroll_is_unresisted_on_a_trackpad() {
        let metrics = ScrollMetrics::new(-100.0, 400.0, 600.0);
        // Past the start, a positive delta walks back towards the content.
        let finger = ScrollPhysics::BOUNCING.apply_user_offset(metrics, 20.0);
        let trackpad =
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast).apply_user_offset(metrics, 20.0);
        assert_eq!(trackpad, 20.0, "every pixel of the gesture comes back");
        assert!(
            finger < 20.0,
            "where a finger still meets some resistance: {finger}"
        );
        // And tensioning is still resisted on both — it is the direction that
        // differs, not the family.
        assert!(
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast)
                .apply_user_offset(metrics, -20.0)
                .abs()
                < 20.0,
            "pulling further out is still resisted"
        );
    }

    /// A pointing device reports speeds no fingertip reaches, so the fast profile
    /// lifts the ceiling rather than clipping what the hardware actually said.
    #[test]
    fn a_trackpad_may_be_flung_eight_times_as_hard() {
        assert_eq!(
            ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast).max_fling_velocity(),
            ScrollPhysics::BOUNCING.max_fling_velocity() * 8.0
        );
        assert_eq!(
            ScrollPhysics::Clamping.max_fling_velocity(),
            ScrollPhysics::BOUNCING.max_fling_velocity(),
            "and the clamping family is untouched"
        );
    }

    /// **The finger profile is exactly what it always was.** Everything above adds
    /// a second answer; none of it may change the first, which is what every
    /// hand-held build of this framework runs.
    #[test]
    fn the_finger_profile_is_unchanged() {
        assert_eq!(friction_factor(ScrollDecelerationRate::Normal, 0.0), 0.52);
        assert_eq!(
            ScrollPhysics::BOUNCING.max_fling_velocity(),
            MAX_FLING_VELOCITY
        );
        // Its fling carries no constant deceleration, so it coasts — the property
        // the closed-form friction has always had.
        let metrics = ScrollMetrics::new(200.0, 200_000.0, 600.0);
        let sim = ScrollPhysics::BOUNCING
            .ballistic(metrics, 3000.0)
            .expect("a fling");
        assert!(
            sim.dx(5.0).abs() > 0.0,
            "still creeping at five seconds, as before"
        );
    }
}

//! Scroll physics: **how a scrollable behaves at its edges and after the finger
//! lifts**.
//!
//! Two families, because the platforms disagree and users notice:
//!
//! - [`ScrollPhysics::Bouncing`] — dragging past the end is allowed but grows
//!   progressively harder, and letting go springs the content back. A fling rolls
//!   on friction and hands over to that spring at the edge.
//! - [`ScrollPhysics::Clamping`] — the offset never leaves the content at all. A
//!   fling follows the platform's spline deceleration and stops dead at the edge.
//!
//! [`ScrollPhysics::platform_default`] picks one at compile time, so an app that
//! says nothing already feels native on each target. A [`crate::scroll::Scroll`]
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
        (self.min - self.pixels).max(self.pixels - self.max).max(0.0)
    }
}

/// The default minimum release speed, in px/s, below which nothing is flung.
pub const MIN_FLING_VELOCITY: f32 = 50.0;
/// The cap applied to a release speed, in px/s.
pub const MAX_FLING_VELOCITY: f32 = 8000.0;

/// The behaviour of a scrollable at its edges and after a fling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollPhysics {
    /// Overscroll is allowed, resisted, and springs back.
    Bouncing,
    /// Overscroll is refused; a fling stops at the edge.
    Clamping,
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
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            ScrollPhysics::Bouncing
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
            ScrollPhysics::Bouncing => MIN_FLING_VELOCITY * 2.0,
            ScrollPhysics::Clamping => MIN_FLING_VELOCITY,
        }
    }

    /// The cap on a release speed.
    pub fn max_fling_velocity(self) -> f32 {
        MAX_FLING_VELOCITY
    }

    /// The distance a finger must cover before the content starts to move, in px.
    ///
    /// Bouncing physics asks for a little more, to swallow the small unintended
    /// motion of a finger lifting off.
    pub fn drag_start_distance_threshold(self) -> f32 {
        match self {
            ScrollPhysics::Bouncing => 3.5,
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
            ScrollPhysics::Bouncing => {
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
        if delta == 0.0 || self != ScrollPhysics::Bouncing || !metrics.out_of_range() {
            return delta;
        }
        let past_start = (metrics.min - metrics.pixels).max(0.0);
        let past_end = (metrics.pixels - metrics.max).max(0.0);
        let past = past_start.max(past_end);
        // `delta` is a change **to the offset**: past the start (`pixels < min`) a
        // positive delta walks back towards the content, and past the end a
        // negative one does. Those are the directions that release the band.
        let easing = (past_start > 0.0 && delta > 0.0) || (past_end > 0.0 && delta < 0.0);
        let fraction = if easing {
            (past - delta.abs()) / metrics.viewport
        } else {
            past / metrics.viewport
        };
        delta.signum() * apply_friction(past, delta.abs(), friction_factor(fraction))
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
        matches!(self, ScrollPhysics::Bouncing)
    }

    /// The motion that follows the finger lifting off at `velocity` (px/s, positive
    /// meaning the offset grows). `None` when nothing should move.
    pub fn ballistic(self, metrics: ScrollMetrics, velocity: f32) -> Option<Ballistic> {
        let tolerance = self.tolerance();
        let velocity = velocity.clamp(-self.max_fling_velocity(), self.max_fling_velocity());
        match self {
            ScrollPhysics::Bouncing => {
                if velocity.abs() >= tolerance.velocity || metrics.out_of_range() {
                    Some(Ballistic::Bouncing(BouncingScrollSimulation::new(
                        metrics.pixels,
                        velocity,
                        metrics.min,
                        metrics.max,
                        self.spring(),
                        tolerance,
                    )))
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
}

/// The friction applied to the first pixels of overscroll, before the band
/// stiffens. It falls off quadratically with how far out the drag already is.
fn friction_factor(overscroll_fraction: f32) -> f32 {
    (1.0 - overscroll_fraction).powi(2) * 0.52
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
        for physics in [ScrollPhysics::Bouncing, ScrollPhysics::Clamping] {
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
        let a = ScrollPhysics::Bouncing.apply_user_offset(near, -20.0);
        let b = ScrollPhysics::Bouncing.apply_user_offset(far, -20.0);
        assert!(a < 0.0 && b < 0.0);
        assert!(a.abs() < 20.0, "the band already resists: {a}");
        assert!(b.abs() < a.abs(), "further out must be harder: {b} vs {a}");
        // Clamping physics has no band at all.
        assert_eq!(ScrollPhysics::Clamping.apply_user_offset(far, -20.0), -20.0);
    }

    #[test]
    fn bouncing_lets_you_ease_back_more_easily_than_you_pulled() {
        let metrics = ScrollMetrics::new(-100.0, 400.0, 600.0);
        let out = ScrollPhysics::Bouncing.apply_user_offset(metrics, -20.0);
        let back = ScrollPhysics::Bouncing.apply_user_offset(metrics, 20.0);
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
            ScrollPhysics::Bouncing.apply_boundary_conditions(at_end, 420.0),
            0.0
        );
    }

    #[test]
    fn a_slow_release_flings_nothing() {
        let metrics = ScrollMetrics::new(100.0, 400.0, 600.0);
        assert!(ScrollPhysics::Clamping.ballistic(metrics, 1.0).is_none());
        assert!(ScrollPhysics::Bouncing.ballistic(metrics, 1.0).is_none());
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
        assert!(ScrollPhysics::Clamping.ballistic(at_start, -900.0).is_none());
    }

    #[test]
    fn clamping_out_of_range_returns_to_the_edge() {
        // Overscroll can still happen without a drag — the content shrank under a
        // resting offset. Clamping physics springs it home rather than flinging.
        let past = ScrollMetrics::new(460.0, 400.0, 600.0);
        let sim = ScrollPhysics::Clamping.ballistic(past, 0.0).unwrap();
        assert!(matches!(sim, Ballistic::Spring(_)));
        assert!((sim.x(2.0) - 400.0).abs() < 0.5, "settles at {}", sim.x(2.0));
    }

    #[test]
    fn bouncing_overscroll_springs_back_even_with_no_velocity() {
        let past = ScrollMetrics::new(-40.0, 400.0, 600.0);
        let sim = ScrollPhysics::Bouncing.ballistic(past, 0.0).unwrap();
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
            ScrollPhysics::Bouncing.min_fling_velocity()
                > ScrollPhysics::Clamping.min_fling_velocity()
        );
        // Repeated swipes build speed only where the platform does.
        assert!(ScrollPhysics::Bouncing.carried_momentum(1000.0) > 0.0);
        assert_eq!(ScrollPhysics::Clamping.carried_momentum(1000.0), 0.0);
        // …and the carry is signed like the motion it continues, and capped.
        assert!(ScrollPhysics::Bouncing.carried_momentum(-1000.0) < 0.0);
        assert!(ScrollPhysics::Bouncing.carried_momentum(100_000.0) <= 40_000.0);
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

    #[test]
    fn the_platform_default_is_the_one_this_build_targets() {
        let expected = if cfg!(any(target_os = "ios", target_os = "macos")) {
            ScrollPhysics::Bouncing
        } else {
            ScrollPhysics::Clamping
        };
        assert_eq!(ScrollPhysics::platform_default(), expected);
        assert_eq!(ScrollPhysics::default(), expected);
    }
}

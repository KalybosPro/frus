//! Physical simulations: **pure** `time → value` functions.
//!
//! A [`Simulation`] describes a motion through its position `x(t)`, its velocity
//! `dx(t)` and a stopping test `is_done(t)` — with no mutable state and no coupling
//! to rendering. It is the abstraction the rest of the animation layer shares: a
//! spring, a friction fling and scroll momentum all take the same path, with a
//! driver sampling `x(t)` every frame.
//!
//! The maths — a spring in its three regimes, friction in closed form — are
//! expressed as immutable values computed once at construction: no re-entrancy and
//! no upward borrowing, which is the shape Rust wants.

/// The thresholds below which a simulation counts as "at rest".
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    /// Negligible distance, in position units.
    pub distance: f32,
    /// Negligible velocity, in position units per second.
    pub velocity: f32,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            distance: 1e-3,
            velocity: 1e-3,
        }
    }
}

impl Tolerance {
    /// A tolerance calibrated in **pixels**, for UI motion: a displacement of less
    /// than half a pixel, at a velocity under 2 px/s, counts as stationary.
    pub const PIXELS: Tolerance = Tolerance {
        distance: 0.5,
        velocity: 2.0,
    };
}

/// A motion described as a pure function of time.
///
/// `time` is in **seconds** since the motion began.
pub trait Simulation {
    /// The position at instant `time`.
    fn x(&self, time: f32) -> f32;
    /// The velocity — the derivative of position — at instant `time`.
    fn dx(&self, time: f32) -> f32;
    /// `true` once the motion has finished, within the tolerances.
    fn is_done(&self, time: f32) -> bool;
    /// The stopping thresholds.
    fn tolerance(&self) -> Tolerance {
        Tolerance::default()
    }
}

/// The parameters of a damped spring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringDescription {
    /// Mass `m` (> 0).
    pub mass: f32,
    /// Stiffness `k` (> 0).
    pub stiffness: f32,
    /// Damping `c` (≥ 0).
    pub damping: f32,
}

impl SpringDescription {
    /// A spring given directly as `(mass, stiffness, damping)`.
    pub fn new(mass: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            mass,
            stiffness,
            damping,
        }
    }

    /// A spring given by its **damping ratio**: `1` is critical (the fastest
    /// settle with no overshoot), `< 1` oscillates, `> 1` is sluggish.
    /// `c = ratio · 2·√(m·k)`.
    pub fn with_damping_ratio(mass: f32, stiffness: f32, ratio: f32) -> Self {
        Self {
            mass,
            stiffness,
            damping: ratio * 2.0 * (mass * stiffness).sqrt(),
        }
    }
}

/// A spring's closed form, chosen from the discriminant `c² − 4mk`.
#[derive(Clone, Copy, Debug)]
enum Solution {
    /// Critically damped (`c² − 4mk == 0`): the fastest settle, with no
    /// oscillation.
    Critical { r: f32, c1: f32, c2: f32 },
    /// Overdamped (`c² − 4mk > 0`): two real exponentials, no oscillation.
    Overdamped { r1: f32, r2: f32, c1: f32, c2: f32 },
    /// Underdamped (`c² − 4mk < 0`): a damped oscillation.
    Underdamped { w: f32, r: f32, c1: f32, c2: f32 },
}

impl Solution {
    fn x(&self, t: f32) -> f32 {
        match *self {
            Solution::Critical { r, c1, c2 } => (c1 + c2 * t) * (r * t).exp(),
            Solution::Overdamped { r1, r2, c1, c2 } => c1 * (r1 * t).exp() + c2 * (r2 * t).exp(),
            Solution::Underdamped { w, r, c1, c2 } => {
                (r * t).exp() * (c1 * (w * t).cos() + c2 * (w * t).sin())
            }
        }
    }

    fn dx(&self, t: f32) -> f32 {
        match *self {
            Solution::Critical { r, c1, c2 } => {
                let power = (r * t).exp();
                r * (c1 + c2 * t) * power + c2 * power
            }
            Solution::Overdamped { r1, r2, c1, c2 } => {
                c1 * r1 * (r1 * t).exp() + c2 * r2 * (r2 * t).exp()
            }
            Solution::Underdamped { w, r, c1, c2 } => {
                let power = (r * t).exp();
                let cosine = (w * t).cos();
                let sine = (w * t).sin();
                power * (c2 * w * cosine - c1 * w * sine) + r * power * (c2 * sine + c1 * cosine)
            }
        }
    }
}

/// A spring pulling a value from `start` towards `end`, started with an initial
/// `velocity`. The reported position is `end + solution(t)`.
#[derive(Clone, Copy, Debug)]
pub struct SpringSimulation {
    end: f32,
    solution: Solution,
    tolerance: Tolerance,
}

impl SpringSimulation {
    /// Builds a spring reaching from `start` to `end`, at initial `velocity`
    /// (px/s), with the given tolerance.
    pub fn new(
        spring: SpringDescription,
        start: f32,
        end: f32,
        velocity: f32,
        tolerance: Tolerance,
    ) -> Self {
        let m = spring.mass;
        let k = spring.stiffness;
        let c = spring.damping;
        let x0 = start - end;
        let v0 = velocity;
        let cmk = c * c - 4.0 * m * k;

        // We treat the regime as critical as soon as the discriminant is negligible
        // next to `4mk`: floating-point rounding can leave `with_damping_ratio(1.0)`
        // with a tiny non-zero `cmk` that we do not want to see oscillate.
        let solution = if cmk.abs() <= 1e-4 * (4.0 * m * k).max(1.0) {
            let r = -c / (2.0 * m);
            Solution::Critical {
                r,
                c1: x0,
                c2: v0 - r * x0,
            }
        } else if cmk > 0.0 {
            let root = cmk.sqrt();
            let r1 = (-c - root) / (2.0 * m);
            let r2 = (-c + root) / (2.0 * m);
            let c2 = (v0 - r1 * x0) / (r2 - r1);
            Solution::Overdamped {
                r1,
                r2,
                c1: x0 - c2,
                c2,
            }
        } else {
            let w = (4.0 * m * k - c * c).sqrt() / (2.0 * m);
            let r = -c / (2.0 * m);
            Solution::Underdamped {
                w,
                r,
                c1: x0,
                c2: (v0 - r * x0) / w,
            }
        };

        Self {
            end,
            solution,
            tolerance,
        }
    }
}

impl Simulation for SpringSimulation {
    fn x(&self, time: f32) -> f32 {
        self.end + self.solution.x(time)
    }

    fn dx(&self, time: f32) -> f32 {
        self.solution.dx(time)
    }

    fn is_done(&self, time: f32) -> bool {
        self.solution.x(time).abs() < self.tolerance.distance
            && self.solution.dx(time).abs() < self.tolerance.velocity
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// Friction deceleration — scroll and fling momentum — in closed form.
///
/// `drag ∈ (0, 1)`: the closer to 1, the longer the motion keeps rolling.
#[derive(Clone, Copy, Debug)]
pub struct FrictionSimulation {
    drag: f32,
    drag_log: f32,
    x0: f32,
    v0: f32,
    tolerance: Tolerance,
}

impl FrictionSimulation {
    /// Friction from `position` at `velocity`, with the coefficient `drag`.
    pub fn new(drag: f32, position: f32, velocity: f32, tolerance: Tolerance) -> Self {
        Self {
            drag,
            drag_log: drag.ln(),
            x0: position,
            v0: velocity,
            tolerance,
        }
    }

    /// A friction calibrated to pass through `start` (at `start_velocity`) **and**
    /// stop at `end` (at `end_velocity`), deriving `drag` from that.
    /// `drag = e^((v₀ − v₁)/(x₀ − x₁))`.
    pub fn through(
        start: f32,
        end: f32,
        start_velocity: f32,
        end_velocity: f32,
        tolerance: Tolerance,
    ) -> Self {
        let drag = ((start_velocity - end_velocity) / (start - end)).exp();
        Self::new(drag, start, start_velocity, tolerance)
    }

    /// The final position — the limit as `t → ∞`.
    pub fn final_x(&self) -> f32 {
        self.x0 - self.v0 / self.drag_log
    }

    /// The instant at which the motion passes through `x`, or `∞` when it never
    /// does — because it starts the wrong way round, or because `x` lies beyond
    /// [`final_x`]. Inverting `x(t)` is what lets a fling hand over to a spring
    /// exactly at the edge of the content.
    pub fn time_at_x(&self, x: f32) -> f32 {
        if x == self.x0 {
            return 0.0;
        }
        let unreachable = if self.v0 > 0.0 {
            x < self.x0 || x > self.final_x()
        } else {
            x > self.x0 || x < self.final_x()
        };
        if self.v0 == 0.0 || unreachable {
            return f32::INFINITY;
        }
        (self.drag_log * (x - self.x0) / self.v0 + 1.0).ln() / self.drag_log
    }
}

impl Simulation for FrictionSimulation {
    fn x(&self, time: f32) -> f32 {
        self.x0 + (self.v0 / self.drag_log) * (self.drag.powf(time) - 1.0)
    }

    fn dx(&self, time: f32) -> f32 {
        self.v0 * self.drag.powf(time)
    }

    fn is_done(&self, time: f32) -> bool {
        self.dx(time).abs() < self.tolerance.velocity
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// A fling that decelerates like Android's `OverScroller`, then stops dead.
///
/// The curve is the one Android's `SplineOverScroller` uses for a fling: distance
/// and duration are derived from the release velocity, and the position follows
/// `1 − (1 − t)^r` over that duration. It **never leaves the content**: reaching
/// an edge is the caller's business (see the clamping scroll policy), which is
/// what makes it the right model for a platform with no bounce.
///
/// Adjusted, as Flutter's is, to be *ballistic* — deceleration depends only on the
/// current velocity, never on how long ago the motion started — so a fling can be
/// restarted mid-flight from its own state alone.
#[derive(Clone, Copy, Debug)]
pub struct ClampingScrollSimulation {
    position: f32,
    velocity: f32,
    /// Total run time, in seconds.
    duration: f32,
    /// Total **signed** distance travelled, in pixels.
    distance: f32,
    tolerance: Tolerance,
}

/// `ln(0.78) / ln(0.9)`, Android's `DECELERATION_RATE`.
const DECELERATION_RATE: f32 = 2.358_201_8;
/// Android's `INFLEXION`.
const INFLEXION: f32 = 0.35;
/// Android's `mPhysicalCoeff`: `g × 39.37 in/m × 160 px/in × 0.84` ("look and feel
/// tuning"), in px·s⁻².
const PHYSICAL_COEFF: f32 = 9.806_65 * 39.37 * 160.0 * 0.84;
/// Android's `mFlingFriction`, the value that makes the fling travel the platform
/// distance.
pub const CLAMPING_FRICTION: f32 = 0.015;

impl ClampingScrollSimulation {
    /// A fling from `position` at `velocity` (px/s), with Android's friction.
    pub fn new(position: f32, velocity: f32, tolerance: Tolerance) -> Self {
        Self::with_friction(position, velocity, CLAMPING_FRICTION, tolerance)
    }

    /// The same fling, with the friction coefficient made explicit: higher means
    /// it stops sooner and travels less far.
    pub fn with_friction(position: f32, velocity: f32, friction: f32, tolerance: Tolerance) -> Self {
        // Below the tolerance there is no fling at all; short-circuiting also keeps
        // the `ln`/`powf` below away from zero.
        let duration = if velocity.abs() < tolerance.velocity.max(f32::EPSILON) {
            0.0
        } else {
            let reference_velocity = friction * PHYSICAL_COEFF / INFLEXION;
            let android_duration =
                (velocity.abs() / reference_velocity).powf(1.0 / (DECELERATION_RATE - 1.0));
            // We finish slightly sooner than Android does, so as to cover the same
            // total distance.
            DECELERATION_RATE * INFLEXION * android_duration
        };
        Self {
            position,
            velocity,
            duration,
            distance: velocity * duration / DECELERATION_RATE,
            tolerance,
        }
    }

    /// The total run time, in seconds.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Where the fling comes to rest.
    pub fn final_x(&self) -> f32 {
        self.position + self.distance
    }

    /// Progress through the motion, in `0..=1`.
    fn progress(&self, time: f32) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (time / self.duration).clamp(0.0, 1.0)
        }
    }
}

impl Simulation for ClampingScrollSimulation {
    fn x(&self, time: f32) -> f32 {
        let t = self.progress(time);
        self.position + self.distance * (1.0 - (1.0 - t).powf(DECELERATION_RATE))
    }

    fn dx(&self, time: f32) -> f32 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        let t = self.progress(time);
        self.velocity * (1.0 - t).powf(DECELERATION_RATE - 1.0)
    }

    fn is_done(&self, time: f32) -> bool {
        time >= self.duration
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// A fling that rolls on friction while inside the content and springs back once
/// past its edge — the behaviour of platforms that bounce.
///
/// It is two simulations end to end: [`FrictionSimulation`] up to the instant the
/// momentum would carry the offset past an edge, then a [`SpringSimulation`] from
/// that edge, seeded with the velocity the friction had left at the hand-over.
/// Starting already out of range skips straight to the spring.
///
/// The hand-over instant is computed once, at construction, from
/// [`FrictionSimulation::time_at_x`] — so sampling stays a pure function of time.
#[derive(Clone, Copy, Debug)]
pub struct BouncingScrollSimulation {
    friction: FrictionSimulation,
    spring: SpringSimulation,
    /// When the spring takes over: `−∞` if it holds from the start, `+∞` if the
    /// motion never leaves the content, otherwise the hand-over instant.
    spring_time: f32,
    tolerance: Tolerance,
}

/// The drag of a scroll fling: the fraction of the velocity left after one second.
/// `0.998^1000 ≈ 0.135`, from the deceleration rate iOS scroll views use.
pub const BOUNCING_DRAG: f32 = 0.135;

/// The cap on the velocity handed from a fling's momentum to the overscroll
/// spring: past this, the bounce would be violent rather than lively.
pub const MAX_SPRING_TRANSFER_VELOCITY: f32 = 5000.0;

impl BouncingScrollSimulation {
    /// A fling from `position` at `velocity`, springing back to `leading` or
    /// `trailing` — the offsets the content is allowed to rest at.
    pub fn new(
        position: f32,
        velocity: f32,
        leading: f32,
        trailing: f32,
        spring: SpringDescription,
        tolerance: Tolerance,
    ) -> Self {
        debug_assert!(leading <= trailing);
        let friction = FrictionSimulation::new(BOUNCING_DRAG, position, velocity, tolerance);
        let to = |edge: f32, from: f32, v: f32| {
            SpringSimulation::new(spring, from, edge, v, tolerance)
        };

        // Already outside: the spring holds from the first instant, and the whole
        // motion is the return.
        if position < leading {
            return Self {
                friction,
                spring: to(leading, position, velocity),
                spring_time: f32::NEG_INFINITY,
                tolerance,
            };
        }
        if position > trailing {
            return Self {
                friction,
                spring: to(trailing, position, velocity),
                spring_time: f32::NEG_INFINITY,
                tolerance,
            };
        }

        // Inside: does the momentum carry us past an edge before it runs out?
        let final_x = friction.final_x();
        let (spring_time, spring) = if velocity > 0.0 && final_x > trailing {
            let t = friction.time_at_x(trailing);
            let carried = friction.dx(t).min(MAX_SPRING_TRANSFER_VELOCITY);
            (t, to(trailing, trailing, carried))
        } else if velocity < 0.0 && final_x < leading {
            let t = friction.time_at_x(leading);
            let carried = friction.dx(t).min(MAX_SPRING_TRANSFER_VELOCITY);
            (t, to(leading, leading, carried))
        } else {
            // The fling dies inside the content: the spring is never reached, and
            // this instance of it is a placeholder that is never sampled.
            (f32::INFINITY, to(position, position, 0.0))
        };
        Self {
            friction,
            spring,
            spring_time,
            tolerance,
        }
    }

    /// The simulation in force at `time`, and the offset to subtract from `time`
    /// before sampling it (the spring counts from its own start).
    fn phase(&self, time: f32) -> (&dyn Simulation, f32) {
        if time > self.spring_time {
            let offset = if self.spring_time.is_finite() {
                self.spring_time
            } else {
                0.0
            };
            (&self.spring, offset)
        } else {
            (&self.friction, 0.0)
        }
    }
}

impl Simulation for BouncingScrollSimulation {
    fn x(&self, time: f32) -> f32 {
        let (sim, offset) = self.phase(time);
        sim.x(time - offset)
    }

    fn dx(&self, time: f32) -> f32 {
        let (sim, offset) = self.phase(time);
        sim.dx(time - offset)
    }

    fn is_done(&self, time: f32) -> bool {
        let (sim, offset) = self.phase(time);
        sim.is_done(time - offset)
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// Pins a simulation's **position** into `[min, max]`; the velocity keeps
/// reporting the underlying simulation's. This is what scrolling needs: momentum
/// is computed freely, but the offset never leaves the bounds.
pub struct ClampedSimulation<S: Simulation> {
    inner: S,
    min: f32,
    max: f32,
}

impl<S: Simulation> ClampedSimulation<S> {
    /// Wraps `inner`, constraining its position to `[min, max]`.
    pub fn new(inner: S, min: f32, max: f32) -> Self {
        Self { inner, min, max }
    }
}

impl<S: Simulation> Simulation for ClampedSimulation<S> {
    fn x(&self, time: f32) -> f32 {
        self.inner.x(time).clamp(self.min, self.max)
    }

    fn dx(&self, time: f32) -> f32 {
        self.inner.dx(time)
    }

    fn is_done(&self, time: f32) -> bool {
        self.inner.is_done(time)
    }

    fn tolerance(&self) -> Tolerance {
        self.inner.tolerance()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Numerically integrates `dx` and checks it against `x`, which guarantees that
    /// a simulation's velocity and position agree with one another.
    fn dx_matches_x<S: Simulation>(sim: &S, t: f32) {
        let h = 1e-4;
        let numeric = (sim.x(t + h) - sim.x(t - h)) / (2.0 * h);
        let analytic = sim.dx(t);
        // A relative tolerance: the centred finite difference has a truncation error
        // proportional to the curvature, which is large at high speed.
        let scale = analytic.abs().max(1.0);
        assert!(
            (numeric - analytic).abs() < 1e-2 * scale,
            "inconsistent dx at t={t}: numeric={numeric}, analytic={analytic}"
        );
    }

    #[test]
    fn critical_spring_settles_without_overshoot() {
        let spring = SpringDescription::with_damping_ratio(1.0, 500.0, 1.0);
        let sim = SpringSimulation::new(spring, 0.0, 1.0, 0.0, Tolerance::default());
        assert!((sim.x(0.0) - 0.0).abs() < 1e-5, "starts at start");
        // Monotonically increasing, never above 1 — no oscillation.
        let mut prev = sim.x(0.0);
        let mut t = 0.0;
        while t < 2.0 {
            let v = sim.x(t);
            assert!(v <= 1.0 + 1e-4, "overshoot at t={t}: {v}");
            assert!(v >= prev - 1e-4, "not monotonic at t={t}");
            prev = v;
            t += 0.01;
        }
        assert!((sim.x(2.0) - 1.0).abs() < 1e-2, "arrives at end");
        assert!(sim.is_done(3.0), "at rest after 3 s");
        dx_matches_x(&sim, 0.2);
    }

    #[test]
    fn underdamped_spring_overshoots_then_returns() {
        let spring = SpringDescription::with_damping_ratio(1.0, 500.0, 0.3);
        let sim = SpringSimulation::new(spring, 0.0, 1.0, 0.0, Tolerance::default());
        // Lightly damped: it overshoots the target at least once.
        let mut max = f32::MIN;
        let mut t = 0.0;
        while t < 2.0 {
            max = max.max(sim.x(t));
            t += 0.005;
        }
        assert!(max > 1.05, "should overshoot the target, max={max}");
        // It settles on the target in the end.
        assert!((sim.x(3.0) - 1.0).abs() < 1e-2);
        dx_matches_x(&sim, 0.1);
    }

    #[test]
    fn overdamped_spring_is_slow_and_monotonic() {
        let spring = SpringDescription::with_damping_ratio(1.0, 100.0, 2.5);
        let sim = SpringSimulation::new(spring, 0.0, 1.0, 0.0, Tolerance::default());
        let mut prev = sim.x(0.0);
        let mut t = 0.0;
        while t < 3.0 {
            let v = sim.x(t);
            assert!(v <= 1.0 + 1e-4, "no overshoot when overdamped, at t={t}");
            assert!(v >= prev - 1e-4, "monotonic at t={t}");
            prev = v;
            t += 0.01;
        }
        dx_matches_x(&sim, 0.3);
    }

    #[test]
    fn spring_honours_initial_velocity() {
        let spring = SpringDescription::with_damping_ratio(1.0, 500.0, 1.0);
        let sim = SpringSimulation::new(spring, 0.0, 1.0, 50.0, Tolerance::default());
        // Initial velocity = 50 px/s towards the target.
        assert!((sim.dx(0.0) - 50.0).abs() < 1e-2, "dx(0) = {}", sim.dx(0.0));
    }

    #[test]
    fn friction_decelerates_to_finite_limit() {
        let sim = FrictionSimulation::new(0.135, 0.0, 1000.0, Tolerance::PIXELS);
        let limit = sim.final_x();
        // The position approaches the finite limit without passing it.
        assert!(sim.x(0.0).abs() < 1e-4);
        assert!(sim.x(10.0) <= limit + 1e-3);
        assert!(
            (sim.x(10.0) - limit).abs() < 1.0,
            "close to the limit after 10 s"
        );
        // The velocity decays towards 0.
        assert!(sim.dx(0.0) > sim.dx(1.0));
        assert!(sim.dx(5.0).abs() < sim.dx(0.0).abs());
        dx_matches_x(&sim, 0.5);
    }

    #[test]
    fn friction_through_hits_endpoints() {
        // It must pass through x=0 at v=1000 and end at x=200 with v about 0.
        let sim = FrictionSimulation::through(0.0, 200.0, 1000.0, 0.0, Tolerance::PIXELS);
        assert!((sim.x(0.0)).abs() < 1e-3);
        assert!(
            (sim.final_x() - 200.0).abs() < 1e-1,
            "final = {}",
            sim.final_x()
        );
    }

    #[test]
    fn friction_time_at_x_inverts_x() {
        let sim = FrictionSimulation::new(0.135, 0.0, 1000.0, Tolerance::PIXELS);
        let t = sim.time_at_x(200.0);
        assert!(t.is_finite(), "200 px is within reach of a 1000 px/s fling");
        assert!((sim.x(t) - 200.0).abs() < 1e-2, "x(time_at_x(200)) = {}", sim.x(t));
        // Behind the start, and past the limit: never reached.
        assert!(sim.time_at_x(-10.0).is_infinite());
        assert!(sim.time_at_x(sim.final_x() + 10.0).is_infinite());
        assert_eq!(sim.time_at_x(0.0), 0.0);
    }

    #[test]
    fn clamping_fling_matches_the_platform_distance() {
        let sim = ClampingScrollSimulation::new(0.0, 1000.0, Tolerance::PIXELS);
        // Derived by hand from the Android constants: a 1000 px/s fling runs about
        // 0.46 s and covers about 194 px.
        assert!(
            (sim.duration() - 0.458).abs() < 0.01,
            "duration = {}",
            sim.duration()
        );
        assert!(
            (sim.final_x() - 194.3).abs() < 1.0,
            "distance = {}",
            sim.final_x()
        );
        // Monotonic, decelerating, and finished at the end of its run.
        let mut prev = sim.x(0.0);
        let mut t = 0.0;
        while t < sim.duration() {
            let v = sim.x(t);
            assert!(v >= prev - 1e-4, "not monotonic at t={t}");
            prev = v;
            t += 0.01;
        }
        assert!((sim.dx(0.0) - 1000.0).abs() < 1e-1, "starts at the release velocity");
        assert!(sim.dx(sim.duration()).abs() < 1e-3, "ends at rest");
        assert!(!sim.is_done(sim.duration() * 0.5));
        assert!(sim.is_done(sim.duration()));
        dx_matches_x(&sim, 0.2);
    }

    #[test]
    fn clamping_fling_is_ballistic() {
        // The defining property: restarting the fling from its own mid-flight state
        // lands in the same place. It is what lets a fling be resumed from nothing
        // but the current offset and velocity.
        let sim = ClampingScrollSimulation::new(0.0, 1200.0, Tolerance::PIXELS);
        let t = 0.15;
        let restarted = ClampingScrollSimulation::new(sim.x(t), sim.dx(t), Tolerance::PIXELS);
        assert!(
            (restarted.final_x() - sim.final_x()).abs() < 1.0,
            "restarted lands at {} instead of {}",
            restarted.final_x(),
            sim.final_x()
        );
    }

    #[test]
    fn clamping_fling_below_tolerance_does_not_move() {
        let sim = ClampingScrollSimulation::new(42.0, 0.0, Tolerance::PIXELS);
        assert_eq!(sim.duration(), 0.0);
        assert_eq!(sim.x(0.0), 42.0);
        assert_eq!(sim.dx(0.0), 0.0);
        assert!(sim.is_done(0.0));
    }

    /// The spring the scroll policies use by default.
    fn scroll_spring() -> SpringDescription {
        SpringDescription::with_damping_ratio(0.5, 100.0, 1.1)
    }

    #[test]
    fn bouncing_fling_springs_back_when_it_starts_outside() {
        // 30 px past the end, released with no velocity: pure return.
        let sim = BouncingScrollSimulation::new(
            130.0,
            0.0,
            0.0,
            100.0,
            scroll_spring(),
            Tolerance::PIXELS,
        );
        assert!((sim.x(0.0) - 130.0).abs() < 1e-3, "starts where it was let go");
        assert!(sim.x(0.3) < 130.0, "comes back towards the edge");
        assert!(
            (sim.x(2.0) - 100.0).abs() < 0.5,
            "settles on the edge, at {}",
            sim.x(2.0)
        );
        assert!(sim.is_done(2.0));
    }

    #[test]
    fn bouncing_fling_hands_friction_over_to_the_spring_at_the_edge() {
        // Fast enough that friction alone would run far past the end.
        let sim = BouncingScrollSimulation::new(
            0.0,
            2000.0,
            0.0,
            100.0,
            scroll_spring(),
            Tolerance::PIXELS,
        );
        let friction = FrictionSimulation::new(0.135, 0.0, 2000.0, Tolerance::PIXELS);
        let handover = friction.time_at_x(100.0);
        assert!(handover.is_finite() && handover > 0.0);
        // Continuous across the hand-over: the position does not jump. The window
        // has to be tight — at the edge the content is still moving at ~1800 px/s,
        // so a millisecond either side is already two pixels of honest travel.
        let before = sim.x(handover - 1e-5);
        let after = sim.x(handover + 1e-5);
        assert!((before - 100.0).abs() < 0.1, "before = {before}");
        assert!((after - 100.0).abs() < 0.1, "after = {after}");
        assert!((after - before).abs() < 0.1, "jump at the hand-over");
        // It overshoots past the edge, then comes back and settles there.
        let mut peak: f32 = 0.0;
        let mut t = 0.0;
        while t < 1.5 {
            peak = peak.max(sim.x(t));
            t += 0.005;
        }
        assert!(peak > 105.0, "the bounce should overshoot, peak = {peak}");
        assert!(
            (sim.x(3.0) - 100.0).abs() < 0.5,
            "settles on the edge, at {}",
            sim.x(3.0)
        );
        assert!(sim.is_done(3.0));
    }

    #[test]
    fn bouncing_fling_that_dies_inside_never_reaches_the_spring() {
        // 300 px/s over a 1000 px range: friction runs out well before the end.
        let sim = BouncingScrollSimulation::new(
            0.0,
            300.0,
            0.0,
            1000.0,
            scroll_spring(),
            Tolerance::PIXELS,
        );
        let expected = FrictionSimulation::new(0.135, 0.0, 300.0, Tolerance::PIXELS);
        assert!(
            (sim.x(0.5) - expected.x(0.5)).abs() < 1e-3,
            "pure friction while inside the content"
        );
        assert!(sim.x(5.0) < 1000.0);
        assert!(sim.is_done(5.0));
    }

    #[test]
    fn clamped_pins_position_but_reports_velocity() {
        let inner = FrictionSimulation::new(0.135, 0.0, 1000.0, Tolerance::PIXELS);
        let uncapped = inner.x(10.0);
        assert!(uncapped > 100.0);
        let clamped = ClampedSimulation::new(inner, 0.0, 100.0);
        assert_eq!(clamped.x(10.0), 100.0, "position pinned to the maximum");
        // The velocity stays that of the free motion, non-zero near the edge.
        assert!(clamped.dx(0.1) > 0.0);
    }
}

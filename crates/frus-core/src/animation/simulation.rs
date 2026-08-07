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

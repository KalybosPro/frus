//! The driver: an [`AnimationController`] that samples a [`Simulation`] frame by
//! frame and exposes a **current value** and a **status**.
//!
//! Everything the controller does — a curve-shaped transition over a duration, or
//! a physical fling — is expressed as a `Simulation`, a pure `x(t)` function. One
//! tick loop therefore drives all of it: *one driver, pluggable time→value
//! functions*. This is the abstraction the shell instantiates by identity
//! (`child_id`); the view reads `value()` at paint time.

use super::curve::Curve;
use super::simulation::{Simulation, SpringDescription, SpringSimulation, Tolerance};

/// How far along an animation controller is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// At the lower bound, at rest.
    Dismissed,
    /// Moving towards the upper bound.
    Forward,
    /// Moving towards the lower bound.
    Reverse,
    /// At the upper bound, at rest.
    Completed,
}

/// A "curve interpolated over a duration" simulation: the *explicit* side
/// (duration plus curve), expressed as a `Simulation` like everything else.
#[derive(Clone, Debug)]
struct InterpolationSimulation {
    begin: f32,
    end: f32,
    duration: f32,
    curve: Curve,
}

impl Simulation for InterpolationSimulation {
    fn x(&self, time: f32) -> f32 {
        if self.duration <= 0.0 {
            return self.end;
        }
        let t = (time / self.duration).clamp(0.0, 1.0);
        let shaped = self.curve.transform(t);
        self.begin + (self.end - self.begin) * shaped
    }

    fn dx(&self, time: f32) -> f32 {
        let h = 1e-3;
        (self.x(time + h) - self.x(time - h)) / (2.0 * h)
    }

    fn is_done(&self, time: f32) -> bool {
        time >= self.duration
    }
}

/// **Repeat** mode: each cycle lasts `period` seconds, shaped by `curve`;
/// `reverse` makes it go there and back, otherwise it restarts from the bottom.
#[derive(Clone)]
struct Repeat {
    period: f32,
    reverse: bool,
    curve: Curve,
}

/// Drives a value bounded to `[lower, upper]` through a pluggable simulation.
pub struct AnimationController {
    value: f32,
    velocity: f32,
    lower: f32,
    upper: f32,
    status: Status,
    /// The active simulation plus time elapsed since it started; `None` at rest.
    active: Option<(Box<dyn Simulation>, f32)>,
    /// The intended direction, which sorts the resting status (`Completed` vs
    /// `Dismissed`).
    heading_up: bool,
    /// The active repeat loop; `None` means a one-shot animation.
    repeat: Option<Repeat>,
}

impl AnimationController {
    /// A controller bounding its value to `[lower, upper]`, starting at `lower`.
    pub fn new(lower: f32, upper: f32) -> Self {
        Self {
            value: lower,
            velocity: 0.0,
            lower,
            upper,
            status: Status::Dismissed,
            active: None,
            heading_up: false,
            repeat: None,
        }
    }

    /// The standard controller over `[0, 1]`.
    pub fn unit() -> Self {
        Self::new(0.0, 1.0)
    }

    /// The controller's `[lower, upper]` bounds.
    pub fn bounds(&self) -> (f32, f32) {
        (self.lower, self.upper)
    }

    /// The current value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// The current velocity, in units per second.
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// The current status.
    pub fn status(&self) -> Status {
        self.status
    }

    /// `true` while a simulation is running, meaning a frame is needed.
    pub fn is_animating(&self) -> bool {
        self.active.is_some()
    }

    /// Sets the value immediately, with no animation, and updates the status.
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.lower, self.upper);
        self.velocity = 0.0;
        self.active = None;
        self.repeat = None;
        self.status = self.rest_status();
    }

    /// Puts the value back to the lower bound, unanimated (`set_value(lower)`).
    pub fn reset(&mut self) {
        self.set_value(self.lower);
    }

    /// Stops any animation, a [`AnimationController::repeat`] loop included, and
    /// freezes the current value.
    pub fn stop(&mut self) {
        self.active = None;
        self.repeat = None;
        self.velocity = 0.0;
        self.status = self.rest_status();
    }

    /// Animates to the upper bound over `duration` seconds, along `curve`.
    pub fn forward(&mut self, duration: f32, curve: Curve) {
        self.animate_to(self.upper, duration, curve);
    }

    /// Animates to the lower bound over `duration` seconds, along `curve`.
    pub fn reverse(&mut self, duration: f32, curve: Curve) {
        self.animate_to(self.lower, duration, curve);
    }

    /// Animates the current value towards `target` over `duration` seconds. This
    /// cancels any `repeat` loop, making it a one-shot animation.
    pub fn animate_to(&mut self, target: f32, duration: f32, curve: Curve) {
        self.repeat = None;
        self.start_interpolation(target, duration, curve);
    }

    /// Starts an interpolation towards `target` **without** touching `repeat` mode
    /// — shared by `animate_to` and by the loop restart inside `tick`.
    fn start_interpolation(&mut self, target: f32, duration: f32, curve: Curve) {
        let target = target.clamp(self.lower, self.upper);
        self.heading_up = target >= self.value;
        self.status = if self.heading_up {
            Status::Forward
        } else {
            Status::Reverse
        };
        self.active = Some((
            Box::new(InterpolationSimulation {
                begin: self.value,
                end: target,
                duration,
                curve,
            }),
            0.0,
        ));
    }

    /// **Repeats** the animation in a loop, each cycle lasting `period` seconds and
    /// shaped by `curve`. With `reverse` it goes there and back (`0→1→0…`);
    /// otherwise each cycle restarts from the bottom (`0→1, 0→1…`). It only ends at
    /// [`stop`](Self::stop), or `set_value` / `animate_to`. This is the idiom for
    /// continuous driven animations: a pulse, a busy indicator, a halo.
    pub fn repeat(&mut self, period: f32, reverse: bool, curve: Curve) {
        self.repeat = Some(Repeat {
            period,
            reverse,
            curve: curve.clone(),
        });
        // Start the first cycle upwards from the current value.
        self.start_interpolation(self.upper, period, curve);
    }

    /// Launches a physical fling: a critical spring started by `velocity`, settling
    /// on the position, bounded to `[lower, upper]`.
    pub fn fling(&mut self, velocity: f32) {
        self.repeat = None;
        // A critically damped spring by default.
        let spring = SpringDescription::with_damping_ratio(1.0, 500.0, 1.0);
        let target = if velocity < 0.0 {
            self.lower
        } else {
            self.upper
        };
        self.heading_up = velocity >= 0.0;
        self.status = if self.heading_up {
            Status::Forward
        } else {
            Status::Reverse
        };
        self.active = Some((
            Box::new(SpringSimulation::new(
                spring,
                self.value,
                target,
                velocity,
                Tolerance::default(),
            )),
            0.0,
        ));
    }

    /// Animates the current value towards `target` with a **spring** described by
    /// `spring`, started at `velocity` (units/s). This is the path taken by
    /// interruptible, gesture-driven transitions — a drag release, a back gesture
    /// carried by the finger's momentum: the spring starts from the current position
    /// *and* the current velocity.
    pub fn spring_to(&mut self, target: f32, spring: SpringDescription, velocity: f32) {
        let target = target.clamp(self.lower, self.upper);
        let heading_up = target >= self.value;
        let sim = SpringSimulation::new(spring, self.value, target, velocity, Tolerance::default());
        self.drive(Box::new(sim), heading_up);
    }

    /// Drives an arbitrary simulation — scroll momentum, a bespoke curve.
    pub fn drive(&mut self, simulation: Box<dyn Simulation>, heading_up: bool) {
        self.repeat = None;
        self.heading_up = heading_up;
        self.status = if heading_up {
            Status::Forward
        } else {
            Status::Reverse
        };
        self.active = Some((simulation, 0.0));
    }

    /// The resting status matching the current value.
    fn rest_status(&self) -> Status {
        if (self.value - self.upper).abs() < 1e-4 {
            Status::Completed
        } else if (self.value - self.lower).abs() < 1e-4 {
            Status::Dismissed
        } else if self.heading_up {
            Status::Completed
        } else {
            Status::Dismissed
        }
    }

    /// Advances the animation by `dt` seconds, updating value, velocity and status.
    /// Returns `true` while the animation continues, meaning another frame is due.
    pub fn tick(&mut self, dt: f32) -> bool {
        let Some((sim, elapsed)) = self.active.as_mut() else {
            return false;
        };
        *elapsed += dt;
        let time = *elapsed;
        let raw = sim.x(time);
        let velocity = sim.dx(time);
        let done = sim.is_done(time);
        // The borrow of `self.active` ends here; `sim` and `elapsed` are done with.
        self.value = raw.clamp(self.lower, self.upper);
        self.velocity = velocity;

        // The end: either the simulation finished, or the value left the bounds — a
        // fling hitting an edge stops dead, with no bounce beyond it.
        let out_of_bounds = raw <= self.lower - 1e-4 || raw >= self.upper + 1e-4;
        if done || out_of_bounds {
            // A loop is active: restart a cycle instead of coming to rest.
            if let Some(rep) = self.repeat.clone() {
                let target = if rep.reverse {
                    // There and back: head for the edge opposite the one reached.
                    if self.heading_up {
                        self.lower
                    } else {
                        self.upper
                    }
                } else {
                    // Sawtooth: start again from the bottom, heading up.
                    self.value = self.lower;
                    self.upper
                };
                self.start_interpolation(target, rep.period, rep.curve);
                return true;
            }
            self.velocity = 0.0;
            self.active = None;
            self.status = self.rest_status();
            false
        } else {
            true
        }
    }
}

impl Default for AnimationController {
    /// A standard controller over `[0, 1]`, at rest at `0`.
    fn default() -> Self {
        Self::unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Advances the controller to rest, or to `max_frames`, in 16 ms steps.
    fn settle(ctrl: &mut AnimationController, max_frames: usize) -> usize {
        let mut frames = 0;
        while ctrl.tick(0.016) {
            frames += 1;
            if frames >= max_frames {
                break;
            }
        }
        frames
    }

    #[test]
    fn forward_reaches_upper_and_completes() {
        let mut ctrl = AnimationController::unit();
        assert_eq!(ctrl.status(), Status::Dismissed);
        ctrl.forward(0.3, Curve::ease_in_out());
        assert_eq!(ctrl.status(), Status::Forward);
        assert!(ctrl.is_animating());
        settle(&mut ctrl, 1000);
        assert!(
            (ctrl.value() - 1.0).abs() < 1e-3,
            "value = {}",
            ctrl.value()
        );
        assert_eq!(ctrl.status(), Status::Completed);
        assert!(!ctrl.is_animating());
    }

    #[test]
    fn reverse_reaches_lower_and_dismisses() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(1.0);
        assert_eq!(ctrl.status(), Status::Completed);
        ctrl.reverse(0.3, Curve::ease_in_out());
        assert_eq!(ctrl.status(), Status::Reverse);
        settle(&mut ctrl, 1000);
        assert!(ctrl.value().abs() < 1e-3);
        assert_eq!(ctrl.status(), Status::Dismissed);
    }

    #[test]
    fn value_is_monotonic_during_forward() {
        let mut ctrl = AnimationController::unit();
        ctrl.forward(0.5, Curve::ease_in_out());
        let mut prev = ctrl.value();
        while ctrl.tick(0.016) {
            assert!(
                ctrl.value() >= prev - 1e-4,
                "recule : {} < {}",
                ctrl.value(),
                prev
            );
            assert!(ctrl.value() <= 1.0 + 1e-4);
            prev = ctrl.value();
        }
    }

    #[test]
    fn fling_upward_settles_at_upper() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.5);
        ctrl.fling(3.0); // a positive velocity heads up
        assert_eq!(ctrl.status(), Status::Forward);
        settle(&mut ctrl, 2000);
        assert!(
            (ctrl.value() - 1.0).abs() < 1e-2,
            "value = {}",
            ctrl.value()
        );
        assert_eq!(ctrl.status(), Status::Completed);
    }

    #[test]
    fn fling_downward_settles_at_lower() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.5);
        ctrl.fling(-3.0);
        settle(&mut ctrl, 2000);
        assert!(ctrl.value().abs() < 1e-2, "value = {}", ctrl.value());
        assert_eq!(ctrl.status(), Status::Dismissed);
    }

    #[test]
    fn spring_to_settles_at_target_from_velocity() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.2);
        // A flick-driven release: a near-critical spring towards the upper bound.
        let spring = SpringDescription::new(1.0, 220.0, 30.0);
        ctrl.spring_to(1.0, spring, 5.0);
        assert_eq!(ctrl.status(), Status::Forward);
        settle(&mut ctrl, 2000);
        assert!(
            (ctrl.value() - 1.0).abs() < 1e-2,
            "value = {}",
            ctrl.value()
        );
        assert_eq!(ctrl.status(), Status::Completed);
    }

    /// `repeat` (sawtooth): never rests, reaches the top, then **drops back** as the
    /// cycle restarts.
    #[test]
    fn repeat_never_settles_and_restarts() {
        let mut ctrl = AnimationController::unit();
        ctrl.repeat(0.1, false, Curve::Linear);
        let (mut max, mut saw_restart, mut prev) = (0.0f32, false, ctrl.value());
        for _ in 0..100 {
            // about 1.6 s = 16 cycles
            ctrl.tick(0.016);
            assert!(ctrl.is_animating(), "une boucle reste toujours animée");
            assert!(ctrl.value() >= -1e-4 && ctrl.value() <= 1.0 + 1e-4);
            max = max.max(ctrl.value());
            if ctrl.value() + 1e-3 < prev {
                saw_restart = true; // the value dropped, so a new cycle began
            }
            prev = ctrl.value();
        }
        assert!(max > 0.9, "atteint le haut (max = {max})");
        assert!(saw_restart, "redémarre (la valeur retombe)");
    }

    /// `repeat(reverse)`: there and back — the value rises, then falls again.
    #[test]
    fn repeat_reverse_ping_pongs() {
        let mut ctrl = AnimationController::unit();
        ctrl.repeat(0.1, true, Curve::Linear);
        let (mut went_up, mut came_down, mut prev) = (false, false, ctrl.value());
        for _ in 0..40 {
            ctrl.tick(0.016);
            if ctrl.value() > prev + 1e-4 {
                went_up = true;
            }
            if went_up && ctrl.value() < prev - 1e-4 {
                came_down = true;
            }
            prev = ctrl.value();
        }
        assert!(went_up && came_down, "aller-retour (montée puis descente)");
        assert!(ctrl.is_animating());
    }

    /// `stop` ends a loop; `reset` brings it back to the bottom.
    #[test]
    fn stop_and_reset_end_a_repeat() {
        let mut ctrl = AnimationController::unit();
        ctrl.repeat(0.1, false, Curve::Linear);
        ctrl.tick(0.05);
        assert!(ctrl.is_animating());
        ctrl.stop();
        assert!(!ctrl.is_animating(), "stop arrête la boucle");
        assert!(!ctrl.tick(0.016), "plus rien à animer après stop");

        ctrl.repeat(0.1, false, Curve::Linear);
        ctrl.tick(0.05);
        ctrl.reset();
        assert!(!ctrl.is_animating(), "reset arrête la boucle");
        assert_eq!(ctrl.value(), 0.0, "reset ramène à la borne basse");
    }

    #[test]
    fn zero_duration_snaps_immediately() {
        let mut ctrl = AnimationController::unit();
        ctrl.forward(0.0, Curve::Linear);
        // A single frame is enough to reach the target.
        ctrl.tick(0.016);
        assert!((ctrl.value() - 1.0).abs() < 1e-4);
        assert!(!ctrl.is_animating());
    }
}

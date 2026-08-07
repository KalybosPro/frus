//! Gesture recognition — **tier 1** of the brief (§3): a "tap-or-long-press"
//! recogniser that already speaks the **arena**'s vocabulary, so that moving to the
//! real arena (tier 2) is a substitution rather than a rewrite.
//!
//! - The **long press** accepts **eagerly** once the delay elapses: it evicts the
//!   tap, and the release that follows is swallowed.
//! - The **tap** accepts **passively**: it wins when nothing evicted it by the time
//!   of the release. That is the shell's existing click path.
//! - Movement beyond the **slop** rejects the long press, the gesture having become
//!   a drag or a scroll; `Cancel` rejects everything.
//!
//! The machine is **pure** — it never reads a clock, instants come in as parameters
//! — and so is testable down to the tick.

use std::time::Duration;

use web_time::Instant;

use frus_widgets::{FrictionSimulation, Point, Tolerance};

/// How long a motionless press must last before the long press is accepted.
pub(crate) const LONG_PRESS_DELAY: Duration = Duration::from_millis(500);
/// The movement, in logical px, beyond which the long press is rejected.
const SLOP: f32 = 8.0;

/// A fling's deceleration: the fraction of the velocity left after 1 s
/// (`dx(t) = v·drag^t`, scrolling's usual friction constant).
const FLING_DRAG: f32 = 0.135;
/// The minimum release velocity, in px/s, that triggers a fling.
const FLING_MIN_VELOCITY: f32 = 50.0;

/// A scroll fling's **ballistic** destination: the final position of a
/// [`FrictionSimulation`] started at `velocity` from `current` — the finger's
/// momentum, in closed form. `None` below the velocity threshold, since a slow
/// release does not carry the content along.
pub(crate) fn fling_destination(current: f32, velocity: f32) -> Option<f32> {
    if velocity.abs() < FLING_MIN_VELOCITY {
        return None;
    }
    Some(FrictionSimulation::new(FLING_DRAG, current, velocity, Tolerance::PIXELS).final_x())
}

/// A **normalised** pointer event, mouse or finger: routing's single input — tier 0
/// of the brief, with an explicit `Cancel`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PointerKind {
    Down,
    Move,
    Up,
    /// The gesture was interrupted — the app went to the background, the touch was
    /// cancelled: give up without a success callback.
    Cancel,
}

/// The normalised event: its nature, its **logical** position, and whether the
/// source is touch — touch is what arms finger scrolling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PointerEvent {
    pub kind: PointerKind,
    pub position: Point,
    pub touch: bool,
}

/// The recogniser's internal states.
#[derive(Debug)]
enum State {
    /// No press is being tracked.
    Idle,
    /// A press is under way and still a candidate for the long press.
    Possible { origin: Point, deadline: Instant },
    /// The long press **accepted**, that is, fired: the next release is swallowed.
    Fired,
    /// The candidate was rejected, movement exceeding the slop; the press carries on
    /// as a tap or a drag.
    Rejected,
}

/// The tap-or-long-press recogniser, one instance per active pointer.
pub(crate) struct PressRecognizer {
    state: State,
}

impl PressRecognizer {
    pub fn new() -> Self {
        Self { state: State::Idle }
    }

    /// A press: arms tracking when the target cares about the long press
    /// (`interested`), and stays inert otherwise.
    pub fn down(&mut self, at: Point, now: Instant, interested: bool) {
        self.state = if interested {
            State::Possible {
                origin: at,
                deadline: now + LONG_PRESS_DELAY,
            }
        } else {
            State::Rejected
        };
    }

    /// Movement: beyond the slop the long press is rejected, the gesture being a
    /// drag — "accepts eagerly on crossing the slop", from the drag's side.
    pub fn moved(&mut self, at: Point) {
        if let State::Possible { origin, .. } = self.state {
            let (dx, dy) = (at.x - origin.x, at.y - origin.y);
            if dx * dx + dy * dy > SLOP * SLOP {
                self.state = State::Rejected;
            }
        }
    }

    /// The deadline at which the long press will fire, if it is still a candidate —
    /// hand it to `ControlFlow::WaitUntil` to be woken at exactly the right moment.
    pub fn deadline(&self) -> Option<Instant> {
        match self.state {
            State::Possible { deadline, .. } => Some(deadline),
            _ => None,
        }
    }

    /// Has time made the long press fire? `true` exactly once.
    pub fn poll(&mut self, now: Instant) -> bool {
        if let State::Possible { deadline, .. } = self.state {
            if now >= deadline {
                self.state = State::Fired;
                return true;
            }
        }
        false
    }

    /// A release: returns `true` when the click must be **swallowed**, a long press
    /// having already accepted and evicted the tap.
    pub fn up(&mut self) -> bool {
        let swallow = matches!(self.state, State::Fired);
        self.state = State::Idle;
        swallow
    }

    /// An interruption: give up without emitting anything.
    pub fn cancel(&mut self) {
        self.state = State::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> Instant {
        Instant::now()
    }

    #[test]
    fn fling_projects_a_friction_final_position() {
        // Below the threshold there is no fling.
        assert_eq!(fling_destination(100.0, 0.0), None);
        assert_eq!(fling_destination(100.0, 30.0), None);

        // Momentum: destination = position + v / ln(1/drag), in closed form.
        let dest = fling_destination(0.0, 2000.0).expect("fling");
        let expected = 2000.0 / (1.0f32 / FLING_DRAG).ln();
        assert!(
            (dest - expected).abs() < 1.0,
            "dest = {dest}, expected ≈ {expected}"
        );
        assert!(dest > 900.0 && dest < 1100.0, "≈ 1000 px of travel: {dest}");

        // Symmetrical, backwards.
        let back = fling_destination(500.0, -2000.0).expect("backward fling");
        assert!((back - (500.0 - expected)).abs() < 1.0);
    }

    #[test]
    fn long_press_fires_after_the_delay_and_swallows_the_release() {
        let mut rec = PressRecognizer::new();
        let t0 = start();
        rec.down(Point::new(10.0, 10.0), t0, true);
        assert_eq!(rec.deadline(), Some(t0 + LONG_PRESS_DELAY));

        // Before the deadline: nothing.
        assert!(!rec.poll(t0 + Duration::from_millis(499)));
        // At the deadline: it fires, exactly once.
        assert!(rec.poll(t0 + LONG_PRESS_DELAY));
        assert!(!rec.poll(t0 + Duration::from_millis(600)));
        // The release that follows is swallowed: the long press evicts the tap.
        assert!(rec.up());
        // And the state is reset.
        assert!(rec.deadline().is_none());
    }

    #[test]
    fn release_before_the_delay_is_a_plain_tap() {
        let mut rec = PressRecognizer::new();
        let t0 = start();
        rec.down(Point::new(0.0, 0.0), t0, true);
        assert!(!rec.up(), "a tap: the click is not swallowed");
        assert!(
            !rec.poll(t0 + Duration::from_secs(1)),
            "nothing left to fire"
        );
    }

    #[test]
    fn movement_beyond_slop_rejects_the_long_press() {
        let mut rec = PressRecognizer::new();
        let t0 = start();
        rec.down(Point::new(0.0, 0.0), t0, true);
        // Below the slop: still a candidate.
        rec.moved(Point::new(4.0, 4.0));
        assert!(rec.deadline().is_some());
        // Beyond it: rejected, the deadline vanishes, time no longer fires anything.
        rec.moved(Point::new(10.0, 10.0));
        assert!(rec.deadline().is_none());
        assert!(!rec.poll(t0 + Duration::from_secs(1)));
        assert!(!rec.up(), "an ordinary click is still possible");
    }

    #[test]
    fn uninterested_target_never_arms() {
        let mut rec = PressRecognizer::new();
        rec.down(Point::new(0.0, 0.0), start(), false);
        assert!(rec.deadline().is_none());
        assert!(!rec.poll(start() + Duration::from_secs(1)));
    }

    #[test]
    fn cancel_abandons_without_firing() {
        let mut rec = PressRecognizer::new();
        let t0 = start();
        rec.down(Point::new(0.0, 0.0), t0, true);
        rec.cancel();
        assert!(!rec.poll(t0 + Duration::from_secs(1)));
        assert!(!rec.up());
    }
}

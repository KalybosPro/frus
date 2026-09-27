//! The **overscroll stretch**: the content itself lengthens towards the edge a finger
//! pulls past, and springs back when it lets go.
//!
//! It is the other answer to "there is nothing more that way", beside the
//! [glow](crate::overscroll). A glow is light drawn over the content; a stretch is the
//! content deformed. Current touch platforms stretch, and an application that glows on
//! them looks older than it is.
//!
//! The amount follows the reference's controller, which ports the platform's own edge
//! effect: a pull is normalised by the viewport's length and shaped by a linear and an
//! exponential term; a fling that lands on the edge, or a finger that lets go, hands the
//! stretch to a damped spring that brings it back to rest. The deformation is the
//! reference's portable one: a **scale along the axis, anchored at the edge pulled**.

use frus_core::animation::{Simulation, SpringDescription, SpringSimulation, Tolerance};

use crate::overscroll::GlowEdge;

/// The largest stretch, either way: the content at most doubles along the axis.
const MAX_STRETCH: f32 = 1.0;
/// The strength of both terms of a pull (`_stretchIntensity`).
const INTENSITY: f32 = 0.016;
/// How fast the exponential term saturates (`_exponentialScalar`, `e / 0.33`).
const EXPONENTIAL_SCALAR: f32 = std::f32::consts::E / 0.33;
/// Turns a fling's velocity, in px/s, into the spring's (`_absorbImpactVelocityFriction`).
const IMPACT_FRICTION: f32 = 1.0 / 3000.0;
/// The most an impact may throw the stretch (`_maxAbsorbImpactVelocity`).
const MAX_IMPACT_VELOCITY: f32 = 1.25;
/// The platform's edge-effect spring: its natural frequency, in rad/s, and its damping.
const NATURAL_FREQUENCY: f32 = 24.657;
const DAMPING_RATIO: f32 = 0.98;
/// The reference slows the platform's spring by this much to match it by eye
/// (`kTimeCorrectionFactor`): the stiffness by its square, the initial velocity by it.
const TIME_CORRECTION: f32 = 0.8;

/// The spring that brings a stretch back to rest.
fn spring() -> SpringDescription {
    let stiffness = NATURAL_FREQUENCY * NATURAL_FREQUENCY * TIME_CORRECTION * TIME_CORRECTION;
    SpringDescription::with_damping_ratio(1.0, stiffness, DAMPING_RATIO)
}

/// The stretch along one axis of one scrollable.
///
/// Its amount is signed: **negative towards the start** of the axis (the top, or the
/// left), positive towards the end — the physical edge, whichever way the content scrolls.
#[derive(Copy, Clone, Debug, Default)]
pub struct OverscrollStretch {
    amount: f32,
    /// The pull accumulated since the gesture started, in pixels, signed like `amount`.
    pulled: f32,
    /// The spring running, and how long it has run, while the stretch returns.
    spring: Option<(SpringSimulation, f32)>,
    /// What a spring interrupted by a new pull had reached: the pull carries on from it,
    /// rather than jumping (`_interruptedOverscroll`).
    interrupted: f32,
}

impl OverscrollStretch {
    /// The stretch now, in `-1..=1`: `0` is the content as it is laid out.
    pub fn amount(&self) -> f32 {
        self.amount
    }

    /// Is there nothing to deform and nothing moving?
    pub fn is_idle(&self) -> bool {
        self.spring.is_none() && self.amount == 0.0
    }

    fn set(&mut self, amount: f32) {
        self.amount = amount.clamp(-MAX_STRETCH, MAX_STRETCH);
    }

    /// A finger dragged `overscroll` pixels past the edge (a magnitude), towards the
    /// start of the axis or its end, in a viewport `extent` long.
    pub fn pull(&mut self, overscroll: f32, towards_start: bool, extent: f32) {
        if let Some((spring, t)) = self.spring.take() {
            self.interrupted = spring.x(t);
        }
        let sign = if towards_start { -1.0 } else { 1.0 };
        self.pulled += sign * overscroll.abs();
        // Clamped to one viewport, the furthest a single finger could pull.
        let distance = (self.pulled / extent.max(1.0)).clamp(-1.0, 1.0);
        let far = distance.abs();
        let linear = INTENSITY * far;
        let exponential = INTENSITY * (1.0 - (-far * EXPONENTIAL_SCALAR).exp());
        self.set(distance.signum() * (linear + exponential) + self.interrupted);
    }

    /// A fling reached the edge at `velocity` px/s (a magnitude), towards the start or the
    /// end: the stretch is thrown that way and springs back.
    pub fn absorb_impact(&mut self, velocity: f32, towards_start: bool) {
        if velocity == 0.0 {
            return;
        }
        let sign = if towards_start { -1.0 } else { 1.0 };
        let thrown = (sign * velocity.abs() * IMPACT_FRICTION)
            .clamp(-MAX_IMPACT_VELOCITY, MAX_IMPACT_VELOCITY);
        self.release_with(thrown);
    }

    /// The gesture is over, or the content moved again: whatever is stretched returns.
    pub fn release(&mut self) {
        self.pulled = 0.0;
        if self.amount == 0.0 || self.spring.is_some() {
            return;
        }
        self.release_with(0.0);
    }

    fn release_with(&mut self, velocity: f32) {
        self.pulled = 0.0;
        self.spring = Some((
            SpringSimulation::new(
                spring(),
                self.amount,
                0.0,
                velocity * TIME_CORRECTION,
                Tolerance::default(),
            ),
            0.0,
        ));
    }

    /// Advances the spring by `dt` seconds. Returns `true` while it is still moving.
    pub fn advance(&mut self, dt: f32) -> bool {
        let Some((spring, t)) = self.spring.as_mut() else {
            return false;
        };
        *t += dt;
        if spring.is_done(*t) {
            self.spring = None;
            self.amount = 0.0;
            self.interrupted = 0.0;
            return false;
        }
        let x = spring.x(*t);
        self.set(x);
        true
    }
}

/// Both axes of one scrollable's stretch.
#[derive(Copy, Clone, Debug, Default)]
pub struct ScrollStretch {
    /// Along the horizontal axis.
    pub horizontal: OverscrollStretch,
    /// Along the vertical axis.
    pub vertical: OverscrollStretch,
}

impl ScrollStretch {
    /// The axis that `edge` lies across, to drive.
    pub fn axis_mut(&mut self, edge: GlowEdge) -> &mut OverscrollStretch {
        match edge {
            GlowEdge::Top | GlowEdge::Bottom => &mut self.vertical,
            GlowEdge::Left | GlowEdge::Right => &mut self.horizontal,
        }
    }

    /// Is neither axis stretched nor moving?
    pub fn is_idle(&self) -> bool {
        self.horizontal.is_idle() && self.vertical.is_idle()
    }

    /// Advances both. Returns `true` while either is still moving.
    pub fn advance(&mut self, dt: f32) -> bool {
        let x = self.horizontal.advance(dt);
        let y = self.vertical.advance(dt);
        x || y
    }

    /// Lets both go.
    pub fn release(&mut self) {
        self.horizontal.release();
        self.vertical.release();
    }

    /// The scale to draw the content with, `(sx, sy)`, and the point it is anchored at,
    /// in `viewport`: the edge each axis is stretched from stays where it is.
    pub fn transform(&self, viewport: frus_core::Rect) -> Option<frus_core::Affine> {
        let (h, v) = (self.horizontal.amount(), self.vertical.amount());
        if h == 0.0 && v == 0.0 {
            return None;
        }
        let anchor = frus_core::Point::new(
            if h > 0.0 {
                viewport.x + viewport.width
            } else {
                viewport.x
            },
            if v > 0.0 {
                viewport.y + viewport.height
            } else {
                viewport.y
            },
        );
        Some(frus_core::Affine::scale(1.0 + h.abs(), 1.0 + v.abs()).about(anchor))
    }
}

/// Whether `edge` is the start of its axis — the top, or the left.
pub(crate) fn is_start(edge: GlowEdge) -> bool {
    matches!(edge, GlowEdge::Top | GlowEdge::Left)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(stretch: &mut OverscrollStretch) -> f32 {
        let mut t = 0.0;
        while stretch.advance(1.0 / 60.0) {
            t += 1.0 / 60.0;
            assert!(t < 5.0, "a stretch comes to rest");
        }
        t
    }

    /// The reference's numbers: a pull of a whole viewport is `0.016 · (1 + 1 − e^(−e/0.33))`.
    #[test]
    fn a_pull_follows_the_platform_curve() {
        let mut stretch = OverscrollStretch::default();
        stretch.pull(600.0, true, 600.0);
        let expected = -(0.016 + 0.016 * (1.0 - (-EXPONENTIAL_SCALAR).exp()));
        assert!(
            (stretch.amount() - expected).abs() < 1e-6,
            "{}",
            stretch.amount()
        );
    }

    /// A pull accumulates: two halves are one whole, and more than a viewport is capped.
    #[test]
    fn a_pull_accumulates_up_to_one_viewport() {
        let mut halves = OverscrollStretch::default();
        halves.pull(300.0, false, 600.0);
        halves.pull(300.0, false, 600.0);
        let mut whole = OverscrollStretch::default();
        whole.pull(600.0, false, 600.0);
        assert!((halves.amount() - whole.amount()).abs() < 1e-6);
        assert!(whole.amount() > 0.0, "towards the end is positive");
        let mut more = OverscrollStretch::default();
        more.pull(2000.0, false, 600.0);
        assert_eq!(more.amount(), whole.amount());
    }

    /// A longer pull stretches more, and the start and the end stretch opposite ways.
    #[test]
    fn a_longer_pull_stretches_more() {
        let mut short = OverscrollStretch::default();
        short.pull(20.0, true, 600.0);
        let mut long = OverscrollStretch::default();
        long.pull(200.0, true, 600.0);
        assert!(long.amount() < short.amount() && short.amount() < 0.0);
    }

    /// Letting go springs back to nothing, and quickly: the platform's spring settles in
    /// well under a second.
    #[test]
    fn letting_go_springs_back() {
        let mut stretch = OverscrollStretch::default();
        stretch.pull(300.0, true, 600.0);
        stretch.release();
        assert!(!stretch.is_idle());
        let took = settle(&mut stretch);
        assert!(stretch.is_idle() && stretch.amount() == 0.0);
        assert!(took < 1.0, "{took}");
    }

    /// A held pull stays: nothing moves it until the finger lets go.
    #[test]
    fn a_held_pull_stays() {
        let mut stretch = OverscrollStretch::default();
        stretch.pull(300.0, true, 600.0);
        let held = stretch.amount();
        assert!(!stretch.advance(0.5));
        assert_eq!(stretch.amount(), held);
    }

    /// A fling that lands throws the stretch out towards its edge, then it returns — and a
    /// harder landing throws it further.
    #[test]
    fn an_impact_throws_the_stretch_and_it_returns() {
        let peak = |velocity: f32| {
            let mut stretch = OverscrollStretch::default();
            stretch.absorb_impact(velocity, false);
            let mut peak: f32 = 0.0;
            while stretch.advance(1.0 / 240.0) {
                peak = peak.max(stretch.amount());
            }
            assert_eq!(stretch.amount(), 0.0);
            peak
        };
        let soft = peak(1000.0);
        let hard = peak(3000.0);
        assert!(soft > 0.0 && hard > soft, "{soft} {hard}");
    }

    /// A pull during the return picks up from where the spring had got to, rather than
    /// snapping back first.
    #[test]
    fn a_pull_mid_return_carries_on_from_there() {
        let mut stretch = OverscrollStretch::default();
        stretch.pull(600.0, true, 600.0);
        stretch.release();
        stretch.advance(0.02);
        let reached = stretch.amount();
        stretch.pull(1.0, true, 600.0);
        assert!(stretch.amount() < reached, "{} {reached}", stretch.amount());
    }

    /// The content is scaled along the stretched axis only, from the edge pulled.
    #[test]
    fn the_transform_holds_the_pulled_edge() {
        let viewport = frus_core::Rect::new(10.0, 20.0, 100.0, 200.0);
        let mut stretch = ScrollStretch::default();
        assert!(stretch.transform(viewport).is_none());
        stretch.axis_mut(GlowEdge::Top).pull(200.0, true, 200.0);
        let m = stretch.transform(viewport).expect("stretched");
        let top = m.apply(frus_core::Point::new(10.0, 20.0));
        let bottom = m.apply(frus_core::Point::new(110.0, 220.0));
        assert!(
            (top.x - 10.0).abs() < 1e-4 && (top.y - 20.0).abs() < 1e-4,
            "{top:?}"
        );
        assert!((bottom.x - 110.0).abs() < 1e-4, "not across: {bottom:?}");
        assert!(bottom.y > 220.0, "down the axis: {bottom:?}");

        let mut end = ScrollStretch::default();
        end.axis_mut(GlowEdge::Bottom).pull(200.0, false, 200.0);
        let m = end.transform(viewport).unwrap();
        let bottom = m.apply(frus_core::Point::new(10.0, 220.0));
        let top = m.apply(frus_core::Point::new(10.0, 20.0));
        assert!((bottom.y - 220.0).abs() < 1e-4, "{bottom:?}");
        assert!(top.y < 20.0, "{top:?}");
    }
}

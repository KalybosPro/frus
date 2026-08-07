//! Typed interpolation: a `[0,1]` progress value drives any interpolable value —
//! a number, a colour, a point, a size.

use super::controller::{AnimationController, Status};
use super::curve::Curve;
use crate::{Alignment, BorderRadius, Color, Insets, Point, Size};

/// A linearly interpolable value.
pub trait Lerp: Copy {
    /// Interpolates from `self` (at `t=0`) to `other` (at `t=1`).
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for Color {
    fn lerp(self, other: Self, t: f32) -> Self {
        Color::lerp(self, other, t)
    }
}

impl Lerp for Point {
    fn lerp(self, other: Self, t: f32) -> Self {
        Point::new(self.x.lerp(other.x, t), self.y.lerp(other.y, t))
    }
}

impl Lerp for Size {
    fn lerp(self, other: Self, t: f32) -> Self {
        Size::new(
            self.width.lerp(other.width, t),
            self.height.lerp(other.height, t),
        )
    }
}

impl Lerp for Insets {
    fn lerp(self, other: Self, t: f32) -> Self {
        Insets::new(
            self.top.lerp(other.top, t),
            self.right.lerp(other.right, t),
            self.bottom.lerp(other.bottom, t),
            self.left.lerp(other.left, t),
        )
    }
}

impl Lerp for BorderRadius {
    fn lerp(self, other: Self, t: f32) -> Self {
        BorderRadius {
            top_left: self.top_left.lerp(other.top_left, t),
            top_right: self.top_right.lerp(other.top_right, t),
            bottom_right: self.bottom_right.lerp(other.bottom_right, t),
            bottom_left: self.bottom_left.lerp(other.bottom_left, t),
        }
    }
}

impl Lerp for Alignment {
    fn lerp(self, other: Self, t: f32) -> Self {
        Alignment::new(self.x.lerp(other.x, t), self.y.lerp(other.y, t))
    }
}

/// Interpolates a value between two bounds, driven by a `[0,1]` progress value.
///
/// A single `[0,1]` driver — a controller, a spring — can therefore animate any
/// number of typed values, each with its own bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween<T> {
    /// The value at `t = 0`.
    pub begin: T,
    /// The value at `t = 1`.
    pub end: T,
}

impl<T: Lerp> Tween<T> {
    /// Creates a tween from `begin` to `end`.
    pub fn new(begin: T, end: T) -> Self {
        Self { begin, end }
    }

    /// The value at progress `t`, usually coming from a [`super::Curve`].
    pub fn eval(&self, t: f32) -> T {
        self.begin.lerp(self.end, t)
    }
}

/// A value that can be **shaped** by a `[0,1]` progress value — the abstraction
/// tweens and curves share. It is the bridge between the *explicit* side (an
/// [`AnimationController`] producing a `[0,1]` frame by frame) and a *typed* value:
/// `tween.animate(&controller).value()` reads the current colour, size or point at
/// any instant, without the view knowing the controller as anything other than
/// that value.
pub trait Animatable {
    /// The type of value produced (colour, size, number).
    type Output;

    /// The value at progress `t ∈ [0,1]`.
    fn evaluate(&self, t: f32) -> Self::Output;

    /// Chains a [`Curve`] **before** evaluation: `t` is shaped by the curve first,
    /// so a single linear progress value can drive a value with non-linear timing.
    fn curved(self, curve: Curve) -> Curved<Self>
    where
        Self: Sized,
    {
        Curved { inner: self, curve }
    }

    /// Binds this animation to an [`AnimationController`]'s progress, producing an
    /// [`Animation`] whose `value()` follows the controller's current value. The
    /// controller's value is **normalised** by its own bounds, so a non-unit
    /// controller still drives a full `[0,1]`.
    fn animate<'a>(&'a self, controller: &'a AnimationController) -> Animation<'a, Self>
    where
        Self: Sized,
    {
        Animation {
            animatable: self,
            controller,
        }
    }
}

impl<T: Lerp> Animatable for Tween<T> {
    type Output = T;

    fn evaluate(&self, t: f32) -> T {
        self.eval(t)
    }
}

/// An [`Animatable`] whose progress is shaped by a [`Curve`] first — the result of
/// [`Animatable::curved`].
#[derive(Clone, Debug)]
pub struct Curved<A> {
    inner: A,
    curve: Curve,
}

impl<A: Animatable> Animatable for Curved<A> {
    type Output = A::Output;

    fn evaluate(&self, t: f32) -> Self::Output {
        self.inner.evaluate(self.curve.transform(t))
    }
}

/// A **live** typed value: an [`Animatable`] bound to an [`AnimationController`]
/// (the result of [`Animatable::animate`]). `value()` samples the controller at the
/// present instant — this is what the view reads at paint time.
pub struct Animation<'a, A: Animatable> {
    animatable: &'a A,
    controller: &'a AnimationController,
}

impl<A: Animatable> Animation<'_, A> {
    /// The current typed value: the controller's normalised progress, evaluated.
    pub fn value(&self) -> A::Output {
        let (lower, upper) = self.controller.bounds();
        let t = if upper > lower {
            (self.controller.value() - lower) / (upper - lower)
        } else {
            0.0
        };
        self.animatable.evaluate(t.clamp(0.0, 1.0))
    }

    /// The underlying controller's status.
    pub fn status(&self) -> Status {
        self.controller.status()
    }

    /// `true` while the underlying controller is still animating.
    pub fn is_animating(&self) -> bool {
        self.controller.is_animating()
    }
}

/// A run of [`Animatable`] segments chained along the `[0,1]` progress value: each
/// segment gets a **share** proportional to its weight. One progress value can thus
/// cross several stages — a multi-step morph (colour A → B → C), a bounce (grow
/// then come back), or a sequence with distinct rhythms (one segment `.curved`, the
/// other linear).
///
/// `TweenSequence` is **itself** an `Animatable`: it can be `.curved()` and
/// `.animate()`d like any other tween.
pub struct TweenSequence<T> {
    /// `(segment, weight)`. Always at least one, guaranteed by [`new`](Self::new).
    items: Vec<(Box<dyn Animatable<Output = T>>, f32)>,
    total_weight: f32,
}

impl<T> TweenSequence<T> {
    /// Starts a run with its first segment and weight; a negative weight is clamped
    /// to zero.
    pub fn new(first: impl Animatable<Output = T> + 'static, weight: f32) -> Self {
        let w = weight.max(0.0);
        Self {
            items: vec![(Box::new(first), w)],
            total_weight: w,
        }
    }

    /// Chains one more segment, taking up `weight` of the total progress.
    pub fn then(mut self, next: impl Animatable<Output = T> + 'static, weight: f32) -> Self {
        let w = weight.max(0.0);
        self.total_weight += w;
        self.items.push((Box::new(next), w));
        self
    }
}

impl<T> Animatable for TweenSequence<T> {
    type Output = T;

    fn evaluate(&self, t: f32) -> T {
        let t = t.clamp(0.0, 1.0);
        let last = self.items.len() - 1;
        // All weights zero: no partition is possible, so use the last segment.
        if self.total_weight <= 0.0 {
            return self.items[last].0.evaluate(t);
        }
        let target = t * self.total_weight;
        let mut acc = 0.0;
        for (i, (seg, w)) in self.items.iter().enumerate() {
            // The last segment catches the remainder, which is rounding-proof.
            if i == last || target <= acc + *w {
                let local = if *w > 0.0 { (target - acc) / *w } else { 0.0 };
                return seg.evaluate(local.clamp(0.0, 1.0));
            }
            acc += *w;
        }
        unreachable!("le dernier segment attrape toujours")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_tween_interpolates() {
        let t = Tween::new(10.0, 20.0);
        assert_eq!(t.eval(0.0), 10.0);
        assert_eq!(t.eval(0.5), 15.0);
        assert_eq!(t.eval(1.0), 20.0);
    }

    #[test]
    fn color_tween_interpolates() {
        let t = Tween::new(Color::BLACK, Color::WHITE);
        let mid = t.eval(0.5);
        assert_eq!(mid, Color::rgb(0.5, 0.5, 0.5));
    }

    #[test]
    fn point_tween_interpolates() {
        let t = Tween::new(Point::new(0.0, 0.0), Point::new(10.0, 20.0));
        assert_eq!(t.eval(0.5), Point::new(5.0, 10.0));
    }

    /// `tween.animate(&controller)`: the typed value follows the controller — at rest
    /// at the bottom it is `begin`, then `end` once the animation has finished.
    #[test]
    fn animate_follows_controller() {
        let mut ctrl = AnimationController::unit();
        let tween = Tween::new(Size::new(100.0, 40.0), Size::new(200.0, 80.0));
        assert_eq!(tween.animate(&ctrl).value(), Size::new(100.0, 40.0));
        assert_eq!(tween.animate(&ctrl).status(), Status::Dismissed);

        ctrl.forward(0.2, Curve::Linear);
        while ctrl.tick(0.016) {}
        assert_eq!(tween.animate(&ctrl).value(), Size::new(200.0, 80.0));
        assert_eq!(tween.animate(&ctrl).status(), Status::Completed);
    }

    /// `.curved(...)` shapes the progress before evaluation: half way through an
    /// `ease_in`, the value sits **below** the linear midpoint.
    #[test]
    fn curved_reshapes_progression() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.5);
        let linear = Tween::new(0.0f32, 100.0);
        let eased = linear.curved(Curve::ease_in());
        let mid = eased.animate(&ctrl).value();
        assert!(mid < 50.0, "ease_in stays below the linear midpoint: {mid}");
        assert!(mid > 0.0);
        // The bounds are still reached, to the Bézier solver's tolerance.
        ctrl.set_value(0.0);
        assert!(eased.animate(&ctrl).value().abs() < 0.5);
        ctrl.set_value(1.0);
        assert!((eased.animate(&ctrl).value() - 100.0).abs() < 0.5);
    }

    /// A non-unit controller still drives a full `[0,1]`: the value is normalised by
    /// its bounds.
    #[test]
    fn non_unit_bounds_are_normalized() {
        let mut ctrl = AnimationController::new(0.0, 2.0);
        ctrl.set_value(1.0); // the middle of [0,2] -> t = 0.5
        let tween = Tween::new(Color::BLACK, Color::WHITE);
        assert_eq!(tween.animate(&ctrl).value(), Color::rgb(0.5, 0.5, 0.5));
    }

    #[test]
    fn alignment_tween_slides_between_anchors() {
        let slide = Tween::new(Alignment::TOP_LEFT, Alignment::BOTTOM_RIGHT);
        assert_eq!(slide.eval(0.0), Alignment::TOP_LEFT);
        assert_eq!(slide.eval(0.5), Alignment::CENTER);
        assert_eq!(slide.eval(1.0), Alignment::BOTTOM_RIGHT);
    }

    #[test]
    fn insets_and_radius_tween_interpolate() {
        let pad = Tween::new(Insets::uniform(0.0), Insets::uniform(20.0));
        assert_eq!(pad.eval(0.5), Insets::uniform(10.0));
        let radius = Tween::new(BorderRadius::uniform(4.0), BorderRadius::uniform(24.0));
        assert_eq!(radius.eval(0.5), BorderRadius::uniform(14.0));
    }

    /// A `TweenSequence` with equal weights: two segments handing over at `t = 0.5`,
    /// each traversed in full over its own half.
    #[test]
    fn tween_sequence_relays_equal_weight_segments() {
        let seq =
            TweenSequence::new(Tween::new(0.0f32, 10.0), 1.0).then(Tween::new(10.0, 30.0), 1.0);
        assert_eq!(seq.evaluate(0.0), 0.0);
        assert_eq!(seq.evaluate(0.25), 5.0); // middle of the 1st segment
        assert_eq!(seq.evaluate(0.5), 10.0); // the seam
        assert_eq!(seq.evaluate(0.75), 20.0); // middle of the 2nd segment
        assert_eq!(seq.evaluate(1.0), 30.0);
    }

    /// Unequal weights: the heavier segment takes up a larger share of the
    /// progress.
    #[test]
    fn tween_sequence_honors_weights() {
        // 3 shares for the 1st, 1 for the 2nd -> the seam falls at t = 0.75.
        let seq =
            TweenSequence::new(Tween::new(0.0f32, 100.0), 3.0).then(Tween::new(100.0, 200.0), 1.0);
        assert_eq!(seq.evaluate(0.75), 100.0); // end of the 1st / start of the 2nd
        assert_eq!(seq.evaluate(0.375), 50.0); // middle of the 1st (0.375 / 0.75)
        assert_eq!(seq.evaluate(0.875), 150.0); // middle of the 2nd
    }

    /// The run is itself an `Animatable`, so a controller can drive it.
    #[test]
    fn tween_sequence_drives_from_controller() {
        let seq = TweenSequence::new(Tween::new(Color::BLACK, Color::WHITE), 1.0)
            .then(Tween::new(Color::WHITE, Color::BLACK), 1.0);
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.5); // the seam -> white
        assert_eq!(seq.animate(&ctrl).value(), Color::WHITE);
        ctrl.set_value(1.0); // back to black
        assert_eq!(seq.animate(&ctrl).value(), Color::BLACK);
    }
}

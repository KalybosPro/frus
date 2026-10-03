//! Easing curves: pure `[0,1] → [0,1]` functions that **shape** a linear progress
//! value — the "how" of an animation, kept separate from the "how long".
//!
//! A single linear progress `t` can drive several values, each through its own
//! curve and — thanks to [`Curve::Interval`] — over its own sub-window of time,
//! which makes staggered animations free.

/// An easing curve remapping `[0,1] → [0,1]`.
///
/// `f(0) = 0` and `f(1) = 1` hold for every variant.
#[derive(Clone, Debug, PartialEq)]
pub enum Curve {
    /// The identity: `f(t) = t`.
    Linear,
    /// A quadratic ease-out, `f(t) = 1 − (1 − t)²`: fast at once, then coasting to
    /// a stop. It is the shape of something *arriving* — a glow flaring at an edge,
    /// a value settling — where the interesting part is the start.
    Decelerate,
    /// A cubic Bézier defined by its two control points `(x1,y1)` and `(x2,y2)` —
    /// the same parameterisation as CSS's `cubic-bezier()`.
    Cubic {
        /// The first control point's x coordinate.
        x1: f32,
        /// The first control point's y coordinate.
        y1: f32,
        /// The second control point's x coordinate.
        x2: f32,
        /// The second control point's y coordinate.
        y2: f32,
    },
    /// The step response of a **critically** damped spring: starts at rest, settles
    /// gently with no overshoot, renormalised so that `f(1) = 1`. `omega` sets how
    /// lively it feels.
    CriticalSpring {
        /// The spring's natural frequency, in radians per second: higher settles faster.
        omega: f32,
    },
    /// Applies `inner` only over the sub-window `[begin, end]`: `0` before, `1`
    /// after. This is what unlocks staggered animations.
    Interval {
        /// The progress at which `inner` starts, as a fraction of `[0,1]`.
        begin: f32,
        /// The progress at which `inner` reaches `1`, as a fraction of `[0,1]`.
        end: f32,
        /// The curve applied within `[begin, end]`.
        inner: Box<Curve>,
    },
    /// Mirrored: `f(t) = 1 − inner(1 − t)`, which turns an *ease-in* into an
    /// *ease-out*.
    Flipped(Box<Curve>),
    /// Two cubic Béziers joined at `midpoint`, each scaled into its own part of the
    /// unit square: the first runs from `(0, 0)` to `midpoint` with control points `a1`
    /// and `b1`, the second from `midpoint` to `(1, 1)` with `a2` and `b2` (milestone
    /// 619). The shape of the reference's emphasized motion, which one cubic cannot
    /// draw: a slow start, a fast middle and a long settle.
    ThreePointCubic {
        /// The first segment's first control point.
        a1: [f32; 2],
        /// The first segment's second control point.
        b1: [f32; 2],
        /// Where the two segments meet.
        midpoint: [f32; 2],
        /// The second segment's first control point.
        a2: [f32; 2],
        /// The second segment's second control point.
        b2: [f32; 2],
    },
}

impl Curve {
    /// The web's `ease` (`cubic-bezier(0.25, 0.1, 0.25, 1.0)`).
    pub fn ease() -> Curve {
        Curve::Cubic {
            x1: 0.25,
            y1: 0.1,
            x2: 0.25,
            y2: 1.0,
        }
    }

    /// `ease-in`: starts slowly.
    pub fn ease_in() -> Curve {
        Curve::Cubic {
            x1: 0.42,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        }
    }

    /// `ease-out`: finishes slowly.
    pub fn ease_out() -> Curve {
        Curve::Cubic {
            x1: 0.0,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0,
        }
    }

    /// `ease-in-out`: slow at both ends.
    pub fn ease_in_out() -> Curve {
        Curve::Cubic {
            x1: 0.42,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0,
        }
    }

    /// The reference's **emphasized** easing, `easeInOutCubicEmphasized`: Material 3's
    /// motion for things that move across the screen — a page arriving, a sheet rising.
    pub fn ease_in_out_cubic_emphasized() -> Curve {
        Curve::ThreePointCubic {
            a1: [0.05, 0.0],
            b1: [0.133333, 0.06],
            midpoint: [0.166666, 0.4],
            a2: [0.208333, 0.82],
            b2: [0.25, 1.0],
        }
    }

    /// The reference's `fastEaseInToSlowEaseOut`: an iOS page sliding in.
    pub fn fast_ease_in_to_slow_ease_out() -> Curve {
        Curve::ThreePointCubic {
            a1: [0.056, 0.024],
            b1: [0.108, 0.3085],
            midpoint: [0.198, 0.541],
            a2: [0.3655, 1.0],
            b2: [0.5465, 0.989],
        }
    }

    /// The reference's `linearToEaseOut`, `cubic-bezier(0.35, 0.91, 0.33, 0.97)`: an
    /// iOS page leaving under the one that arrives.
    pub fn linear_to_ease_out() -> Curve {
        Curve::Cubic {
            x1: 0.35,
            y1: 0.91,
            x2: 0.33,
            y2: 0.97,
        }
    }

    /// The reference's `easeInToLinear`, `cubic-bezier(0.67, 0.03, 0.65, 0.09)`: the
    /// same page coming back.
    pub fn ease_in_to_linear() -> Curve {
        Curve::Cubic {
            x1: 0.67,
            y1: 0.03,
            x2: 0.65,
            y2: 0.09,
        }
    }

    /// The framework's default spring curve (`omega = 8`): the feel of screen and
    /// sheet transitions, in closed form.
    pub fn critical_spring() -> Curve {
        Curve::CriticalSpring { omega: 8.0 }
    }

    /// Evaluates the curve at `t`, clamped to `[0,1]`.
    pub fn transform(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Curve::Linear => t,
            Curve::Decelerate => {
                let remaining = 1.0 - t;
                1.0 - remaining * remaining
            }
            Curve::Cubic { x1, y1, x2, y2 } => cubic_bezier(*x1, *y1, *x2, *y2, t),
            Curve::CriticalSpring { omega } => {
                // y(tau) = 1 - e^{-omega*tau}(1 + omega*tau), renormalised by y(1).
                let resp = |x: f32| 1.0 - (-omega * x).exp() * (1.0 + omega * x);
                let end = resp(1.0);
                if end == 0.0 {
                    t
                } else {
                    resp(t) / end
                }
            }
            Curve::Interval { begin, end, inner } => {
                if t <= *begin {
                    0.0
                } else if t >= *end {
                    1.0
                } else {
                    inner.transform((t - begin) / (end - begin))
                }
            }
            Curve::Flipped(inner) => 1.0 - inner.transform(1.0 - t),
            Curve::ThreePointCubic {
                a1,
                b1,
                midpoint,
                a2,
                b2,
            } => {
                // Exactly as the reference scales its two segments
                // (`curves.dart:496`): each one is a unit cubic stretched over its own
                // part of the square.
                // The ends are the ends, as every curve's are in the reference
                // (`Curve.transform`); it also keeps a midpoint on the square's edge
                // from dividing by nothing.
                if t <= 0.0 || t >= 1.0 {
                    return t;
                }
                let [mx, my] = *midpoint;
                if t < mx {
                    cubic_bezier(a1[0] / mx, a1[1] / my, b1[0] / mx, b1[1] / my, t / mx) * my
                } else {
                    let (sx, sy) = (1.0 - mx, 1.0 - my);
                    cubic_bezier(
                        (a2[0] - mx) / sx,
                        (a2[1] - my) / sy,
                        (b2[0] - mx) / sx,
                        (b2[1] - my) / sy,
                        (t - mx) / sx,
                    ) * sy
                        + my
                }
            }
        }
    }
}

/// Evaluates the `y` coordinate of a cubic Bézier `cubic-bezier(x1, y1, x2, y2)`
/// for an abscissa `t ∈ [0,1]`. The anchor points are `(0,0)` and `(1,1)`; `t` is
/// the **x** coordinate wanted, so we look for the parameter `s` such that
/// `Bx(s) = t`, then return `By(s)`.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    // A 1D cubic Bézier: anchors 0 and 1, controls p1 and p2.
    let bezier = |p1: f32, p2: f32, s: f32| {
        let inv = 1.0 - s;
        // 3(1-s)^2*s*p1 + 3(1-s)*s^2*p2 + s^3
        3.0 * inv * inv * s * p1 + 3.0 * inv * s * s * p2 + s * s * s
    };
    // Binary search for the parameter `s` giving `Bx(s) = t`. Bx is monotonic as
    // long as the x control points lie in [0,1].
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    let mut s = t;
    for _ in 0..24 {
        s = 0.5 * (lo + hi);
        let x = bezier(x1, x2, s);
        if (x - t).abs() < 1e-5 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
    }
    bezier(y1, y2, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The reference's three-point curves** (milestone 619): each passes through its
    /// midpoint exactly, keeps its ends, never runs backwards, and gives the reference's
    /// values along the way.
    #[test]
    fn the_three_point_curves_meet_at_their_midpoints() {
        let emphasized = Curve::ease_in_out_cubic_emphasized();
        let ios = Curve::fast_ease_in_to_slow_ease_out();
        for (curve, mx, my) in [(&emphasized, 0.166666, 0.4), (&ios, 0.198, 0.541)] {
            endpoints(curve);
            assert!((curve.transform(mx) - my).abs() < 1e-3, "{curve:?} at {mx}");
            let mut last = 0.0;
            for i in 0..=200 {
                let y = curve.transform(i as f32 / 200.0);
                assert!(y + 1e-4 >= last, "{curve:?} runs back at {i}");
                last = y;
            }
        }
        // And the reference's values, computed by its own bisection, which stops within a
        // thousandth.
        for (curve, table) in [
            (&emphasized, [0.0937, 0.7715, 0.9508, 0.9914]),
            (&ios, [0.2386, 0.6668, 0.9423, 0.9916]),
        ] {
            for (t, want) in [0.1, 0.25, 0.5, 0.75].into_iter().zip(table) {
                let got = curve.transform(t);
                assert!(
                    (got - want).abs() < 5e-3,
                    "{curve:?} at {t}: {got} vs {want}"
                );
            }
        }
    }

    /// **The iOS pair**: `linearToEaseOut` is ahead of linear, `easeInToLinear` behind it,
    /// each by the reference's margin.
    #[test]
    fn the_ios_cubics_lead_and_lag() {
        let lead = Curve::linear_to_ease_out();
        let lag = Curve::ease_in_to_linear();
        endpoints(&lead);
        endpoints(&lag);
        for t in [0.25, 0.5, 0.75] {
            assert!(lead.transform(t) > t && lag.transform(t) < t, "at {t}");
        }
        // And by as much as the reference's: its own evaluation, within a few thousandths.
        for (curve, table) in [
            (&lead, [0.6237, 0.9198, 0.9826]),
            (&lag, [0.0174, 0.0802, 0.3763]),
        ] {
            for (t, want) in [0.25, 0.5, 0.75].into_iter().zip(table) {
                let got = curve.transform(t);
                assert!(
                    (got - want).abs() < 5e-3,
                    "{curve:?} at {t}: {got} vs {want}"
                );
            }
        }
    }

    fn endpoints(curve: &Curve) {
        assert!(curve.transform(0.0).abs() < 1e-4, "f(0) != 0: {:?}", curve);
        assert!(
            (curve.transform(1.0) - 1.0).abs() < 1e-4,
            "f(1) != 1: {:?}",
            curve
        );
    }

    #[test]
    fn all_curves_hit_endpoints() {
        endpoints(&Curve::Linear);
        endpoints(&Curve::ease());
        endpoints(&Curve::ease_in());
        endpoints(&Curve::ease_out());
        endpoints(&Curve::ease_in_out());
        endpoints(&Curve::critical_spring());
    }

    #[test]
    fn linear_is_identity() {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((Curve::Linear.transform(t) - t).abs() < 1e-6);
        }
    }

    #[test]
    fn ease_in_starts_slow() {
        // Half way through, an ease-in sits below the diagonal.
        assert!(Curve::ease_in().transform(0.5) < 0.5);
        // An ease-out sits above it.
        assert!(Curve::ease_out().transform(0.5) > 0.5);
    }

    #[test]
    fn curves_are_monotonic() {
        for curve in [
            Curve::ease(),
            Curve::ease_in(),
            Curve::ease_out(),
            Curve::ease_in_out(),
            Curve::critical_spring(),
        ] {
            let mut prev = 0.0;
            for i in 0..=100 {
                let v = curve.transform(i as f32 / 100.0);
                assert!(v >= prev - 1e-4, "{:?} decreases at {i}", curve);
                assert!(v <= 1.0 + 1e-4, "{:?} exceeds 1 at {i}", curve);
                prev = v;
            }
        }
    }

    #[test]
    fn interval_gates_the_subwindow() {
        let staggered = Curve::Interval {
            begin: 0.5,
            end: 1.0,
            inner: Box::new(Curve::Linear),
        };
        assert_eq!(staggered.transform(0.25), 0.0, "still before begin");
        assert!(
            (staggered.transform(0.75) - 0.5).abs() < 1e-4,
            "half way at the middle of the window"
        );
        assert_eq!(staggered.transform(1.0), 1.0);
    }

    #[test]
    fn flipped_mirrors() {
        let flipped = Curve::Flipped(Box::new(Curve::ease_in()));
        // A flipped ease-in behaves like an ease-out: above the diagonal.
        assert!(flipped.transform(0.5) > 0.5);
        endpoints(&flipped);
    }

    #[test]
    fn critical_spring_matches_reference_shape() {
        // Well advanced at the midpoint (it settles gently), bounded, no overshoot.
        let c = Curve::critical_spring();
        assert!(c.transform(0.5) > 0.7);
        assert!(c.transform(1.0) <= 1.0 + 1e-6);
    }
}

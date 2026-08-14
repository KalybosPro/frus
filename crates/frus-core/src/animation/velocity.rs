//! Estimating how fast a pointer was moving when it let go.
//!
//! This is the **input** to every ballistic motion in the framework: a fling's
//! distance, whether a swipe dismisses a route, how far a pan coasts. Everything
//! downstream — the scroll physics, the navigation spring — is only as good as
//! this number.
//!
//! The naive answer, and the one frus used until now, is to smooth successive
//! deltas: `v = v/2 + instant/2`. It is cheap and it is wrong in the way that
//! matters. A finger does not move at a constant speed; it accelerates, wobbles,
//! and — the case that ruins the estimate — often **slows down just before
//! lifting**. An exponential average weights the last sample heavily, so it reads
//! that deceleration as the gesture's speed and the content barely moves. The
//! user's thumb says "throw"; the framework hears "nudge".
//!
//! The answer here is the one mature toolkits converged on: keep a short history
//! of positions and **fit a curve through it**, taking the fit's derivative at the
//! release as the velocity. A quadratic least-squares regression over the last
//! 100 ms sees the acceleration, so a last-instant slowdown moves the estimate a
//! little instead of dominating it.
//!
//! Some platforms do something different for scroll flings — a weighted average of
//! the last three sample-to-sample velocities, which deliberately leans on the
//! *older* samples. [`VelocityStrategy`] carries both, and
//! [`VelocityTracker::platform_default`] picks the one the running platform
//! expects, exactly as [`crate::animation::simulation`]'s scroll physics does.
//!
//! Everything here is pure: samples come in stamped with a time, and the estimate
//! is a function of the history. No clock is read.

use crate::geometry::Point;

/// A pointer speed, in logical pixels per second.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

impl Velocity {
    /// Not moving.
    pub const ZERO: Velocity = Velocity { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The speed, regardless of direction.
    pub fn magnitude(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// The same direction, with the speed brought into `[min, max]`.
    ///
    /// A gesture below `min` is *sped up* to it rather than dropped: callers that
    /// want to drop it test the magnitude first.
    pub fn clamp_magnitude(self, min: f32, max: f32) -> Velocity {
        debug_assert!(min >= 0.0 && max >= min);
        let magnitude = self.magnitude();
        if magnitude == 0.0 {
            return self;
        }
        let target = magnitude.clamp(min, max);
        if target == magnitude {
            return self;
        }
        let scale = target / magnitude;
        Velocity::new(self.x * scale, self.y * scale)
    }
}

/// A velocity, with what the tracker knows about how much to trust it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelocityEstimate {
    /// The estimated speed at the last sample, in px/s.
    pub velocity: Velocity,
    /// How much of the samples' variance the fit explains, in `0..=1` (an
    /// r-squared). `1.0` for the strategies that do not fit anything.
    pub confidence: f32,
    /// The time the samples used span, in seconds.
    pub duration: f32,
    /// The total displacement over those samples, in px. A gesture that was fast
    /// but covered no ground is a twitch, not a fling — which is why this travels
    /// with the velocity.
    pub offset: (f32, f32),
}

impl VelocityEstimate {
    /// An estimate that says "not moving", with nothing to doubt about it.
    pub const STILL: VelocityEstimate = VelocityEstimate {
        velocity: Velocity::ZERO,
        confidence: 1.0,
        duration: 0.0,
        offset: (0.0, 0.0),
    };
}

/// How many samples are kept. At a 120 Hz sampling rate this is ~166 ms of
/// history, comfortably more than the horizon below.
const HISTORY: usize = 20;
/// Samples older than this (in ms) are not part of the same motion.
const HORIZON_MS: f32 = 100.0;
/// A gap longer than this (in ms) means the finger stopped: the history before it
/// belongs to a different gesture, and a release after it is not a fling.
const STOPPED_MS: f32 = 40.0;
/// Below this many samples there is nothing to regress.
const MIN_SAMPLES: usize = 3;

/// The weights a platform that bounces applies to the last three sample-to-sample
/// velocities, oldest first.
pub const BOUNCING_FLING_WEIGHTS: [f32; 3] = [0.6, 0.35, 0.05];
/// The same, for the desktop variant of that platform.
pub const DESKTOP_FLING_WEIGHTS: [f32; 3] = [0.15, 0.65, 0.2];

/// How a [`VelocityTracker`] turns a history of positions into a velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VelocityStrategy {
    /// A quadratic least-squares regression over the samples inside the horizon,
    /// with the velocity read off as the fit's first-order coefficient. Sees
    /// acceleration; robust to a noisy or decelerating tail.
    Regression,
    /// A weighted average of the last three sample-to-sample velocities, oldest
    /// first. Cheap, and deliberately biased towards the *older* samples — which
    /// is how the platforms that bounce read a fling.
    RecentAverage([f32; 3]),
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    point: Point,
    /// Seconds, on whatever clock the caller is stamping with.
    time: f32,
}

/// A rolling history of pointer positions that can be asked for a velocity.
///
/// Adding a position is cheap; asking for the estimate is not free (the
/// regression strategy solves a small system), so ask once, at the release.
#[derive(Clone, Debug)]
pub struct VelocityTracker {
    strategy: VelocityStrategy,
    samples: [Option<Sample>; HISTORY],
    /// Where the newest sample sits in the ring.
    index: usize,
}

impl Default for VelocityTracker {
    fn default() -> Self {
        Self::platform_default()
    }
}

impl VelocityTracker {
    /// A tracker using `strategy`.
    pub fn new(strategy: VelocityStrategy) -> Self {
        Self {
            strategy,
            samples: [None; HISTORY],
            index: 0,
        }
    }

    /// The strategy the running platform reads flings with.
    ///
    /// Resolved at compile time from the target, like the scroll physics it feeds:
    /// a build is for one platform.
    pub fn platform_default() -> Self {
        #[cfg(target_os = "ios")]
        let strategy = VelocityStrategy::RecentAverage(BOUNCING_FLING_WEIGHTS);
        #[cfg(target_os = "macos")]
        let strategy = VelocityStrategy::RecentAverage(DESKTOP_FLING_WEIGHTS);
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        let strategy = VelocityStrategy::Regression;
        Self::new(strategy)
    }

    /// The strategy in force.
    pub fn strategy(&self) -> VelocityStrategy {
        self.strategy
    }

    /// Records where the pointer was, and when (in seconds).
    pub fn add_position(&mut self, time: f32, point: Point) {
        self.index = (self.index + 1) % HISTORY;
        self.samples[self.index] = Some(Sample { point, time });
    }

    /// Forgets everything — a new gesture starts from nothing.
    pub fn clear(&mut self) {
        self.samples = [None; HISTORY];
        self.index = 0;
    }

    /// The estimate as of `now` (seconds, same clock as the samples), or `None`
    /// when not a single position has been recorded.
    pub fn estimate(&self, now: f32) -> Option<VelocityEstimate> {
        let newest = self.samples[self.index]?;
        // The finger came to rest before letting go. Whatever it was doing 100 ms
        // ago, it is not throwing anything now.
        if (now - newest.time) * 1000.0 > STOPPED_MS {
            return Some(VelocityEstimate::STILL);
        }
        match self.strategy {
            VelocityStrategy::Regression => Some(self.regress(newest)),
            VelocityStrategy::RecentAverage(weights) => Some(self.recent_average(newest, weights)),
        }
    }

    /// The estimated velocity as of `now`, or [`Velocity::ZERO`] if there is
    /// nothing to go on.
    pub fn velocity(&self, now: f32) -> Velocity {
        self.estimate(now)
            .map(|e| e.velocity)
            .unwrap_or(Velocity::ZERO)
    }

    /// Fits a quadratic through the samples that belong to the current motion and
    /// reads the velocity off its first-order term.
    fn regress(&self, newest: Sample) -> VelocityEstimate {
        // Ages are in **milliseconds**, negative and increasing backwards: keeping
        // the abscissae small and centred on zero is what keeps the fit stable.
        let mut ages = [0.0f64; HISTORY];
        let mut xs = [0.0f64; HISTORY];
        let mut ys = [0.0f64; HISTORY];
        let mut count = 0;
        let mut oldest = newest;
        let mut previous = newest;
        let mut index = self.index;

        while count < HISTORY {
            let Some(sample) = self.samples[index] else {
                break;
            };
            let age = (newest.time - sample.time) * 1000.0;
            let gap = ((sample.time - previous.time) * 1000.0).abs();
            previous = sample;
            // Too old to be this motion, or separated from it by a pause.
            if age > HORIZON_MS || gap > STOPPED_MS {
                break;
            }
            oldest = sample;
            ages[count] = -age as f64;
            xs[count] = sample.point.x as f64;
            ys[count] = sample.point.y as f64;
            count += 1;
            index = if index == 0 { HISTORY } else { index } - 1;
        }

        let duration = newest.time - oldest.time;
        let offset = (
            newest.point.x - oldest.point.x,
            newest.point.y - oldest.point.y,
        );
        if count >= MIN_SAMPLES {
            let x_fit = PolynomialFit::solve(&ages[..count], &xs[..count], 2);
            let y_fit = PolynomialFit::solve(&ages[..count], &ys[..count], 2);
            if let (Some(x_fit), Some(y_fit)) = (x_fit, y_fit) {
                return VelocityEstimate {
                    // px/ms → px/s.
                    velocity: Velocity::new(
                        (x_fit.coefficients[1] * 1000.0) as f32,
                        (y_fit.coefficients[1] * 1000.0) as f32,
                    ),
                    confidence: (x_fit.confidence * y_fit.confidence) as f32,
                    duration,
                    offset,
                };
            }
        }
        // Not enough to fit, but we did see the pointer: report the travel, no
        // speed. A caller gating on distance can still tell what happened.
        VelocityEstimate {
            velocity: Velocity::ZERO,
            confidence: 1.0,
            duration,
            offset,
        }
    }

    /// The velocity between the two samples `offset` steps back from the newest.
    fn pair_velocity(&self, offset: isize) -> Velocity {
        let size = HISTORY as isize;
        let end = self.samples[((self.index as isize + offset).rem_euclid(size)) as usize];
        let start = self.samples[((self.index as isize + offset - 1).rem_euclid(size)) as usize];
        let (Some(end), Some(start)) = (end, start) else {
            return Velocity::ZERO;
        };
        let dt = end.time - start.time;
        if dt <= 0.0 {
            return Velocity::ZERO;
        }
        Velocity::new(
            (end.point.x - start.point.x) / dt,
            (end.point.y - start.point.y) / dt,
        )
    }

    /// A weighted average of the last three pair velocities, oldest first.
    fn recent_average(&self, newest: Sample, weights: [f32; 3]) -> VelocityEstimate {
        let mut velocity = Velocity::ZERO;
        for (i, weight) in weights.iter().enumerate() {
            let pair = self.pair_velocity(i as isize - (weights.len() as isize - 1));
            velocity.x += pair.x * weight;
            velocity.y += pair.y * weight;
        }
        // The span is measured over the whole history, not the three pairs: it is
        // there to tell a fling from a twitch, and needs the real travel.
        let mut oldest = newest;
        for i in 1..=HISTORY {
            if let Some(sample) = self.samples[(self.index + i) % HISTORY] {
                oldest = sample;
                break;
            }
        }
        VelocityEstimate {
            velocity,
            confidence: 1.0,
            duration: newest.time - oldest.time,
            offset: (
                newest.point.x - oldest.point.x,
                newest.point.y - oldest.point.y,
            ),
        }
    }
}

/// A polynomial fitted to a set of weighted points, and how well it fits.
///
/// Only what the velocity estimate needs: degree ≤ 2, so three coefficients.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolynomialFit {
    /// `coefficients[i]` multiplies `xⁱ`.
    pub coefficients: [f64; 3],
    /// The fraction of the data's variance the fit explains (an r-squared), in
    /// `0..=1`. Higher is better.
    pub confidence: f64,
}

/// Below this, a basis vector counts as degenerate and the system has no solution.
const PRECISION_TOLERANCE: f64 = 1e-10;

impl PolynomialFit {
    /// Fits a polynomial of `degree` (≤ 2) to `(x, y)` by least squares, all points
    /// weighted equally. `None` when there is not enough data, or when the points
    /// are degenerate (all at the same instant, say).
    ///
    /// The method is a QR decomposition by Gram–Schmidt, then back-substitution —
    /// the standard approach, and numerically far better behaved than forming and
    /// inverting the normal equations. It runs in `f64` even though frus is `f32`
    /// throughout: orthogonalising powers of the abscissae loses precision quickly,
    /// and this is the number every fling depends on.
    pub fn solve(x: &[f64], y: &[f64], degree: usize) -> Option<PolynomialFit> {
        debug_assert_eq!(x.len(), y.len());
        debug_assert!(degree <= 2);
        let m = x.len();
        let n = degree + 1;
        if degree > m {
            return None; // not enough data to fit a curve
        }

        // A: the Vandermonde matrix of the abscissae, n rows of m columns.
        let mut a = vec![0.0f64; n * m];
        for h in 0..m {
            a[h] = 1.0;
            for i in 1..n {
                a[i * m + h] = a[(i - 1) * m + h] * x[h];
            }
        }

        // Gram–Schmidt: Q is the orthonormal basis, R the upper triangle.
        let mut q = vec![0.0f64; n * m];
        let mut r = vec![0.0f64; n * n];
        for j in 0..n {
            for h in 0..m {
                q[j * m + h] = a[j * m + h];
            }
            for i in 0..j {
                let dot: f64 = (0..m).map(|h| q[j * m + h] * q[i * m + h]).sum();
                for h in 0..m {
                    q[j * m + h] -= dot * q[i * m + h];
                }
            }
            let norm = (0..m)
                .map(|h| q[j * m + h] * q[j * m + h])
                .sum::<f64>()
                .sqrt();
            if norm < PRECISION_TOLERANCE {
                return None; // linearly dependent or zero: no solution
            }
            let inverse = 1.0 / norm;
            for h in 0..m {
                q[j * m + h] *= inverse;
            }
            for i in 0..n {
                r[j * n + i] = if i < j {
                    0.0
                } else {
                    (0..m).map(|h| q[j * m + h] * a[i * m + h]).sum()
                };
            }
        }

        // R·B = Qᵀ·Y, solved bottom-right to top-left since R is upper triangular.
        let mut coefficients = [0.0f64; 3];
        for i in (0..n).rev() {
            let mut c: f64 = (0..m).map(|h| q[i * m + h] * y[h]).sum();
            for j in (i + 1..n).rev() {
                c -= r[i * n + j] * coefficients[j];
            }
            coefficients[i] = c / r[i * n + i];
        }

        // The r-squared: how much of the data's spread the fit accounts for.
        let mean = y.iter().sum::<f64>() / m as f64;
        let mut squared_error = 0.0;
        let mut squared_total = 0.0;
        for h in 0..m {
            let mut term = 1.0;
            let mut error = y[h] - coefficients[0];
            for c in coefficients.iter().take(n).skip(1) {
                term *= x[h];
                error -= term * c;
            }
            squared_error += error * error;
            let deviation = y[h] - mean;
            squared_total += deviation * deviation;
        }
        let confidence = if squared_total <= PRECISION_TOLERANCE {
            1.0
        } else {
            1.0 - (squared_error / squared_total)
        };

        Some(PolynomialFit {
            coefficients,
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds a straight line at `speed` px/s, one sample every 8 ms.
    fn steady(tracker: &mut VelocityTracker, speed: f32, samples: usize) -> f32 {
        let step = 0.008;
        let mut t = 0.0;
        for i in 0..samples {
            t = i as f32 * step;
            tracker.add_position(t, Point::new(speed * t, 2.0 * speed * t));
        }
        t
    }

    /// Feeds one sample every 8 ms, moving by each of `deltas` in turn (plus the
    /// resting sample the first delta moves away from). Returns the last sample's
    /// time.
    ///
    /// Stating a gesture as its per-step deltas leaves no room for the off-by-one
    /// that hides when positions are accumulated in the loop that adds them — and
    /// it makes each step's speed readable: 8 px per 8 ms is 1000 px/s.
    fn feed(tracker: &mut VelocityTracker, deltas: &[f32]) -> f32 {
        let step = 0.008;
        let mut t = 0.0;
        let mut x = 0.0;
        tracker.add_position(t, Point::new(x, 0.0));
        for delta in deltas {
            t += step;
            x += delta;
            tracker.add_position(t, Point::new(x, 0.0));
        }
        t
    }

    /// A throw at 1000 px/s, then `slow` steps at 125 px/s — the finger easing off
    /// as it lifts. The gesture that an exponential average reads as a nudge.
    fn throw_then_lift(tracker: &mut VelocityTracker, slow: usize) -> f32 {
        let mut deltas = vec![8.0f32; 7];
        deltas.extend(std::iter::repeat_n(1.0f32, slow));
        feed(tracker, &deltas)
    }

    #[test]
    fn a_steady_drag_is_read_exactly() {
        let mut tracker = VelocityTracker::new(VelocityStrategy::Regression);
        let now = steady(&mut tracker, 600.0, 10);
        let estimate = tracker.estimate(now).expect("samples were added");
        assert!(
            (estimate.velocity.x - 600.0).abs() < 1.0,
            "x = {}",
            estimate.velocity.x
        );
        assert!(
            (estimate.velocity.y - 1200.0).abs() < 2.0,
            "y = {}",
            estimate.velocity.y
        );
        assert!(estimate.confidence > 0.99, "a line fits a line");
        assert!(estimate.offset.0 > 0.0 && estimate.duration > 0.0);
    }

    #[test]
    fn the_regression_survives_a_last_instant_slowdown() {
        let mut tracker = VelocityTracker::new(VelocityStrategy::Regression);
        let now = throw_then_lift(&mut tracker, 2);
        let estimate = tracker.estimate(now).expect("samples");
        // The old exponential average read ~190 px/s here (½·125 + ½·250 after two
        // slow steps) and the content barely moved. The regression sees the throw.
        assert!(
            estimate.velocity.x > 400.0,
            "the throw should survive the lift-off, got {}",
            estimate.velocity.x
        );
    }

    #[test]
    fn a_finger_that_stopped_before_lifting_flings_nothing() {
        let mut tracker = VelocityTracker::new(VelocityStrategy::Regression);
        let last = steady(&mut tracker, 900.0, 8);
        // Held still for 100 ms, well past the 40 ms threshold, then released.
        let estimate = tracker.estimate(last + 0.1).expect("samples");
        assert_eq!(estimate.velocity, Velocity::ZERO);
        assert_eq!(estimate, VelocityEstimate::STILL);
    }

    #[test]
    fn samples_older_than_the_horizon_are_not_part_of_the_motion() {
        let mut tracker = VelocityTracker::new(VelocityStrategy::Regression);
        // A first gesture, then a long pause, then a slow one. Only the slow one
        // may contribute: the pause exceeds the stop threshold.
        for i in 0..6 {
            tracker.add_position(i as f32 * 0.008, Point::new(i as f32 * 16.0, 0.0));
        }
        let base = 1.0;
        for i in 0..6 {
            tracker.add_position(
                base + i as f32 * 0.008,
                Point::new(1000.0 + i as f32 * 1.6, 0.0),
            );
        }
        let now = base + 5.0 * 0.008;
        let estimate = tracker.estimate(now).expect("samples");
        assert!(
            estimate.velocity.x < 400.0,
            "the old fast gesture leaked in: {}",
            estimate.velocity.x
        );
        assert!(estimate.duration < 0.1, "span = {}", estimate.duration);
    }

    #[test]
    fn too_few_samples_give_travel_but_no_speed() {
        let mut tracker = VelocityTracker::new(VelocityStrategy::Regression);
        tracker.add_position(0.0, Point::new(0.0, 0.0));
        tracker.add_position(0.008, Point::new(10.0, 0.0));
        let estimate = tracker.estimate(0.008).expect("samples");
        assert_eq!(
            estimate.velocity,
            Velocity::ZERO,
            "2 samples cannot regress"
        );
        assert_eq!(estimate.offset.0, 10.0, "but the travel is still known");
    }

    #[test]
    fn an_empty_tracker_has_nothing_to_say() {
        let tracker = VelocityTracker::new(VelocityStrategy::Regression);
        assert!(tracker.estimate(0.0).is_none());
        assert_eq!(tracker.velocity(0.0), Velocity::ZERO);
    }

    #[test]
    fn clearing_starts_the_gesture_over() {
        let mut tracker = VelocityTracker::new(VelocityStrategy::Regression);
        let now = steady(&mut tracker, 600.0, 10);
        tracker.clear();
        assert!(tracker.estimate(now).is_none());
    }

    #[test]
    fn the_recent_average_leans_on_the_older_samples() {
        // Same lift-off gesture. The weighted average is meant to read the throw,
        // not the lift, because most of the weight sits on the older pairs.
        let mut tracker =
            VelocityTracker::new(VelocityStrategy::RecentAverage(BOUNCING_FLING_WEIGHTS));
        let now = throw_then_lift(&mut tracker, 1);
        let estimate = tracker.estimate(now).expect("samples");
        // The newest pair is the lift and carries only 0.05:
        // 0.6·1000 + 0.35·1000 + 0.05·125 ≈ 956.
        assert!(
            (estimate.velocity.x - 956.0).abs() < 20.0,
            "got {}",
            estimate.velocity.x
        );
    }

    #[test]
    fn the_recent_average_needs_no_history_to_answer() {
        let mut tracker =
            VelocityTracker::new(VelocityStrategy::RecentAverage(DESKTOP_FLING_WEIGHTS));
        tracker.add_position(0.0, Point::new(0.0, 0.0));
        tracker.add_position(0.008, Point::new(8.0, 0.0));
        let estimate = tracker.estimate(0.008).expect("samples");
        // Only the newest pair exists; it carries the 0.2 weight. 1000·0.2 = 200.
        assert!(
            (estimate.velocity.x - 200.0).abs() < 1.0,
            "got {}",
            estimate.velocity.x
        );
    }

    #[test]
    fn the_ring_buffer_wraps_without_losing_the_motion() {
        let mut tracker = VelocityTracker::new(VelocityStrategy::Regression);
        // Far more samples than the history holds, all at the same speed.
        let now = steady(&mut tracker, 500.0, HISTORY * 3);
        let estimate = tracker.estimate(now).expect("samples");
        assert!(
            (estimate.velocity.x - 500.0).abs() < 2.0,
            "got {}",
            estimate.velocity.x
        );
    }

    #[test]
    fn a_quadratic_is_recovered_exactly() {
        // y = 3 + 2x + 4x², sampled: the fit must find those coefficients back.
        let x: Vec<f64> = (0..8).map(|i| i as f64 * 0.5).collect();
        let y: Vec<f64> = x.iter().map(|x| 3.0 + 2.0 * x + 4.0 * x * x).collect();
        let fit = PolynomialFit::solve(&x, &y, 2).expect("a well-posed system");
        assert!((fit.coefficients[0] - 3.0).abs() < 1e-6, "{:?}", fit);
        assert!((fit.coefficients[1] - 2.0).abs() < 1e-6, "{:?}", fit);
        assert!((fit.coefficients[2] - 4.0).abs() < 1e-6, "{:?}", fit);
        assert!((fit.confidence - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_degenerate_system_has_no_fit() {
        // Every point at the same abscissa: the basis vectors collapse.
        let x = [1.0, 1.0, 1.0, 1.0];
        let y = [0.0, 1.0, 2.0, 3.0];
        assert!(PolynomialFit::solve(&x, &y, 2).is_none());
    }

    #[test]
    fn confidence_falls_when_the_points_do_not_follow_the_curve() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let clean: Vec<f64> = x.iter().map(|x| 2.0 * x).collect();
        let noisy: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, x)| 2.0 * x + if i % 2 == 0 { 40.0 } else { -40.0 })
            .collect();
        let a = PolynomialFit::solve(&x, &clean, 2).unwrap();
        let b = PolynomialFit::solve(&x, &noisy, 2).unwrap();
        assert!(
            a.confidence > b.confidence,
            "{} vs {}",
            a.confidence,
            b.confidence
        );
        assert!(b.confidence < 0.9);
    }

    #[test]
    fn a_velocity_can_be_brought_into_range() {
        let v = Velocity::new(3000.0, 4000.0); // magnitude 5000
        assert!((v.magnitude() - 5000.0).abs() < 1e-3);
        let capped = v.clamp_magnitude(0.0, 1000.0);
        assert!((capped.magnitude() - 1000.0).abs() < 1e-2);
        // Direction preserved.
        assert!((capped.x / capped.y - v.x / v.y).abs() < 1e-4);
        // Inside the range, untouched.
        assert_eq!(v.clamp_magnitude(0.0, 8000.0), v);
        // Zero has no direction to preserve.
        assert_eq!(Velocity::ZERO.clamp_magnitude(50.0, 100.0), Velocity::ZERO);
    }
}

//! Vector paths: **arbitrary** 2D geometry (straight segments and Bézier curves),
//! filled and/or stroked. This is the building block for icons and for custom
//! drawing (`CustomPaint`), beyond what rectangles can express.
//!
//! A [`Path`] is purely declarative — a sequence of [`PathVerb`]. It knows nothing
//! about the GPU: `frus-gpu` tessellates it into triangles at render time.
//! Coordinates are in logical pixels, with the same conventions as [`crate::Rect`]
//! (top-left origin, Y pointing down).

use crate::{Color, Point, Rect};

/// A single drawing command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathVerb {
    /// Lift the pen and put it down again: starts a new **sub-path** at this point.
    MoveTo(Point),
    /// A straight segment from the current point to `to`.
    LineTo(Point),
    /// A **quadratic** Bézier curve (one control point).
    QuadTo { ctrl: Point, to: Point },
    /// A **cubic** Bézier curve (two control points).
    CubicTo { c1: Point, c2: Point, to: Point },
    /// Closes the current sub-path, joining it back to its starting point.
    Close,
}

impl PathVerb {
    fn scaled(self, f: f32) -> PathVerb {
        let s = |p: Point| Point::new(p.x * f, p.y * f);
        match self {
            PathVerb::MoveTo(p) => PathVerb::MoveTo(s(p)),
            PathVerb::LineTo(p) => PathVerb::LineTo(s(p)),
            PathVerb::QuadTo { ctrl, to } => PathVerb::QuadTo {
                ctrl: s(ctrl),
                to: s(to),
            },
            PathVerb::CubicTo { c1, c2, to } => PathVerb::CubicTo {
                c1: s(c1),
                c2: s(c2),
                to: s(to),
            },
            PathVerb::Close => PathVerb::Close,
        }
    }

    fn translated(self, dx: f32, dy: f32) -> PathVerb {
        let t = |p: Point| Point::new(p.x + dx, p.y + dy);
        match self {
            PathVerb::MoveTo(p) => PathVerb::MoveTo(t(p)),
            PathVerb::LineTo(p) => PathVerb::LineTo(t(p)),
            PathVerb::QuadTo { ctrl, to } => PathVerb::QuadTo {
                ctrl: t(ctrl),
                to: t(to),
            },
            PathVerb::CubicTo { c1, c2, to } => PathVerb::CubicTo {
                c1: t(c1),
                c2: t(c2),
                to: t(to),
            },
            PathVerb::Close => PathVerb::Close,
        }
    }
}

/// An **outline** (the line a path is drawn with): colour and width, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

impl Stroke {
    /// A stroke of the given colour and width.
    pub const fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
}

/// A vector path: a sequence of [`PathVerb`]. Built in *builder* style (chainable),
/// and consumed at render time.
///
/// ```
/// use frus_core::{Path, Point};
/// let triangle = Path::new()
///     .move_to(Point::new(0.0, 0.0))
///     .line_to(Point::new(10.0, 0.0))
///     .line_to(Point::new(5.0, 8.0))
///     .close();
/// assert_eq!(triangle.verbs().len(), 4);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    verbs: Vec<PathVerb>,
}

impl Path {
    /// An empty path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a sub-path at `p`.
    pub fn move_to(mut self, p: Point) -> Self {
        self.verbs.push(PathVerb::MoveTo(p));
        self
    }

    /// A straight segment to `p`.
    pub fn line_to(mut self, p: Point) -> Self {
        self.verbs.push(PathVerb::LineTo(p));
        self
    }

    /// A quadratic curve (one control point, `ctrl`) to `to`.
    pub fn quad_to(mut self, ctrl: Point, to: Point) -> Self {
        self.verbs.push(PathVerb::QuadTo { ctrl, to });
        self
    }

    /// A cubic curve (two control points, `c1` and `c2`) to `to`.
    pub fn cubic_to(mut self, c1: Point, c2: Point, to: Point) -> Self {
        self.verbs.push(PathVerb::CubicTo { c1, c2, to });
        self
    }

    /// Closes the current sub-path.
    pub fn close(mut self) -> Self {
        self.verbs.push(PathVerb::Close);
        self
    }

    /// The path's commands, in order.
    pub fn verbs(&self) -> &[PathVerb] {
        &self.verbs
    }

    /// `true` when the path holds no commands at all.
    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }

    /// A filled rectangle (a closed outline, clockwise).
    pub fn rect(r: Rect) -> Self {
        Self::new()
            .move_to(Point::new(r.x, r.y))
            .line_to(Point::new(r.x + r.width, r.y))
            .line_to(Point::new(r.x + r.width, r.y + r.height))
            .line_to(Point::new(r.x, r.y + r.height))
            .close()
    }

    /// Continues the path along a **circular arc**, from `start` to `end` (radians,
    /// measured from the positive x axis, y downwards), around `center`.
    ///
    /// The current point is assumed to be the arc's start; nothing moves it there,
    /// because an arc that jumped to its own beginning could not be part of an
    /// outline. Approximated by cubics, split so that no piece spans more than a
    /// quarter turn — which is where the constant used by [`Path::circle`] stops
    /// being accurate enough.
    pub fn arc_to(mut self, center: Point, radius: f32, start: f32, end: f32) -> Self {
        let sweep = end - start;
        if sweep.abs() < 1e-6 || radius <= 0.0 {
            return self;
        }
        let pieces = (sweep.abs() / std::f32::consts::FRAC_PI_2).ceil().max(1.0);
        let step = sweep / pieces;
        // The control-point distance for a cubic that fits a `step` arc exactly at
        // its ends and its middle. At a quarter turn this is the familiar 0.5523.
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let mut angle = start;
        for _ in 0..pieces as usize {
            let next = angle + step;
            let (s0, c0) = angle.sin_cos();
            let (s1, c1) = next.sin_cos();
            let p0 = Point::new(center.x + radius * c0, center.y + radius * s0);
            let p1 = Point::new(center.x + radius * c1, center.y + radius * s1);
            self = self.cubic_to(
                Point::new(p0.x - k * radius * s0, p0.y + k * radius * c0),
                Point::new(p1.x + k * radius * s1, p1.y - k * radius * c1),
                p1,
            );
            angle = next;
        }
        self
    }

    /// A circle, approximated by **four cubic arcs** (the Bézier constant `0.5523`,
    /// which is exact at the nodes).
    pub fn circle(center: Point, radius: f32) -> Self {
        const K: f32 = 0.552_284_75;
        let (cx, cy, r) = (center.x, center.y, radius);
        let k = r * K;
        Self::new()
            .move_to(Point::new(cx, cy - r))
            .cubic_to(
                Point::new(cx + k, cy - r),
                Point::new(cx + r, cy - k),
                Point::new(cx + r, cy),
            )
            .cubic_to(
                Point::new(cx + r, cy + k),
                Point::new(cx + k, cy + r),
                Point::new(cx, cy + r),
            )
            .cubic_to(
                Point::new(cx - k, cy + r),
                Point::new(cx - r, cy + k),
                Point::new(cx - r, cy),
            )
            .cubic_to(
                Point::new(cx - r, cy - k),
                Point::new(cx - k, cy - r),
                Point::new(cx, cy - r),
            )
            .close()
    }

    /// An **ellipse** inscribed in `r`, by the same four-cubic approximation as
    /// [`Path::circle`] — which it generalises: a square `r` gives that circle back.
    ///
    /// A circle scaled along one axis, in other words, without needing a transform
    /// to say so: it is how a wide, shallow arc (an overscroll glow, a soft
    /// highlight) is drawn as a plain filled path.
    pub fn oval(r: Rect) -> Self {
        const K: f32 = 0.552_284_75;
        let (rx, ry) = (r.width * 0.5, r.height * 0.5);
        let (cx, cy) = (r.x + rx, r.y + ry);
        let (kx, ky) = (rx * K, ry * K);
        Self::new()
            .move_to(Point::new(cx, cy - ry))
            .cubic_to(
                Point::new(cx + kx, cy - ry),
                Point::new(cx + rx, cy - ky),
                Point::new(cx + rx, cy),
            )
            .cubic_to(
                Point::new(cx + rx, cy + ky),
                Point::new(cx + kx, cy + ry),
                Point::new(cx, cy + ry),
            )
            .cubic_to(
                Point::new(cx - kx, cy + ry),
                Point::new(cx - rx, cy + ky),
                Point::new(cx - rx, cy),
            )
            .cubic_to(
                Point::new(cx - rx, cy - ky),
                Point::new(cx - kx, cy - ry),
                Point::new(cx, cy - ry),
            )
            .close()
    }

    /// A copy scaled by `factor` about the origin — used for logical-to-physical
    /// conversion, or to fit a `24×24` icon to its real size.
    pub fn scaled(&self, factor: f32) -> Path {
        Path {
            verbs: self.verbs.iter().map(|v| v.scaled(factor)).collect(),
        }
    }

    /// A copy translated by `(dx, dy)` — used to place an icon inside its box.
    pub fn translated(&self, dx: f32, dy: f32) -> Path {
        Path {
            verbs: self.verbs.iter().map(|v| v.translated(dx, dy)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_records_verbs_in_order() {
        let p = Path::new()
            .move_to(Point::new(1.0, 2.0))
            .line_to(Point::new(3.0, 4.0))
            .quad_to(Point::new(5.0, 6.0), Point::new(7.0, 8.0))
            .close();
        assert_eq!(
            p.verbs(),
            &[
                PathVerb::MoveTo(Point::new(1.0, 2.0)),
                PathVerb::LineTo(Point::new(3.0, 4.0)),
                PathVerb::QuadTo {
                    ctrl: Point::new(5.0, 6.0),
                    to: Point::new(7.0, 8.0)
                },
                PathVerb::Close,
            ]
        );
    }

    /// An arc lands where it was told to and stays on the circle in between — the
    /// property that matters, since the arc is one segment of a larger outline.
    #[test]
    fn an_arc_ends_on_the_circle_it_was_given() {
        use std::f32::consts::PI;
        let centre = Point::new(50.0, 40.0);
        let radius = 12.0;
        for (start, end) in [
            (0.0, PI),           // half a turn
            (PI, 0.0),           // and back the other way
            (0.3, 0.5),          // a sliver
            (-PI, PI * 0.75),    // more than a half turn: several pieces
        ] {
            let on = |a: f32| {
                Point::new(centre.x + radius * a.cos(), centre.y + radius * a.sin())
            };
            let path = Path::new().move_to(on(start)).arc_to(centre, radius, start, end);
            let last = match path.verbs().last() {
                Some(PathVerb::CubicTo { to, .. }) => *to,
                other => panic!("an arc ends on a cubic, got {other:?}"),
            };
            let want = on(end);
            assert!(
                (last.x - want.x).abs() < 0.01 && (last.y - want.y).abs() < 0.01,
                "arc {start}..{end} ended at {last:?}, wanted {want:?}"
            );
            // Every node the arc puts down sits on the circle.
            for verb in path.verbs() {
                if let PathVerb::CubicTo { to, .. } = verb {
                    let d = ((to.x - centre.x).powi(2) + (to.y - centre.y).powi(2)).sqrt();
                    assert!((d - radius).abs() < 0.01, "node off the circle: {d}");
                }
            }
        }
    }

    #[test]
    fn rect_is_a_closed_quad() {
        let p = Path::rect(Rect::new(0.0, 0.0, 10.0, 20.0));
        // move + 3× line + close = 5 commands (`close` supplies the fourth side).
        assert_eq!(p.verbs().len(), 5);
        assert!(matches!(p.verbs().last(), Some(PathVerb::Close)));
    }

    #[test]
    fn scaled_and_translated_transform_points() {
        let p = Path::new()
            .move_to(Point::new(2.0, 3.0))
            .line_to(Point::new(4.0, 5.0));
        let s = p.scaled(2.0);
        assert_eq!(s.verbs()[0], PathVerb::MoveTo(Point::new(4.0, 6.0)));
        let t = p.translated(10.0, 100.0);
        assert_eq!(t.verbs()[1], PathVerb::LineTo(Point::new(14.0, 105.0)));
    }

    #[test]
    fn circle_starts_at_top_and_closes() {
        let c = Path::circle(Point::new(12.0, 12.0), 10.0);
        assert_eq!(c.verbs()[0], PathVerb::MoveTo(Point::new(12.0, 2.0)));
        assert!(matches!(c.verbs().last(), Some(PathVerb::Close)));
    }
}

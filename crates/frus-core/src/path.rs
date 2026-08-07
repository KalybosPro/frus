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

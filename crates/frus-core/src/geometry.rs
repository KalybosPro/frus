//! Geometric primitives, expressed in logical pixels.
//!
//! frus's coordinate convention: the origin sits at the **top left**, X runs
//! right, and Y runs **down** — the same convention as CSS.

/// A **2D affine transform** (a 2×3 matrix). A point `(x, y)` becomes
/// `(a·x + c·y + e, b·x + d·y + f)`: a 2×2 linear part `[[a, c], [b, d]]` (scale,
/// rotation, shear) plus a translation `(e, f)`.
///
/// Compositions read "right to left": `a.then(b)` applies `a` **then** `b`. This is
/// the unified representation of a [`crate::LayerTransform`]'s paint transforms:
/// translation, scale (uniform or per axis) and rotation all fold into a **single**
/// matrix, with no approximation introduced by composing them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    /// `[a, b, c, d, e, f]`: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
    pub m: [f32; 6],
}

impl Default for Affine {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine {
    /// The identity — no transform at all.
    pub const IDENTITY: Affine = Affine {
        m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };

    /// A pure translation by `(tx, ty)`.
    pub const fn translation(tx: f32, ty: f32) -> Affine {
        Affine {
            m: [1.0, 0.0, 0.0, 1.0, tx, ty],
        }
    }

    /// A per-axis scale, about the origin.
    pub const fn scale(sx: f32, sy: f32) -> Affine {
        Affine {
            m: [sx, 0.0, 0.0, sy, 0.0, 0.0],
        }
    }

    /// A rotation of `angle` radians (clockwise, with y pointing down), about the
    /// origin.
    pub fn rotation(angle: f32) -> Affine {
        let (s, c) = angle.sin_cos();
        Affine {
            m: [c, s, -s, c, 0.0, 0.0],
        }
    }

    /// Composition: `self.then(next)` applies `self` **then** `next`.
    pub fn then(self, next: Affine) -> Affine {
        // next ∘ self: expand next(self(p)).
        let [a, b, c, d, e, f] = self.m;
        let [a2, b2, c2, d2, e2, f2] = next.m;
        Affine {
            m: [
                a2 * a + c2 * b,
                b2 * a + d2 * b,
                a2 * c + c2 * d,
                b2 * c + d2 * d,
                a2 * e + c2 * f + e2,
                b2 * e + d2 * f + f2,
            ],
        }
    }

    /// The same transform, but **about `pivot`**, which stays fixed:
    /// `T(pivot) ∘ self ∘ T(-pivot)`.
    pub fn about(self, pivot: Point) -> Affine {
        Affine::translation(-pivot.x, -pivot.y)
            .then(self)
            .then(Affine::translation(pivot.x, pivot.y))
    }

    /// Applies the transform to a point.
    pub fn apply(self, p: Point) -> Point {
        let [a, b, c, d, e, f] = self.m;
        Point::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f)
    }

    /// `true` when the transform is **axis-aligned**: its linear part is diagonal
    /// (scale and/or translation, with no rotation and no shear), so the image of a
    /// rectangle is still a rectangle.
    pub fn is_axis_aligned(self) -> bool {
        self.m[1].abs() < 1e-4 && self.m[2].abs() < 1e-4
    }

    /// The image of a rectangle under the transform. **Exact** when the matrix is
    /// axis-aligned ([`Affine::is_axis_aligned`]); otherwise it returns the image's
    /// **bounding box**, since a rectangle cannot represent a rotated shape.
    pub fn apply_rect(self, r: Rect) -> Rect {
        let a = self.apply(Point::new(r.x, r.y));
        let b = self.apply(Point::new(r.x + r.width, r.y + r.height));
        Rect::new(
            a.x.min(b.x),
            a.y.min(b.y),
            (a.x - b.x).abs(),
            (a.y - b.y).abs(),
        )
    }

    /// The inverse transform — the identity if the matrix is degenerate.
    pub fn inverse(self) -> Affine {
        let [a, b, c, d, e, f] = self.m;
        let det = a * d - b * c;
        if det.abs() < f32::EPSILON {
            return Affine::IDENTITY;
        }
        let inv = 1.0 / det;
        Affine {
            m: [
                d * inv,
                -b * inv,
                -c * inv,
                a * inv,
                (c * f - d * e) * inv,
                (b * e - a * f) * inv,
            ],
        }
    }
}

/// A 2D point.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Multiplies both coordinates by `factor` (logical → physical conversion).
    pub fn scale(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    /// Multiplies the coordinates **per axis** (`sx`, `sy`).
    pub fn scale_xy(self, sx: f32, sy: f32) -> Self {
        Self::new(self.x * sx, self.y * sy)
    }
}

/// The **reading direction** of text and layout. An ambient context: in RTL, rows,
/// alignment and directional insets all flip horizontally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextDirection {
    /// Left to right (Latin scripts; the default).
    #[default]
    Ltr,
    /// Right to left (Arabic, Hebrew, and others).
    Rtl,
}

impl TextDirection {
    /// `true` when right-to-left.
    pub fn is_rtl(self) -> bool {
        matches!(self, TextDirection::Rtl)
    }
}

/// Insets — padding or margin — per side, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same inset on all four sides.
    pub const fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }
}

/// **Directional** insets: `start`/`end` instead of `left`/`right`. In LTR, `start`
/// is the left; in RTL it is the right. Resolved into concrete [`Insets`] at layout
/// time.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct InsetsDirectional {
    pub top: f32,
    pub end: f32,
    pub bottom: f32,
    pub start: f32,
}

impl InsetsDirectional {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(top: f32, end: f32, bottom: f32, start: f32) -> Self {
        Self {
            top,
            end,
            bottom,
            start,
        }
    }

    /// A symmetric horizontal `start`/`end` inset.
    pub const fn horizontal(value: f32) -> Self {
        Self::new(0.0, value, 0.0, value)
    }

    /// Resolves to concrete insets for a direction: in RTL, `start` maps to right.
    pub fn resolve(self, direction: TextDirection) -> Insets {
        let (left, right) = if direction.is_rtl() {
            (self.end, self.start)
        } else {
            (self.start, self.end)
        };
        Insets::new(self.top, right, self.bottom, left)
    }
}

/// A 2D size (width × height).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Where to place a child inside its containing box: two **continuous** fractions
/// in `[-1, 1]`. `x = -1` hugs the left, `0` centres, `+1` hugs the right; `y = -1`
/// is the top, `+1` the bottom. Being continuous, it **interpolates**
/// ([`crate::Lerp`]): a `Tween<Alignment>` slides a child from one anchor to
/// another. The nine usual anchors are provided as constants
/// (`Alignment::CENTER`, `Alignment::TOP_LEFT`, and so on).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Alignment {
    /// Horizontal fraction: `-1` (left) … `0` (centre) … `+1` (right).
    pub x: f32,
    /// Vertical fraction: `-1` (top) … `0` (centre) … `+1` (bottom).
    pub y: f32,
}

impl Alignment {
    pub const TOP_LEFT: Self = Self::new(-1.0, -1.0);
    pub const TOP_CENTER: Self = Self::new(0.0, -1.0);
    pub const TOP_RIGHT: Self = Self::new(1.0, -1.0);
    pub const CENTER_LEFT: Self = Self::new(-1.0, 0.0);
    pub const CENTER: Self = Self::new(0.0, 0.0);
    pub const CENTER_RIGHT: Self = Self::new(1.0, 0.0);
    pub const BOTTOM_LEFT: Self = Self::new(-1.0, 1.0);
    pub const BOTTOM_CENTER: Self = Self::new(0.0, 1.0);
    pub const BOTTOM_RIGHT: Self = Self::new(1.0, 1.0);

    /// An `(x, y)` anchor, with fractions in `[-1, 1]`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The horizontal fraction mapped into `[0, 1]` (`0` = left, `1` = right): the
    /// share of the free space to leave **before** the child on the x axis.
    pub fn fraction_x(self) -> f32 {
        ((self.x + 1.0) * 0.5).clamp(0.0, 1.0)
    }

    /// The vertical fraction mapped into `[0, 1]` (`0` = top, `1` = bottom).
    pub fn fraction_y(self) -> f32 {
        ((self.y + 1.0) * 0.5).clamp(0.0, 1.0)
    }
}

/// A **directional** anchor: the horizontal axis is expressed **start → end**
/// rather than left → right. `x_start = -1` hugs the **start** edge (left in LTR,
/// right in RTL), `+1` the **end** edge. Resolved into a physical [`Alignment`] at
/// render time from the reading direction — the anchor follows the text without the
/// caller ever testing which way it runs.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct AlignmentDirectional {
    /// Start-to-end fraction: `-1` (start) … `0` (centre) … `+1` (end).
    pub x_start: f32,
    /// Vertical fraction: `-1` (top) … `+1` (bottom).
    pub y: f32,
}

impl AlignmentDirectional {
    pub const TOP_START: Self = Self::new(-1.0, -1.0);
    pub const TOP_CENTER: Self = Self::new(0.0, -1.0);
    pub const TOP_END: Self = Self::new(1.0, -1.0);
    pub const CENTER_START: Self = Self::new(-1.0, 0.0);
    pub const CENTER: Self = Self::new(0.0, 0.0);
    pub const CENTER_END: Self = Self::new(1.0, 0.0);
    pub const BOTTOM_START: Self = Self::new(-1.0, 1.0);
    pub const BOTTOM_CENTER: Self = Self::new(0.0, 1.0);
    pub const BOTTOM_END: Self = Self::new(1.0, 1.0);

    /// A directional `(x_start, y)` anchor, with fractions in `[-1, 1]`.
    pub const fn new(x_start: f32, y: f32) -> Self {
        Self { x_start, y }
    }

    /// Resolves to a **physical** anchor: in RTL, start maps to right (x is
    /// mirrored); in LTR, start is left (x unchanged). `y` never depends on it.
    pub fn resolve(self, direction: TextDirection) -> Alignment {
        let x = if direction.is_rtl() {
            -self.x_start
        } else {
            self.x_start
        };
        Alignment::new(x, self.y)
    }
}

/// A **resolvable** anchor — either physical ([`Alignment`]) or directional
/// ([`AlignmentDirectional`]). A widget accepts either one, through `Into`, and
/// resolves it against the reading direction at render time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlignmentGeometry {
    /// A physical anchor, with absolute left and right.
    Physical(Alignment),
    /// A directional anchor (start/end, mirrored in RTL).
    Directional(AlignmentDirectional),
}

impl AlignmentGeometry {
    /// Resolves to a physical anchor for a reading direction; an already physical
    /// anchor is returned unchanged.
    pub fn resolve(self, direction: TextDirection) -> Alignment {
        match self {
            AlignmentGeometry::Physical(a) => a,
            AlignmentGeometry::Directional(d) => d.resolve(direction),
        }
    }
}

impl From<Alignment> for AlignmentGeometry {
    fn from(a: Alignment) -> Self {
        AlignmentGeometry::Physical(a)
    }
}

impl From<AlignmentDirectional> for AlignmentGeometry {
    fn from(d: AlignmentDirectional) -> Self {
        AlignmentGeometry::Directional(d)
    }
}

/// The **window** insets, split by nature: `padding` is the area **permanently**
/// taken by the system (status and navigation bars, the notch — all static), while
/// `view_insets` is the area covered by **transient** UI, chiefly the soft
/// keyboard. Avoiding the keyboard means keeping content clear of
/// [`WindowInsets::safe`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowInsets {
    /// Permanent system areas (bars, notch) **that are still worth avoiding**: the
    /// intrusions, less whatever [`view_insets`](Self::view_insets) already covers.
    ///
    /// Zero at the bottom while the keyboard is up, because the navigation bar it
    /// hides is not an edge anything needs to stay clear of any more. Padding a screen
    /// by it as well would leave a strip of nothing between the content and the
    /// keyboard.
    pub padding: Insets,
    /// Transient areas (the soft keyboard); in practice only the bottom moves.
    ///
    /// Measured from the window edge, so it already includes whatever bar it covers.
    pub view_insets: Insets,
    /// The intrusions **ignoring** anything transient: what the notch and the bars take
    /// whether or not the keyboard is over them.
    ///
    /// The one that does not move when the keyboard opens, which is why a layout that
    /// must not shift reads this rather than [`padding`](Self::padding).
    pub view_padding: Insets,
}

impl WindowInsets {
    /// No insets at all.
    pub const ZERO: Self = Self {
        padding: Insets::ZERO,
        view_insets: Insets::ZERO,
        view_padding: Insets::ZERO,
    };

    /// The intrusions of a surface with nothing transient over them: the bars and the
    /// notch, and no keyboard.
    ///
    /// Here so that nobody has to assemble the three by hand and get them disagreeing.
    /// `padding` and `view_padding` are the same thing while there is no keyboard, and
    /// a hand-written literal that says otherwise describes a surface no platform can
    /// report.
    pub const fn bars(intrusions: Insets) -> Self {
        Self {
            padding: intrusions,
            view_insets: Insets::ZERO,
            view_padding: intrusions,
        }
    }

    /// The total area to avoid: the per-side **maximum** of the two kinds. The
    /// keyboard covers the navigation bar, so they are not added together.
    pub fn safe(&self) -> Insets {
        Insets::new(
            self.padding.top.max(self.view_insets.top),
            self.padding.right.max(self.view_insets.right),
            self.padding.bottom.max(self.view_insets.bottom),
            self.padding.left.max(self.view_insets.left),
        )
    }

    /// Splits raw insets into `(padding, view_insets)` given a **keyboard-free**
    /// reference. A **bottom** excess beyond that reference signals the keyboard;
    /// `view_insets.bottom` then measures the **total** occlusion **from the window
    /// edge**, bar included. That is what makes combining the two with `max`
    /// correct.
    pub fn from_baseline(baseline: Insets, current: Insets) -> WindowInsets {
        let keyboard = (current.bottom - baseline.bottom).max(0.0);
        // The intrusions with the keyboard taken back out: `current.bottom - keyboard`
        // is the baseline's bottom by construction, and the other three sides do not
        // move with a keyboard.
        let view_padding = Insets::new(
            current.top,
            current.right,
            current.bottom - keyboard,
            current.left,
        );
        let view_insets = Insets::new(
            0.0,
            0.0,
            if keyboard > 0.0 { current.bottom } else { 0.0 },
            0.0,
        );
        WindowInsets {
            padding: subtract(view_padding, view_insets),
            view_insets,
            view_padding,
        }
    }
}

/// `a` less `b`, side by side, never below zero.
///
/// What makes the padding the *remaining* intrusion rather than the whole one: a
/// navigation bar under an open keyboard is covered, so nothing has to avoid it, and
/// counting it twice would leave a strip of nothing above the keys.
fn subtract(a: Insets, b: Insets) -> Insets {
    Insets::new(
        (a.top - b.top).max(0.0),
        (a.right - b.right).max(0.0),
        (a.bottom - b.bottom).max(0.0),
        (a.left - b.left).max(0.0),
    )
}

impl Default for WindowInsets {
    fn default() -> Self {
        Self::ZERO
    }
}

/// An axis-aligned rectangle, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width, extending to the right.
    pub width: f32,
    /// Height, extending downwards.
    pub height: f32,
}

impl Rect {
    /// An "infinite" rectangle, used as a neutral clip that clips nothing.
    pub const UNBOUNDED: Rect = Rect::new(-1.0e7, -1.0e7, 2.0e7, 2.0e7);

    /// Builds a rectangle from its position (top-left corner) and its size.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Offsets the rectangle by `(dx, dy)`.
    pub fn translate(self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// Multiplies position and size by `factor` (logical → physical conversion).
    pub fn scale(self, factor: f32) -> Self {
        Self::new(
            self.x * factor,
            self.y * factor,
            self.width * factor,
            self.height * factor,
        )
    }

    /// Scales the rectangle by `factor` **about `pivot`**, which stays fixed:
    /// `pos' = pivot + (pos - pivot) * factor`, with size × `factor`.
    pub fn scale_about(self, pivot: Point, factor: f32) -> Self {
        self.scale_about_xy(pivot, factor, factor)
    }

    /// Scales the rectangle **per axis** about `pivot`.
    pub fn scale_about_xy(self, pivot: Point, sx: f32, sy: f32) -> Self {
        Self::new(
            pivot.x + (self.x - pivot.x) * sx,
            pivot.y + (self.y - pivot.y) * sy,
            self.width * sx,
            self.height * sy,
        )
    }

    /// Multiplies position and size **per axis** (`sx`, `sy`).
    pub fn scale_xy(self, sx: f32, sy: f32) -> Self {
        Self::new(self.x * sx, self.y * sy, self.width * sx, self.height * sy)
    }

    /// The intersection of two rectangles; zero-sized when they are disjoint.
    pub fn intersect(self, other: Rect) -> Self {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.width).min(other.x + other.width);
        let y1 = (self.y + self.height).min(other.y + other.height);
        Self::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }

    /// The smallest rectangle enclosing both `self` **and** `other`.
    pub fn union(self, other: Rect) -> Self {
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = (self.x + self.width).max(other.x + other.width);
        let y1 = (self.y + self.height).max(other.y + other.height);
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// `true` when `point` lies inside; the left and top edges count as inside.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    /// Builds a rectangle from an origin point and a size.
    pub const fn from_point_size(origin: Point, size: Size) -> Self {
        Self::new(origin.x, origin.y, size.width, size.height)
    }

    /// The top-left corner.
    pub const fn origin(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// The rectangle's size.
    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    /// The `[x, y, width, height]` form, ready for the GPU.
    pub const fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_composes_scale_then_rotate_about_a_pivot() {
        use std::f32::consts::FRAC_PI_2;
        let pivot = Point::new(10.0, 10.0);
        // Scale ×2 then rotate +90°, both about the pivot.
        let m = Affine::scale(2.0, 2.0)
            .about(pivot)
            .then(Affine::rotation(FRAC_PI_2).about(pivot));
        // The pivot stays put.
        let c = m.apply(pivot);
        assert!(
            (c.x - 10.0).abs() < 1e-3 && (c.y - 10.0).abs() < 1e-3,
            "pivot fixe : {c:?}"
        );
        // Linear part = rotation(90°) ∘ scale(2) = [0, 2, -2, 0].
        assert!(m.m[0].abs() < 1e-3 && (m.m[1] - 2.0).abs() < 1e-3);
        assert!((m.m[2] + 2.0).abs() < 1e-3 && m.m[3].abs() < 1e-3);
    }

    #[test]
    fn affine_inverse_round_trips() {
        use std::f32::consts::FRAC_PI_3;
        let m = Affine::scale(1.5, 0.5)
            .about(Point::new(4.0, 7.0))
            .then(Affine::rotation(FRAC_PI_3).about(Point::new(2.0, 3.0)))
            .then(Affine::translation(5.0, -2.0));
        let inv = m.inverse();
        for p in [
            Point::new(0.0, 0.0),
            Point::new(12.0, -3.0),
            Point::new(-5.0, 8.0),
        ] {
            let back = inv.apply(m.apply(p));
            assert!(
                (back.x - p.x).abs() < 1e-2 && (back.y - p.y).abs() < 1e-2,
                "{p:?} -> {back:?}"
            );
        }
    }

    #[test]
    fn directional_alignment_resolves_by_direction() {
        // start = the reading edge: left in LTR, right in RTL.
        let start = AlignmentDirectional::CENTER_START;
        assert_eq!(start.resolve(TextDirection::Ltr), Alignment::CENTER_LEFT);
        assert_eq!(start.resolve(TextDirection::Rtl), Alignment::CENTER_RIGHT);
        // The centre and the vertical axis do not depend on direction.
        let top = AlignmentDirectional::TOP_CENTER;
        assert_eq!(top.resolve(TextDirection::Rtl), Alignment::TOP_CENTER);
    }

    #[test]
    fn alignment_geometry_unifies_physical_and_directional() {
        // Physical: resolved as-is, whatever the direction.
        let phys: AlignmentGeometry = Alignment::CENTER_LEFT.into();
        assert_eq!(phys.resolve(TextDirection::Rtl), Alignment::CENTER_LEFT);
        // Directional: follows the reading direction.
        let dir: AlignmentGeometry = AlignmentDirectional::CENTER_START.into();
        assert_eq!(dir.resolve(TextDirection::Ltr), Alignment::CENTER_LEFT);
        assert_eq!(dir.resolve(TextDirection::Rtl), Alignment::CENTER_RIGHT);
    }

    #[test]
    fn directional_insets_flip_start_end() {
        // start=10, end=4: in LTR start is left; in RTL start is right.
        let d = InsetsDirectional::new(1.0, 4.0, 2.0, 10.0);
        let ltr = d.resolve(TextDirection::Ltr);
        assert_eq!(
            (ltr.left, ltr.right, ltr.top, ltr.bottom),
            (10.0, 4.0, 1.0, 2.0)
        );
        let rtl = d.resolve(TextDirection::Rtl);
        assert_eq!(
            (rtl.left, rtl.right),
            (4.0, 10.0),
            "start moves to the right under RTL"
        );
        // The vertical axis never moves.
        assert_eq!((rtl.top, rtl.bottom), (1.0, 2.0));
    }

    #[test]
    fn window_insets_split_and_safe_area() {
        // Keyboard-free reference: the top and bottom system bars.
        let baseline = Insets::new(84.0, 0.0, 45.0, 0.0);

        // Keyboard closed: everything is static padding, nothing transient, and the
        // intrusion that does not move is the same as the one that does.
        let closed = WindowInsets::from_baseline(baseline, baseline);
        assert_eq!(closed.padding, baseline);
        assert_eq!(closed.view_padding, baseline);
        assert_eq!(closed.view_insets, Insets::ZERO);
        assert_eq!(closed.safe(), baseline);

        // Keyboard open (bottom excess of 300): `view_insets.bottom` measures the
        // total occlusion from the edge (345, bar included).
        let open = WindowInsets::from_baseline(baseline, Insets::new(84.0, 0.0, 345.0, 0.0));
        assert_eq!(open.view_insets, Insets::new(0.0, 0.0, 345.0, 0.0));
        // The bar has not moved, so `view_padding` still reports it — while `padding`
        // reports what is **left** to avoid, which at the bottom is nothing at all. The
        // keyboard is over the bar; padding by both would leave a strip of nothing
        // between the content and the keys.
        assert_eq!(open.view_padding, baseline);
        assert_eq!(open.padding, Insets::new(84.0, 0.0, 0.0, 0.0));
        // The safe area is the per-side max — the keyboard covers the bar.
        assert_eq!(open.safe(), Insets::new(84.0, 0.0, 345.0, 0.0));

        // Current bottom BELOW the reference (bars hidden): no negative keyboard.
        let hidden = WindowInsets::from_baseline(baseline, Insets::new(84.0, 0.0, 10.0, 0.0));
        assert_eq!(hidden.view_insets, Insets::ZERO);
        assert_eq!(hidden.padding.bottom, 10.0);
        assert_eq!(hidden.view_padding.bottom, 10.0);
    }

    #[test]
    fn rect_from_point_size_roundtrips() {
        let r = Rect::from_point_size(Point::new(3.0, 4.0), Size::new(10.0, 20.0));
        assert_eq!(r, Rect::new(3.0, 4.0, 10.0, 20.0));
        assert_eq!(r.origin(), Point::new(3.0, 4.0));
        assert_eq!(r.size(), Size::new(10.0, 20.0));
        assert_eq!(r.to_array(), [3.0, 4.0, 10.0, 20.0]);
    }
}

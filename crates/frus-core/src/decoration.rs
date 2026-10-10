//! The **box decoration** model: the vocabulary for painting a rectangle
//! (background, gradient, border, rounded corners, shadow), independent of any
//! widget or theme.
//!
//! A [`BoxDecoration`] is a pure `Copy` value that a widget assembles at paint
//! time, then **lowers** into [`Scene`] primitives through
//! [`BoxDecoration::paint_into`], in a **fixed order**: shadow → background
//! (colour or gradient) → border. It feeds layout too:
//! [`BoxDecoration::content_padding`] reserves room for the border on taffy's
//! behalf.

use crate::{BorderSide, Color, Insets, Path, Point, Rect, Scene, ShapeBorder, TextDirection};

/// Corner radii, **per corner** (logical px). `From<f32>` covers the uniform case:
/// anywhere a radius is expected, a plain `10.0` still works.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BorderRadius {
    /// The top-left corner's radius, in logical pixels.
    pub top_left: f32,
    /// The top-right corner's radius, in logical pixels.
    pub top_right: f32,
    /// The bottom-right corner's radius, in logical pixels.
    pub bottom_right: f32,
    /// The bottom-left corner's radius, in logical pixels.
    pub bottom_left: f32,
}

/// **A corner radius named by the reading direction rather than by the wall**
/// (`border_radius.dart:621`).
///
/// A [`BorderRadius`] says *top left*. This says *top start* — the corner where the text
/// begins, which is the left one in English and the right one in Arabic. [`resolve`] turns
/// one into the other once the direction is known.
///
/// The distinction is not decorative. A drawer rounds its **inner** edge, the one facing
/// the page: for a leading drawer that is the *end* side in either direction, and a radius
/// written as *right* is correct in English and wrong in Arabic. Every asymmetric radius
/// in an interface that mirrors has this question, and answering it by hand at each site
/// is how one of them ends up answered differently.
///
/// The reference has one type hierarchy for this (`BorderRadiusGeometry`, with a resolved
/// and an unresolved subclass); this is two plain types and a `resolve`, because a widget
/// here is handed a `&Theme` and so always has a direction available when it needs one.
///
/// [`resolve`]: Self::resolve
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BorderRadiusDirectional {
    /// The corner at the top of the line's **beginning**.
    pub top_start: f32,
    /// The corner at the top of the line's **end**.
    pub top_end: f32,
    /// The corner at the bottom of the line's **end**.
    pub bottom_end: f32,
    /// The corner at the bottom of the line's **beginning**.
    pub bottom_start: f32,
}

impl BorderRadiusDirectional {
    /// No rounding at all.
    pub const ZERO: Self = Self::uniform(0.0);

    /// The same radius on all four corners — which needs no direction, and is here so a
    /// caller can write one without changing type halfway through an expression.
    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_start: radius,
            top_end: radius,
            bottom_end: radius,
            bottom_start: radius,
        }
    }

    /// The two corners at the line's **beginning** — the left pair in English, the right
    /// pair in Arabic.
    pub const fn start(radius: f32) -> Self {
        Self {
            top_start: radius,
            top_end: 0.0,
            bottom_end: 0.0,
            bottom_start: radius,
        }
    }

    /// The two corners at the line's **end**.
    pub const fn end(radius: f32) -> Self {
        Self {
            top_start: 0.0,
            top_end: radius,
            bottom_end: radius,
            bottom_start: 0.0,
        }
    }

    /// Both sides at once, each with its own radius — the reference's
    /// `BorderRadiusDirectional.horizontal`.
    pub const fn horizontal(start: f32, end: f32) -> Self {
        Self {
            top_start: start,
            top_end: end,
            bottom_end: end,
            bottom_start: start,
        }
    }

    /// The top pair and the bottom pair. Neither depends on the direction, so this is the
    /// same as [`BorderRadius`]'s — and is here for the same reason as
    /// [`uniform`](Self::uniform).
    pub const fn vertical(top: f32, bottom: f32) -> Self {
        Self {
            top_start: top,
            top_end: top,
            bottom_end: bottom,
            bottom_start: bottom,
        }
    }

    /// **The concrete radius**, once the direction is known: `start` becomes left where
    /// the text runs left to right, and right where it does not.
    pub const fn resolve(self, direction: TextDirection) -> BorderRadius {
        match direction {
            TextDirection::Ltr => BorderRadius {
                top_left: self.top_start,
                top_right: self.top_end,
                bottom_right: self.bottom_end,
                bottom_left: self.bottom_start,
            },
            TextDirection::Rtl => BorderRadius {
                top_left: self.top_end,
                top_right: self.top_start,
                bottom_right: self.bottom_start,
                bottom_left: self.bottom_end,
            },
        }
    }
}

impl From<f32> for BorderRadiusDirectional {
    fn from(radius: f32) -> Self {
        Self::uniform(radius)
    }
}

impl BorderRadius {
    /// Each corner `t` of the way from `self` to `other`.
    ///
    /// Not clamped: a corner past its target is what a spring asks for at the end of its
    /// travel, and the one rule for a radius in flight is this one — an animated container
    /// and a decoration transition both go through it.
    #[must_use]
    pub fn lerp(self, other: BorderRadius, t: f32) -> BorderRadius {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        BorderRadius {
            top_left: mix(self.top_left, other.top_left),
            top_right: mix(self.top_right, other.top_right),
            bottom_right: mix(self.bottom_right, other.bottom_right),
            bottom_left: mix(self.bottom_left, other.bottom_left),
        }
    }

    /// No rounding at all.
    pub const ZERO: Self = Self::uniform(0.0);

    /// The same radius on all four corners.
    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Only the **top** corners rounded (headers, rising sheets).
    pub const fn top(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }

    /// Only the **bottom** corners rounded.
    pub const fn bottom(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Only the **left** corners rounded.
    pub const fn left(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: 0.0,
            bottom_right: 0.0,
            bottom_left: radius,
        }
    }

    /// Only the **right** corners rounded.
    ///
    /// With [`left`](Self::left) this completes the set: a shape rounded on one
    /// **vertical** edge is what a side panel wants — square where it meets the window,
    /// round where it meets the content — and until now only the horizontal pair
    /// ([`top`](Self::top), [`bottom`](Self::bottom)) existed.
    pub const fn right(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: radius,
            bottom_right: radius,
            bottom_left: 0.0,
        }
    }

    /// Radii **clamped at zero** — a negative radius is meaningless when painting.
    pub fn clamped(self) -> Self {
        Self {
            top_left: self.top_left.max(0.0),
            top_right: self.top_right.max(0.0),
            bottom_right: self.bottom_right.max(0.0),
            bottom_left: self.bottom_left.max(0.0),
        }
    }

    /// Every corner grown by `by` — the envelope of a blurred shadow.
    pub fn inflate(self, by: f32) -> Self {
        Self {
            top_left: self.top_left + by,
            top_right: self.top_right + by,
            bottom_right: self.bottom_right + by,
            bottom_left: self.bottom_left + by,
        }
    }

    /// Every radius multiplied by `factor` (DPI scaling).
    pub fn scale(self, factor: f32) -> Self {
        Self {
            top_left: self.top_left * factor,
            top_right: self.top_right * factor,
            bottom_right: self.bottom_right * factor,
            bottom_left: self.bottom_left * factor,
        }
    }

    /// `[tl, tr, br, bl]`, ready for the GPU.
    pub fn to_array(self) -> [f32; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }
}

impl From<f32> for BorderRadius {
    fn from(radius: f32) -> Self {
        Self::uniform(radius)
    }
}

/// **The shape a box decoration paints** — the reference's `BoxShape`
/// (`box_border.dart:26`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxShape {
    /// A rectangle, its corners rounded by the decoration's radius.
    #[default]
    Rectangle,
    /// A circle, centred in the box, as wide as the box's shorter side. The decoration's
    /// radius means nothing to it.
    Circle,
}

/// One side of a [`BorderSide`] pair, the same on both sides of a lerp.
fn lerp_side(a: BorderSide, b: BorderSide, t: f32) -> BorderSide {
    // A side on one end only arrives by thickening, in its own colour.
    match (a.width > 0.0, b.width > 0.0) {
        (true, true) => {
            BorderSide::new(a.color.lerp(b.color, t), a.width + (b.width - a.width) * t)
        }
        (true, false) => BorderSide::new(a.color, a.width * (1.0 - t)),
        (false, true) => BorderSide::new(b.color, b.width * t),
        (false, false) => BorderSide::NONE,
    }
}

/// **A box's border, side by side** — the reference's `Border` (`box_border.dart:431`):
/// each of the four sides its own colour and width.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Border {
    /// The top side.
    pub top: BorderSide,
    /// The right side.
    pub right: BorderSide,
    /// The bottom side.
    pub bottom: BorderSide,
    /// The left side.
    pub left: BorderSide,
}

impl Border {
    /// No border on any side.
    pub const NONE: Self = Self {
        top: BorderSide::NONE,
        right: BorderSide::NONE,
        bottom: BorderSide::NONE,
        left: BorderSide::NONE,
    };

    /// The same width and colour on all four sides.
    pub const fn new(width: f32, color: Color) -> Self {
        Self::all(BorderSide::new(color, width))
    }

    /// `side` on all four sides — the reference's `Border.all` and `fromBorderSide`.
    pub const fn all(side: BorderSide) -> Self {
        Self {
            top: side,
            right: side,
            bottom: side,
            left: side,
        }
    }

    /// `vertical` on the left and the right, `horizontal` on the top and the bottom — the
    /// reference's `Border.symmetric`.
    pub const fn symmetric(vertical: BorderSide, horizontal: BorderSide) -> Self {
        Self {
            top: horizontal,
            right: vertical,
            bottom: horizontal,
            left: vertical,
        }
    }

    /// The four sides, top, right, bottom, left.
    fn sides(&self) -> [BorderSide; 4] {
        [self.top, self.right, self.bottom, self.left]
    }

    /// Whether the four sides are the same.
    pub fn is_uniform(&self) -> bool {
        let [t, r, b, l] = self.sides();
        t == r && r == b && b == l
    }

    /// `true` when some side is visible — non-zero width and non-zero alpha.
    pub fn is_visible(&self) -> bool {
        self.sides().iter().any(BorderSide::is_drawn)
    }

    /// The room the border takes inside the box, side by side.
    pub fn dimensions(&self) -> Insets {
        let w = |s: BorderSide| if s.is_drawn() { s.width } else { 0.0 };
        Insets::new(w(self.top), w(self.right), w(self.bottom), w(self.left))
    }

    /// The border `t` of the way to `other`, side by side.
    #[must_use]
    pub fn lerp(self, other: Border, t: f32) -> Border {
        Border {
            top: lerp_side(self.top, other.top, t),
            right: lerp_side(self.right, other.right, t),
            bottom: lerp_side(self.bottom, other.bottom, t),
            left: lerp_side(self.left, other.left, t),
        }
    }
}

/// **A box's border, by the line's start and end** — the reference's `BorderDirectional`
/// (`box_border.dart:799`): the start side is the left in a left-to-right script and the
/// right in a right-to-left one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BorderDirectional {
    /// The top side.
    pub top: BorderSide,
    /// The side where a line of text starts.
    pub start: BorderSide,
    /// The side where a line of text ends.
    pub end: BorderSide,
    /// The bottom side.
    pub bottom: BorderSide,
}

impl BorderDirectional {
    /// The sides on the screen, for `direction`.
    pub fn resolve(&self, direction: TextDirection) -> Border {
        let (left, right) = match direction {
            TextDirection::Ltr => (self.start, self.end),
            TextDirection::Rtl => (self.end, self.start),
        };
        Border {
            top: self.top,
            right,
            bottom: self.bottom,
            left,
        }
    }
}

/// A box's border, by its physical sides or by the line's start and end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoxBorder {
    /// Left, right, top, bottom.
    Physical(Border),
    /// Start, end, top, bottom.
    Directional(BorderDirectional),
}

impl BoxBorder {
    /// The sides on the screen, for `direction`.
    pub fn resolve(&self, direction: TextDirection) -> Border {
        match self {
            BoxBorder::Physical(border) => *border,
            BoxBorder::Directional(border) => border.resolve(direction),
        }
    }
}

impl From<Border> for BoxBorder {
    fn from(border: Border) -> Self {
        BoxBorder::Physical(border)
    }
}

impl From<BorderDirectional> for BoxBorder {
    fn from(border: BorderDirectional) -> Self {
        BoxBorder::Directional(border)
    }
}

/// A soft drop shadow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    /// Colour; its alpha sets the intensity.
    pub color: Color,
    /// Offset `(dx, dy)`, in logical pixels.
    pub offset: (f32, f32),
    /// Blur radius.
    pub blur: f32,
    /// How far the shadow grows beyond the box, before blurring.
    pub spread: f32,
}

impl BoxShadow {
    /// A shadow offset by `(dx, dy)` with `blur`, and no spread.
    pub const fn new(dx: f32, dy: f32, blur: f32, color: Color) -> Self {
        Self {
            color,
            offset: (dx, dy),
            blur,
            spread: 0.0,
        }
    }

    /// Sets the `spread`.
    pub const fn spread(mut self, spread: f32) -> Self {
        self.spread = spread;
        self
    }

    /// **The three shadows a surface `elevation` high casts**, in `color`: a sharp one close
    /// under it, a softer one further down, and a wide faint one all round. The reference's
    /// own table, and its three strengths — a fifth, a seventh and an eighth of `color`'s
    /// opacity — so `color` is the shadow's colour at full strength, black by default
    /// (milestone 606).
    ///
    /// Between two heights of the table the shadows are interpolated, so a height that
    /// animates moves smoothly; past twenty-four, it is twenty-four.
    pub fn for_elevation(elevation: f32, color: Color) -> [BoxShadow; 3] {
        let e = elevation.clamp(0.0, 24.0);
        let upper = ELEVATION_TABLE
            .iter()
            .position(|(level, _)| *level >= e)
            .unwrap_or(ELEVATION_TABLE.len() - 1);
        let lower = upper.saturating_sub(1);
        let (a, b) = (ELEVATION_TABLE[lower], ELEVATION_TABLE[upper]);
        let t = if b.0 > a.0 {
            (e - a.0) / (b.0 - a.0)
        } else {
            1.0
        };
        let lerp = |x: f32, y: f32| x + (y - x) * t;
        let layer = |i: usize, strength: f32| {
            let (ya, blur_a, spread_a) = a.1[i];
            let (yb, blur_b, spread_b) = b.1[i];
            // A surface on the page casts nothing: its shadows fade in as it lifts.
            let fade = if e < 1.0 { e } else { 1.0 };
            BoxShadow {
                color: color.with_alpha(color.a * strength * fade),
                offset: (0.0, lerp(ya, yb)),
                blur: lerp(blur_a, blur_b),
                spread: lerp(spread_a, spread_b),
            }
        };
        [layer(0, 0.2), layer(1, 0.14), layer(2, 0.12)]
    }

    /// The rectangle the shadow occupies around `rect` (offset + blur + spread).
    pub fn bounds(&self, rect: Rect) -> Rect {
        let grow = self.blur + self.spread;
        Rect::new(
            rect.x + self.offset.0 - grow,
            rect.y + self.offset.1 - grow,
            rect.width + 2.0 * grow,
            rect.height + 2.0 * grow,
        )
    }
}

/// The reference's shadows per height: `(height, [(drop, blur, spread); 3])`, the key light's
/// umbra, its penumbra, then the ambient light.
/// One shadow of a height: its drop, its blur and its spread.
type Layer = (f32, f32, f32);
/// A height and its three shadows.
type Height = (f32, [Layer; 3]);

const ELEVATION_TABLE: [Height; 11] = [
    (0.0, [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]),
    (1.0, [(2.0, 1.0, -1.0), (1.0, 1.0, 0.0), (1.0, 3.0, 0.0)]),
    (2.0, [(3.0, 1.0, -2.0), (2.0, 2.0, 0.0), (1.0, 5.0, 0.0)]),
    (3.0, [(3.0, 3.0, -2.0), (3.0, 4.0, 0.0), (1.0, 8.0, 0.0)]),
    (4.0, [(2.0, 4.0, -1.0), (4.0, 5.0, 0.0), (1.0, 10.0, 0.0)]),
    (6.0, [(3.0, 5.0, -1.0), (6.0, 10.0, 0.0), (1.0, 18.0, 0.0)]),
    (8.0, [(5.0, 5.0, -3.0), (8.0, 10.0, 1.0), (3.0, 14.0, 2.0)]),
    (9.0, [(5.0, 6.0, -3.0), (9.0, 12.0, 1.0), (3.0, 16.0, 2.0)]),
    (
        12.0,
        [(7.0, 8.0, -4.0), (12.0, 17.0, 2.0), (5.0, 22.0, 4.0)],
    ),
    (
        16.0,
        [(8.0, 10.0, -5.0), (16.0, 24.0, 2.0), (6.0, 30.0, 5.0)],
    ),
    (
        24.0,
        [(11.0, 15.0, -7.0), (24.0, 38.0, 3.0), (9.0, 46.0, 8.0)],
    ),
];

/// Paints the shadows of a surface `elevation` high, with corners `radius`, behind `rect`:
/// [`BoxShadow::for_elevation`]'s three, each as [`BoxDecoration`] paints one. Nothing at
/// a height of nought or in a colour with no opacity (milestone 606).
pub fn paint_elevation(
    scene: &mut Scene,
    rect: Rect,
    radius: BorderRadius,
    elevation: f32,
    color: Color,
) {
    if elevation <= 0.0 || color.a <= 0.0 {
        return;
    }
    for shadow in BoxShadow::for_elevation(elevation, color) {
        let grow = (shadow.blur + shadow.spread).max(0.0);
        scene.shadow(
            shadow.bounds(rect),
            shadow.color,
            radius.inflate(grow),
            shadow.blur,
        );
    }
}

/// The complete decoration of a box — the reference's `BoxDecoration`
/// (`box_decoration.dart:81`).
///
/// The paint order is **fixed**, the reference's (`box_decoration.dart:571`): the shadows,
/// in order, then the background, then the border. The background is either flat (`color`)
/// or a gradient (`color` → `gradient.end`). A border with no background paints an outline
/// over transparency; a wholly empty decoration paints nothing at all.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoxDecoration {
    /// Background colour.
    pub color: Option<Color>,
    /// A gradient filling the background, in place of `color` — linear, radial or sweep
    /// (milestone 644).
    pub gradient: Option<crate::Gradient>,
    /// The border, side by side.
    pub border: Option<BoxBorder>,
    /// Corner radii, per corner. A [`BoxShape::Circle`] has none.
    pub radius: BorderRadius,
    /// The shadows, painted in order, behind the box (`box_decoration.dart:448`).
    pub shadows: Vec<BoxShadow>,
    /// A rectangle or a circle.
    pub shape: BoxShape,
}

/// A colour that may be absent on either side, `t` of the way across.
///
/// Absent means **nothing painted**, so a colour on one side only fades: it keeps its hue
/// and only its alpha travels. Interpolating towards `Color::TRANSPARENT` instead would be
/// interpolating towards transparent *black* — a red fill fading out would go dark red on
/// the way, in a movement that was only ever about disappearing.
fn fade_between(a: Option<Color>, b: Option<Color>, t: f32) -> Option<Color> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.lerp(b, t)),
        (Some(a), None) => Some(a.fade(1.0 - t)),
        (None, Some(b)) => Some(b.fade(t)),
        (None, None) => None,
    }
}

impl BoxDecoration {
    /// The decoration `t` of the way from `self` to `other` — what a decoration transition
    /// paints mid-flight. Each part has its own rule, and the rules are the point:
    ///
    /// - **Colours mix** where both sides have one, in the space colours are stated in,
    ///   as every animated colour in this framework does.
    /// - **A part on one side only arrives or leaves; it does not come from nowhere.** A
    ///   fill fades, keeping its hue. A border thickens from nought in its own colour. A
    ///   shadow grows from under the box as it fades in. Each is the part scaled by how far
    ///   along it is, which is also what the reference does with a part one side lacks.
    /// - **A flat fill is a gradient whose two ends agree**, so flat to graded spreads the
    ///   far end out of the fill rather than laying a gradient over it. Here a gradient
    ///   *starts* at the fill colour, so the two always arrive and leave together.
    /// - **Corners** travel one by one.
    ///
    /// The absent colour here is **not** the absent colour of a text style, and the two
    /// rules differ for that reason. A text style that names no colour is asking the theme
    /// for one — a real, visible colour — so it holds still rather than fade. A decoration
    /// that names no fill paints nothing, so fading is exactly what reaching it means.
    ///
    /// `t` is clamped to `0..=1`, and a `t` that is not a number is no progress: past its
    /// ends a colour has nowhere to go, and a decoration whose colours stopped at the end
    /// while its corners kept going would be two clocks. At `0` and `1` the answer is the
    /// end itself, exactly.
    #[must_use]
    pub fn lerp(self, other: BoxDecoration, t: f32) -> BoxDecoration {
        if t.is_nan() || t <= 0.0 {
            return self;
        }
        if t >= 1.0 {
            return other;
        }
        // A rectangle and a circle do not blend: the shape changes half way, as the
        // reference's does (`box_decoration.dart:301`).
        let shape = if t < 0.5 { self.shape } else { other.shape };
        let mix = |a: f32, b: f32| a + (b - a) * t;
        // The reference's `Gradient.lerp` (`box_decoration.dart:309`).
        let gradient = crate::Gradient::lerp(self.gradient.as_ref(), other.gradient.as_ref(), t);
        // Side by side, each arriving or leaving by thickening in its own colour. A
        // directional border is read left to right here; two directional ones stay so.
        let border = match (self.border, other.border) {
            (Some(BoxBorder::Directional(a)), Some(BoxBorder::Directional(b))) => {
                Some(BoxBorder::Directional(BorderDirectional {
                    top: lerp_side(a.top, b.top, t),
                    start: lerp_side(a.start, b.start, t),
                    end: lerp_side(a.end, b.end, t),
                    bottom: lerp_side(a.bottom, b.bottom, t),
                }))
            }
            (Some(a), Some(b)) => Some(BoxBorder::Physical(
                a.resolve(TextDirection::Ltr)
                    .lerp(b.resolve(TextDirection::Ltr), t),
            )),
            (Some(a), None) => Some(BoxBorder::Physical(
                a.resolve(TextDirection::Ltr).lerp(Border::NONE, t),
            )),
            (None, Some(b)) => Some(BoxBorder::Physical(
                Border::NONE.lerp(b.resolve(TextDirection::Ltr), t),
            )),
            (None, None) => None,
        };
        // A shadow on one side only, scaled: it grows out from under the box as it fades
        // in. Scaling the geometry alone would leave a hard-edged block of shadow colour
        // under a box whose own fill may be fading too.
        let grown = |s: BoxShadow, f: f32| BoxShadow {
            color: s.color.fade(f),
            offset: (s.offset.0 * f, s.offset.1 * f),
            blur: s.blur * f,
            spread: s.spread * f,
        };
        // The shadows pair by pair, and those one list has more of grow or shrink — the
        // reference's `BoxShadow.lerpList` (`box_shadow.dart`).
        let longest = self.shadows.len().max(other.shadows.len());
        let shadows = (0..longest)
            .map(|i| match (self.shadows.get(i), other.shadows.get(i)) {
                (Some(a), Some(b)) => BoxShadow {
                    color: a.color.lerp(b.color, t),
                    offset: (mix(a.offset.0, b.offset.0), mix(a.offset.1, b.offset.1)),
                    blur: mix(a.blur, b.blur),
                    spread: mix(a.spread, b.spread),
                },
                (Some(a), None) => grown(*a, 1.0 - t),
                (None, Some(b)) => grown(*b, t),
                (None, None) => unreachable!("within the longer list"),
            })
            .collect();
        BoxDecoration {
            color: fade_between(self.color, other.color, t),
            gradient,
            border,
            radius: self.radius.lerp(other.radius, t),
            shadows,
            shape,
        }
    }
}

impl BoxDecoration {
    /// A decoration with a flat background.
    pub fn filled(color: Color) -> Self {
        Self {
            color: Some(color),
            ..Default::default()
        }
    }

    /// Sets the corner radii — uniform through `f32`, per corner through
    /// [`BorderRadius`].
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = radius.into();
        self
    }

    /// Sets the border, side by side or by the line's start and end.
    pub fn border(mut self, border: impl Into<BoxBorder>) -> Self {
        self.border = Some(border.into());
        self
    }

    /// Adds a shadow, after (so over) those already there.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.shadows.push(shadow);
        self
    }

    /// Sets the shadows, painted in order.
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.shadows = shadows.into_iter().collect();
        self
    }

    /// Paints a rectangle or a circle.
    pub fn shape(mut self, shape: BoxShape) -> Self {
        self.shape = shape;
        self
    }

    /// Fills the background with a gradient — linear, radial or sweep — in place of the
    /// colour.
    pub fn gradient(mut self, gradient: impl Into<crate::Gradient>) -> Self {
        self.gradient = Some(gradient.into());
        self
    }

    /// The inner margin the border needs — add it to the padding so the content is
    /// not eaten by the line. This is what feeds taffy. A directional border is read
    /// left to right; see [`Self::content_padding_in`].
    pub fn content_padding(&self) -> Insets {
        self.content_padding_in(TextDirection::Ltr)
    }

    /// The inner margin the border needs, for a box in `direction`.
    pub fn content_padding_in(&self, direction: TextDirection) -> Insets {
        self.border
            .map(|b| b.resolve(direction).dimensions())
            .unwrap_or(Insets::ZERO)
    }

    /// The box the decoration paints in `rect`: `rect` itself, or the square a circle is
    /// inscribed in (`box_decoration.dart:436`).
    fn painted(&self, rect: Rect) -> (Rect, BorderRadius) {
        match self.shape {
            BoxShape::Rectangle => (rect, self.radius),
            BoxShape::Circle => {
                let side = rect.width.min(rect.height);
                let square = Rect::new(
                    rect.x + (rect.width - side) * 0.5,
                    rect.y + (rect.height - side) * 0.5,
                    side,
                    side,
                );
                (square, BorderRadius::uniform(side * 0.5))
            }
        }
    }

    /// Lowers the decoration into `scene` primitives, in the fixed order
    /// shadows → background → border, for a box read left to right. See
    /// [`Self::paint_into_in`].
    pub fn paint_into(&self, scene: &mut Scene, rect: Rect, opacity: f32) {
        self.paint_into_in(scene, rect, opacity, TextDirection::Ltr);
    }

    /// Lowers the decoration into `scene` primitives, in the fixed order
    /// shadows → background → border. `opacity` (`0..=1`) modulates **every** colour,
    /// which is how a fade-in works. `rect` is the box in absolute coordinates;
    /// `direction` places a directional border's start and end.
    pub fn paint_into_in(
        &self,
        scene: &mut Scene,
        rect: Rect,
        opacity: f32,
        direction: TextDirection,
    ) {
        let (shape_rect, radius) = self.painted(rect);

        // 1) The shadows, in order, behind everything else.
        for shadow in &self.shadows {
            scene.shadow(
                shadow.bounds(shape_rect),
                shadow.color.fade(opacity),
                radius.inflate(shadow.blur + shadow.spread),
                shadow.blur,
            );
        }

        // 2/3) Background (flat or gradient), and a uniform border in the same primitive.
        let border = self.border.map(|b| b.resolve(direction));
        let uniform = border.filter(|b| b.is_uniform());
        let (border_width, border_color) = match uniform {
            Some(b) if b.top.width > 0.0 => (b.top.width, b.top.color.fade(opacity)),
            _ => (0.0, Color::TRANSPARENT),
        };
        let has_border = uniform.is_some_and(|b| b.is_visible());

        match (self.color, &self.gradient) {
            (_, Some(gradient)) => scene.shaded_rect(
                shape_rect,
                gradient.resolve(direction),
                opacity,
                radius,
                border_width,
                border_color,
            ),
            (Some(color), None) => scene.draw_rect(
                shape_rect,
                color.fade(opacity),
                radius,
                border_width,
                border_color,
            ),
            // Border only, with no background: an outline over transparency.
            (None, _) if has_border => scene.draw_rect(
                shape_rect,
                Color::TRANSPARENT,
                radius,
                border_width,
                border_color,
            ),
            // Nothing to paint.
            (None, _) => {}
        }

        // 3) A border whose sides differ.
        if let Some(border) = border.filter(|b| !b.is_uniform() && b.is_visible()) {
            paint_sides(scene, shape_rect, radius, self.shape, &border, opacity);
        }
    }
}

/// **A border whose sides differ** (`box_border.dart:681`). In one colour, the band between
/// the box and the box inset by each side, which keeps its corners and its circle; in
/// several, on a rectangle, each side as its own trapezoid, mitred at the corners.
fn paint_sides(
    scene: &mut Scene,
    rect: Rect,
    radius: BorderRadius,
    shape: BoxShape,
    border: &Border,
    opacity: f32,
) {
    let drawn: Vec<BorderSide> = border
        .sides()
        .into_iter()
        .filter(BorderSide::is_drawn)
        .collect();
    let one_colour = drawn.windows(2).all(|w| w[0].color == w[1].color);
    let rounded = shape == BoxShape::Circle || radius != BorderRadius::ZERO;
    let w = border.dimensions();
    let inner = Rect::new(
        rect.x + w.left,
        rect.y + w.top,
        (rect.width - w.left - w.right).max(0.0),
        (rect.height - w.top - w.bottom).max(0.0),
    );
    if one_colour && rounded {
        let color = drawn.first().map_or(Color::TRANSPARENT, |s| s.color);
        // The inner corners are the outer ones less the thicker of the two sides that meet
        // there.
        let less = |r: f32, a: f32, b: f32| (r - a.max(b)).max(0.0);
        let inner_radius = BorderRadius {
            top_left: less(radius.top_left, w.top, w.left),
            top_right: less(radius.top_right, w.top, w.right),
            bottom_right: less(radius.bottom_right, w.bottom, w.right),
            bottom_left: less(radius.bottom_left, w.bottom, w.left),
        };
        let band = ShapeBorder::rounded(radius)
            .outline(rect)
            .append(ShapeBorder::rounded(inner_radius).outline(inner));
        scene.fill_path(&band, color.fade(opacity));
        return;
    }
    // Each side as a trapezoid from the outer edge to the inner one.
    let (l, t, r, b) = (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
    let (il, it, ir, ib) = (
        inner.x,
        inner.y,
        inner.x + inner.width,
        inner.y + inner.height,
    );
    let quads = [
        (border.top, [(l, t), (r, t), (ir, it), (il, it)]),
        (border.right, [(r, t), (r, b), (ir, ib), (ir, it)]),
        (border.bottom, [(r, b), (l, b), (il, ib), (ir, ib)]),
        (border.left, [(l, b), (l, t), (il, it), (il, ib)]),
    ];
    for (side, [a, b2, c, d]) in quads {
        if !side.is_drawn() {
            continue;
        }
        let path = Path::new()
            .move_to(Point::new(a.0, a.1))
            .line_to(Point::new(b2.0, b2.1))
            .line_to(Point::new(c.0, c.1))
            .line_to(Point::new(d.0, d.1))
            .close();
        scene.fill_path(&path, side.color.fade(opacity));
    }
}

#[cfg(test)]
mod tests {
    /// **A directional radius names the corner by the line, not by the wall**
    /// (`border_radius.dart:621`), and `resolve` says which wall that is.
    #[test]
    fn a_directional_radius_follows_the_reading_direction() {
        let end = BorderRadiusDirectional::end(16.0);
        assert_eq!(
            end.resolve(TextDirection::Ltr),
            BorderRadius::right(16.0),
            "the line ends on the right in English"
        );
        assert_eq!(
            end.resolve(TextDirection::Rtl),
            BorderRadius::left(16.0),
            "and on the left in Arabic"
        );

        let start = BorderRadiusDirectional::start(16.0);
        assert_eq!(start.resolve(TextDirection::Ltr), BorderRadius::left(16.0));
        assert_eq!(start.resolve(TextDirection::Rtl), BorderRadius::right(16.0));

        // Both sides at once, each keeping its own number across the mirror.
        let both = BorderRadiusDirectional::horizontal(4.0, 12.0);
        assert_eq!(
            both.resolve(TextDirection::Ltr),
            BorderRadius {
                top_left: 4.0,
                top_right: 12.0,
                bottom_right: 12.0,
                bottom_left: 4.0,
            }
        );
        assert_eq!(
            both.resolve(TextDirection::Rtl),
            BorderRadius {
                top_left: 12.0,
                top_right: 4.0,
                bottom_right: 4.0,
                bottom_left: 12.0,
            }
        );

        // What has no side does not move.
        for direction in [TextDirection::Ltr, TextDirection::Rtl] {
            assert_eq!(
                BorderRadiusDirectional::uniform(8.0).resolve(direction),
                BorderRadius::uniform(8.0)
            );
            assert_eq!(
                BorderRadiusDirectional::vertical(8.0, 0.0).resolve(direction),
                BorderRadius::top(8.0)
            );
        }
    }
    use super::*;
    use crate::Primitive;

    fn rect() -> Rect {
        Rect::new(10.0, 20.0, 100.0, 40.0)
    }

    #[test]
    fn content_padding_reserves_the_border() {
        let deco = BoxDecoration::filled(Color::WHITE).border(Border::new(2.0, Color::BLACK));
        assert_eq!(deco.content_padding(), Insets::uniform(2.0));
        // An invisible border (zero width) → no padding.
        let none = BoxDecoration::filled(Color::WHITE).border(Border::new(0.0, Color::BLACK));
        assert_eq!(none.content_padding(), Insets::ZERO);
    }

    #[test]
    fn empty_decoration_paints_nothing() {
        let mut scene = Scene::new();
        BoxDecoration::default().paint_into(&mut scene, rect(), 1.0);
        assert!(scene.is_empty());
    }

    #[test]
    fn fixed_paint_order_shadow_then_fill() {
        let mut scene = Scene::new();
        let deco = BoxDecoration::filled(Color::WHITE)
            .radius(6.0)
            .shadow(BoxShadow::new(
                0.0,
                4.0,
                8.0,
                Color::rgba(0.0, 0.0, 0.0, 0.5),
            ));
        deco.paint_into(&mut scene, rect(), 1.0);
        // Two primitives: the shadow first, then the background.
        assert_eq!(scene.len(), 2);
        match scene.primitives()[0] {
            Primitive::Rect { blur, .. } => {
                assert!(blur > 0.0, "first primitive = the blurred shadow")
            }
            _ => panic!("expected a rectangle"),
        }
        match scene.primitives()[1] {
            Primitive::Rect { blur, color, .. } => {
                assert_eq!(blur, 0.0, "second primitive = the crisp background");
                assert_eq!(color, Color::WHITE);
            }
            _ => panic!("expected a rectangle"),
        }
    }

    #[test]
    fn opacity_fades_all_colours() {
        let mut scene = Scene::new();
        BoxDecoration::filled(Color::rgb(1.0, 0.0, 0.0))
            .border(Border::new(2.0, Color::rgb(0.0, 1.0, 0.0)))
            .paint_into(&mut scene, rect(), 0.5);
        match scene.primitives()[0] {
            Primitive::Rect {
                color,
                border_color,
                ..
            } => {
                assert_eq!(color.a, 0.5);
                assert_eq!(border_color.a, 0.5);
            }
            _ => panic!("expected a rectangle"),
        }
    }

    #[test]
    fn border_only_paints_transparent_fill_with_stroke() {
        let mut scene = Scene::new();
        BoxDecoration::default()
            .border(Border::new(1.0, Color::WHITE))
            .paint_into(&mut scene, rect(), 1.0);
        assert_eq!(scene.len(), 1);
        match scene.primitives()[0] {
            Primitive::Rect {
                color,
                border_width,
                ..
            } => {
                assert_eq!(color, Color::TRANSPARENT);
                assert_eq!(border_width, 1.0);
            }
            _ => panic!("expected a rectangle"),
        }
    }

    #[test]
    fn shadow_bounds_grow_with_blur_and_spread() {
        let s = BoxShadow::new(0.0, 0.0, 4.0, Color::BLACK).spread(2.0);
        let b = s.bounds(Rect::new(0.0, 0.0, 10.0, 10.0));
        // grow = blur + spread = 6 on every side.
        assert_eq!(b, Rect::new(-6.0, -6.0, 22.0, 22.0));
    }
}

/// The decoration in flight (milestone 501): one rule per part, and the ends exact.
#[cfg(test)]
mod lerp_tests {
    use super::*;

    const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    /// **The ends are the ends, exactly** — and past them, and at a progress that is not a
    /// number, the nearer end.
    #[test]
    fn the_ends_are_the_ends_exactly() {
        let a = BoxDecoration::filled(RED).radius(4.0);
        let b = BoxDecoration::filled(BLUE).border(Border::new(2.0, RED));
        let lerp = |t: f32| a.clone().lerp(b.clone(), t);
        assert_eq!(lerp(0.0), a);
        assert_eq!(lerp(1.0), b);
        assert_eq!(lerp(-0.5), a, "an undershoot stops at the start");
        assert_eq!(lerp(1.5), b, "an overshoot stops at the end");
        assert_eq!(lerp(f32::NAN), a, "and no progress is no progress");
    }

    /// **A fill fading out keeps its hue.** The obvious interpolation — towards
    /// `Color::TRANSPARENT` — is towards transparent *black*, and half-way there a red is a
    /// half-opaque dark red: a movement that was only about disappearing darkens on the way.
    #[test]
    fn a_fill_fading_out_does_not_pass_through_black() {
        let mid = BoxDecoration::filled(RED).lerp(BoxDecoration::default(), 0.5);
        let c = mid.color.expect("still painted, half faded");
        assert_eq!((c.r, c.g, c.b), (1.0, 0.0, 0.0), "the same red");
        assert!((c.a - 0.5).abs() < 1e-6, "at half its opacity: {c:?}");
        let naive = RED.lerp(Color::TRANSPARENT, 0.5);
        assert!(naive.r < 0.6, "the trap this avoids: {naive:?}");
    }

    /// And one fading in arrives in its own colour, only fainter.
    #[test]
    fn a_fill_fading_in_arrives_in_its_own_colour() {
        let c = BoxDecoration::default()
            .lerp(BoxDecoration::filled(BLUE), 0.25)
            .color
            .expect("already there, faintly");
        assert_eq!((c.r, c.g, c.b), (0.0, 0.0, 1.0));
        assert!((c.a - 0.25).abs() < 1e-6, "{c:?}");
    }

    /// **A gradient on one side only fades in by its opacity** over the fill, the
    /// reference's `Gradient.scale` (`box_decoration.dart:309`): its colours keep their hue.
    #[test]
    fn a_gradient_on_one_side_fades_in() {
        let flat = BoxDecoration::filled(RED);
        let graded =
            BoxDecoration::filled(RED).gradient(crate::LinearGradient::new(vec![RED, BLUE]));
        let mid = flat.lerp(graded, 0.25);
        assert_eq!(mid.color, Some(RED), "the fill does not move");
        let g = mid.gradient.expect("a gradient fading in");
        assert_eq!(g.colors()[1], BLUE.fade(0.25));
    }

    /// **A border arrives by thickening, in its own colour** — not as a colour on its way
    /// from nowhere.
    #[test]
    fn a_border_arrives_by_thickening_in_its_own_colour() {
        let lined = BoxDecoration::filled(RED).border(Border::new(4.0, BLUE));
        let quarter = BoxDecoration::filled(RED)
            .lerp(lined, 0.25)
            .border
            .expect("a line already")
            .resolve(TextDirection::Ltr);
        assert_eq!(quarter.top.width, 1.0);
        assert_eq!(quarter.top.color, BLUE);
    }

    /// **A shadow arrives by growing out from under the box as it fades in.**
    #[test]
    fn a_shadow_grows_and_fades_in() {
        let raised = BoxDecoration::filled(RED).shadow(BoxShadow::new(
            0.0,
            8.0,
            16.0,
            Color::rgba(0.0, 0.0, 0.0, 0.4),
        ));
        let half = BoxDecoration::filled(RED).lerp(raised, 0.5).shadows[0];
        assert_eq!(half.offset, (0.0, 4.0));
        assert_eq!(half.blur, 8.0);
        assert!((half.color.a - 0.2).abs() < 1e-6, "{:?}", half.color);
    }

    /// Corners travel one by one, each from its own start.
    #[test]
    fn corners_travel_one_by_one() {
        let r = BoxDecoration::default()
            .radius(BorderRadius::top(8.0))
            .lerp(BoxDecoration::default().radius(16.0), 0.5)
            .radius;
        assert_eq!((r.top_left, r.bottom_left), (12.0, 8.0));
    }

    /// **At a height of the reference's table, its three shadows exactly**, each carrying its
    /// share of the colour: a fifth, a seventh, an eighth.
    #[test]
    fn a_height_of_the_table_casts_its_three_shadows() {
        let [umbra, penumbra, ambient] = BoxShadow::for_elevation(3.0, Color::BLACK);
        assert_eq!(
            (umbra.offset, umbra.blur, umbra.spread),
            ((0.0, 3.0), 3.0, -2.0)
        );
        assert_eq!(
            (penumbra.offset, penumbra.blur, penumbra.spread),
            ((0.0, 3.0), 4.0, 0.0)
        );
        assert_eq!(
            (ambient.offset, ambient.blur, ambient.spread),
            ((0.0, 1.0), 8.0, 0.0)
        );
        assert_eq!(
            [umbra.color.a, penumbra.color.a, ambient.color.a],
            [0.2, 0.14, 0.12]
        );
        let [_, twenty_four, _] = BoxShadow::for_elevation(24.0, Color::BLACK);
        assert_eq!((twenty_four.offset, twenty_four.blur), ((0.0, 24.0), 38.0));
    }

    /// **Between two heights, between their shadows**; past twenty-four, twenty-four; and
    /// the colour's own opacity scales all three.
    #[test]
    fn heights_between_and_beyond_the_table() {
        let [_, half, _] = BoxShadow::for_elevation(5.0, Color::BLACK);
        assert_eq!(
            (half.offset.1, half.blur),
            (5.0, 7.5),
            "half way from 4 to 6"
        );
        assert_eq!(
            BoxShadow::for_elevation(40.0, Color::BLACK),
            BoxShadow::for_elevation(24.0, Color::BLACK)
        );
        let [faint, ..] = BoxShadow::for_elevation(1.0, Color::rgba(0.0, 0.0, 1.0, 0.5));
        assert_eq!(faint.color, Color::rgba(0.0, 0.0, 1.0, 0.1));
        // Lifting off the page fades in rather than jumping to a height of one.
        let [low, ..] = BoxShadow::for_elevation(0.5, Color::BLACK);
        assert!((low.color.a - 0.1).abs() < 1e-6, "{low:?}");
    }

    /// **Painted, a height is three soft shapes** grown past the box by their blur and
    /// spread, with the box's corners grown with them; at nought, or in a colour nobody can
    /// see, nothing.
    #[test]
    fn a_height_paints_three_shadows_and_nought_paints_none() {
        let rect = Rect::new(10.0, 20.0, 100.0, 40.0);
        let mut scene = Scene::new();
        paint_elevation(
            &mut scene,
            rect,
            BorderRadius::uniform(8.0),
            3.0,
            Color::BLACK,
        );
        let shadows: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                crate::Primitive::Rect {
                    rect, radius, blur, ..
                } => Some((*rect, *radius, *blur)),
                _ => None,
            })
            .collect();
        assert_eq!(shadows.len(), 3);
        let (penumbra, radius, blur) = shadows[1];
        assert_eq!(blur, 4.0);
        assert_eq!(
            penumbra,
            Rect::new(6.0, 19.0, 108.0, 48.0),
            "dropped 3, grown 4"
        );
        assert_eq!(radius, BorderRadius::uniform(12.0));

        for (height, colour) in [(0.0, Color::BLACK), (3.0, Color::TRANSPARENT)] {
            let mut none = Scene::new();
            paint_elevation(&mut none, rect, BorderRadius::ZERO, height, colour);
            assert!(none.primitives().is_empty(), "{height} {colour:?}");
        }
    }
}

/// The decoration's shape, its borders side by side, and its list of shadows
/// (milestone 641).
#[cfg(test)]
mod box_decoration_tests {
    use super::*;
    use crate::{PathVerb, Primitive};

    const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);

    fn painted(deco: &BoxDecoration, rect: Rect, direction: TextDirection) -> Vec<Primitive> {
        let mut scene = Scene::new();
        deco.paint_into_in(&mut scene, rect, 1.0, direction);
        scene.primitives().to_vec()
    }

    /// **A circle is centred in the box, as wide as its shorter side**, and its shadow is a
    /// circle too (`box_decoration.dart:436`). The radius it was given is not read.
    #[test]
    fn a_circle_is_centred_on_the_shorter_side() {
        let deco = BoxDecoration::filled(RED)
            .radius(3.0)
            .shape(BoxShape::Circle)
            .shadow(BoxShadow::new(0.0, 0.0, 4.0, BLUE));
        let prims = painted(&deco, Rect::new(0.0, 0.0, 100.0, 40.0), TextDirection::Ltr);
        match &prims[1] {
            Primitive::Rect {
                rect,
                radius,
                color,
                ..
            } => {
                assert_eq!(*color, RED);
                assert_eq!(*rect, Rect::new(30.0, 0.0, 40.0, 40.0));
                assert_eq!(*radius, BorderRadius::uniform(20.0), "round, not 3");
            }
            other => panic!("the fill, not {other:?}"),
        }
        match &prims[0] {
            Primitive::Rect { radius, .. } => {
                assert_eq!(*radius, BorderRadius::uniform(24.0), "a round shadow")
            }
            other => panic!("the shadow, not {other:?}"),
        }
    }

    /// **The shadows are painted in order, behind the box** (`box_decoration.dart:452`).
    #[test]
    fn the_shadows_are_painted_in_order_behind_the_box() {
        let deco = BoxDecoration::filled(RED).shadows([
            BoxShadow::new(0.0, 1.0, 2.0, GREEN),
            BoxShadow::new(0.0, 4.0, 8.0, BLUE),
        ]);
        let prims = painted(&deco, Rect::new(0.0, 0.0, 50.0, 50.0), TextDirection::Ltr);
        let colours: Vec<Color> = prims
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        assert_eq!(colours, vec![GREEN, BLUE, RED]);
    }

    /// **Sides of different colours are four trapezoids**, each its own colour, mitred at
    /// the corners; a side with no width is not drawn.
    #[test]
    fn sides_of_different_colours_are_drawn_one_by_one() {
        let border = Border {
            top: BorderSide::new(RED, 2.0),
            right: BorderSide::new(GREEN, 4.0),
            bottom: BorderSide::new(BLUE, 2.0),
            left: BorderSide::NONE,
        };
        let deco = BoxDecoration::default().border(border);
        let prims = painted(&deco, Rect::new(0.0, 0.0, 50.0, 30.0), TextDirection::Ltr);
        let fills: Vec<Color> = prims
            .iter()
            .filter_map(|p| match p {
                Primitive::Path { fill, .. } => *fill,
                _ => None,
            })
            .collect();
        assert_eq!(
            fills,
            vec![RED, GREEN, BLUE],
            "three sides, the fourth has none"
        );
        assert_eq!(deco.content_padding(), Insets::new(2.0, 4.0, 2.0, 0.0));
        // With round corners as well, still side by side: one band would be one colour.
        // (The reference refuses several colours with round corners, `box_border.dart:711`.)
        let rounded = BoxDecoration::default().border(border).radius(8.0);
        let fills: Vec<Color> = painted(
            &rounded,
            Rect::new(0.0, 0.0, 50.0, 30.0),
            TextDirection::Ltr,
        )
        .iter()
        .filter_map(|p| match p {
            Primitive::Path { fill, .. } => *fill,
            _ => None,
        })
        .collect();
        assert_eq!(fills, vec![RED, GREEN, BLUE]);
    }

    /// **One colour, uneven widths, round corners: one band** between the box and the box
    /// inset by each side (`box_border.dart:686`).
    #[test]
    fn one_colour_with_corners_is_one_band() {
        let border = Border {
            top: BorderSide::new(RED, 6.0),
            ..Border::new(2.0, RED)
        };
        let deco = BoxDecoration::default().border(border).radius(8.0);
        let prims = painted(&deco, Rect::new(0.0, 0.0, 50.0, 30.0), TextDirection::Ltr);
        let bands: Vec<&Primitive> = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .collect();
        assert_eq!(bands.len(), 1, "one band");
        match bands[0] {
            Primitive::Path { path, fill, .. } => {
                assert_eq!(*fill, Some(RED));
                let moves = path
                    .verbs()
                    .iter()
                    .filter(|v| matches!(v, PathVerb::MoveTo(_)))
                    .count();
                assert_eq!(moves, 2, "the outline and the hole in it");
                // The hole's corners are the box's less the thicker side meeting there:
                // 8 less 6 at the top left, so the hole starts 2 in from its own left.
                let hole = path
                    .verbs()
                    .iter()
                    .filter_map(|v| match v {
                        PathVerb::MoveTo(p) => Some(*p),
                        _ => None,
                    })
                    .nth(1)
                    .expect("the hole");
                assert_eq!((hole.x, hole.y), (4.0, 6.0));
            }
            _ => unreachable!(),
        }
    }

    /// **A uniform border stays one primitive with the fill**, as before.
    #[test]
    fn a_uniform_border_is_drawn_with_the_fill() {
        let deco = BoxDecoration::filled(RED).border(Border::new(3.0, BLUE));
        let prims = painted(&deco, Rect::new(0.0, 0.0, 50.0, 30.0), TextDirection::Ltr);
        assert_eq!(prims.len(), 1);
        match &prims[0] {
            Primitive::Rect {
                border_width,
                border_color,
                ..
            } => assert_eq!((*border_width, *border_color), (3.0, BLUE)),
            other => panic!("{other:?}"),
        }
    }

    /// **A directional border's start is the left in a left-to-right script and the right
    /// in a right-to-left one**, for the paint and for the room it takes.
    #[test]
    fn a_directional_border_follows_the_script() {
        let border = BorderDirectional {
            start: BorderSide::new(RED, 5.0),
            ..Default::default()
        };
        let deco = BoxDecoration::default().border(border);
        assert_eq!(deco.content_padding_in(TextDirection::Ltr).left, 5.0);
        assert_eq!(deco.content_padding_in(TextDirection::Rtl).right, 5.0);
        let first_x =
            |direction| match &painted(&deco, Rect::new(0.0, 0.0, 50.0, 30.0), direction)[0] {
                Primitive::Path { path, .. } => match path.verbs()[0] {
                    PathVerb::MoveTo(p) => p.x,
                    _ => unreachable!(),
                },
                other => panic!("{other:?}"),
            };
        // The left side's trapezoid starts at the bottom left; the right side's at the top
        // right.
        assert_eq!(first_x(TextDirection::Ltr), 0.0);
        assert_eq!(first_x(TextDirection::Rtl), 50.0);
    }

    /// **Two lists of shadows pair up; the longer one's extras grow or shrink**, and a
    /// circle and a rectangle swap half way.
    #[test]
    fn shadow_lists_pair_up_and_shapes_swap_half_way() {
        let one = BoxDecoration::default().shadow(BoxShadow::new(0.0, 2.0, 4.0, BLUE));
        let two = BoxDecoration::default().shape(BoxShape::Circle).shadows([
            BoxShadow::new(0.0, 6.0, 8.0, BLUE),
            BoxShadow::new(0.0, 10.0, 20.0, RED),
        ]);
        let mid = one.clone().lerp(two.clone(), 0.5);
        assert_eq!(mid.shadows.len(), 2);
        assert_eq!(mid.shadows[0].offset, (0.0, 4.0));
        assert_eq!(
            mid.shadows[1].offset,
            (0.0, 5.0),
            "the extra grows from nothing"
        );
        assert!((mid.shadows[1].color.a - 0.5).abs() < 1e-6);
        assert_eq!(mid.shape, BoxShape::Circle);
        assert_eq!(one.lerp(two, 0.49).shape, BoxShape::Rectangle);
    }

    /// **The border's constructors** are the reference's.
    #[test]
    fn the_border_constructors() {
        let side = BorderSide::new(RED, 1.0);
        let other = BorderSide::new(BLUE, 2.0);
        assert!(Border::all(side).is_uniform());
        let sym = Border::symmetric(side, other);
        assert_eq!(
            (sym.left, sym.right, sym.top, sym.bottom),
            (side, side, other, other)
        );
        assert!(!sym.is_uniform());
        assert!(!Border::NONE.is_visible());
        let half = Border::NONE.lerp(Border::all(other), 0.5);
        assert_eq!(
            half.top,
            BorderSide::new(BLUE, 1.0),
            "thickening in its own colour"
        );
    }
}

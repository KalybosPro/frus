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

use crate::{Color, Insets, Rect, Scene, TextDirection};

/// Corner radii, **per corner** (logical px). `From<f32>` covers the uniform case:
/// anywhere a radius is expected, a plain `10.0` still works.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
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

/// A uniform border — the same width and colour on all four sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    /// Width, in logical pixels.
    pub width: f32,
    /// The line's colour.
    pub color: Color,
}

impl Border {
    /// A uniform border.
    pub const fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }

    /// `true` when the border is visible — non-zero width and non-zero alpha.
    pub fn is_visible(&self) -> bool {
        self.width > 0.0 && self.color.a > 0.0
    }
}

/// A **linear** gradient: from the background (`BoxDecoration::color`) to `end`,
/// along `direction`, expressed in `[0,1]²` space (`[0,1]` = top→bottom, `[1,0]` =
/// left→right).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearGradient {
    /// The end colour; the start colour is the decoration's background.
    pub end: Color,
    /// The gradient's direction, in `[0,1]²` space.
    pub direction: [f32; 2],
}

impl LinearGradient {
    /// A linear gradient towards `end`, in the given direction.
    pub const fn new(end: Color, direction: [f32; 2]) -> Self {
        Self { end, direction }
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

/// The complete decoration of a rectangular box.
///
/// The paint order is **fixed**: shadow → background → border. The background is
/// either flat (`color`) or a gradient (`color` → `gradient.end`). A border with no
/// background paints an outline over transparency; a wholly empty decoration paints
/// nothing at all.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxDecoration {
    /// Background colour, which doubles as the start colour of any gradient.
    pub color: Option<Color>,
    /// A linear gradient for the background.
    pub gradient: Option<LinearGradient>,
    /// A uniform border.
    pub border: Option<Border>,
    /// Corner radii, per corner.
    pub radius: BorderRadius,
    /// Drop shadow.
    pub shadow: Option<BoxShadow>,
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

    /// Adds a uniform border.
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    /// Adds a shadow.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// Adds a linear gradient to the background.
    pub fn gradient(mut self, gradient: LinearGradient) -> Self {
        self.gradient = Some(gradient);
        self
    }

    /// The inner margin the border needs — add it to the padding so the content is
    /// not eaten by the line. This is what feeds taffy.
    pub fn content_padding(&self) -> Insets {
        match self.border {
            Some(b) if b.is_visible() => Insets::uniform(b.width),
            _ => Insets::ZERO,
        }
    }

    /// Lowers the decoration into `scene` primitives, in the fixed order
    /// shadow → background → border. `opacity` (`0..=1`) modulates **every** colour,
    /// which is how a fade-in works. `rect` is the box in absolute coordinates.
    pub fn paint_into(&self, scene: &mut Scene, rect: Rect, opacity: f32) {
        // 1) The shadow, behind everything else.
        if let Some(shadow) = self.shadow {
            scene.shadow(
                shadow.bounds(rect),
                shadow.color.fade(opacity),
                self.radius.inflate(shadow.blur + shadow.spread),
                shadow.blur,
            );
        }

        // 2/3) Background (flat or gradient) plus border, in a single primitive.
        let (border_width, border_color) = match self.border {
            Some(b) => (b.width, b.color.fade(opacity)),
            None => (0.0, Color::TRANSPARENT),
        };
        let has_border = self.border.map(|b| b.is_visible()).unwrap_or(false);

        match (self.color, self.gradient) {
            (Some(color), Some(gradient)) => scene.gradient_rect(
                rect,
                color.fade(opacity),
                gradient.end.fade(opacity),
                gradient.direction,
                self.radius,
                border_width,
                border_color,
            ),
            (Some(color), None) => scene.draw_rect(
                rect,
                color.fade(opacity),
                self.radius,
                border_width,
                border_color,
            ),
            // Border only, with no background: an outline over transparency.
            (None, _) if has_border => scene.draw_rect(
                rect,
                Color::TRANSPARENT,
                self.radius,
                border_width,
                border_color,
            ),
            // Nothing to paint.
            (None, _) => {}
        }
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

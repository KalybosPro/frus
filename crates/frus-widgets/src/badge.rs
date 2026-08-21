//! [`Badge`]: the small mark that says *there is something here* — a count on an inbox,
//! a dot on a bell.
//!
//! Two shapes, and the reference draws the same distinction. With a label it is a pill
//! carrying a number or a word; without one it is a **dot**, which says the same thing
//! without saying how much. A dot is what most notification marks are, and it is the one
//! this widget could not draw at all before milestone 378.
//!
//! It is a **standalone** mark: a pill you put where you like, in a row or beside a
//! title. The reference's also takes a child and pins itself to that child's corner — see
//! the note for milestone 378 for why that half is a separate step and not a builder.

use frus_core::{Color, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The reference's `smallSize`: the diameter of a badge with no label.
const DOT: f32 = 6.0;
/// The reference's `largeSize`: the height of a badge that carries one.
const PILL: f32 = 16.0;
/// The label's room either side, inside the pill.
const PAD_X: f32 = 4.0;

/// A small mark: a counted pill, or a dot.
pub struct Badge {
    /// The text on the pill. `None` is a **dot**, which is a shape rather than an empty
    /// label — see [`Badge::dot`].
    label: Option<String>,
    /// Whether the label is shown at all; `false` falls back to the dot. The reference's
    /// `isLabelVisible`, and what a count of zero wants.
    label_visible: bool,
    background_color: Option<Color>,
    text_color: Option<Color>,
    text_style: Option<TextStyle>,
    small_size: Option<f32>,
    large_size: Option<f32>,
    padding: Option<f32>,
}

impl Badge {
    /// A badge carrying `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            label: Some(text.into()),
            label_visible: true,
            background_color: None,
            text_color: None,
            text_style: None,
            small_size: None,
            large_size: None,
            padding: None,
        }
    }

    /// A badge with **no** label: a small dot.
    ///
    /// What most notification marks are. *Something happened* is the whole message, and a
    /// count that nobody reads is a number taking up room on a bell.
    pub fn dot() -> Self {
        let mut badge = Self::new(String::new());
        badge.label = None;
        badge
    }

    /// Hides the label without changing the badge to a dot at the call site.
    ///
    /// The reference's `isLabelVisible`, and it earns its place: a count of zero wants
    /// the mark gone or shrunk, and deciding that at the call site means an `if` around
    /// a widget rather than a value inside one.
    pub fn label_visible(mut self, visible: bool) -> Self {
        self.label_visible = visible;
        self
    }

    /// The pill's fill; the theme's `error` otherwise, as the reference has it — a badge
    /// is an alert, not an accent.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// The label's colour on that fill; the theme's `on_error` otherwise.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// The label's whole style — size, weight, family. Its colour is
    /// [`text_color`](Badge::text_color)'s.
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self
    }

    /// The diameter of a badge with no label. The reference's `smallSize`.
    pub fn small_size(mut self, size: f32) -> Self {
        self.small_size = Some(size);
        self
    }

    /// The height of a badge that carries one. The reference's `largeSize`.
    pub fn large_size(mut self, size: f32) -> Self {
        self.large_size = Some(size);
        self
    }

    /// The label's room either side.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(padding);
        self
    }

    /// The text actually drawn, or `None` for a dot.
    fn shown(&self) -> Option<&str> {
        match self.label_visible {
            true => self.label.as_deref(),
            false => None,
        }
    }

    fn fill(&self, theme: &Theme) -> Color {
        self.background_color
            .or(theme.widgets.badge.background_color)
            .unwrap_or(theme.scheme.error)
    }

    fn ink(&self, theme: &Theme) -> Color {
        self.text_color
            .or(theme.widgets.badge.text_color)
            .unwrap_or(theme.scheme.on_error)
    }

    fn label_style(&self, theme: &Theme) -> TextStyle {
        self.text_style
            .or(theme.widgets.badge.text_style)
            .unwrap_or(TextStyle {
                size: 11.0,
                ..theme.text.label_small
            })
    }

    fn dot_size(&self, theme: &Theme) -> f32 {
        self.small_size
            .or(theme.widgets.badge.small_size)
            .unwrap_or(DOT)
    }

    fn pill_height(&self, theme: &Theme) -> f32 {
        self.large_size
            .or(theme.widgets.badge.large_size)
            .unwrap_or(PILL)
    }

    fn pad(&self, theme: &Theme) -> f32 {
        self.padding
            .or(theme.widgets.badge.padding)
            .unwrap_or(PAD_X)
    }

    /// The box the mark itself occupies.
    fn mark_size(&self, theme: &Theme) -> (f32, f32) {
        let Some(text) = self.shown() else {
            let d = self.dot_size(theme);
            return (d, d);
        };
        let style = self.label_style(theme);
        let measured = frus_text::measure_styled(text, style.size, style.weight, style.italic);
        let height = self.pill_height(theme);
        // Never narrower than it is tall: a single digit in a wide pill reads as a
        // mistake, and the reference rounds a one-character badge to a circle for the
        // same reason.
        (
            (measured.width + self.pad(theme) * 2.0).max(height).ceil(),
            height,
        )
    }
}

impl<Msg> Widget<Msg> for Badge {
    fn style(&self) -> Style {
        <Self as Widget<Msg>>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let (width, height) = self.mark_size(theme);
        Style {
            width: Dimension::Length(width),
            height: Dimension::Length(height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        scene.draw_rect(
            bounds,
            self.fill(theme).fade(o),
            bounds.height * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        let Some(text) = self.shown() else {
            return;
        };
        let style = self.label_style(theme);
        let measured = frus_text::measure_styled(text, style.size, style.weight, style.italic);
        // Centred both ways rather than placed from the corner: the pill is as wide as
        // its label needs *or* as wide as it is tall, whichever is more, so a one-digit
        // badge has room to spare and a corner offset would sit it off to one side.
        scene.text_styled(
            Point::new(
                bounds.x + (bounds.width - measured.width) / 2.0,
                bounds.y + (bounds.height - measured.height) / 2.0,
            ),
            text.to_string(),
            &style,
            self.ink(theme).fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // A dot has nothing to read out. A count does, and a screen reader that never
        // hears it is a reader who never learns there are three unread messages.
        let text = self.shown()?;
        Some(frus_core::Semantics::new(frus_core::Role::Label).label(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn painted(badge: &Badge, bounds: Rect, theme: &Theme) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(badge, bounds, Status::default(), theme, &mut scene);
        scene.primitives().to_vec()
    }

    #[test]
    fn paints_pill_and_text() {
        let primitives = painted(
            &Badge::new("3"),
            Rect::new(0.0, 0.0, 24.0, 18.0),
            &Theme::default(),
        );
        assert!(primitives
            .iter()
            .any(|p| matches!(p, Primitive::Rect { .. })));
        assert!(primitives
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "3")));
    }

    /// A dot is a **shape**, not an empty label: no text primitive at all, and a box as
    /// wide as it is tall.
    #[test]
    fn a_dot_carries_no_text() {
        let theme = Theme::default();
        let dot = Badge::dot();
        let primitives = painted(&dot, Rect::new(0.0, 0.0, 6.0, 6.0), &theme);
        assert!(!primitives
            .iter()
            .any(|p| matches!(p, Primitive::Text { .. })));
        let style = Widget::<()>::style_themed(&dot, &theme);
        assert_eq!(style.width, Dimension::Length(DOT));
        assert_eq!(style.height, Dimension::Length(DOT));
    }

    /// Hiding the label falls back to the dot rather than to an empty pill, which would
    /// be a wide blank mark saying nothing.
    #[test]
    fn a_hidden_label_becomes_the_dot() {
        let theme = Theme::default();
        let hidden = Badge::new("99").label_visible(false);
        assert_eq!(
            Widget::<()>::style_themed(&hidden, &theme).width,
            Dimension::Length(DOT)
        );
    }

    /// A one-character badge is never narrower than it is tall: a lone digit in a wide
    /// pill reads as a mistake.
    #[test]
    fn a_single_digit_is_round_rather_than_wide() {
        let theme = Theme::default();
        let one = Badge::new("1");
        let style = Widget::<()>::style_themed(&one, &theme);
        assert_eq!(style.width, style.height);
    }

    /// A long count still grows: the floor is a minimum, not a size.
    #[test]
    fn a_long_count_widens_the_pill() {
        let theme = Theme::default();
        let width = |text: &str| match Widget::<()>::style_themed(&Badge::new(text), &theme).width {
            Dimension::Length(px) => px,
            other => panic!("a stated width, not {other:?}"),
        };
        let (narrow, wide) = (width("1"), width("1024"));
        assert!(wide > narrow, "{wide} should exceed {narrow}");
    }

    /// The colours resolve **instance, then theme, then the scheme's role** — the chain
    /// every widget since `Chip` has followed.
    #[test]
    fn the_colours_are_the_instances_then_the_themes() {
        let mut theme = Theme::default();
        let default_fill = |primitives: &[Primitive]| match primitives.first() {
            Some(Primitive::Rect { color, .. }) => *color,
            _ => panic!("a pill"),
        };
        let plain = painted(&Badge::new("3"), Rect::new(0.0, 0.0, 24.0, 18.0), &theme);
        assert_eq!(
            default_fill(&plain),
            theme.scheme.error,
            "the scheme's role"
        );

        theme.widgets.badge.background_color = Some(Color::rgb(0.0, 1.0, 0.0));
        let themed = painted(&Badge::new("3"), Rect::new(0.0, 0.0, 24.0, 18.0), &theme);
        assert_eq!(
            default_fill(&themed),
            Color::rgb(0.0, 1.0, 0.0),
            "the theme"
        );

        let overridden = painted(
            &Badge::new("3").background_color(Color::rgb(0.0, 0.0, 1.0)),
            Rect::new(0.0, 0.0, 24.0, 18.0),
            &theme,
        );
        assert_eq!(
            default_fill(&overridden),
            Color::rgb(0.0, 0.0, 1.0),
            "the instance wins"
        );
    }

    /// A count is read out. A dot is not: there is nothing to say beyond the mark itself,
    /// and announcing an empty label would interrupt a reader with nothing.
    #[test]
    fn a_count_is_announced_and_a_dot_is_not() {
        let counted = Widget::<()>::semantics(&Badge::new("3")).expect("announced");
        assert_eq!(counted.label.as_deref(), Some("3"));
        assert!(Widget::<()>::semantics(&Badge::dot()).is_none());
    }
}

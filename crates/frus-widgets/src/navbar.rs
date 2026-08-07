//! [`NavBar`]: a persistent navigation bar — a centred title, an optional back
//! button on the left. Placed at the head of a screen, it **slides and fades
//! with it** during [`crate::Navigator`] transitions.

use frus_core::{FontWeight, Insets, Point, Rect, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::button::{Button, Variant};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Bar height, in logical pixels.
const HEIGHT: f32 = 56.0;
/// Left margin: beyond the back-gesture zone, so the button stays clickable
/// without triggering the swipe.
const PAD_LEFT: f32 = 28.0;
/// Title size.
const TITLE_SIZE: f32 = 20.0;

/// A navigation bar: a title + an optional back button.
pub struct NavBar<Msg> {
    title: String,
    /// Title style (default: 20 px, medium weight, the theme's color).
    title_style: TextStyle,
    /// Bar height (default: [`HEIGHT`]).
    height: f32,
    /// `[]` (root) or `[back button]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavBar<Msg> {
    /// Creates a root bar: the title alone, with no back button.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            title_style: TextStyle::new(TITLE_SIZE).weight(FontWeight::Medium),
            height: HEIGHT,
            children: Vec::new(),
        }
    }

    /// Overrides the title style (size/weight/italic/color).
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = style;
        self
    }

    /// Overrides the bar height (56 px by default).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Adds a back button that emits `message`.
    pub fn on_back(mut self, message: Msg) -> Self {
        self.children = vec![Box::new(
            Button::new("←")
                .variant(Variant::Secondary)
                .size(16.0)
                .on_press(message),
        )];
        self
    }
}

impl<Msg: Clone> Widget<Msg> for NavBar<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            height: Dimension::Length(self.height),
            flex_direction: FlexDirection::Row,
            justify: Justify::Start,
            align: Align::Center,
            padding: Insets::new(6.0, 16.0, 6.0, PAD_LEFT),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Background + a thin bottom separator.
        scene.fill_rect(bounds, theme.background.fade(o));
        scene.fill_rect(
            Rect::new(bounds.x, bounds.y + bounds.height - 1.0, bounds.width, 1.0),
            theme.border.fade(o),
        );

        // The title is centred horizontally in the bar, following `title_style`
        // (medium weight by default — a bar title is a "title", not body text;
        // the style's color is inherited from the theme when absent).
        let style = self.title_style;
        let measured =
            frus_text::measure_styled(&self.title, style.size, style.weight, style.italic);
        let tx = bounds.x + (bounds.width - measured.width) * 0.5;
        let ty = bounds.y + (bounds.height - measured.height) * 0.5;
        scene.text_styled(
            Point::new(tx, ty),
            self.title.clone(),
            &style,
            style.color.unwrap_or(theme.on_surface).fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Back,
    }

    #[test]
    fn root_bar_has_no_back_button() {
        let bar: NavBar<Msg> = NavBar::new("Home");
        assert!(Widget::children(&bar).is_empty());
    }

    #[test]
    fn title_style_and_height_are_customizable() {
        // Overridden title style and height (defaults: medium 20, 56 px).
        let bar: NavBar<Msg> = NavBar::new("Title")
            .title_style(TextStyle::new(24.0).weight(FontWeight::Bold).italic())
            .height(72.0);
        match Widget::style(&bar).height {
            Dimension::Length(h) => assert_eq!(h, 72.0),
            _ => panic!("an imposed height was expected"),
        }
        let ui = build_ui(
            &bar,
            Size::new(400.0, 72.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let styled = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                Primitive::Text { text, size, weight, italic, .. }
                    if text == "Title" && *size == 24.0 && *weight == FontWeight::Bold && *italic
            )
        });
        assert!(styled, "the title must carry the overridden style");
    }

    #[test]
    fn back_button_emits_message() {
        let bar: NavBar<Msg> = NavBar::new("Settings").on_back(Msg::Back);
        let ui = build_ui(
            &bar,
            Size::new(400.0, 56.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The back button is on the left; a click there returns the back message.
        let id = ui.hit(Point::new(40.0, 28.0)).expect("back button");
        assert_eq!(ui.msg_for(id), Some(Msg::Back));
    }

    #[test]
    fn bar_paints_title_and_divider() {
        let bar: NavBar<Msg> = NavBar::new("Title");
        let ui = build_ui(
            &bar,
            Size::new(400.0, 56.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has_text = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Title"));
        assert!(has_text, "the title is painted");
    }
}

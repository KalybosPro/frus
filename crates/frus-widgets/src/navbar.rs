//! [`NavigationBar`]: a persistent navigation bar — a centred title, an optional back
//! button on the left. Placed at the head of a screen, it **slides and fades
//! with it** during [`crate::Navigator`] transitions.

#[cfg(test)]
use frus_core::FontWeight;
use frus_core::{Insets, Point, Rect, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Bar height, in logical pixels.
const HEIGHT: f32 = 56.0;
/// Left margin: beyond the back-gesture zone, so the button stays clickable
/// without triggering the swipe.
const PAD_LEFT: f32 = 28.0;
/// The title's type: what the caller said, else the step the reference gives an app bar's
/// title — `titleLarge`. It used to be a private `20.0` at a medium weight, which had **both
/// halves wrong**: the reference's is 22 and regular.
fn title_style_of(over: Option<TextStyle>, theme: &Theme) -> TextStyle {
    over.unwrap_or(theme.text.title_large)
}

/// A navigation bar: a title + an optional back button.
pub struct NavigationBar<Msg> {
    title: String,
    /// The caller's title style, if one was named. Unset, the theme's `titleLarge`.
    title_style: Option<TextStyle>,
    /// Bar height (default: [`HEIGHT`]).
    height: f32,
    /// `[]` (root) or `[back button]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavigationBar<Msg> {
    /// Creates a root bar: the title alone, with no back button.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            title_style: None,
            height: HEIGHT,
            children: Vec::new(),
        }
    }

    /// Overrides the title style (size/weight/italic/color).
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = Some(style);
        self
    }

    /// Overrides the bar height (56 px by default).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Adds a back button that emits `message`.
    ///
    /// It used to build `IconButton::glyph("←")` — a **character**, at the mercy of
    /// whatever font is loaded, with no control over its weight and none over its size
    /// beside the glyphs around it, and named by hand here rather than by the thing that
    /// knows what it is. [`BackButton`](crate::BackButton) is that thing.
    pub fn on_back(mut self, message: Msg) -> Self {
        self.children = vec![Box::new(
            crate::BackButton::new().icon_size(20.0).on_press(message),
        )];
        self
    }
}

impl<Msg: Clone> Widget<Msg> for NavigationBar<Msg> {
    fn style(&self) -> Style {
        Style {
            // **The bar fills the width it is offered.** It used to say `Auto`, which in a
            // row means *hug your children*: given no width by its parent it came out the
            // size of the back button, and `paint` — which centres the title in the box it
            // is given, the only thing it can do — put the title underneath the button.
            //
            // Every screen in the demo happens to hand it a width, so this never showed in
            // the application; it showed the first time the widget was rendered on its own
            // (milestone 296). A chrome that spans the head of a screen has no other
            // sensible answer to "how wide would you like to be", and an app bar gives the
            // same one.
            width: Dimension::Percent(1.0),
            height: Dimension::Length(self.height),
            flex_direction: FlexDirection::Row,
            justify: Justify::Start,
            align: Align::Center,
            // **No vertical padding**, and that is the reference's arrangement rather than
            // a saving: a toolbar is 56 tall and holds a 48-pixel button centred in it
            // (`constants.dart:27`, `constants.dart:30`), with the four pixels either side
            // coming from the difference and not from a rule. Six pixels of padding left 44
            // for the button, which is under the target it now reserves (milestone 442) —
            // the bar squeezed the one control in it.
            padding: Insets::new(0.0, 16.0, 0.0, PAD_LEFT),
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
            theme.scheme.outline_variant.fade(o),
        );

        // The title is centred horizontally in the bar, following `title_style`
        // (medium weight by default — a bar title is a "title", not body text;
        // the style's color is inherited from the theme when absent).
        let style = title_style_of(self.title_style, theme);
        let measured = frus_text::measure_style(&self.title, style);
        let tx = bounds.x + (bounds.width - measured.width) * 0.5;
        let ty = bounds.y + (bounds.height - measured.height) * 0.5;
        scene.text(
            Point::new(tx, ty),
            self.title.clone(),
            &style.resolved(),
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
        let bar: NavigationBar<Msg> = NavigationBar::new("Home");
        assert!(Widget::children(&bar).is_empty());
    }

    #[test]
    fn title_style_and_height_are_customizable() {
        // Overridden title style and height (defaults: medium 20, 56 px).
        let bar: NavigationBar<Msg> = NavigationBar::new("Title")
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
        let bar: NavigationBar<Msg> = NavigationBar::new("Settings").on_back(Msg::Back);
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

    /// **A bar given no width used to hug its back button**, and `paint` — which centres
    /// the title in the box it is handed, the only thing it can do — put the title
    /// underneath the button.
    ///
    /// A row is the arrangement that shows it: a row hands each child the width it asks
    /// for, and `Auto` asks for the width of its contents. Every screen in the demo
    /// happens to give the bar a width, which is why this survived 296 milestones without
    /// being seen and then showed the first time the widget was rendered on its own.
    #[test]
    fn a_bar_given_no_width_still_spans_what_it_is_offered() {
        const FRAME: f32 = 400.0;
        let bar: NavigationBar<Msg> = NavigationBar::new("Settings").on_back(Msg::Back);
        // A row of a known width whose child is asked for its own: the row spans the
        // frame, and a child saying `Auto` there is handed exactly what it hugs. The
        // width has to be definite for the bar's own answer to mean anything — a
        // percentage of an undecided width is undecided too.
        let root = crate::Flex::row().width(FRAME).child(bar);
        let theme = Theme::default();
        let ui = build_ui(&root, Size::new(FRAME, HEIGHT), &Runtime::default(), &theme);
        let at = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Text { text, position, .. } if text == "Settings" => Some(*position),
                _ => None,
            })
            .expect("the title is painted");
        let measured = frus_text::measure_style("Settings", title_style_of(None, &theme));
        let centred = (FRAME - measured.width) * 0.5;
        assert!(
            (at.x - centred).abs() < 1.0,
            "the title is centred in the frame: {} against {centred}",
            at.x
        );
        // And the thing the bug actually looked like: the title sat on the button.
        assert!(
            at.x > PAD_LEFT + crate::ICON_BUTTON_SIZE,
            "the title is clear of the back button: {} against {}",
            at.x,
            PAD_LEFT + crate::ICON_BUTTON_SIZE
        );
    }

    #[test]
    fn bar_paints_title_and_divider() {
        let bar: NavigationBar<Msg> = NavigationBar::new("Title");
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

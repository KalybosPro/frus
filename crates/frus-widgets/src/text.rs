//! [`Text`]: a widget that displays a line of text.

use frus_core::{Color, FontWeight, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A single-line text widget.
///
/// Its layout size comes from a **styled** measurement (`frus-text`, weight and
/// italic included); it pushes a text primitive into the scene when painting. The
/// [`TextStyle`]'s color is inherited from the theme when absent.
pub struct Text {
    content: String,
    style: TextStyle,
    /// Paragraph: wraps at the width the parent offers.
    wrap: bool,
}

impl Text {
    /// Creates a text (16 px, regular weight, the theme's color by default).
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: TextStyle::new(16.0),
            wrap: false,
        }
    }

    /// Creates a text from a full [`TextStyle`] — typically one step of the theme's
    /// scale (`Text::styled("Title", theme.text.title_large)`).
    pub fn styled(content: impl Into<String>, style: TextStyle) -> Self {
        Self {
            content: content.into(),
            style,
            wrap: false,
        }
    }

    /// Turns the text into a **paragraph**: it wraps at the width the parent
    /// offers (measured under constraints through taffy) instead of stretching
    /// out on a single line.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Sets the font size, in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.style.size = size;
        self
    }

    /// Fixe la graisse.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.style.weight = weight;
        self
    }

    /// Passe en italique.
    pub fn italic(mut self) -> Self {
        self.style.italic = true;
        self
    }

    /// Sets the text color (otherwise the theme's).
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }

    /// Underlines the text.
    pub fn underline(mut self) -> Self {
        self.style = self.style.underline();
        self
    }

    /// Strikes the text through.
    pub fn strikethrough(mut self) -> Self {
        self.style = self.style.strikethrough();
        self
    }

    /// Sets the decoration lines (freely combined).
    pub fn decoration(mut self, decoration: frus_core::TextDecoration) -> Self {
        self.style.decoration = decoration;
        self
    }

    /// Sets the decoration color (otherwise the text's).
    pub fn decoration_color(mut self, color: Color) -> Self {
        self.style.decoration_color = Some(color);
        self
    }
}

impl<Msg> Widget<Msg> for Text {
    fn style(&self) -> Style {
        // A paragraph: free dimensions, the size comes from `measure()`.
        if self.wrap {
            return Style::default();
        }
        let measured = frus_text::measure_styled(
            &self.content,
            self.style.size,
            self.style.weight,
            self.style.italic,
        );
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(measured.height.ceil()),
            ..Default::default()
        }
    }

    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        if !self.wrap {
            return None;
        }
        let content = self.content.clone();
        let style = self.style;
        Some(Box::new(move |max_width, _| {
            frus_text::measure_wrapped(&content, style.size, style.weight, style.italic, max_width)
        }))
    }

    fn measure_key(&self) -> Option<u64> {
        if !self.wrap {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.content.hash(&mut hasher);
        self.style.size.to_bits().hash(&mut hasher);
        self.style.weight.to_u16().hash(&mut hasher);
        self.style.italic.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self
            .style
            .color
            .unwrap_or(theme.on_surface)
            .fade(status.opacity);
        if self.wrap {
            // Rendering wraps at the width the layout gives it.
            scene.text_wrapped(
                Point::new(bounds.x, bounds.y),
                self.content.clone(),
                &self.style,
                color,
                bounds.width,
            );
        } else {
            scene.text_styled(
                Point::new(bounds.x, bounds.y),
                self.content.clone(),
                &self.style,
                color,
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // A text carries its content as its accessible label.
        Some(frus_core::Semantics::new(frus_core::Role::Label).label(self.content.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    /// A paragraph **sized to fit** — centred on a column's cross axis, so it gets
    /// its natural width rather than the whole row — must be given a box the text
    /// still fits in, and the thing below it must clear the lines it really has.
    ///
    /// This failed on a device before the measurement was rounded up: the natural
    /// width came back as a fraction, the layout rounded the box down below it, the
    /// text wrapped onto a second line when painted, and the label underneath sat on
    /// top of that line — the layout having reserved the height of one.
    #[test]
    fn a_paragraph_sized_to_fit_is_not_squeezed_into_wrapping() {
        use crate::{build_ui, Container, Flex, Runtime, Size};
        use frus_core::FontWeight;
        use frus_layout::Align;

        const MARK: Color = Color {
            r: 1.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        // Two labels: one that fits on a line at its natural width, one that cannot
        // fit the box at all and must reserve every line it wraps onto.
        for (label, lines) in [("Write code", 1.0_f32), ("A rather long task name that certainly wraps", 2.0)] {
            let tree = Container::<()>::new().width(400.0).height(600.0).child(
                Flex::column()
                    .width(376.0)
                    .align(Align::Center)
                    .gap(18.0)
                    .child(Text::new(label).size(24.0).weight(FontWeight::Bold).wrap())
                    .child(Container::new().height(20.0).color(MARK)),
            );
            let ui = build_ui(
                &tree,
                Size::new(400.0, 600.0),
                &Runtime::default(),
                &Theme::default(),
            );
            let paragraph = ui
                .scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Text {
                        position, max_width, ..
                    } => Some((*position, max_width.unwrap_or(f32::MAX))),
                    _ => None,
                })
                .expect("the paragraph is painted");
            let below = ui
                .scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if *color == MARK => Some(*rect),
                    _ => None,
                })
                .expect("the label under it is painted");
            // What the box it was given really costs, shaped at that width.
            let painted = frus_text::measure_wrapped(
                label,
                24.0,
                FontWeight::Bold,
                false,
                Some(paragraph.1),
            );
            assert!(
                (painted.height / frus_text::line_height(24.0) - lines).abs() < 0.01,
                "{label:?} wrapped onto {} lines in a box of {}",
                painted.height / frus_text::line_height(24.0),
                paragraph.1
            );
            assert!(
                below.y >= paragraph.0.y + painted.height - 0.01,
                "{label:?}: the label underneath overlaps the paragraph ({below:?} vs {painted:?})"
            );
        }
    }

    #[test]
    fn text_paints_a_text_primitive() {
        let text = Text::new("Salut")
            .size(20.0)
            .color(Color::rgb(1.0, 0.0, 0.0));
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(5.0, 6.0, 100.0, 24.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );

        assert_eq!(scene.primitives().len(), 1);
        assert_eq!(
            scene.primitives()[0],
            Primitive::Text {
                position: Point::new(5.0, 6.0),
                text: "Salut".to_string(),
                size: 20.0,
                color: Color::rgb(1.0, 0.0, 0.0),
                weight: FontWeight::Regular,
                italic: false,
                max_width: None,
                decoration: frus_core::TextDecoration::NONE,
                decoration_color: None,
                clip: Rect::UNBOUNDED,
                // Painted directly rather than through the widget walk, so no box was
                // declared: unknown, which the renderer reads as "covers everything".
                bounds: Rect::UNBOUNDED,
                owner: 0,
            }
        );
    }

    #[test]
    fn wrapped_text_measures_to_the_offered_width() {
        let long = "a paragraph long enough to wrap onto several lines";
        let text = Text::new(long).wrap();
        // Measuring under constraints wraps: taller at 120 px than when free.
        let measure = Widget::<()>::measure(&text).expect("measure closure");
        let free = measure(None, None);
        let narrow = measure(Some(120.0), None);
        assert!(narrow.width <= 120.0);
        assert!(narrow.height > free.height, "wrapped → taller");
        // And the measure key changes with the content (the cache fix).
        let other = Text::new("short").wrap();
        assert_ne!(
            Widget::<()>::measure_key(&text),
            Widget::<()>::measure_key(&other)
        );
        // A text with no wrapping exposes neither a measure nor a key.
        let plain = Text::new(long);
        assert!(Widget::<()>::measure(&plain).is_none());
        assert!(Widget::<()>::measure_key(&plain).is_none());
    }

    #[test]
    fn styled_text_carries_weight_and_italic() {
        let theme = Theme::default();
        // One step of the theme's scale (title_medium = 16 px, medium weight).
        let text = Text::styled("Title", theme.text.title_medium).italic();
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, 100.0, 24.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Text {
                size,
                weight,
                italic,
                color,
                ..
            } => {
                assert_eq!(*size, 16.0);
                assert_eq!(*weight, FontWeight::Medium);
                assert!(*italic);
                // Color inherited from the theme (the style did not set one).
                assert_eq!(*color, theme.on_surface);
            }
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn bold_text_lays_out_wider() {
        let regular: Style = Widget::<()>::style(&Text::new("Width"));
        let bold: Style = Widget::<()>::style(&Text::new("Width").weight(FontWeight::Bold));
        let w = |s: &Style| match s.width {
            Dimension::Length(v) => v,
            _ => panic!("a measured width was expected"),
        };
        assert!(w(&bold) > w(&regular), "bold must be wider");
    }
}

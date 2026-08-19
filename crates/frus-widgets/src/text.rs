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
    /// Single line, cut with an ellipsis at whatever width the layout gives it —
    /// and, crucially, willing to *be* given less than it asked for.
    ellipsis: bool,
}

/// The ellipsis a cut line ends in.
pub(crate) const ELLIPSIS: &str = "…";

/// The longest prefix of `content` that fits in `max_width`, ending in an ellipsis when
/// anything was cut. Returns `content` untouched when it already fits.
///
/// Character by character from the end: the strings this is used on are a line, not a
/// document, and a binary search over char boundaries would buy nothing at that length.
pub(crate) fn truncate(content: &str, style: &TextStyle, max_width: f32) -> String {
    let measure =
        |text: &str| frus_text::measure_styled(text, style.size, style.weight, style.italic).width;
    if max_width <= 0.0 || measure(content) <= max_width {
        return content.to_string();
    }
    let mut chars: Vec<char> = content.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let kept: String = chars.iter().collect();
        let candidate = format!("{}{ELLIPSIS}", kept.trim_end());
        if measure(&candidate) <= max_width {
            return candidate;
        }
    }
    ELLIPSIS.to_string()
}

impl Text {
    /// Creates a text (16 px, regular weight, the theme's color by default).
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: TextStyle::new(16.0),
            wrap: false,
            ellipsis: false,
        }
    }

    /// Creates a text from a full [`TextStyle`] — typically one step of the theme's
    /// scale (`Text::styled("Title", theme.text.title_large)`).
    pub fn styled(content: impl Into<String>, style: TextStyle) -> Self {
        Self {
            content: content.into(),
            style,
            wrap: false,
            ellipsis: false,
        }
    }

    /// Turns the text into a **paragraph**: it wraps at the width the parent
    /// offers (measured under constraints through taffy) instead of stretching
    /// out on a single line.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self.ellipsis = false;
        self
    }

    /// Keeps the text on **one line and inside its box**, cut with an ellipsis when it
    /// does not fit — the reference's `TextOverflow.ellipsis`.
    ///
    /// This is two things, and the second is the one that matters. The obvious half is the
    /// cut. The other is that an ellipsising text tells the layout it may be given *less
    /// than it asked for*: a flex item's automatic minimum size is its content, so a plain
    /// `Text` in a row refuses to shrink and pushes its siblings out of the box instead —
    /// which is how a long task name evicted its own delete button, off the card and out of
    /// the hit registry, on a device (milestone 333). `min_width: 0` is what lets flexbox
    /// do its job, and it is the same fix the web has needed for the same reason forever.
    ///
    /// Mutually exclusive with [`Text::wrap`], which is the other answer to the same
    /// question: wrap grows downwards, this one cuts. The last one called wins.
    pub fn ellipsis(mut self) -> Self {
        self.ellipsis = true;
        self.wrap = false;
        self
    }

    /// Sets the font size, in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.style.size = size;
        self
    }

    /// Sets the weight.
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
            // A flex item's automatic minimum size is its content, so a plain text
            // refuses to shrink and pushes its siblings out instead. An ellipsising one
            // says it may be given less; the paint cuts to whatever it gets.
            min_width: if self.ellipsis {
                Dimension::Length(0.0)
            } else {
                Dimension::Auto
            },
            ..Default::default()
        }
    }

    /// The line this text sits on, measured from the top of its box. A `Text` is the
    /// widget that actually *has* a baseline; every alignment that talks about one is
    /// ultimately asking one of these.
    fn text_baseline(&self, _theme: &Theme) -> Option<f32> {
        Some(frus_text::baseline(
            self.style.size,
            self.style.weight,
            self.style.italic,
        ))
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
            let content = if self.ellipsis {
                truncate(&self.content, &self.style, bounds.width)
            } else {
                self.content.clone()
            };
            scene.text_styled(Point::new(bounds.x, bounds.y), content, &self.style, color);
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
    /// An ellipsising text is **cut to the box it is given** instead of drawing past it.
    ///
    /// It also tells the layout it may be given less than it asked for (`min_width: 0`),
    /// which is half of what a row needs to keep its trailing widget intact. The other
    /// half — a trailing widget that refuses to shrink — is not expressible yet: there is
    /// no `flex_shrink` in `frus_layout::Style`, so a deficit is always shared out in
    /// proportion to base size. See the roadmap; milestone 333 measured it.
    #[test]
    fn an_ellipsising_text_is_cut_to_its_box() {
        let long = "A task name that is really rather long indeed and keeps going";
        let style = TextStyle::new(18.0);
        let full = frus_text::measure_styled(long, style.size, style.weight, style.italic).width;
        assert!(full > 300.0, "the fixture has to overflow: {full}");

        let cut = truncate(long, &style, 150.0);
        assert!(cut.ends_with(ELLIPSIS), "cut with an ellipsis: {cut}");
        let cut_width =
            frus_text::measure_styled(&cut, style.size, style.weight, style.italic).width;
        assert!(cut_width <= 150.0, "and it fits: {cut_width}");
        assert!(cut.len() > 4, "without cutting everything: {cut}");

        // A width it already fits in leaves it alone, ellipsis and all.
        assert_eq!(truncate("Short", &style, 500.0), "Short");
        // A box with no room at all is left alone — inherited from the app bar, where
        // a zero width means "the layout has not run yet" rather than "there is no
        // room". It is a wart: a genuinely collapsed box draws the whole string. Pinned
        // here so that changing it is a decision rather than a surprise.
        assert_eq!(truncate(long, &style, 0.0), long);
    }

    /// The layout half: an ellipsising text accepts less width than it asked for, a plain
    /// one does not.
    #[test]
    fn an_ellipsising_text_lets_the_layout_shrink_it() {
        use frus_layout::Dimension;
        let plain = Widget::<()>::style(&Text::new("A rather long label indeed").size(18.0));
        let cut = Widget::<()>::style(
            &Text::new("A rather long label indeed")
                .size(18.0)
                .ellipsis(),
        );
        assert_eq!(plain.min_width, Dimension::Auto, "content is its own floor");
        assert_eq!(
            cut.min_width,
            Dimension::Length(0.0),
            "and this one has none"
        );
        assert_eq!(plain.width, cut.width, "both still ask for the same width");
    }

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
        for (label, lines) in [
            ("Write code", 1.0_f32),
            ("A rather long task name that certainly wraps", 2.0),
        ] {
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
                        position,
                        max_width,
                        ..
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
            let painted =
                frus_text::measure_wrapped(label, 24.0, FontWeight::Bold, false, Some(paragraph.1));
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

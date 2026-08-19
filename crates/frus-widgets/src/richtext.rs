//! [`RichText`]: a **rich text** paragraph — a [`TextSpan`] tree (mixed styles,
//! cascading inheritance) flattened into resolved runs and laid out in one
//! piece.
//!
//! ```ignore
//! RichText::new(
//!     TextSpan::new("frus is ")
//!         .child(TextSpan::new("fast").bold())
//!         .child(TextSpan::new(" and portable.").italic()),
//! )
//! ```

use frus_core::{Color, Point, Rect, Scene, TextRun, TextSpan, TextStyle};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A rich text paragraph. Natural size by default (explicit `\n`s make the
/// lines); with [`RichText::wrap`], it wraps at the width the parent offers.
pub struct RichText {
    span: TextSpan,
    /// The paragraph's **base** style: the root of the cascade (what the spans
    /// inherit when they set nothing themselves).
    base: TextStyle,
    /// A paragraph wrapped to the offered width.
    wrap: bool,
}

impl RichText {
    /// Creates a paragraph (base: 16 px, regular weight, the theme's color).
    pub fn new(span: TextSpan) -> Self {
        Self {
            span,
            base: TextStyle::new(16.0),
            wrap: false,
        }
    }

    /// Overrides the base style (the root of the cascade) — typically one step of
    /// the theme's scale (`.base_style(theme.text.body_large)`).
    pub fn base_style(mut self, style: TextStyle) -> Self {
        self.base = style;
        self
    }

    /// Turns the paragraph into **wrapped** text: it wraps at the width the
    /// parent offers (measured under constraints through taffy).
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Resolved runs: the inherited color is settled against `fallback` and
    /// modulated by `opacity`. (For measuring, the color makes no difference.)
    fn runs(&self, fallback: Color, opacity: f32) -> Vec<TextRun> {
        self.span
            .flatten(self.base)
            .into_iter()
            .map(|(text, style)| TextRun {
                text,
                size: style.size,
                weight: style.weight,
                italic: style.italic,
                color: style.color.unwrap_or(fallback).fade(opacity),
                decoration: style.decoration,
                // Inherited = the run's color: resolved here so that the
                // fade-out applies to the decorations too.
                decoration_color: style.decoration_color.map(|c| c.fade(opacity)),
            })
            .collect()
    }
}

impl<Msg> Widget<Msg> for RichText {
    fn style(&self) -> Style {
        // A wrapped paragraph: free dimensions, the size comes from `measure()`.
        if self.wrap {
            return Style::default();
        }
        let measured = frus_text::measure_runs(&self.runs(Color::WHITE, 1.0));
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(measured.height.ceil()),
            ..Default::default()
        }
    }

    /// The line the first row of runs sits on. Runs of different sizes share a line,
    /// and the tallest ascender is what decides where it is.
    fn text_baseline(&self, _theme: &Theme) -> Option<f32> {
        frus_text::baseline_of_runs(&self.runs(Color::WHITE, 1.0))
    }

    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        if !self.wrap {
            return None;
        }
        let runs = self.runs(Color::WHITE, 1.0);
        Some(Box::new(move |max_width, _| {
            frus_text::measure_runs_wrapped(&runs, max_width)
        }))
    }

    fn measure_key(&self) -> Option<u64> {
        if !self.wrap {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.span.measure_hash(&mut hasher);
        self.base.size.to_bits().hash(&mut hasher);
        self.base.weight.to_u16().hash(&mut hasher);
        self.base.italic.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let runs = self.runs(theme.on_surface, status.opacity);
        if self.wrap {
            scene.rich_text_wrapped(Point::new(bounds.x, bounds.y), runs, bounds.width);
        } else {
            scene.rich_text(Point::new(bounds.x, bounds.y), runs);
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{FontWeight, Primitive};

    #[test]
    fn paints_resolved_runs_with_theme_colour() {
        let theme = Theme::default();
        let rich = RichText::new(
            TextSpan::new("plain ")
                .child(TextSpan::new("bold").bold())
                .child(TextSpan::new("red").color(Color::rgb(1.0, 0.0, 0.0))),
        )
        .base_style(theme.text.body_large);

        let mut scene = Scene::new();
        Widget::<()>::paint(
            &rich,
            Rect::new(2.0, 3.0, 300.0, 24.0),
            Status::default(),
            &theme,
            &mut scene,
        );

        assert_eq!(scene.primitives().len(), 1);
        match &scene.primitives()[0] {
            Primitive::RichText { position, runs, .. } => {
                assert_eq!(*position, Point::new(2.0, 3.0));
                assert_eq!(runs.len(), 3);
                // Run 0: inherits everything from the base (16 px, the theme's color).
                assert_eq!(runs[0].size, 16.0);
                assert_eq!(runs[0].color, theme.on_surface);
                // Run 1: bold, size and color inherited.
                assert_eq!(runs[1].weight, FontWeight::Bold);
                assert_eq!(runs[1].size, 16.0);
                assert_eq!(runs[1].color, theme.on_surface);
                // Run 2: an explicit color.
                assert_eq!(runs[2].color, Color::rgb(1.0, 0.0, 0.0));
            }
            _ => panic!("expected rich text"),
        }
    }

    #[test]
    fn wrapped_rich_text_measures_and_keys_by_content() {
        let para = |text: &str| {
            RichText::new(TextSpan::new(text).child(TextSpan::new(" gras").bold())).wrap()
        };
        let rich = para("a rich paragraph long enough to wrap onto several lines");
        // Measuring under constraints wraps: taller at 120 px than when free.
        let measure = Widget::<()>::measure(&rich).expect("measure closure");
        let free = measure(None, None);
        let narrow = measure(Some(120.0), None);
        assert!(narrow.width <= 120.0);
        assert!(narrow.height > free.height, "wrapped → taller");
        // The measure key follows the content (the relayout cache fix)…
        assert_ne!(
            Widget::<()>::measure_key(&rich),
            Widget::<()>::measure_key(&para("short"))
        );
        // … but not the color (no effect on geometry).
        let recolored = RichText::new(
            TextSpan::new("a rich paragraph long enough to wrap onto several lines")
                .color(Color::rgb(1.0, 0.0, 0.0))
                .child(TextSpan::new(" gras").bold()),
        )
        .wrap();
        assert_eq!(
            Widget::<()>::measure_key(&rich),
            Widget::<()>::measure_key(&recolored),
            "the color must not invalidate the layout"
        );
        // Without `.wrap()`: neither a measure nor a key.
        let plain = RichText::new(TextSpan::new("x"));
        assert!(Widget::<()>::measure(&plain).is_none());
        assert!(Widget::<()>::measure_key(&plain).is_none());
    }

    #[test]
    fn layout_accounts_for_the_largest_run() {
        // A 28 px run in the middle: the measured height exceeds that of a 16 px one.
        let small: Style = Widget::<()>::style(&RichText::new(TextSpan::new("plain")));
        let tall: Style = Widget::<()>::style(&RichText::new(
            TextSpan::new("plain").child(TextSpan::new("BIG").size(28.0)),
        ));
        let h = |s: &Style| match s.height {
            Dimension::Length(v) => v,
            _ => panic!("a measured height was expected"),
        };
        assert!(h(&tall) > h(&small), "the large run must grow the line");
    }
}

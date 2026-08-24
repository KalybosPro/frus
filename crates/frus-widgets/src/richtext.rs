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

use frus_core::{
    Color, MaskShader, Point, Rect, Scene, ShaderMask, TextAlign, TextBlock, TextOverflow, TextRun,
    TextSpan, TextStyle,
};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A rich text paragraph. It wraps at the width it is offered, as [`crate::Text`] does
/// and for the same reason: a piece of prose in a box narrower than itself is a paragraph.
///
/// Everything milestones 343 and 344 gave `Text` is here — [`RichText::align`],
/// [`RichText::max_lines`], [`RichText::overflow`], [`RichText::no_wrap`] — because the
/// three questions they answer are questions about *text*, and the styles being mixed
/// changes none of them.
pub struct RichText {
    span: TextSpan,
    /// The paragraph's **base** style: the root of the cascade (what the spans
    /// inherit when they set nothing themselves).
    base: TextStyle,
    /// A paragraph wrapped to the offered width.
    wrap: bool,
    /// What becomes of runs that do not fit their box.
    overflow: TextOverflow,
    /// At most this many lines.
    max_lines: Option<usize>,
    /// Where the lines sit inside the box.
    align: TextAlign,
    /// Whether this paragraph is willing to be given less than it asked for. See
    /// [`crate::Text`], where the same distinction is drawn for the same reason.
    shrinkable: bool,
}

impl RichText {
    /// Creates a paragraph (base: 16 px, regular weight, the theme's color).
    pub fn new(span: TextSpan) -> Self {
        Self {
            span,
            base: TextStyle::new(16.0),
            wrap: true,
            overflow: TextOverflow::Clip,
            max_lines: None,
            align: TextAlign::Start,
            shrinkable: false,
        }
    }

    /// Overrides the base style (the root of the cascade) — typically one step of
    /// the theme's scale (`.base_style(theme.text.body_large)`).
    pub fn base_style(mut self, style: TextStyle) -> Self {
        self.base = style;
        self
    }

    /// Wraps at the width the parent offers. This is the default; the call is kept
    /// because saying so at the call site is not redundant when it is the point.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Keeps the runs on **one line**, explicit newlines aside.
    pub fn no_wrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    /// Where the lines sit **inside the box** the paragraph was given. As with
    /// [`crate::Text`], it does nothing to a paragraph that shrink-wraps — and makes the
    /// paragraph take the width it is offered, so that there is something to sit in.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// At most `max_lines` lines; the rest is dropped and [`RichText::overflow`] decides
    /// how the last one ends. Saying so also tells the layout this paragraph may be given
    /// **less than it asked for**.
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = Some(max_lines.max(1));
        self.shrinkable = true;
        self
    }

    /// What becomes of runs that do not fit: cut, cut with an ellipsis, faded out, or
    /// drawn past the box.
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self.shrinkable = true;
        self
    }

    /// Whether this paragraph needs the whole width it is offered — see
    /// [`crate::Text`], where the same question is asked of a single style.
    fn fills(&self) -> bool {
        self.align != TextAlign::Start
    }

    /// The runs that actually fit `width`, and whether anything was left over.
    ///
    /// The cut lands on a break the shaper chose, which is why the offset comes from
    /// `frus-text` rather than from arithmetic here; splitting the runs at it is the easy
    /// half. The ellipsis inherits the style of the run it ends, because it is that run's
    /// last character as far as a reader is concerned.
    fn fitted(&self, runs: Vec<TextRun>, width: f32) -> (Vec<TextRun>, bool) {
        let Some(max) = self.max_lines else {
            return (runs, false);
        };
        let Some(cut) = frus_text::runs_cut_at(&runs, Some(width), self.wrap, max) else {
            return (runs, false);
        };
        let mut kept: Vec<TextRun> = Vec::new();
        let mut at = 0usize;
        for run in runs {
            let end = at + run.text.len();
            if cut >= end {
                at = end;
                kept.push(run);
                continue;
            }
            if cut > at {
                let mut head = run.clone();
                head.text = run.text[..cut - at].to_string();
                kept.push(head);
            }
            break;
        }
        if self.overflow == TextOverflow::Ellipsis {
            if let Some(last) = kept.last_mut() {
                let style = TextStyle::new(last.size)
                    .weight(last.weight)
                    .color(last.color);
                let style = if last.italic { style.italic() } else { style };
                last.text = crate::text::ellipsise(&last.text, &style.resolved(), width);
            }
        }
        (kept, true)
    }

    /// A line limit is a height cap and nothing else.
    fn capped(&self, height: f32) -> f32 {
        match self.max_lines {
            Some(max) => height.min(frus_text::line_height(self.base.resolved().size) * max as f32),
            None => height,
        }
    }

    /// The fade that ends a cut paragraph — the same rule as [`crate::Text`]'s.
    fn fade(&self, bounds: Rect, horizontal: bool) -> ShaderMask {
        let extent = if horizontal {
            bounds.width
        } else {
            bounds.height
        };
        let run = (extent * 0.2).min(frus_text::line_height(self.base.resolved().size) * 3.0);
        let (from, to) = if horizontal {
            (
                Point::new(bounds.x + bounds.width - run, bounds.y),
                Point::new(bounds.x + bounds.width, bounds.y),
            )
        } else {
            (
                Point::new(bounds.x, bounds.y + bounds.height - run),
                Point::new(bounds.x, bounds.y + bounds.height),
            )
        };
        ShaderMask::new(MaskShader::Linear {
            from,
            to,
            from_color: Color::WHITE,
            to_color: Color::WHITE.fade(0.0),
        })
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
        let min_width = if self.shrinkable {
            Dimension::Length(0.0)
        } else {
            Dimension::Auto
        };
        // A wrapped paragraph: free dimensions, the size comes from `measure()`.
        if self.wrap || self.fills() {
            return Style {
                min_width,
                ..Default::default()
            };
        }
        let measured = frus_text::measure_runs(&self.runs(Color::WHITE, 1.0));
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(self.capped(measured.height).ceil()),
            min_width,
            max_width: if self.shrinkable {
                Dimension::Percent(1.0)
            } else {
                Dimension::Auto
            },
            ..Default::default()
        }
    }

    /// A paragraph will not be squeezed along a row — see [`crate::Text`], which explains
    /// which axis this is about and why the parent has to be the one to apply it.
    fn main_axis_floor(&self, _theme: &Theme) -> Option<f32> {
        if self.shrinkable || !(self.wrap || self.fills()) {
            return None;
        }
        Some(
            frus_text::measure_runs(&self.runs(Color::WHITE, 1.0))
                .width
                .ceil(),
        )
    }

    /// The line the first row of runs sits on. Runs of different sizes share a line,
    /// and the tallest ascender is what decides where it is.
    fn text_baseline(&self, _theme: &Theme) -> Option<f32> {
        frus_text::baseline_of_runs(&self.runs(Color::WHITE, 1.0))
    }

    fn measure(&self, _theme: &Theme) -> Option<frus_layout::MeasureFn<'_>> {
        if !self.wrap && !self.fills() {
            return None;
        }
        let runs = self.runs(Color::WHITE, 1.0);
        let wrap = self.wrap;
        let max_lines = self.max_lines;
        let base = self.base.resolved().size;
        Some(Box::new(move |max_width, _| {
            let mut size =
                frus_text::measure_runs_wrapped(&runs, if wrap { max_width } else { None });
            if let Some(max) = max_lines {
                size.height = size.height.min(frus_text::line_height(base) * max as f32);
            }
            size
        }))
    }

    fn measure_key(&self, _theme: &Theme) -> Option<u64> {
        if !self.wrap && !self.fills() {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.span.measure_hash(&mut hasher);
        let base = self.base.resolved();
        base.size.to_bits().hash(&mut hasher);
        base.weight.to_u16().hash(&mut hasher);
        base.italic.hash(&mut hasher);
        self.max_lines.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let runs = self.runs(theme.on_surface, status.opacity);
        let natural = frus_text::measure_runs(&runs).width;
        let (runs, too_tall) = self.fitted(runs, bounds.width);
        // A paragraph that wraps has no line wider than its box; one that does not may.
        let too_wide = !self.wrap && natural > bounds.width + 0.5;
        let over = too_tall || too_wide;
        let block = TextBlock {
            width: (self.wrap || self.align != TextAlign::Start).then_some(bounds.width),
            soft_wrap: self.wrap,
            align: self.align,
        };
        let at = Point::new(bounds.x, bounds.y);
        let draw = |scene: &mut Scene| {
            scene.rich_text_block(at, runs.clone(), block);
        };
        if !over
            || self.overflow == TextOverflow::Visible
            || self.overflow == TextOverflow::Ellipsis
        {
            draw(scene);
            return;
        }
        let outer = scene.current_clip();
        scene.set_clip(outer.intersect(bounds));
        match self.overflow {
            TextOverflow::Fade => scene.masked(self.fade(bounds, too_wide && !too_tall), draw),
            _ => draw(scene),
        }
        scene.set_clip(outer);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::Status;
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
        let theme = Theme::default();
        let measure = Widget::<()>::measure(&rich, &theme).expect("measure closure");
        let free = measure(None, None);
        let narrow = measure(Some(120.0), None);
        assert!(narrow.width <= 120.0);
        assert!(narrow.height > free.height, "wrapped → taller");
        // The measure key follows the content (the relayout cache fix)…
        assert_ne!(
            Widget::<()>::measure_key(&rich, &theme),
            Widget::<()>::measure_key(&para("short"), &theme)
        );
        // … but not the color (no effect on geometry).
        let recolored = RichText::new(
            TextSpan::new("a rich paragraph long enough to wrap onto several lines")
                .color(Color::rgb(1.0, 0.0, 0.0))
                .child(TextSpan::new(" gras").bold()),
        )
        .wrap();
        assert_eq!(
            Widget::<()>::measure_key(&rich, &Theme::default()),
            Widget::<()>::measure_key(&recolored, &Theme::default()),
            "the color must not invalidate the layout"
        );
        // A paragraph told not to wrap is a box of a known size and says so in its
        // style, exactly as a `Text` does — see the reasoning there.
        let plain = RichText::new(TextSpan::new("x")).no_wrap();
        assert!(Widget::<()>::measure(&plain, &Theme::default()).is_none());
        assert!(Widget::<()>::measure_key(&plain, &Theme::default()).is_none());
    }

    /// A line limit cuts the runs where the **shaper** broke them, keeps the styles of
    /// what is left, and ends the last line in an ellipsis when asked.
    #[test]
    fn a_limit_cuts_the_runs_and_keeps_their_styles() {
        let span = TextSpan::new("one two three four ")
            .child(TextSpan::new("five six seven eight").bold())
            .child(TextSpan::new(" nine ten eleven twelve"));
        let rich = RichText::new(span)
            .base_style(TextStyle::new(12.0))
            .max_lines(2)
            .overflow(TextOverflow::Ellipsis);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &rich,
            Rect::new(0.0, 0.0, 90.0, 40.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let runs = match &scene.primitives()[0] {
            Primitive::RichText { runs, .. } => runs.clone(),
            other => panic!("rich text, not {other:?}"),
        };
        let text: String = runs.iter().map(|r| r.text.as_str()).collect();
        assert!(text.ends_with('…'), "cut and marked: {text:?}");
        assert!(text.len() < 60, "and shorter than the whole: {text:?}");
        // The bold run is still bold in what is left, which is the half a plain-text cut
        // does not have to get right.
        assert!(
            runs.iter().any(|r| r.weight == FontWeight::Bold),
            "the styles survived the cut: {runs:?}"
        );
    }

    /// A limit it does not reach leaves the runs exactly as they were.
    #[test]
    fn a_limit_it_does_not_reach_leaves_the_runs_alone() {
        let rich = RichText::new(TextSpan::new("short").child(TextSpan::new(" and bold").bold()))
            .base_style(TextStyle::new(12.0))
            .max_lines(4);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &rich,
            Rect::new(0.0, 0.0, 400.0, 40.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::RichText { runs, .. } => {
                let text: String = runs.iter().map(|r| r.text.as_str()).collect();
                assert_eq!(text, "short and bold");
            }
            other => panic!("rich text, not {other:?}"),
        }
    }

    /// Alignment reaches the primitive along with the width it aligns inside — and a
    /// start-aligned, non-wrapping paragraph is deliberately given neither.
    #[test]
    fn alignment_travels_with_the_width_it_aligns_in() {
        let painted = |rich: RichText| {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &rich,
                Rect::new(0.0, 0.0, 200.0, 40.0),
                Status::default(),
                &Theme::default(),
                &mut scene,
            );
            match &scene.primitives()[0] {
                Primitive::RichText {
                    align, max_width, ..
                } => (*align, *max_width),
                other => panic!("rich text, not {other:?}"),
            }
        };
        assert_eq!(
            painted(RichText::new(TextSpan::new("x")).align(TextAlign::Center)),
            (TextAlign::Center, Some(200.0))
        );
        assert_eq!(
            painted(RichText::new(TextSpan::new("x")).no_wrap()),
            (TextAlign::Start, None)
        );
    }

    #[test]
    fn layout_accounts_for_the_largest_run() {
        // A 28 px run in the middle: the measured height exceeds that of a 16 px one.
        let h = |rich: RichText| {
            Widget::<()>::measure(&rich, &Theme::default()).expect("measured")(None, None).height
        };
        let small = h(RichText::new(TextSpan::new("plain")));
        let tall = h(RichText::new(
            TextSpan::new("plain").child(TextSpan::new("BIG").size(28.0)),
        ));
        assert!(tall > small, "the large run must grow the line");
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

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
//!
//! Inside a [`SelectionArea`](crate::SelectionArea) a rich paragraph takes part in the
//! selection as a [`Text`](crate::Text) does, and [`RichText::selectable`] makes one
//! selectable on its own. Either way what is selected, highlighted and copied is its **plain
//! text** — the runs' words in order — whatever styles they are drawn in.

use std::cell::{Cell, OnceCell};

use frus_core::{
    Color, MaskShader, Point, Rect, Scene, ShaderMask, TextAlign, TextBlock, TextOverflow, TextRun,
    TextSpan, TextStyle,
};
use frus_layout::{Dimension, Style};
use frus_text::TextLayout;

use crate::interaction::{Key, Status};
use crate::runtime::Edit;
use crate::selectiontoolbar::{ToolbarContext, ToolbarItem};
use crate::text::{area_bar, paint_grips, paint_highlight, AreaBars, Selection};
use crate::textinput::{handle_at, handles_in, move_caret, selection_box_in, word_range};
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
    /// The paragraph's **plain text**: every run's words, in reading order. What a selection
    /// counts its characters in, and what Copy takes — the styles are the drawing's and not
    /// the words'.
    plain: String,
    /// What makes the paragraph **selectable** on its own — see [`RichText::selectable`].
    /// Boxed and absent unless asked for, as a [`crate::Text`]'s is.
    selection: Option<Box<Selection>>,
    /// The reader's font-size setting the paragraph was **last painted** under (at first, the
    /// one in force when it was built). A hit or a highlight is asked for outside a frame,
    /// where no setting is in force, and has to be measured against the runs as they were
    /// drawn.
    scale: Cell<f32>,
    /// The bars this paragraph shows when a [`SelectionArea`](crate::SelectionArea)'s
    /// selection is placed against it, one per context, built the first time each is asked
    /// for.
    area_bars: OnceCell<Box<AreaBars>>,
}

impl RichText {
    /// Creates a paragraph (base: 16 px, regular weight, the theme's color).
    pub fn new(span: TextSpan) -> Self {
        let plain = span
            .flatten(TextStyle::NONE)
            .into_iter()
            .map(|(text, _)| text)
            .collect();
        Self {
            span,
            base: TextStyle::new(16.0),
            wrap: true,
            overflow: TextOverflow::Clip,
            max_lines: None,
            align: TextAlign::Start,
            shrinkable: false,
            plain,
            selection: None,
            scale: Cell::new(frus_core::text_scale()),
            area_bars: OnceCell::new(),
        }
    }

    /// Makes this paragraph **selectable**, as [`Text::selectable`](crate::Text::selectable)
    /// makes a text: a press, a drag, a double click or a long press selects words in it,
    /// across its runs whatever their styles, and Ctrl+C, or the bar that opens over the
    /// selection, copies them as plain text. It takes focus and does not open the software
    /// keyboard.
    ///
    /// ```
    /// use frus_widgets::{RichText, TextSpan};
    ///
    /// let note = RichText::new(TextSpan::new("Order ").child(TextSpan::new("4417").bold()))
    ///     .selectable();
    /// # let _ = note;
    /// ```
    ///
    /// What is highlighted, hit and given handles is what is drawn, as for a selectable
    /// text: an aligned or right-to-left line where its glyphs are, and a paragraph a line
    /// limit cuts short only as far as it is drawn, while Copy takes the whole selected words.
    pub fn selectable(mut self) -> Self {
        self.selection_mut().on = true;
        self
    }

    /// Changes what the bar over a selectable paragraph's selection offers — see
    /// [`Text::selection_toolbar`](crate::Text::selection_toolbar), which this is for a
    /// paragraph of mixed styles. Only for a paragraph that is
    /// [`selectable`](Self::selectable).
    pub fn selection_toolbar<Msg: 'static>(
        mut self,
        build: impl Fn(&ToolbarContext, Vec<ToolbarItem<Msg>>) -> Vec<ToolbarItem<Msg>> + 'static,
    ) -> Self {
        self.selection_mut().set_toolbar::<Msg>(Box::new(build));
        self
    }

    /// The state of the selection, made when first asked for.
    fn selection_mut(&mut self) -> &mut Selection {
        self.selection
            .get_or_insert_with(|| Box::new(Selection::new()))
    }

    /// The selection state, if this paragraph is selectable.
    fn selecting(&self) -> Option<&Selection> {
        self.selection.as_deref().filter(|selection| selection.on)
    }

    /// How many characters the plain text has: the last boundary a selection can end on.
    fn len(&self) -> usize {
        self.plain.chars().count()
    }

    /// The layout of the runs as they were drawn in a box `width` wide — wrapped as the
    /// paint wraps them, under the reader's setting they were painted with: what a hit, a
    /// highlight, a handle or the bar's anchor is worked out from. For any paragraph: a
    /// [`SelectionArea`](crate::SelectionArea) selects in paragraphs that never asked to be.
    ///
    /// The runs are laid out **as they were drawn**: in the box the paint hands the renderer,
    /// aligned as it aligns them, and, for a paragraph a line limit cut short, only what was
    /// kept — with its indices still the whole plain text's (see [`TextLayout::elided`]).
    fn text_layout(&self, width: f32) -> TextLayout {
        frus_core::with_text_scale(self.scale.get(), || {
            let (drawn, cut) = self.fitted(self.runs(Color::WHITE, 1.0), width);
            let layout = TextLayout::of_runs_in(&drawn, self.block(width));
            if !cut {
                return layout;
            }
            // What was kept is a prefix of the plain text, an ellipsis perhaps after it.
            let visible = drawn
                .iter()
                .flat_map(|run| run.text.chars())
                .zip(self.plain.chars())
                .take_while(|(a, b)| a == b)
                .count();
            layout.elided(visible, self.len())
        })
    }

    /// The box the runs are drawn in at `width`.
    fn block(&self, width: f32) -> TextBlock {
        TextBlock {
            width: (self.wrap || self.align != TextAlign::Start).then_some(width),
            soft_wrap: self.wrap,
            align: self.align,
        }
    }

    /// [`text_layout`](Self::text_layout), for a paragraph that is selectable on its own.
    fn selection_layout(&self, width: f32) -> Option<TextLayout> {
        self.selecting()?;
        Some(self.text_layout(width))
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
            // The last run was ellipsised to the width on its own, but it shares its line
            // with the runs before it: with them, it may still not fit and wrap onto a line
            // past the limit. Shortened a character at a time until the kept runs fit.
            let ellipsis = crate::text::ELLIPSIS;
            while frus_text::runs_cut_at(&kept, Some(width), self.wrap, max).is_some() {
                let Some(last) = kept.last_mut() else {
                    break;
                };
                let mut body = last
                    .text
                    .strip_suffix(ellipsis)
                    .unwrap_or(&last.text)
                    .to_string();
                if body.is_empty() {
                    // Only the ellipsis is left of this run: the one before it ends in it.
                    if kept.len() < 2 {
                        break;
                    }
                    kept.pop();
                    if let Some(previous) = kept.last_mut() {
                        previous.text.push_str(ellipsis);
                    }
                    continue;
                }
                body.pop();
                last.text = format!("{}{ellipsis}", body.trim_end());
            }
        }
        (kept, true)
    }

    /// A line limit is a height cap and nothing else.
    fn capped(&self, height: f32) -> f32 {
        match self.max_lines {
            // Rounded up for the reason `Text`'s own clamp is (issue #54): the pixels
            // the allowed lines occupy are pixels a layout will round.
            Some(max) => height.min(frus_core::fits(
                self.base.resolved().line_height() * max as f32,
            )),
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
        let run = (extent * 0.2).min(self.base.resolved().line_height() * 3.0);
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
                letter_spacing: style.letter_spacing,
            })
            .collect()
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for RichText {
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
                // Same clamp as `capped`, and rounded up for the same reason (issue #54).
                size.height = size
                    .height
                    .min(frus_core::fits(frus_text::line_height(base) * max as f32));
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
        // What every later question about the selection is measured against.
        self.scale.set(frus_core::text_scale());
        // The highlight below the words: a selection area's whatever the focus, the
        // paragraph's own while it is focused.
        let runs = self.runs(theme.on_surface, status.opacity);
        let natural = frus_text::measure_runs(&runs).width;
        let (runs, too_tall) = self.fitted(runs, bounds.width);
        // A paragraph that wraps has no line wider than its box; one that does not may.
        let too_wide = !self.wrap && natural > bounds.width + 0.5;
        let over = too_tall || too_wide;
        let block = self.block(bounds.width);
        paint_highlight(
            || self.text_layout(bounds.width),
            self.len(),
            self.selecting().is_some(),
            bounds,
            over && self.overflow != TextOverflow::Visible,
            &status,
            theme,
            scene,
        );
        let at = Point::new(bounds.x, bounds.y);
        let draw = |scene: &mut Scene| {
            scene.rich_text_block(at, runs.clone(), block);
        };
        if !over
            || self.overflow == TextOverflow::Visible
            || self.overflow == TextOverflow::Ellipsis
        {
            draw(scene);
        } else {
            let outer = scene.current_clip();
            scene.set_clip(outer.intersect(bounds));
            match self.overflow {
                TextOverflow::Fade => scene.masked(self.fade(bounds, too_wide && !too_tall), draw),
                _ => draw(scene),
            }
            scene.set_clip(outer);
        }
        // The handles of a touch selection, over the words and outside the clip.
        paint_grips(
            || self.text_layout(bounds.width),
            self.len(),
            self.selecting().is_some(),
            bounds,
            &status,
            theme,
            scene,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    // What follows is how a paragraph takes part in a selection: a selection area's, for any
    // paragraph, and its own, for a selectable one — the hooks a `Text` answers, over the
    // plain text and the layout of the runs.

    fn focusable(&self) -> bool {
        self.selecting().is_some()
    }

    fn text_value(&self) -> Option<&str> {
        self.selecting().map(|_| self.plain.as_str())
    }

    fn takes_typing(&self) -> bool {
        false
    }

    /// The plain text, for a selection that spans texts: any paragraph with words in it.
    fn selection_text(&self) -> Option<&str> {
        (!self.plain.is_empty()).then_some(self.plain.as_str())
    }

    fn region_toolbar(
        &self,
        context: ToolbarContext,
        width: f32,
        range: (usize, usize),
    ) -> Option<(Rect, &dyn Widget<Msg>)> {
        let edit = Edit {
            cursor: range.1,
            anchor: Some(range.0),
            composing: None,
        };
        let anchor = selection_box_in(
            &self.text_layout(width),
            Point::new(0.0, 0.0),
            self.len(),
            &edit,
        );
        let bar = area_bar(&self.area_bars, context)?;
        Some((anchor, bar))
    }

    fn selection_grip(
        &self,
        width: f32,
        index: usize,
        start: bool,
    ) -> Option<crate::SelectionHandle> {
        Some(handle_at(
            &self.text_layout(width),
            Point::new(0.0, 0.0),
            index.min(self.len()),
            start,
        ))
    }

    fn selection_hit(&self, local_x: f32, local_y: f32, width: f32) -> Option<usize> {
        if self.plain.is_empty() {
            return None;
        }
        Some(
            self.text_layout(width)
                .hit_test(Point::new(local_x, local_y)),
        )
    }

    fn cursor_at(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        _scroll_cursor: usize,
    ) -> Option<usize> {
        let layout = self.selection_layout(width)?;
        Some(layout.hit_test(Point::new(local_x, local_y)))
    }

    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        self.selecting()?;
        let chars: Vec<char> = self.plain.chars().collect();
        word_range(&chars, index)
    }

    fn selected_text(&self, edit: &Edit) -> Option<String> {
        self.selecting()?;
        let chars: Vec<char> = self.plain.chars().collect();
        let len = chars.len();
        let (start, end) = edit.selection_range()?;
        let (start, end) = (start.min(len), end.min(len));
        (start < end).then(|| chars[start..end].iter().collect())
    }

    /// The caret's keys and nothing else, as a selectable [`crate::Text`]'s.
    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        self.selecting()?;
        let chars: Vec<char> = self.plain.chars().collect();
        let len = chars.len();
        let mut cursor = edit.cursor.min(len);
        let mut anchor = edit.anchor.map(|anchor| anchor.min(len));
        if move_caret(&chars, &mut cursor, &mut anchor, key) {
            edit.cursor = cursor;
            edit.anchor = anchor;
        }
        None
    }

    fn selection_handles(
        &self,
        width: f32,
        edit: &Edit,
        _scroll_y: f32,
    ) -> Option<[crate::SelectionHandle; 2]> {
        let layout = self.selection_layout(width)?;
        handles_in(&layout, Point::new(0.0, 0.0), self.len(), edit)
    }

    fn selection_anchor(&self, width: f32, edit: &Edit, _scroll_y: f32) -> Option<Rect> {
        let layout = self.selection_layout(width)?;
        Some(selection_box_in(
            &layout,
            Point::new(0.0, 0.0),
            self.len(),
            edit,
        ))
    }

    fn selection_toolbar(&self, context: ToolbarContext) -> Option<&dyn Widget<Msg>> {
        self.selecting()?.toolbar(context)
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

    // What follows is a paragraph's part in a selection (milestone 738).

    /// "plain " in the base style, "bold" bold, " tail" plain again: three runs, one text.
    fn mixed() -> RichText {
        RichText::new(
            TextSpan::new("plain ")
                .child(TextSpan::new("bold").bold())
                .child(TextSpan::new(" tail")),
        )
        .base_style(TextStyle::new(16.0))
    }

    fn width(text: &str, style: TextStyle) -> f32 {
        frus_text::measure_resolved(text, &style.resolved()).width
    }

    fn paint_with(rich: &RichText, status: Status) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            rich,
            Rect::new(10.0, 20.0, 300.0, 40.0),
            status,
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    fn highlights(primitives: &[Primitive]) -> Vec<Rect> {
        let selection = Theme::default().selection;
        primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if *color == selection => Some(*rect),
                _ => None,
            })
            .collect()
    }

    /// **A paragraph offers its plain text to a selection**, whatever its runs are drawn in —
    /// and only one made [`selectable`](RichText::selectable) takes focus; none takes typing.
    #[test]
    fn a_paragraph_offers_its_plain_text_to_a_selection() {
        let plain = mixed();
        assert_eq!(
            Widget::<()>::selection_text(&plain),
            Some("plain bold tail")
        );
        assert!(!Widget::<()>::focusable(&plain), "read, not selected alone");
        assert_eq!(Widget::<()>::text_value(&plain), None);
        let empty = RichText::new(TextSpan::new(""));
        assert_eq!(
            Widget::<()>::selection_text(&empty),
            None,
            "nothing to select"
        );

        let selectable = mixed().selectable();
        assert!(Widget::<()>::focusable(&selectable));
        assert!(!Widget::<()>::takes_typing(&selectable));
        assert_eq!(
            Widget::<()>::text_value(&selectable),
            Some("plain bold tail")
        );
    }

    /// **A press lands between the letters it is over, across runs of different sizes**: the
    /// layout is the runs', not the base style's. Past a 40-px letter, a layout at 16 px
    /// would find a character three or four further on.
    #[test]
    fn a_press_lands_where_the_runs_are_drawn() {
        let rich = RichText::new(
            TextSpan::new("ab")
                .child(TextSpan::new("CD").size(40.0))
                .child(TextSpan::new("ef")),
        )
        .base_style(TextStyle::new(16.0));
        let x = width("ab", TextStyle::new(16.0)) + width("C", TextStyle::new(40.0)) + 1.0;
        assert_eq!(Widget::<()>::selection_hit(&rich, x, 20.0, 300.0), Some(3));
        let far = width("ab", TextStyle::new(16.0)) + width("CD", TextStyle::new(40.0)) + 1.0;
        assert_eq!(
            Widget::<()>::selection_hit(&rich, far, 20.0, 300.0),
            Some(4)
        );
    }

    /// **The highlight covers the glyphs selected, in the style they are drawn in**: the bold
    /// run's box starts after the plain run and is as wide as the bold word.
    #[test]
    fn a_selection_over_a_bold_run_is_highlighted_under_it() {
        let status = Status {
            region: Some((6, 10)),
            ..Status::default()
        };
        let rects = highlights(&paint_with(&mixed(), status));
        assert_eq!(rects.len(), 1, "one line: {rects:?}");
        let start = 10.0 + width("plain ", TextStyle::new(16.0));
        let bold = width("bold", TextStyle::new(16.0).weight(FontWeight::Bold));
        assert!((rects[0].x - start).abs() < 1.5, "{rects:?} from {start}");
        assert!((rects[0].width - bold).abs() < 1.5, "{rects:?} {bold} wide");
        // The highlight is under the words, not over them.
        let primitives = paint_with(&mixed(), status);
        let rect = primitives
            .iter()
            .position(|p| matches!(p, Primitive::Rect { .. }));
        let words = primitives
            .iter()
            .position(|p| matches!(p, Primitive::RichText { .. }));
        assert!(rect < words, "{primitives:?}");
        // No selection, no highlight.
        assert!(highlights(&paint_with(&mixed(), Status::default())).is_empty());
    }

    /// **A selection that crosses the runs reads their plain text**, a double click takes the
    /// word inside a run, and its own selection is highlighted only while it has the focus.
    #[test]
    fn a_selectable_paragraph_selects_across_its_runs() {
        let rich = mixed().selectable();
        let edit = Edit {
            cursor: 8,
            anchor: Some(3),
            composing: None,
        };
        assert_eq!(
            Widget::<()>::selected_text(&rich, &edit).as_deref(),
            Some("in bo")
        );
        assert_eq!(Widget::<()>::word_at(&rich, 7), Some((6, 10)), "bold");
        assert_eq!(Widget::<()>::word_at(&mixed(), 7), None, "not alone");

        let own = Status {
            selection: Some((3, 8)),
            ..Status::default()
        };
        assert!(highlights(&paint_with(&rich, own)).is_empty(), "unfocused");
        let focused = Status {
            focused: true,
            ..own
        };
        assert_eq!(highlights(&paint_with(&rich, focused)).len(), 1);
        // The same status on a paragraph that is not selectable draws nothing.
        assert!(highlights(&paint_with(&mixed(), focused)).is_empty());
    }

    /// **A touch selection's handles hang from the paragraph**: a selection area's two ends,
    /// painted after the words.
    #[test]
    fn the_handles_of_an_area_hang_from_a_paragraph() {
        let paths = |status: Status| {
            paint_with(&mixed(), status)
                .iter()
                .filter(|p| matches!(p, Primitive::Path { .. }))
                .count()
        };
        assert_eq!(paths(Status::default()), 0);
        let grips = Status {
            region: Some((6, 10)),
            region_grips: (Some(6), Some(10)),
            ..Status::default()
        };
        assert_eq!(paths(grips), 2);
        assert!(Widget::<()>::selection_grip(&mixed(), 300.0, 6, true).is_some());
    }

    /// **A hit follows the reader's font setting the paragraph was painted under**, though it
    /// is asked outside any frame.
    #[test]
    fn a_hit_follows_the_setting_it_was_painted_under() {
        let rich = RichText::new(TextSpan::new("abcdefgh")).base_style(TextStyle::new(16.0));
        frus_core::with_text_scale(2.0, || {
            paint_with(&rich, Status::default());
        });
        let x = frus_core::with_text_scale(2.0, || width("abc", TextStyle::new(16.0)));
        assert_eq!(Widget::<()>::selection_hit(&rich, x, 10.0, 300.0), Some(3));
    }
    /// **A centred paragraph is highlighted and hit where its runs are drawn** (milestone
    /// 742): the bold run's box is the bold word's, half the room left over in from the edge.
    #[test]
    fn a_centred_paragraph_is_selected_where_it_is_drawn() {
        let rich = mixed().align(TextAlign::Center);
        let plain = TextStyle::new(16.0);
        let bold = plain.weight(FontWeight::Bold);
        let natural = width("plain ", plain) + width("bold", bold) + width(" tail", plain);
        let left = (300.0 - natural) / 2.0;
        let status = Status {
            region: Some((6, 10)),
            ..Status::default()
        };
        let rects = highlights(&paint_with(&rich, status));
        assert_eq!(rects.len(), 1, "{rects:?}");
        let start = 10.0 + left + width("plain ", plain);
        assert!((rects[0].x - start).abs() < 1.5, "{rects:?} from {start}");
        assert!((rects[0].width - width("bold", bold)).abs() < 1.5);
        // A press on the "o" of "bold" lands in it.
        let x = left + width("plain ", plain) + width("b", bold) + 1.0;
        assert_eq!(Widget::<()>::selection_hit(&rich, x, 10.0, 300.0), Some(7));
        // The handles hang from where the word is.
        let handle = Widget::<()>::selection_grip(&rich, 300.0, 6, true).expect("a handle");
        assert!(
            (handle.line_center.x - (left + width("plain ", plain))).abs() < 1.5,
            "{handle:?}"
        );
    }

    /// An **end-aligned** paragraph's last run is highlighted against the right edge.
    #[test]
    fn an_end_aligned_paragraph_ends_at_the_edge() {
        let rich = mixed().align(TextAlign::End);
        let status = Status {
            region: Some((10, 15)),
            ..Status::default()
        };
        let rects = highlights(&paint_with(&rich, status));
        assert_eq!(rects.len(), 1, "{rects:?}");
        assert!(
            (rects[0].x + rects[0].width - 310.0).abs() < 1.0,
            "{rects:?}"
        );
        assert_eq!(
            Widget::<()>::selection_hit(&rich, 299.0, 10.0, 300.0),
            Some(15)
        );
    }

    /// **A paragraph a line limit cuts short** is highlighted only on the line drawn, a hit
    /// below it is the end of its words, and its own selection copies all of them.
    #[test]
    fn a_cut_paragraph_is_highlighted_as_drawn_and_copied_whole() {
        let words = "one two three four five six seven eight nine ten eleven twelve";
        let rich =
            RichText::new(TextSpan::new("one two ").child(TextSpan::new(&words[8..]).bold()))
                .base_style(TextStyle::new(16.0))
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis)
                .selectable();
        let len = words.chars().count();
        let status = Status {
            region: Some((0, len)),
            ..Status::default()
        };
        let rects = highlights(&paint_with(&rich, status));
        assert_eq!(rects.len(), 1, "one line drawn: {rects:?}");
        assert!(rects[0].x + rects[0].width <= 310.0 + 0.5, "{rects:?}");
        assert_eq!(
            Widget::<()>::selection_hit(&rich, 5.0, 200.0, 300.0),
            Some(len)
        );
        let edit = Edit {
            cursor: len,
            anchor: Some(0),
            composing: None,
        };
        assert_eq!(
            Widget::<()>::selected_text(&rich, &edit).as_deref(),
            Some(words)
        );
    }
}

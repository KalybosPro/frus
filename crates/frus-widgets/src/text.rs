//! [`Text`]: a widget that displays a line of text.

use frus_core::{
    Color, FontWeight, MaskShader, Point, Rect, Scene, ShaderMask, TextAlign, TextBlock,
    TextOverflow, TextStyle,
};
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
    /// Whether the text wraps at the width it is given. **On** by default, as in the
    /// reference: a piece of prose put in a box narrower than itself is a paragraph.
    wrap: bool,
    /// What becomes of text that does not fit its box.
    overflow: TextOverflow,
    /// At most this many lines; the rest is dropped and `overflow` decides how the last
    /// one ends.
    max_lines: Option<usize>,
    /// Where the lines sit inside the box.
    align: TextAlign,
    /// Whether this text is willing to be given **less than it asked for**.
    ///
    /// It is not the same question as what happens when it overflows, and conflating the
    /// two would change every text in the framework: a flex item's automatic minimum size
    /// is its content, so a plain text refuses to shrink and its siblings are pushed out
    /// instead. Saying what to do on overflow is what says it may be squeezed.
    shrinkable: bool,
}

/// The separator the fitted lines are joined with before being handed to the renderer.
/// They were broken where the shaper chose to break them, and an explicit newline is what
/// keeps them broken there.
pub(crate) const NEWLINE: &str = "\n";

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

/// `line` cut so that it and an ellipsis together fit `max_width` — **always** ending in
/// one, where [`truncate`] leaves a line that already fits alone.
///
/// The difference is which question is being asked. `truncate` asks whether this line
/// fits; this asks how to end a line that is being cut because the *next* one was
/// dropped, and there the line usually does fit and still has to say so.
pub(crate) fn ellipsise(line: &str, style: &TextStyle, max_width: f32) -> String {
    let measure =
        |text: &str| frus_text::measure_styled(text, style.size, style.weight, style.italic).width;
    let mut chars: Vec<char> = line.trim_end().chars().collect();
    loop {
        let kept: String = chars.iter().collect();
        let candidate = format!("{}{ELLIPSIS}", kept.trim_end());
        if chars.is_empty() || max_width <= 0.0 || measure(&candidate) <= max_width {
            return candidate;
        }
        chars.pop();
    }
}

/// What a text came to after being fitted to its box: what to draw, and which way it ran
/// over — the two are different questions, and a fade needs to know which.
pub(crate) struct Fitted {
    /// The text to draw, with the dropped lines gone and any ellipsis already in it.
    pub text: String,
    /// The last line is wider than the box.
    pub too_wide: bool,
    /// Lines were dropped off the bottom.
    pub too_tall: bool,
}

impl Fitted {
    /// Whether anything ran over at all.
    fn over(&self) -> bool {
        self.too_wide || self.too_tall
    }
}

impl Text {
    /// Creates a text (16 px, regular weight, the theme's color by default).
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: TextStyle::new(16.0),
            ..Self::defaults()
        }
    }

    /// Creates a text from a full [`TextStyle`] — typically one step of the theme's
    /// scale (`Text::styled("Title", theme.text.title_large)`).
    pub fn styled(content: impl Into<String>, style: TextStyle) -> Self {
        Self {
            content: content.into(),
            style,
            ..Self::defaults()
        }
    }

    /// The settings every constructor starts from, so that adding one does not mean
    /// touching each of them.
    fn defaults() -> Self {
        Self {
            content: String::new(),
            style: TextStyle::new(16.0),
            wrap: true,
            overflow: TextOverflow::Clip,
            max_lines: None,
            align: TextAlign::Start,
            shrinkable: false,
        }
    }

    /// Wraps at the width the parent offers. This is the default, and the call is kept
    /// because saying so at the call site is not redundant when it is the whole point of
    /// the widget being there.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Keeps the text on **one line**, explicit newlines aside. It then runs past its box
    /// rather than folding, and [`Text::overflow`] decides what becomes of the part that
    /// hangs over.
    pub fn no_wrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    /// Whether the text wraps at the width it is given. `wrap()` is `soft_wrap(true)`.
    ///
    /// Off, the text stays on one line — explicit newlines aside — and runs past its box
    /// or is cut, according to [`Text::overflow`].
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.wrap = soft_wrap;
        self
    }

    /// Where the lines sit **inside the box** the text was given.
    ///
    /// It does nothing to a text that shrink-wraps, and that is not a limitation of this
    /// implementation: a box exactly as wide as its text has nowhere to align it to. It
    /// takes effect once something has made the box wider — a stretched column, an
    /// [`crate::Expanded`], a width.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// At most `max_lines` lines; what is past them is dropped, and [`Text::overflow`]
    /// decides how the last one ends.
    ///
    /// Asking for a limit also tells the layout this text may be given **less width than
    /// it asked for** — a limit is only meaningful for a text expected to be squeezed.
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = Some(max_lines.max(1));
        self.shrinkable = true;
        self
    }

    /// What becomes of text that does not fit: cut, cut with an ellipsis, faded out, or
    /// drawn past the box.
    ///
    /// Saying so also tells the layout this text may be given **less than it asked for**.
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self.shrinkable = true;
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
        self.wrap = false;
        self.overflow = TextOverflow::Ellipsis;
        self.shrinkable = true;
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

    /// Switches to italic.
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

impl Text {
    /// The text that actually fits `width`, and which way it ran over.
    ///
    /// Everything the box does to the words happens here: dropping the lines past the
    /// limit, cutting the last one, deciding whether a line ran past the edge. The paint
    /// draws what comes back, and the overflow mode decides how.
    pub(crate) fn fitted(&self, width: f32) -> Fitted {
        let (size, weight, italic) = (self.style.size, self.style.weight, self.style.italic);
        // Only a line **limit** makes the words the widget's business. Left alone, the
        // text goes to the renderer whole and is broken there — which matters beyond the
        // shaping saved: a paragraph handed over as lines is a paragraph *per line*, and
        // rules that span a paragraph (a justified block leaves its last line ragged) no
        // longer know where it ends.
        let (mut lines, too_tall) = if self.max_lines.is_some() {
            frus_text::visual_lines(
                &self.content,
                size,
                weight,
                italic,
                Some(width),
                self.wrap,
                self.max_lines,
            )
        } else {
            (vec![self.content.clone()], false)
        };
        // A line can only be wider than the box when nothing may push it onto the next
        // one. Where the text wraps, every line fits by construction.
        let too_wide = !self.wrap
            && lines.last().is_some_and(|line| {
                frus_text::measure_styled(line, size, weight, italic).width > width + 0.5
            });
        if (too_tall || too_wide) && self.overflow == TextOverflow::Ellipsis {
            if let Some(last) = lines.last_mut() {
                *last = ellipsise(last, &self.style, width);
            }
        }
        Fitted {
            text: lines.join(NEWLINE),
            too_wide,
            too_tall,
        }
    }

    /// Whether this text needs the **whole width it is offered** rather than its own.
    ///
    /// Alignment is the only thing that does. A box exactly as wide as its text has
    /// nowhere to align it to, so a text asked to centre itself and then given its own
    /// width would be centred and look untouched — the setting would appear to do nothing
    /// and there would be nothing to see. Left alone, a text still shrink-wraps.
    fn fills(&self) -> bool {
        self.align != TextAlign::Start
    }

    /// A line limit is a **height** cap and nothing else: the words break where they
    /// broke, and the ones past the limit are not drawn.
    fn capped(&self, height: f32) -> f32 {
        match self.max_lines {
            Some(max) => height.min(frus_text::line_height(self.style.size) * max as f32),
            None => height,
        }
    }

    /// The fade that ends a cut text: opaque until the last stretch of the box, then out
    /// to nothing at the edge it ran past.
    fn fade(&self, bounds: Rect, horizontal: bool) -> ShaderMask {
        // A fifth of the box, and never more than three line heights of it. Over a long
        // line a proportional fade would start halfway through words that are perfectly
        // legible; over a short one an absolute fade would swallow the lot.
        let extent = if horizontal {
            bounds.width
        } else {
            bounds.height
        };
        let run = (extent * 0.2).min(frus_text::line_height(self.style.size) * 3.0);
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
}

impl<Msg> Widget<Msg> for Text {
    fn style(&self) -> Style {
        // A flex item's automatic minimum size is its content, so a plain text refuses to
        // shrink and pushes its siblings out instead. One that has said what to do when it
        // overflows may be given less; the paint fits the words to whatever it gets.
        let min_width = if self.shrinkable {
            Dimension::Length(0.0)
        } else {
            Dimension::Auto
        };
        // A paragraph, or a text that has to know how wide its box is: free dimensions,
        // and the size comes from `measure` — the only way a box can be *given* a width
        // and answer with a height.
        if self.wrap || self.fills() {
            return Style {
                min_width,
                ..Default::default()
            };
        }
        // A single line is a box of a known size, and saying so is what keeps it from
        // being folded: a measured leaf reports its narrowest useful width as its
        // minimum content, and a row would take that as leave to squeeze it there.
        let measured = frus_text::measure_styled(
            &self.content,
            self.style.size,
            self.style.weight,
            self.style.italic,
        );
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(self.capped(measured.height).ceil()),
            min_width,
            // A text that has said what to do when it overflows is **clamped to its
            // parent**, which is what the reference does to every one of them: a
            // paragraph is laid out at `constraints.constrain(its own size)`. Without it a
            // text declares the width it wants, a narrower box does not take it away, and
            // the overflow mode never fires — the words simply draw past the edge, which
            // is the behaviour it was set to prevent.
            max_width: if self.shrinkable {
                Dimension::Percent(1.0)
            } else {
                Dimension::Auto
            },
            ..Default::default()
        }
    }

    /// A text will not be **squeezed along a row**: it runs past the end of one rather
    /// than being folded into a column of single words, which is the reference's rule and
    /// was, until now, the only thing a declared width was doing here.
    ///
    /// A text that has said what to do when it overflows has already said the opposite,
    /// and is left alone.
    fn main_axis_floor(&self) -> Option<f32> {
        // A single line already carries its width in its style; only a text measured
        // under constraints needs to be told where to stop giving way.
        if self.shrinkable || !(self.wrap || self.fills()) {
            return None;
        }
        Some(
            frus_text::measure_styled(
                &self.content,
                self.style.size,
                self.style.weight,
                self.style.italic,
            )
            .width
            .ceil(),
        )
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
        if !self.wrap && !self.fills() {
            return None;
        }
        let content = self.content.clone();
        let style = self.style;
        let max_lines = self.max_lines;
        let wrap = self.wrap;
        Some(Box::new(move |max_width, _| {
            let mut size = frus_text::measure_wrapped(
                &content,
                style.size,
                style.weight,
                style.italic,
                // A text that only wanted the width to align inside is still one line:
                // the constraint tells it where its box ends, not where to break.
                if wrap { max_width } else { None },
            );
            if let Some(max) = max_lines {
                size.height = size
                    .height
                    .min(frus_text::line_height(style.size) * max as f32);
            }
            size
        }))
    }

    fn measure_key(&self) -> Option<u64> {
        if !self.wrap && !self.fills() {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.content.hash(&mut hasher);
        self.style.size.to_bits().hash(&mut hasher);
        self.style.weight.to_u16().hash(&mut hasher);
        self.style.italic.hash(&mut hasher);
        self.max_lines.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    /// An aligned text takes the width its parent offers: a box exactly as wide as its
    /// text has nowhere to align it to. It is the same request a [`crate::Row`] makes,
    /// and it is answered by the same walk.
    fn main_axis_fill(&self) -> Option<frus_layout::FlexDirection> {
        self.fills().then_some(frus_layout::FlexDirection::Row)
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self
            .style
            .color
            .unwrap_or(theme.on_surface)
            .fade(status.opacity);
        let fitted = self.fitted(bounds.width);
        let block = TextBlock {
            // A width is handed over only when something is going to use it. Giving the
            // renderer one it did not have changes where right-to-left text lands, which
            // is a bug this codebase has already had once.
            width: (self.wrap || self.align != TextAlign::Start).then_some(bounds.width),
            soft_wrap: self.wrap,
            align: self.align,
        };
        let draw = |scene: &mut Scene| {
            scene.text_block(
                Point::new(bounds.x, bounds.y),
                fitted.text.clone(),
                &self.style,
                color,
                block,
            );
        };
        // Nothing hanging over the edge, or already cut to size, or told to spill: the
        // three cases where the mode has nothing left to do.
        if !fitted.over()
            || self.overflow == TextOverflow::Visible
            || self.overflow == TextOverflow::Ellipsis
        {
            draw(scene);
            return;
        }
        // Only where it genuinely does not fit: a clip around every text would put a hard
        // edge through the antialiasing of every one that does.
        let outer = scene.current_clip();
        scene.set_clip(outer.intersect(bounds));
        match self.overflow {
            TextOverflow::Fade => {
                let horizontal = fitted.too_wide && !fitted.too_tall;
                scene.masked(self.fade(bounds, horizontal), draw);
            }
            _ => draw(scene),
        }
        scene.set_clip(outer);
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
        let plain =
            Widget::<()>::style(&Text::new("A rather long label indeed").size(18.0).no_wrap());
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
    use frus_core::{MaskShader, TextAlign, TextOverflow};

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

    /// A line limit drops what is past it and, asked for an ellipsis, says so on the
    /// last line it kept.
    #[test]
    fn a_line_limit_cuts_the_paragraph_and_marks_the_cut() {
        let long = "one two three four five six seven eight nine ten eleven twelve";
        let text = Text::new(long)
            .size(14.0)
            .wrap()
            .max_lines(2)
            .overflow(TextOverflow::Ellipsis);
        let fitted = text.fitted(90.0);
        assert_eq!(
            fitted.text.lines().count(),
            2,
            "two lines: {:?}",
            fitted.text
        );
        assert!(fitted.too_tall, "and there was more");
        assert!(
            fitted.text.ends_with(ELLIPSIS),
            "ending in an ellipsis: {:?}",
            fitted.text
        );
    }

    /// The same paragraph with room for every line is left exactly as it was written.
    #[test]
    fn a_limit_it_does_not_reach_changes_nothing() {
        let text = Text::new("one two").size(14.0).wrap().max_lines(4);
        let fitted = text.fitted(400.0);
        assert_eq!(fitted.text, "one two");
        assert!(!fitted.too_tall && !fitted.too_wide);
    }

    /// A limit is a height cap: the words still wrap where they wrapped, and the box
    /// stops at the line it was allowed.
    #[test]
    fn a_limit_caps_the_measured_height() {
        let long = "one two three four five six seven eight nine ten eleven twelve";
        let free = Text::new(long).size(14.0).wrap();
        let capped = Text::new(long).size(14.0).wrap().max_lines(2);
        let at = |t: &Text| Widget::<()>::measure(t).expect("measure")(Some(90.0), None).height;
        assert!(at(&free) > at(&capped), "the limit is doing something");
        assert!(
            (at(&capped) - frus_text::line_height(14.0) * 2.0).abs() < 0.5,
            "two lines exactly: {}",
            at(&capped)
        );
    }

    /// `Visible` draws past the box; `Clip` puts the box in the primitive's clip so the
    /// renderer stops at the edge. Both draw the whole string — the difference is the
    /// clip, not the text.
    #[test]
    fn clip_and_visible_differ_by_the_clip_and_not_the_words() {
        let long = "a label far too long for the box it was given";
        let paint = |overflow: TextOverflow| {
            let text = Text::new(long).size(16.0).no_wrap().overflow(overflow);
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &text,
                Rect::new(0.0, 0.0, 60.0, 20.0),
                Status::default(),
                &Theme::default(),
                &mut scene,
            );
            match &scene.primitives()[0] {
                Primitive::Text { text, clip, .. } => (text.clone(), *clip),
                other => panic!("a text, not {other:?}"),
            }
        };
        let (visible_text, visible_clip) = paint(TextOverflow::Visible);
        let (clipped_text, clipped_clip) = paint(TextOverflow::Clip);
        assert_eq!(visible_text, long, "nothing is cut");
        assert_eq!(clipped_text, long, "nothing is cut here either");
        assert_eq!(visible_clip, Rect::UNBOUNDED, "and nothing stops it");
        assert_eq!(
            clipped_clip,
            Rect::new(0.0, 0.0, 60.0, 20.0),
            "this one stops"
        );
    }

    /// A text that fits is never clipped, whatever it asked for: a clip on every text
    /// would put a hard edge through the antialiasing of every one of them.
    #[test]
    fn a_text_that_fits_is_not_clipped() {
        let text = Text::new("short")
            .size(16.0)
            .no_wrap()
            .overflow(TextOverflow::Clip);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, 300.0, 20.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Text { clip, .. } => assert_eq!(*clip, Rect::UNBOUNDED),
            other => panic!("a text, not {other:?}"),
        }
    }

    /// `Fade` wraps the text in a masked group — the fade has to be a group, or the two
    /// halves of an overlapping glyph would fade against the background separately.
    #[test]
    fn fade_wraps_the_text_in_a_masked_group() {
        let text = Text::new("a label far too long for the box it was given")
            .size(16.0)
            .no_wrap()
            .overflow(TextOverflow::Fade);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, 60.0, 20.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Layer {
                primitives, filter, ..
            } => {
                assert_eq!(primitives.len(), 1, "the text, and only the text");
                let mask = filter.mask.expect("a mask");
                match mask.shader {
                    // The fade runs to the right edge of the box, horizontally, because
                    // that is the edge the line ran past.
                    MaskShader::Linear { from, to, .. } => {
                        assert!(
                            to.x > from.x && (to.x - 60.0).abs() < 0.01,
                            "{from:?} {to:?}"
                        );
                        assert!((to.y - from.y).abs() < 0.01, "horizontal");
                    }
                    other => panic!("a linear fade, not {other:?}"),
                }
            }
            other => panic!("a masked layer, not {other:?}"),
        }
    }

    /// Alignment reaches the primitive, and with it the width it aligns inside — which a
    /// start-aligned text is deliberately not given.
    #[test]
    fn alignment_hands_the_renderer_a_width_and_start_does_not() {
        let painted = |align: TextAlign| {
            let text = Text::new("x").size(16.0).no_wrap().align(align);
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &text,
                Rect::new(0.0, 0.0, 200.0, 20.0),
                Status::default(),
                &Theme::default(),
                &mut scene,
            );
            match &scene.primitives()[0] {
                Primitive::Text {
                    align, max_width, ..
                } => (*align, *max_width),
                other => panic!("a text, not {other:?}"),
            }
        };
        assert_eq!(painted(TextAlign::Center), (TextAlign::Center, Some(200.0)));
        assert_eq!(painted(TextAlign::Start), (TextAlign::Start, None));
    }

    /// Saying what to do on overflow is what tells the layout the text may be squeezed;
    /// a plain one still refuses, which is what keeps a row from losing its last widget.
    #[test]
    fn an_overflow_mode_is_what_makes_a_text_shrinkable() {
        use frus_layout::Dimension;
        let plain = Widget::<()>::style(&Text::new("a long enough label"));
        let clipped =
            Widget::<()>::style(&Text::new("a long enough label").overflow(TextOverflow::Clip));
        assert_eq!(plain.min_width, Dimension::Auto);
        assert_eq!(clipped.min_width, Dimension::Length(0.0));
    }

    #[test]
    fn text_paints_a_text_primitive() {
        let text = Text::new("Salut")
            .size(20.0)
            .no_wrap()
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
                soft_wrap: false,
                align: TextAlign::Start,
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
        // A text that does not wrap is a box of a known size, and says so in its style
        // rather than through a measurement: a measured leaf reports its narrowest useful
        // width as its minimum content, and a row would take that as leave to fold it.
        let plain = Text::new(long).no_wrap();
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
        let w = |text: &Text| Widget::<()>::measure(text).expect("measure")(None, None).width;
        assert!(
            w(&Text::new("Width").weight(FontWeight::Bold)) > w(&Text::new("Width")),
            "bold must be wider"
        );
    }

    /// The default is the reference's: prose put in a box narrower than itself is a
    /// paragraph. Along a **row** it is not — a text runs past the end of one rather than
    /// folding into a column of single words — and that is the floor the parent applies.
    #[test]
    fn text_wraps_down_a_column_and_runs_on_along_a_row() {
        use crate::{build_ui, Container, Flex, Runtime, Size};
        let long = "one two three four five six seven eight nine ten eleven twelve";
        // The painted box, read from the scene: the text primitive carries the box it
        // was laid out in, which is the number this is about.
        let painted = |root: &dyn Widget<()>| {
            let rt = Runtime::default();
            let theme = Theme::default();
            let ui = build_ui(root, Size::new(120.0, 300.0), &rt, &theme);
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Text { bounds, .. } => Some(*bounds),
                    _ => None,
                })
                .expect("the text")
        };
        let column: Container<()> = Container::new()
            .width(120.0)
            .child(Flex::column().child(Text::new(long).size(12.0)));
        // Two children, because a box with a single child is handing a width down rather
        // than dividing a line up — and there a paragraph does wrap.
        let row: Container<()> = Container::new().width(120.0).child(
            Flex::row()
                .child(Text::new(long).size(12.0))
                .child(Container::new().width(10.0).height(10.0)),
        );
        let tall = painted(&column);
        let wide = painted(&row);
        assert!(
            tall.width <= 120.5,
            "the column gave it 120: {}",
            tall.width
        );
        assert!(tall.height > 20.0, "so it wrapped: {}", tall.height);
        assert!(wide.width > 120.0, "the row did not: {}", wide.width);
    }
}

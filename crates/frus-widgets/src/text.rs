//! [`Text`]: a widget that displays a line of text.

use frus_core::{
    Color, FontWeight, MaskShader, Point, Rect, ResolvedTextStyle, Scene, ShaderMask, TextAlign,
    TextBlock, TextOverflow, TextStyle,
};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::widgettheme::DefaultTextStyle;

/// A single-line text widget.
///
/// Its layout size comes from a **styled** measurement (`frus-text`, weight and
/// italic included); it pushes a text primitive into the scene when painting. The
/// [`TextStyle`]'s color is inherited from the theme when absent.
pub struct Text {
    content: String,
    style: TextStyle,
    /// Whether the text wraps at the width it is given. Unset, **on**, as in the
    /// reference: a piece of prose put in a box narrower than itself is a paragraph.
    wrap: Option<bool>,
    /// What becomes of text that does not fit its box. Unset, clipped.
    overflow: Option<TextOverflow>,
    /// At most this many lines; the rest is dropped and `overflow` decides how the last
    /// one ends.
    max_lines: Option<usize>,
    /// Where the lines sit inside the box. Unset, at the start.
    align: Option<TextAlign>,
    /// Whether this text is a **heading** — a landmark a screen reader's user can jump
    /// to, rather than one more piece of prose. It changes nothing that is drawn.
    heading: bool,
    /// Whether this text is willing to be given **less than it asked for**.
    ///
    /// It is not the same question as what happens when it overflows, and conflating the
    /// two would change every text in the framework: a flex item's automatic minimum size
    /// is its content, so a plain text refuses to shrink and its siblings are pushed out
    /// instead. Saying what to do on overflow is what says it may be squeezed — and an
    /// **inherited** limit says it just as much as a called one, which is why this is
    /// derived at resolution rather than stored.
    shrinkable: bool,
}

/// A [`Text`]'s questions, all answered: what the caller said where they said it, what the
/// subtree's [`DefaultTextStyle`](crate::DefaultTextStyle) said where they did not, and
/// the framework's own default where neither did.
///
/// Everything the widget does — the width it asks for, the words it keeps, the glyphs it
/// draws — reads from one of these, and every hook builds it the same way. That is not
/// tidiness: a size resolved one way for the measurement and another way for the paint is
/// a layout that is wrong everywhere at once, with nothing in the picture to say which of
/// the two numbers was the mistake.
pub(crate) struct Resolved {
    pub style: ResolvedTextStyle,
    pub align: TextAlign,
    pub wrap: bool,
    pub overflow: TextOverflow,
    pub max_lines: Option<usize>,
    /// Whether this text is willing to be given less than it asked for — which an
    /// inherited overflow or line limit says just as much as a called one.
    pub shrinkable: bool,
}

/// The ellipsis a cut line ends in.
pub(crate) const ELLIPSIS: &str = "…";

/// The longest prefix of `content` that fits in `max_width`, ending in an ellipsis when
/// anything was cut. Returns `content` untouched when it already fits.
///
/// A box with **no room at all** gets an ellipsis and nothing else. It used to get the
/// whole string, on the reasoning that a zero width meant "the layout has not run yet"
/// rather than "there is no room" — inherited from the app bar, which cannot produce a
/// zero anyway (its title room has a floor of 64 px). What the exception actually did was
/// let a genuinely collapsed box draw its whole label, over whatever was beside it, which
/// is the one thing ellipsising exists to prevent.
///
/// Character by character from the end: the strings this is used on are a line, not a
/// document, and a binary search over char boundaries would buy nothing at that length.
pub(crate) fn truncate(content: &str, style: &ResolvedTextStyle, max_width: f32) -> String {
    let measure = |text: &str| frus_text::measure_resolved(text, style).width;
    if measure(content) <= max_width {
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
pub(crate) fn ellipsise(line: &str, style: &ResolvedTextStyle, max_width: f32) -> String {
    let measure = |text: &str| frus_text::measure_resolved(text, style).width;
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
    /// Creates a text that has **chosen nothing**: it takes its size, weight and colour
    /// from whatever subtree it is put in, and the framework's own where nothing says
    /// otherwise (16 px, regular, the theme's `on_surface`).
    ///
    /// It used to be written as a 16 px style, which read the same and meant something
    /// else: a style that *answered* 16, so an app bar or a section could not dress it.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Self::defaults()
        }
    }

    /// Creates a text from a full [`TextStyle`] — typically one step of the theme's
    /// scale (`Text::styled("Title", theme.text.title_large)`).
    /// The style is taken **exactly as written**: whatever it names it answers, and
    /// whatever it leaves open stays open to a subtree.
    ///
    /// So `Text::styled(s, theme.text.title_medium)` wears that step of the scale and
    /// still picks up a colour from the section it sits in, while
    /// `Text::styled(s, TextStyle::new(20.0))` fixes the size alone and inherits the rest.
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
            style: TextStyle::NONE,
            wrap: None,
            overflow: None,
            max_lines: None,
            align: None,
            heading: false,
            shrinkable: false,
        }
    }

    /// Wraps at the width the parent offers. This is the default, and the call is kept
    /// because saying so at the call site is not redundant when it is the whole point of
    /// the widget being there.
    /// Marks this text as a **heading**: a landmark assistive technology can jump
    /// between, instead of one more run of prose.
    ///
    /// It changes nothing that is drawn. A screen reader's user moves through a screen by
    /// its headings, and a title announced as a label gives them nothing to move between
    /// — which is what an app bar's title was until milestone 397. The reference says the
    /// same thing with `SemanticsProperties(header: true)` around its title.
    pub fn heading(mut self) -> Self {
        self.heading = true;
        self
    }

    pub fn wrap(mut self) -> Self {
        self.wrap = Some(true);
        self
    }

    /// Keeps the text on **one line**, explicit newlines aside. It then runs past its box
    /// rather than folding, and [`Text::overflow`] decides what becomes of the part that
    /// hangs over.
    pub fn no_wrap(mut self) -> Self {
        self.wrap = Some(false);
        self
    }

    /// Whether the text wraps at the width it is given. `wrap()` is `soft_wrap(true)`.
    ///
    /// Off, the text stays on one line — explicit newlines aside — and runs past its box
    /// or is cut, according to [`Text::overflow`].
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.wrap = Some(soft_wrap);
        self
    }

    /// Where the lines sit **inside the box** the text was given.
    ///
    /// It does nothing to a text that shrink-wraps, and that is not a limitation of this
    /// implementation: a box exactly as wide as its text has nowhere to align it to. It
    /// takes effect once something has made the box wider — a stretched column, an
    /// [`crate::Expanded`], a width.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = Some(align);
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
        self.overflow = Some(overflow);
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
        self.wrap = Some(false);
        self.overflow = Some(TextOverflow::Ellipsis);
        self.shrinkable = true;
        self
    }

    /// Sets the font size, in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.style.size = Some(size);
        self
    }

    /// Sets the weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.style.weight = Some(weight);
        self
    }

    /// Switches to italic.
    pub fn italic(mut self) -> Self {
        self.style.italic = Some(true);
        self
    }

    /// Sets the line height as a **multiple of the font size** — the reference's
    /// [`TextStyle::height`](frus_core::TextStyle::height).
    ///
    /// `1.0` packs the lines to exactly the type's size, `1.6` opens a paragraph up.
    /// Unset, the line height is whatever a surrounding [`crate::DefaultTextStyle`] said,
    /// or [`DEFAULT_LINE_HEIGHT`](frus_core::DEFAULT_LINE_HEIGHT) at the end of the chain.
    ///
    /// A ratio and not a length, so the leading grows with the letters when a reader turns
    /// the type up instead of staying where it was set.
    pub fn height(mut self, height: f32) -> Self {
        self.style.height = Some(height);
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
        self.style.decoration = Some(decoration);
        self
    }

    /// Sets the decoration color (otherwise the text's).
    pub fn decoration_color(mut self, color: Color) -> Self {
        self.style.decoration_color = Some(color);
        self
    }
}

impl Text {
    /// Every question this text asks of itself, answered once: **what the caller said ??
    /// what the subtree handed down ?? what the framework ships**.
    ///
    /// `theme` is an `Option` because [`Widget::style`] has no theme to give — a widget
    /// asked for its style outside a walk. `None` means nothing was handed down, which is
    /// exactly what a fresh theme carries, so the two agree and the unthemed case costs
    /// nothing.
    ///
    /// The typography is one `merge` and one `resolved`, and it was fourteen lines of
    /// bookkeeping until [`TextStyle`]'s fields could each say *unset*. A style that never
    /// named a size does not outrank a subtree that did; one that named it keeps it. The
    /// same rule as before, now expressed by the type instead of shadowed by a flag.
    pub(crate) fn resolved(&self, theme: Option<&Theme>) -> Resolved {
        let handed = theme.map_or(DefaultTextStyle::NONE, |t| t.widgets.text);
        Resolved {
            style: handed.style.merge(self.style).resolved(),
            align: self.align.or(handed.align).unwrap_or(TextAlign::Start),
            wrap: self.wrap.or(handed.soft_wrap).unwrap_or(true),
            overflow: self
                .overflow
                .or(handed.overflow)
                .unwrap_or(TextOverflow::Clip),
            max_lines: self.max_lines.or(handed.max_lines),
            // An **inherited** overflow or line limit says the same thing a called one
            // does: this text may be given less than it asked for. Leaving it out would
            // ship a subtree that ellipsises its texts and never gets the chance to,
            // because each of them still refuses to be squeezed.
            shrinkable: self.shrinkable
                || (self.overflow.is_none() && handed.overflow.is_some())
                || (self.max_lines.is_none() && handed.max_lines.is_some()),
        }
    }

    /// The text that actually fits `width`, and which way it ran over.
    ///
    /// Everything the box does to the words happens here: dropping the lines past the
    /// limit, cutting the last one, deciding whether a line ran past the edge. The paint
    /// draws what comes back, and the overflow mode decides how.
    pub(crate) fn fitted(&self, width: f32, r: &Resolved) -> Fitted {
        let (size, weight, italic) = (r.style.size, r.style.weight, r.style.italic);
        // Only a line **limit** makes the words the widget's business. Left alone, the
        // text goes to the renderer whole and is broken there.
        //
        // And what goes over is a **prefix**, cut at a break the shaper chose — never a
        // list of lines glued back together. Lines glued back together are a paragraph
        // *per line*, and rules that span a paragraph stop working: a justified block can
        // only leave its last line ragged if it knows which line is the last.
        let mut text = self.content.clone();
        let mut too_tall = false;
        if let Some(max) = r.max_lines {
            let spans =
                frus_text::line_spans(&self.content, size, weight, italic, Some(width), r.wrap);
            if spans.len() > max {
                too_tall = true;
                let cut = spans[max].start;
                let last = spans[max - 1].start;
                text = if r.overflow == TextOverflow::Ellipsis {
                    let ended = ellipsise(&self.content[last..cut], &r.style, width);
                    format!("{}{ended}", &self.content[..last])
                } else {
                    self.content[..cut].to_string()
                };
            }
        }
        // A line can only be wider than the box when nothing may push it onto the next
        // one. Where the text wraps, every line fits by construction.
        let too_wide =
            !r.wrap && frus_text::measure_styled(&text, size, weight, italic).width > width + 0.5;
        if too_wide && !too_tall && r.overflow == TextOverflow::Ellipsis {
            text = ellipsise(&text, &r.style, width);
        }
        Fitted {
            text,
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
    fn fills(r: &Resolved) -> bool {
        r.align != TextAlign::Start
    }

    /// A line limit is a **height** cap and nothing else: the words break where they
    /// broke, and the ones past the limit are not drawn.
    fn capped(height: f32, r: &Resolved) -> f32 {
        match r.max_lines {
            Some(max) => height.min(frus_text::line_height(r.style.size) * max as f32),
            None => height,
        }
    }

    /// The fade that ends a cut text: opaque until the last stretch of the box, then out
    /// to nothing at the edge it ran past.
    fn fade(bounds: Rect, horizontal: bool, r: &Resolved) -> ShaderMask {
        // A fifth of the box, and never more than three line heights of it. Over a long
        // line a proportional fade would start halfway through words that are perfectly
        // legible; over a short one an absolute fade would swallow the lot.
        let extent = if horizontal {
            bounds.width
        } else {
            bounds.height
        };
        let run = (extent * 0.2).min(frus_text::line_height(r.style.size) * 3.0);
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

impl Text {
    /// The box this text asks for, once every question has been answered.
    ///
    /// Split out from the two style hooks rather than duplicated across them: they differ
    /// only in whether a theme was there to be asked, and a second copy of this reasoning
    /// is a second place for the themed and the unthemed answers to drift apart.
    fn boxed(&self, r: &Resolved) -> Style {
        // A flex item's automatic minimum size is its content, so a plain text refuses to
        // shrink and pushes its siblings out instead. One that has said what to do when it
        // overflows may be given less; the paint fits the words to whatever it gets.
        let min_width = if r.shrinkable {
            Dimension::Length(0.0)
        } else {
            Dimension::Auto
        };
        // A paragraph, or a text that has to know how wide its box is: free dimensions,
        // and the size comes from `measure` — the only way a box can be *given* a width
        // and answer with a height.
        if r.wrap || Self::fills(r) {
            return Style {
                min_width,
                ..Default::default()
            };
        }
        // A single line is a box of a known size, and saying so is what keeps it from
        // being folded: a measured leaf reports its narrowest useful width as its
        // minimum content, and a row would take that as leave to squeeze it there.
        let measured = frus_text::measure_resolved(&self.content, &r.style);
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(Self::capped(measured.height, r).ceil()),
            min_width,
            // A text that has said what to do when it overflows is **clamped to its
            // parent**, which is what the reference does to every one of them: a
            // paragraph is laid out at `constraints.constrain(its own size)`. Without it a
            // text declares the width it wants, a narrower box does not take it away, and
            // the overflow mode never fires — the words simply draw past the edge, which
            // is the behaviour it was set to prevent.
            max_width: if r.shrinkable {
                Dimension::Percent(1.0)
            } else {
                Dimension::Auto
            },
            ..Default::default()
        }
    }
}

impl<Msg> Widget<Msg> for Text {
    fn style(&self) -> Style {
        self.boxed(&self.resolved(None))
    }

    /// A subtree can hand down a size, a weight and a line limit, and every one of those
    /// is a **box**, not a colour. Resolving them here rather than at paint is what lets
    /// an app bar make the words inside it smaller and have them take less room, instead
    /// of the same room with smaller writing in it.
    fn style_themed(&self, theme: &Theme) -> Style {
        self.boxed(&self.resolved(Some(theme)))
    }

    /// A text will not be **squeezed along a row**: it runs past the end of one rather
    /// than being folded into a column of single words, which is the reference's rule and
    /// was, until now, the only thing a declared width was doing here.
    ///
    /// A text that has said what to do when it overflows has already said the opposite,
    /// and is left alone.
    fn main_axis_floor(&self, theme: &Theme) -> Option<f32> {
        let r = self.resolved(Some(theme));
        // A single line already carries its width in its style; only a text measured
        // under constraints needs to be told where to stop giving way.
        if r.shrinkable || !(r.wrap || Self::fills(&r)) {
            return None;
        }
        Some(
            frus_text::measure_resolved(&self.content, &r.style)
                .width
                .ceil(),
        )
    }

    /// The line this text sits on, measured from the top of its box. A `Text` is the
    /// widget that actually *has* a baseline; every alignment that talks about one is
    /// ultimately asking one of these.
    fn text_baseline(&self, theme: &Theme) -> Option<f32> {
        let r = self.resolved(Some(theme));
        Some(frus_text::baseline(
            r.style.size,
            r.style.weight,
            r.style.italic,
        ))
    }

    fn measure(&self, theme: &Theme) -> Option<frus_layout::MeasureFn<'_>> {
        let r = self.resolved(Some(theme));
        if !r.wrap && !Self::fills(&r) {
            return None;
        }
        let content = self.content.clone();
        let style = r.style;
        let max_lines = r.max_lines;
        let wrap = r.wrap;
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

    fn measure_key(&self, theme: &Theme) -> Option<u64> {
        let r = self.resolved(Some(theme));
        if !r.wrap && !Self::fills(&r) {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.content.hash(&mut hasher);
        // The **resolved** style, not the written one: an inherited size changes the
        // measurement, and a key that ignored it would hand back the geometry the text had
        // before the subtree said anything.
        r.style.size.to_bits().hash(&mut hasher);
        r.style.weight.to_u16().hash(&mut hasher);
        r.style.italic.hash(&mut hasher);
        r.max_lines.hash(&mut hasher);
        r.wrap.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    /// An aligned text takes the width its parent offers: a box exactly as wide as its
    /// text has nowhere to align it to. It is the same request a [`crate::Row`] makes,
    /// and it is answered by the same walk.
    fn fill_axes(&self, theme: &Theme) -> crate::widget::FillAxes {
        if Self::fills(&self.resolved(Some(theme))) {
            crate::widget::FillAxes::WIDTH
        } else {
            crate::widget::FillAxes::NONE
        }
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let r = self.resolved(Some(theme));
        let color = r
            .style
            .color
            .unwrap_or(theme.on_surface)
            .fade(status.opacity);
        let fitted = self.fitted(bounds.width, &r);
        let block = TextBlock {
            // A width is handed over only when something is going to use it. Giving the
            // renderer one it did not have changes where right-to-left text lands, which
            // is a bug this codebase has already had once.
            width: (r.wrap || r.align != TextAlign::Start).then_some(bounds.width),
            soft_wrap: r.wrap,
            align: r.align,
        };
        let draw = |scene: &mut Scene| {
            scene.text_block(
                Point::new(bounds.x, bounds.y),
                fitted.text.clone(),
                &r.style,
                color,
                block,
            );
        };
        // Nothing hanging over the edge, or already cut to size, or told to spill: the
        // three cases where the mode has nothing left to do.
        if !fitted.over()
            || r.overflow == TextOverflow::Visible
            || r.overflow == TextOverflow::Ellipsis
        {
            draw(scene);
            return;
        }
        // Only where it genuinely does not fit: a clip around every text would put a hard
        // edge through the antialiasing of every one that does.
        let outer = scene.current_clip();
        scene.set_clip(outer.intersect(bounds));
        match r.overflow {
            TextOverflow::Fade => {
                let horizontal = fitted.too_wide && !fitted.too_tall;
                scene.masked(Self::fade(bounds, horizontal, &r), draw);
            }
            _ => draw(scene),
        }
        scene.set_clip(outer);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // A text carries its content as its accessible label — or as a **heading**, which
        // is a landmark rather than a label and is what a screen reader's user navigates by.
        let role = if self.heading {
            frus_core::Role::Heading
        } else {
            frus_core::Role::Label
        };
        Some(frus_core::SemanticsProperties::new(role).label(self.content.clone()))
    }
}

#[cfg(test)]
mod tests {
    /// An ellipsising text is **cut to the box it is given** instead of drawing past it.
    ///
    /// It also tells the layout it may be given less than it asked for (`min_width: 0`),
    /// which is half of what a row needs to keep its trailing widget intact. The other
    /// half arrived in milestone 349: a widget that did not ask to give way is not
    /// squeezed, so the trailing button keeps its width without saying anything.
    #[test]
    fn an_ellipsising_text_is_cut_to_its_box() {
        let long = "A task name that is really rather long indeed and keeps going";
        let style = TextStyle::new(18.0);
        let full = frus_text::measure_style(long, style).width;
        assert!(full > 300.0, "the fixture has to overflow: {full}");

        let cut = truncate(long, &style.resolved(), 150.0);
        assert!(cut.ends_with(ELLIPSIS), "cut with an ellipsis: {cut}");
        let cut_width = frus_text::measure_style(&cut, style).width;
        assert!(cut_width <= 150.0, "and it fits: {cut_width}");
        assert!(cut.len() > 4, "without cutting everything: {cut}");

        // A width it already fits in leaves it alone, ellipsis and all.
        assert_eq!(truncate("Short", &style.resolved(), 500.0), "Short");
        // A box with no room at all draws an ellipsis and nothing else. It used to draw
        // the whole string, which is the one thing ellipsising exists to prevent.
        assert_eq!(truncate(long, &style.resolved(), 0.0), ELLIPSIS);
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
        let fitted = text.fitted(90.0, &text.resolved(None));
        // What comes back is a **prefix**, not a list of lines: it wraps into two when the
        // renderer breaks it, which is where the breaking belongs.
        assert!(
            fitted.text.lines().count() == 1,
            "one string, no newlines in it: {:?}",
            fitted.text
        );
        let lines = frus_text::line_spans(
            &fitted.text,
            14.0,
            FontWeight::Regular,
            false,
            Some(90.0),
            true,
        );
        assert_eq!(lines.len(), 2, "and it breaks into two: {:?}", fitted.text);
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
        let fitted = text.fitted(400.0, &text.resolved(None));
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
        let at = |t: &Text| {
            Widget::<()>::measure(t, &Theme::default()).expect("measure")(Some(90.0), None).height
        };
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

    /// A **subtree** can hand a text its size, and a text that never chose one takes it.
    ///
    /// The half that matters is where it is taken: the size has to reach the **box**, not
    /// only the glyphs. A text drawn at 24 px inside a box measured for 16 leaves every
    /// row on the screen the wrong height, and nothing in the picture says which of the
    /// two numbers was the mistake.
    #[test]
    fn a_subtree_hands_a_text_its_size() {
        let mut theme = Theme::default();
        theme.widgets.text.style.size = Some(30.0);
        // One line, so the box it asks for is a number this test can read.
        let text = Text::new("Handed down").no_wrap();

        let alone = Widget::<()>::style(&text);
        let handed = Widget::<()>::style_themed(&text, &theme);
        let (Dimension::Length(alone_w), Dimension::Length(handed_w)) = (alone.width, handed.width)
        else {
            panic!("a single line is a box of a known size");
        };
        assert!(
            handed_w > alone_w * 1.5,
            "the box grew with the type: {alone_w} → {handed_w}"
        );

        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, handed_w, 40.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Text { size, .. } => assert_eq!(
                *size, 30.0,
                "and the glyphs agree with the box they were measured into"
            ),
            other => panic!("a text, not {other:?}"),
        }
    }

    /// A size the **caller** chose is not overruled — and a default nobody chose is.
    ///
    /// The two halves are the whole rule, and either alone would be wrong: without the
    /// first, a subtree silently resizes the one text that had asked to be different;
    /// without the second, the feature does nothing at all, because every `Text` already
    /// holds a size and a number cannot say whether anybody picked it.
    #[test]
    fn a_chosen_size_outranks_a_handed_one() {
        let mut theme = Theme::default();
        theme.widgets.text.style.size = Some(30.0);
        let width = |text: &Text| match Widget::<()>::style_themed(text, &theme).width {
            Dimension::Length(w) => w,
            other => panic!("a known width, not {other:?}"),
        };
        // The same 16 px, once as a default and once as an answer.
        let untouched = width(&Text::new("Handed down").no_wrap());
        let chosen = width(&Text::new("Handed down").no_wrap().size(16.0));
        assert!(
            untouched > chosen,
            "the one that chose kept its size: {untouched} vs {chosen}"
        );
        assert_eq!(
            chosen,
            width(&Text::new("Handed down").no_wrap().size(16.0)),
            "and it keeps it however often it is asked"
        );
    }

    /// The merge is **field by field**: a subtree that sets a colour and nothing else
    /// leaves every size alone.
    ///
    /// A whole-style handover would be the easy implementation and the wrong one — an app
    /// bar recolouring its words would flatten the type scale of everything inside it.
    #[test]
    fn handing_down_a_colour_leaves_the_sizes_alone() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let mut theme = Theme::default();
        theme.widgets.text.style.color = Some(red);
        let text = Text::new("Words").no_wrap().size(22.0);
        assert_eq!(
            Widget::<()>::style_themed(&text, &theme).width,
            Widget::<()>::style(&text).width,
            "the box did not move"
        );
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, 300.0, 40.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Text { size, color, .. } => {
                assert_eq!(*color, red, "the colour came down");
                assert_eq!(*size, 22.0, "the size stayed put");
            }
            other => panic!("a text, not {other:?}"),
        }
    }

    /// A handed-down line limit also says the text **may be squeezed**.
    ///
    /// It is the easy half to leave out, and leaving it out ships a subtree that
    /// ellipsises its texts and never gets the chance to: a flex item's automatic minimum
    /// size is its content, so each of them still refuses to give way and the mode never
    /// fires. Saying what to do on overflow is what says it may be squeezed — however the
    /// text was told.
    #[test]
    fn a_handed_down_limit_lets_the_text_give_way() {
        let mut theme = Theme::default();
        theme.widgets.text.max_lines = Some(1);
        theme.widgets.text.overflow = Some(TextOverflow::Ellipsis);
        let text = Text::new("a label far too long for the box it will be given");
        assert_eq!(
            Widget::<()>::style(&text).min_width,
            Dimension::Auto,
            "left alone it refuses to shrink"
        );
        assert_eq!(
            Widget::<()>::style_themed(&text, &theme).min_width,
            Dimension::Length(0.0),
            "handed a limit, it gives way"
        );
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, 80.0, 20.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Text { text, .. } => {
                assert!(text.ends_with(ELLIPSIS), "and it was cut: {text:?}")
            }
            other => panic!("a text, not {other:?}"),
        }
    }

    /// A handed-down **alignment** also makes the text ask for its parent's width.
    ///
    /// Alignment is the one style setting that is also a *request*: a box exactly as wide
    /// as its own text has nowhere to centre it in, so a text told to centre asks to fill
    /// the line first. That request is made by `fill_axes`, which is why that hook
    /// had to see the theme too — without it, the one setting that arrives by inheritance
    /// would resolve correctly everywhere except where it takes effect, and centring a
    /// subtree's texts would silently do nothing.
    #[test]
    fn a_handed_down_alignment_still_asks_for_the_width() {
        let mut theme = Theme::default();
        theme.widgets.text.align = Some(TextAlign::Center);
        let text = Text::new("Centred").no_wrap();
        assert_eq!(
            Widget::<()>::fill_axes(&text, &Theme::default()),
            crate::widget::FillAxes::NONE,
            "left alone it hugs its words"
        );
        assert_eq!(
            Widget::<()>::fill_axes(&text, &theme),
            crate::widget::FillAxes::WIDTH,
            "handed a centring, it asks for the line"
        );
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, 300.0, 20.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Text {
                align, max_width, ..
            } => {
                assert_eq!(*align, TextAlign::Center);
                assert_eq!(*max_width, Some(300.0), "and it was given a box to sit in");
            }
            other => panic!("a text, not {other:?}"),
        }
    }

    /// **Two nested wrappers each setting one field leave a text wearing both.**
    ///
    /// This is what makes the merge worth its complexity. A whole-style handover would
    /// look identical in a single-wrapper test and be wrong here: the inner wrapper would
    /// replace the outer one's colour with nothing at all, and a section that set a size
    /// inside a screen that set a colour would silently drop the colour.
    #[test]
    fn nested_wrappers_compose_field_by_field() {
        use crate::{build_ui, Runtime, Size};
        let muted = Color::rgb(0.4, 0.4, 0.45);
        let outer = crate::DefaultTextStyle::from_text_style(TextStyle::NONE.color(muted));
        let inner = crate::DefaultTextStyle::from_text_style(TextStyle::new(9.0));
        let root: Box<dyn Widget<()>> =
            Box::new(outer.around(inner.around(Text::new("Both").no_wrap())));
        let ui = build_ui(
            root.as_ref(),
            Size::new(300.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let found = ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Text { size, color, .. } => Some((*size, *color)),
            _ => None,
        });
        assert_eq!(
            found,
            Some((9.0, muted)),
            "the size came from the inner wrapper and the colour from the outer"
        );
    }

    /// **A style may name a size and leave the weight to the subtree** — the thing this
    /// framework could not express until `TextStyle`'s fields could each say *unset*.
    ///
    /// `Text::styled(s, TextStyle::new(20.0))` used to answer a size, a weight *and* a
    /// slant, because the type had nowhere to put "not said". So a caller who wanted one
    /// step of the scale at a different size had to restate the weight, and a section that
    /// set a weight for its labels could not reach a single one of them.
    #[test]
    fn a_style_may_fix_the_size_and_still_inherit_the_weight() {
        let mut theme = Theme::default();
        theme.widgets.text.style.weight = Some(FontWeight::Bold);
        let text = Text::styled("Words", TextStyle::new(20.0)).no_wrap();

        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, 300.0, 40.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Text { size, weight, .. } => {
                assert_eq!(*size, 20.0, "the size it named");
                assert_eq!(*weight, FontWeight::Bold, "the weight it did not");
            }
            other => panic!("a text, not {other:?}"),
        }
        // And the box was measured at the same pair, not at one of each.
        let bold_20 = frus_text::measure_styled("Words", 20.0, FontWeight::Bold, false).width;
        match Widget::<()>::style_themed(&text, &theme).width {
            Dimension::Length(w) => assert!(
                (w - bold_20.ceil()).abs() < 0.01,
                "measured {w}, drawn at {bold_20}"
            ),
            other => panic!("a known width, not {other:?}"),
        }
    }

    /// **The box grows with the reader's font size, and the glyphs agree with the box.**
    ///
    /// The second half is the whole risk of this feature and the reason it is resolved at
    /// one place rather than applied by each widget. Text measured at 16 and drawn at 24
    /// leaves every row on the screen the wrong height at once, and nothing in the picture
    /// says which of the two numbers was the mistake. So this asserts the pair, not the
    /// paint: what the layout reserved is what the renderer was asked to draw.
    #[test]
    fn a_reader_who_asked_for_larger_text_gets_a_larger_box_too() {
        use crate::{build_ui, MediaQuery, Runtime, Size};
        let drawn = |scaler: f32| {
            MediaQuery::new(Size::new(400.0, 200.0))
                .with_text_scaler(scaler)
                .scope(|| {
                    let text: Box<dyn Widget<()>> = Box::new(Text::new("Readable").no_wrap());
                    let ui = build_ui(
                        text.as_ref(),
                        Size::new(400.0, 200.0),
                        &Runtime::default(),
                        &Theme::default(),
                    );
                    let rect = ui.scene().primitives().iter().find_map(|p| match p {
                        Primitive::Text { size, bounds, .. } => Some((*size, *bounds)),
                        _ => None,
                    });
                    rect.expect("the text is drawn")
                })
        };
        let (plain_size, plain_box) = drawn(1.0);
        let (big_size, big_box) = drawn(1.5);

        assert_eq!(plain_size, 16.0, "the framework's own, unscaled");
        assert_eq!(big_size, 24.0, "and the reader's 1.5 of it");
        assert!(
            big_box.width > plain_box.width * 1.4,
            "the box grew with the glyphs: {} → {}",
            plain_box.width,
            big_box.width
        );
        // The pair, which is the thing that must never come apart: the box the layout gave
        // this text is the box its own drawn size measures to.
        let measured = frus_text::measure_styled("Readable", big_size, FontWeight::Regular, false);
        assert!(
            (big_box.width - measured.width.ceil()).abs() < 0.51,
            "reserved {} for glyphs measuring {}",
            big_box.width,
            measured.width
        );
    }

    /// Outside a described surface **nothing scales**, which is what every test in this
    /// file and every golden depends on: they build widgets with no `MediaQuery` around
    /// them, and a framework that scaled by default would move all of them at once.
    #[test]
    fn with_no_surface_described_nothing_is_scaled() {
        assert_eq!(
            Text::new("x").resolved(None).style.size,
            frus_core::DEFAULT_TEXT_SIZE
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
                height: None,
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
        let measure = Widget::<()>::measure(&text, &Theme::default()).expect("measure closure");
        let free = measure(None, None);
        let narrow = measure(Some(120.0), None);
        assert!(narrow.width <= 120.0);
        assert!(narrow.height > free.height, "wrapped → taller");
        // And the measure key changes with the content (the cache fix).
        let other = Text::new("short").wrap();
        assert_ne!(
            Widget::<()>::measure_key(&text, &Theme::default()),
            Widget::<()>::measure_key(&other, &Theme::default())
        );
        // A text that does not wrap is a box of a known size, and says so in its style
        // rather than through a measurement: a measured leaf reports its narrowest useful
        // width as its minimum content, and a row would take that as leave to fold it.
        let plain = Text::new(long).no_wrap();
        assert!(Widget::<()>::measure(&plain, &Theme::default()).is_none());
        assert!(Widget::<()>::measure_key(&plain, &Theme::default()).is_none());
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
        let w = |text: &Text| {
            Widget::<()>::measure(text, &Theme::default()).expect("measure")(None, None).width
        };
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

#[cfg(test)]
mod reader_font_size {
    use frus_core::{Primitive, Size};

    use crate::theme::Theme;
    use crate::{build_ui_inspected, MediaQuery, Runtime, Widget};

    type W = Box<dyn Widget<()>>;
    type Case = (&'static str, fn() -> W);

    /// Every glyph a widget paints, and the size it was drawn at.
    fn glyphs(make: fn() -> W, scale: f32) -> Vec<(String, f32)> {
        let root = make();
        MediaQuery::new(Size::new(400.0, 300.0))
            .with_text_scaler(scale)
            .scope(|| {
                let (ui, _) = build_ui_inspected(
                    root.as_ref(),
                    Size::new(400.0, 300.0),
                    &Runtime::default(),
                    &Theme::default(),
                );
                ui.scene()
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Text { text, size, .. } => Some((text.clone(), *size)),
                        _ => None,
                    })
                    .collect()
            })
    }

    fn cases() -> Vec<Case> {
        vec![
            ("Chip", || Box::new(crate::Chip::<()>::new("Filter"))),
            ("Button", || Box::new(crate::Button::<()>::new("Save"))),
            ("ListTile", || {
                Box::new(crate::ListTile::<()>::new().title("A row"))
            }),
            ("SnackBar", || Box::new(crate::SnackBar::<()>::new("Saved"))),
            ("Kbd", || Box::new(crate::Kbd::new("Ctrl"))),
            ("DropdownButton", || {
                Box::new(crate::DropdownButton::new("Pick", ()))
            }),
            ("Text", || Box::new(crate::Text::new("A line"))),
            ("TextField", || {
                Box::new(crate::TextField::<()>::new("typed"))
            }),
            ("Table", || {
                Box::new(crate::Table::<()>::new(2).row(&["one", "two"]))
            }),
            ("Tree", || {
                Box::new(crate::Tree::new(|_| ()).node(1, 0, "a branch", false, false))
            }),
        ]
    }

    /// **The reader's font size reaches the text a widget paints itself.**
    ///
    /// `TextStyle::resolved` is the single place the setting is applied (milestone 403),
    /// and forty-seven paint sites went around it: they named an `f32` and handed it
    /// straight to the scene, so their text was the one size a reader could not change.
    /// Nothing failed and nothing was reported — the widget simply ignored the request.
    ///
    /// The door is shut now: [`frus_core::Scene::text`] takes a
    /// [`frus_core::ResolvedTextStyle`], so a bare number does not compile. This test is
    /// what keeps it shut, because a new widget can still reach for
    /// [`frus_core::ResolvedTextStyle::exact`] — which is right for an icon and wrong for
    /// a word.
    #[test]
    fn the_text_a_widget_paints_follows_the_readers_font_size() {
        for (name, make) in cases() {
            let plain = glyphs(make, 1.0);
            let doubled = glyphs(make, 2.0);
            assert!(!plain.is_empty(), "{name} paints no text at all");
            assert_eq!(
                plain.len(),
                doubled.len(),
                "{name}: a different number of runs at twice the size"
            );
            let grew = plain
                .iter()
                .zip(&doubled)
                .any(|((_, a), (_, b))| *b > *a + 0.01);
            assert!(
                grew,
                "{name}: not one glyph followed the reader — sizes {plain:?}"
            );
        }
    }

    /// **And the box grows to hold it.** A default height is a *floor*, as it is in the
    /// reference — `max(_targetTileHeight, contentHeight)` — not a ceiling. A chip whose
    /// height was a constant needed 34 px of glyphs inside 32 px and cut them.
    #[test]
    fn a_box_that_holds_text_grows_with_it() {
        for (name, make) in cases() {
            let root = make();
            MediaQuery::new(Size::new(400.0, 300.0))
                .with_text_scaler(2.0)
                .scope(|| {
                    let (ui, _) = build_ui_inspected(
                        root.as_ref(),
                        Size::new(400.0, 300.0),
                        &Runtime::default(),
                        &Theme::default(),
                    );
                    let mut checked = 0;
                    for p in ui.scene().primitives() {
                        if let Primitive::Text {
                            text, size, bounds, ..
                        } = p
                        {
                            // The box the text was **laid out in**, not the widget's outermost
                            // rect: a table's outer box is enormous and would pass whatever its
                            // rows did. Checking the emitting box is what caught the table.
                            if bounds.height <= 0.0 {
                                continue;
                            }
                            // What the **shaper** gives back, not the nominal `line_height`:
                            // a face's real line box can be a fraction under the metric, and a
                            // box sized to the smaller number clips nothing.
                            let style = frus_core::ResolvedTextStyle::exact(*size);
                            let needed = frus_text::measure_resolved(text, &style).height;
                            checked += 1;
                            assert!(
                                needed <= bounds.height + 0.51,
                                "{name}: {text:?} needs {needed:.1} px in a box of {:.1}",
                                bounds.height
                            );
                        }
                    }
                    assert!(checked > 0, "{name}: no text was checked at all");
                });
        }
    }

    /// **Chrome caps the type instead of growing**, which is the reference's other answer
    /// and the reason both exist. An app bar keeps its height — a toolbar that grew would
    /// push every screen down — so it clamps the title's scaler to 1.34 rather than let
    /// the reader out. Below that cap it follows like anything else.
    #[test]
    fn an_app_bar_caps_its_title_rather_than_growing() {
        let title_size = |scale: f32| {
            let bar = crate::AppBar::<()>::new("A title");
            MediaQuery::new(Size::new(400.0, 300.0))
                .with_text_scaler(scale)
                .scope(|| {
                    let root = bar.build();
                    let (ui, _) = build_ui_inspected(
                        root.as_ref(),
                        Size::new(400.0, 300.0),
                        &Runtime::default(),
                        &Theme::default(),
                    );
                    ui.scene()
                        .primitives()
                        .iter()
                        .find_map(|p| match p {
                            Primitive::Text { text, size, .. } if text == "A title" => Some(*size),
                            _ => None,
                        })
                        .expect("the bar paints its title")
                })
        };
        let plain = title_size(1.0);
        assert!(
            (title_size(1.2) - plain * 1.2).abs() < 0.1,
            "under the cap the title follows the reader"
        );
        let capped = plain * crate::APP_BAR_MAX_TITLE_SCALE;
        for scale in [1.5, 2.0, 4.0] {
            assert!(
                (title_size(scale) - capped).abs() < 0.1,
                "at x{scale} the title is {} rather than the capped {capped}",
                title_size(scale)
            );
        }
    }
}

//! [`TextStyle`]: the typographic attributes of a piece of text (size, weight,
//! italic, colour), independent of any widget or theme.
//!
//! A `TextStyle` is a pure `Copy` value. Its `color` is optional, and `None`
//! means "inherit" — the widget resolves it to the theme colour at paint time.
//! Named typographic scales, in the style of a Material `TextTheme`, are built
//! out of this type.

use crate::Color;

/// How the lines of a piece of text are **aligned inside the box** it was given.
///
/// It is not the same question as where the box goes. A centred paragraph and a centred
/// box look identical while the text is one line and stop looking identical the moment it
/// wraps, which is when the setting starts to matter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// The start of the reading direction: the left in a left-to-right script, the right
    /// in a right-to-left one. The default, and the only one that follows the text.
    #[default]
    Start,
    /// The end of the reading direction.
    End,
    /// Centred in the box.
    Center,
    /// Both edges flush, the space stretched between the words. Every line but the last.
    Justify,
    /// The left edge, whatever the script. A column of figures wants this and not
    /// [`TextAlign::Start`], because the figures do not change direction with the prose
    /// around them.
    Left,
    /// The right edge, whatever the script.
    Right,
}

/// What becomes of text that does not fit the box it was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextOverflow {
    /// Cut at the edge of the box. The default, and the quiet one: nothing marks the
    /// place where the words stopped.
    #[default]
    Clip,
    /// Cut, with the last line ending in an ellipsis. The only mode that tells the reader
    /// something is missing, which is usually the one wanted.
    Ellipsis,
    /// Cut, with the last line fading out into the background rather than stopping at a
    /// hard edge.
    Fade,
    /// Draw past the box. Not an accident: a badge or a decoration that is *meant* to
    /// spill says so this way, rather than by being given a box it does not fit.
    Visible,
}

/// Font weight — a useful subset, mapped onto the CSS/OpenType weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontWeight {
    /// 400 — regular.
    #[default]
    Regular,
    /// 500.
    Medium,
    /// 600.
    SemiBold,
    /// 700 — bold.
    Bold,
}

impl FontWeight {
    /// The numeric OpenType weight (400/500/600/700).
    pub fn to_u16(self) -> u16 {
        match self {
            FontWeight::Regular => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
        }
    }
}

/// A text's **decoration** lines, which combine with one another. They have no
/// effect on measurement: the text's geometry does not change, and the lines are
/// drawn by the backend from the baseline metrics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextDecoration {
    /// Underline, below the baseline.
    pub underline: bool,
    /// A line above the text.
    pub overline: bool,
    /// Struck through, at the middle of the x-height.
    pub strikethrough: bool,
}

impl TextDecoration {
    /// No decoration at all.
    pub const NONE: Self = Self {
        underline: false,
        overline: false,
        strikethrough: false,
    };
    /// Underline only.
    pub const UNDERLINE: Self = Self {
        underline: true,
        ..Self::NONE
    };
    /// Overline only.
    pub const OVERLINE: Self = Self {
        overline: true,
        ..Self::NONE
    };
    /// Strikethrough only.
    pub const STRIKETHROUGH: Self = Self {
        strikethrough: true,
        ..Self::NONE
    };

    /// `true` when no line at all is asked for.
    pub const fn is_none(self) -> bool {
        !self.underline && !self.overline && !self.strikethrough
    }

    /// Combines two decorations — the union of their lines.
    pub const fn combine(self, other: Self) -> Self {
        Self {
            underline: self.underline || other.underline,
            overline: self.overline || other.overline,
            strikethrough: self.strikethrough || other.strikethrough,
        }
    }
}

/// The font size a text ends up at when nothing anywhere in the chain named one.
pub const DEFAULT_TEXT_SIZE: f32 = 16.0;

/// The line height a style that says nothing gets, as a **multiple of the font size**.
///
/// 1.2 is what the bundled faces want, and what every part of the framework used before it
/// was expressible at all — see [`TextStyle::height`].
pub const DEFAULT_LINE_HEIGHT: f32 = 1.2;

/// The typographic attributes of a single line of text — **every field optional**.
///
/// `None` is not a missing value but an answer: *this style does not say*. That is what
/// makes a style inheritable field by field, and it is the reference's shape — every field
/// of its `TextStyle` is nullable for exactly this reason.
///
/// It matters more than it looks. `TextStyle::new(20.0)` says a size and **nothing else**,
/// so a text wearing it still takes its weight from whatever subtree it is in. Before this
/// was optional, a style named all three at once and *size 20, inherit the weight* was
/// simply not expressible.
///
/// Ask [`TextStyle::resolved`] for the concrete numbers to measure and draw with.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextStyle {
    /// Font size, in logical pixels. Unset, [`DEFAULT_TEXT_SIZE`].
    pub size: Option<f32>,
    /// Weight. Unset, [`FontWeight::Regular`].
    pub weight: Option<FontWeight>,
    /// Italic. Unset, upright.
    pub italic: Option<bool>,
    /// Colour. Unset, the widget resolves it against the theme at paint.
    pub color: Option<Color>,
    /// Decoration lines (underline, strikethrough, and so on). Unset, none — and set to
    /// [`TextDecoration::NONE`] is a **different** answer, being how a caller takes an
    /// underline back off a run of words a subtree underlined.
    pub decoration: Option<TextDecoration>,
    /// Decoration colour; unset, the text's own.
    pub decoration_color: Option<Color>,
    /// The line's height, as a **multiple of the font size** — the reference's `height`.
    ///
    /// `1.0` sets each line box to exactly the font size, `1.5` to half again; unset says
    /// nothing, and a style that says nothing inherits whatever a subtree handed down, or
    /// [`DEFAULT_LINE_HEIGHT`] at the end of that chain.
    ///
    /// It is a **ratio, not a length**, for the reason the reference gives: a paragraph
    /// keeps its rhythm when the reader turns the type up, because the leading grows with
    /// the letters instead of staying where a designer left it.
    pub height: Option<f32>,
}

/// A [`TextStyle`] with every question answered: what to measure with, and what to draw.
///
/// The colour stays optional, and deliberately. Size, weight and slant have a *framework*
/// default that is right everywhere; a colour's last word belongs to the theme, which this
/// type cannot see. A widget resolves it at paint against `on_surface`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedTextStyle {
    /// Font size, in logical pixels.
    pub size: f32,
    /// Weight.
    pub weight: FontWeight,
    /// Italic.
    pub italic: bool,
    /// Explicit colour; `None` means the widget resolves it against the theme.
    pub color: Option<Color>,
    /// Decoration lines.
    pub decoration: TextDecoration,
    /// Decoration colour; `None` means the text's own colour.
    pub decoration_color: Option<Color>,
    /// The line's height as a **multiple of the font size**; `None` means
    /// [`DEFAULT_LINE_HEIGHT`]. Read it through [`line_height`](Self::line_height).
    pub height: Option<f32>,
}

impl ResolvedTextStyle {
    /// The height of one line, in logical pixels.
    ///
    /// **The one place this number is decided.** It used to be a `LINE_HEIGHT_FACTOR`
    /// constant in `frus-text` and *another* in `frus-gpu`, which is two constants that
    /// had to agree: a measure and a paint disagreeing about how tall a line is puts the
    /// second line of every paragraph somewhere the layout did not reserve.
    pub fn line_height(&self) -> f32 {
        self.size * self.height.unwrap_or(DEFAULT_LINE_HEIGHT)
    }
    /// A style of **exactly** this size — regular, upright, undecorated, and *not* put
    /// through the reader's font setting.
    ///
    /// For glyphs whose size is geometry rather than type: an icon on its 24 px grid, a
    /// marker that has to stay inside a circle drawn beside it. The reference's `Icon` is
    /// sized by `IconTheme` and ignores the text scaler for the same reason — an icon that
    /// grew with the type would leave its own box.
    ///
    /// Everything a reader *reads* wants
    /// [`TextStyle::new(size).resolved()`](TextStyle::resolved) instead. The two are one
    /// keyword apart on purpose: the choice is worth stating at every call, which is why
    /// there is no third form that guesses.
    pub const fn exact(size: f32) -> Self {
        Self {
            size,
            weight: FontWeight::Regular,
            italic: false,
            color: None,
            decoration: TextDecoration::NONE,
            decoration_color: None,
            height: None,
        }
    }
}

impl TextStyle {
    /// A style that says **nothing at all** — every field inherited.
    pub const NONE: Self = Self {
        size: None,
        weight: None,
        italic: None,
        color: None,
        decoration: None,
        decoration_color: None,
        height: None,
    };

    /// A style that names a `size` and nothing else.
    pub const fn new(size: f32) -> Self {
        Self {
            size: Some(size),
            ..Self::NONE
        }
    }

    /// Sets the weight.
    pub const fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Switches to italic.
    pub const fn italic(mut self) -> Self {
        self.italic = Some(true);
        self
    }

    /// Sets the size.
    pub const fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the colour.
    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the decoration lines.
    pub const fn decoration(mut self, decoration: TextDecoration) -> Self {
        self.decoration = Some(decoration);
        self
    }

    /// Adds an underline, combining with any other decoration **this style** already
    /// carries — never with an inherited one, which this style cannot see.
    pub const fn underline(mut self) -> Self {
        let current = match self.decoration {
            Some(d) => d,
            None => TextDecoration::NONE,
        };
        self.decoration = Some(current.combine(TextDecoration::UNDERLINE));
        self
    }

    /// Adds a strikethrough, combining with any other decoration this style carries.
    pub const fn strikethrough(mut self) -> Self {
        let current = match self.decoration {
            Some(d) => d,
            None => TextDecoration::NONE,
        };
        self.decoration = Some(current.combine(TextDecoration::STRIKETHROUGH));
        self
    }

    /// Sets the decoration colour; otherwise the text's colour is used.
    pub const fn decoration_color(mut self, color: Color) -> Self {
        self.decoration_color = Some(color);
        self
    }

    /// **Merges** `over` on top of `self`, **field by field**: where `over` said nothing,
    /// this one's answer survives.
    ///
    /// The cascade, and it is one operation now rather than three. Until every field could
    /// say "unset" this codebase carried the same idea three times — a private `Overrides`
    /// struct for rich-text spans, a `Chosen` record of booleans beside a `Text`'s style,
    /// and half of the widgets' `DefaultTextStyle` — because a whole-style merge could only
    /// replace the typography wholesale and each caller needed the other behaviour.
    #[must_use]
    pub fn merge(self, over: TextStyle) -> TextStyle {
        TextStyle {
            size: over.size.or(self.size),
            weight: over.weight.or(self.weight),
            italic: over.italic.or(self.italic),
            color: over.color.or(self.color),
            decoration: over.decoration.or(self.decoration),
            decoration_color: over.decoration_color.or(self.decoration_color),
            height: over.height.or(self.height),
        }
    }

    /// Every question answered: the framework's own default wherever nothing in the chain
    /// said anything, and **the reader's font size applied**.
    ///
    /// This is where the chain **stops**. Above it a style may leave a field open for a
    /// subtree or a theme to answer; below it a shaper needs a number, and there is nobody
    /// left to ask.
    ///
    /// Which is exactly why the user's *Font size* setting is applied here and nowhere
    /// else. A phone's slider goes to 1.3 on Android and past 3 with iOS's larger
    /// accessibility sizes, and an interface that ignores it is one a lot of people cannot
    /// read. Scaling it at the one place a size becomes a number is what keeps the
    /// **measurement and the paint agreeing**: text measured at one size and drawn at
    /// another is a layout that is wrong everywhere at once, with nothing in the picture to
    /// say which of the two numbers was the mistake.
    ///
    /// See [`with_text_scale`]. Outside any scope the scale is 1 and this is the identity.
    /// Caps how far the **reader's font setting** may enlarge this style.
    ///
    /// The size is declared smaller in exact proportion, so that resolving it lands at
    /// `max_scale` times the size asked for and no more. Below `max_scale` nothing changes.
    ///
    /// This is the reference's second answer to a reader who turns the system type up, and
    /// it belongs to **chrome**. A component grows: its default height is a floor and the
    /// content wins. A toolbar cannot — it would push the whole screen down — so the
    /// reference keeps its bar at `kToolbarHeight` and clamps the title's scaler to 1.34
    /// instead, "to keep the visual hierarchy the same even with larger font sizes".
    ///
    /// It reads the ambient scale, so it must be called where that scale is in force —
    /// which for a widget means while it is being built, the same place it reads a theme.
    #[must_use]
    pub fn clamp_scale(mut self, max_scale: f32) -> Self {
        let scale = text_scale();
        if scale > max_scale && scale > 0.0 {
            if let Some(size) = self.size {
                self.size = Some(size * max_scale / scale);
            }
        }
        self
    }

    pub fn resolved(self) -> ResolvedTextStyle {
        ResolvedTextStyle {
            size: self.size.unwrap_or(DEFAULT_TEXT_SIZE) * text_scale(),
            weight: self.weight.unwrap_or(FontWeight::Regular),
            italic: self.italic.unwrap_or(false),
            color: self.color,
            decoration: self.decoration.unwrap_or(TextDecoration::NONE),
            decoration_color: self.decoration_color,
            height: self.height,
        }
    }
}

/// The reader's font-size setting in force on this thread, or `1.0` outside any
/// [`with_text_scale`].
///
/// Read once, by [`TextStyle::resolved`]. Nothing else should consult it: a second reader
/// is a second chance to scale a size twice, or to scale one a sibling did not.
pub fn text_scale() -> f32 {
    TEXT_SCALE.with(|s| s.get())
}

/// Runs `f` with the reader's font-size setting in force, restoring whatever was there
/// before — including while a panic unwinds, so one bad frame cannot leave a stale scale
/// installed for every frame after it.
///
/// **Ambient rather than threaded**, and that is the decision worth stating. Passing a
/// scaler down would mean every widget that measures a label remembering to apply it, and
/// the one that forgot would draw text the layout never measured. There is no diagnostic
/// for that — only a screen that is subtly wrong. Ambient makes forgetting impossible: the
/// only place a size becomes a number already reads it.
///
/// The framework installs this around `view` from `MediaQuery::text_scaler`; an
/// application does not normally call it. Scales at or below zero are ignored, a font of
/// no size being a screen with no words on it.
///
/// **A closure is not always a long enough life.** A size becomes a number during layout
/// and again during paint, which happen after the widgets have been built — see
/// [`install_text_scale`], which the shell uses to cover a whole frame.
pub fn with_text_scale<R>(scale: f32, f: impl FnOnce() -> R) -> R {
    let guard = install_text_scale(scale);
    let out = f();
    drop(guard);
    out
}

/// Installs the reader's font-size setting **until the returned guard is dropped**.
///
/// [`with_text_scale`] covers a closure, which is right for a subtree and wrong for a
/// frame: a widget is *built* first, then measured, laid out and painted, and every one of
/// those later steps resolves sizes too. Scoping only the build leaves the scale at 1 for
/// the two steps that decide how big the text actually is — so the layout measures one
/// size and the renderer draws another, which is the exact failure
/// [`TextStyle::resolved`](TextStyle::resolved) exists to make impossible.
///
/// That is not a hypothetical either: the shell wrapped `view` alone, and the setting
/// reached a device without changing a single pixel (milestone 407).
///
/// Scales at or below zero are ignored, as in [`with_text_scale`].
#[must_use = "the scale is restored the moment the guard is dropped"]
pub fn install_text_scale(scale: f32) -> TextScaleGuard {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    TextScaleGuard(TEXT_SCALE.with(|s| s.replace(scale)))
}

/// Puts back the scale that was in force before [`install_text_scale`], when dropped —
/// including while a panic unwinds, so one bad frame cannot leave a stale scale installed
/// for every frame after it.
pub struct TextScaleGuard(f32);

impl Drop for TextScaleGuard {
    fn drop(&mut self) {
        TEXT_SCALE.with(|s| s.set(self.0));
    }
}

thread_local! {
    /// The reader's font-size setting on this thread. `Cell`, not `RefCell`: an `f32` is
    /// `Copy` and every access is a whole get or a whole set.
    static TEXT_SCALE: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
}

/// A **rich-text tree**: each node carries a fragment of text, *partial* style
/// overrides — whatever is unspecified **inherits** from the parent, so a
/// `.bold()` child keeps its parent's size — and children of its own.
///
/// ```
/// # use frus_core::{TextSpan, TextStyle};
/// let span = TextSpan::new("Hello ")
///     .child(TextSpan::new("bold").bold())
///     .child(TextSpan::new(" world"));
/// let runs = span.flatten(TextStyle::new(20.0));
/// assert_eq!(runs.len(), 3);
/// assert_eq!(runs[1].1.size, 20.0); // the bold child inherits the size
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSpan {
    text: String,
    overrides: TextStyle,
    children: Vec<TextSpan>,
}

impl TextSpan {
    /// A text fragment with no overrides — it inherits everything.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            overrides: TextStyle::NONE,
            children: Vec::new(),
        }
    }

    /// Makes this subtree bold.
    pub fn bold(mut self) -> Self {
        self.overrides.weight = Some(FontWeight::Bold);
        self
    }

    /// Sets this subtree's weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.overrides.weight = Some(weight);
        self
    }

    /// Makes this subtree italic.
    pub fn italic(mut self) -> Self {
        self.overrides.italic = Some(true);
        self
    }

    /// Sets this subtree's size.
    pub fn size(mut self, size: f32) -> Self {
        self.overrides.size = Some(size);
        self
    }

    /// Sets this subtree's colour.
    pub fn color(mut self, color: Color) -> Self {
        self.overrides.color = Some(color);
        self
    }

    /// Underlines this subtree. It combines with the decoration already set on
    /// this node, but not with the parent's.
    pub fn underline(mut self) -> Self {
        self.overrides = self.overrides.underline();
        self
    }

    /// Strikes through this subtree.
    pub fn strikethrough(mut self) -> Self {
        self.overrides = self.overrides.strikethrough();
        self
    }

    /// Sets this subtree's decoration lines.
    pub fn decoration(mut self, decoration: TextDecoration) -> Self {
        self.overrides.decoration = Some(decoration);
        self
    }

    /// Sets this subtree's decoration colour.
    pub fn decoration_color(mut self, color: Color) -> Self {
        self.overrides.decoration_color = Some(color);
        self
    }

    /// Applies a whole [`TextStyle`] as this subtree's override, **replacing** whatever
    /// this span had said before.
    ///
    /// It used to force every typographic field to *answered*, because a `TextStyle` could
    /// not say otherwise. It no longer has to: a style naming only a size now overrides
    /// only the size, and the weight goes on being inherited from the parent span.
    pub fn style(mut self, style: TextStyle) -> Self {
        self.overrides = style;
        self
    }

    /// Adds a child, rendered after this node's own text.
    pub fn child(mut self, child: TextSpan) -> Self {
        self.children.push(child);
        self
    }

    /// Flattens the tree into **resolved runs** `(text, style)`, in reading order,
    /// cascading the overrides down from `base`, the paragraph's default style.
    /// Nodes with no text of their own produce no empty run.
    pub fn flatten(&self, base: TextStyle) -> Vec<(String, ResolvedTextStyle)> {
        let mut runs = Vec::new();
        self.collect(base, &mut runs);
        runs
    }

    /// Mixes into `hasher` everything that affects the tree's **measurement**:
    /// text, sizes, weights and italics. Colours and **decorations** are excluded,
    /// having no effect on geometry. This serves as the measurement fingerprint
    /// (`measure_key`) without having to flatten the tree.
    pub fn measure_hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        use std::hash::Hash;
        self.text.hash(hasher);
        self.overrides.size.map(f32::to_bits).hash(hasher);
        self.overrides.weight.map(FontWeight::to_u16).hash(hasher);
        self.overrides.italic.hash(hasher);
        self.children.len().hash(hasher);
        for child in &self.children {
            child.measure_hash(hasher);
        }
    }

    fn collect(&self, inherited: TextStyle, out: &mut Vec<(String, ResolvedTextStyle)>) {
        let effective = inherited.merge(self.overrides);
        if !self.text.is_empty() {
            out.push((self.text.clone(), effective.resolved()));
        }
        // The children inherit the **unresolved** style, not the run's numbers. Handing
        // them the resolved one would turn every framework default into an answer, and a
        // child that wanted to inherit a weight nobody had named would find one waiting.
        for child in &self.children {
            child.collect(effective, out);
        }
    }
}

/// A text run **ready to render**: a fragment plus its typographic attributes and
/// colour, all **resolved** — nothing left to inherit.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub size: f32,
    pub weight: FontWeight,
    pub italic: bool,
    pub color: Color,
    /// The run's decoration lines (no effect on measurement).
    pub decoration: TextDecoration,
    /// Decoration colour; `None` means the run's own colour.
    pub decoration_color: Option<Color>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_map_to_opentype() {
        assert_eq!(FontWeight::Regular.to_u16(), 400);
        assert_eq!(FontWeight::Bold.to_u16(), 700);
    }

    #[test]
    fn builders_compose() {
        let s = TextStyle::new(20.0).weight(FontWeight::Bold).italic();
        assert_eq!(s.size, Some(20.0));
        assert_eq!(s.weight, Some(FontWeight::Bold));
        assert_eq!(s.italic, Some(true));
        assert_eq!(s.color, None);
    }

    /// **A style can name one thing and leave the rest open**, which is the whole reason
    /// the fields are optional.
    ///
    /// `TextStyle::new(20.0)` used to answer a size, a weight *and* a slant, because the
    /// type had nowhere to put "unset" — so *size 20, inherit the weight* was not
    /// expressible, however the caller wrote it. The reference writes it
    /// `TextStyle(fontSize: 20)`, and now so do we.
    #[test]
    fn a_style_may_name_one_thing_and_leave_the_rest_open() {
        let only_size = TextStyle::new(20.0);
        assert_eq!(only_size.size, Some(20.0));
        assert_eq!(only_size.weight, None, "and says nothing about the weight");

        let inherited = TextStyle::NONE.weight(FontWeight::Bold).merge(only_size);
        assert_eq!(inherited.size, Some(20.0), "the size it named");
        assert_eq!(
            inherited.weight,
            Some(FontWeight::Bold),
            "the weight it did not"
        );
    }

    /// The chain **stops** somewhere: a shaper needs a number and there is nobody left to
    /// ask. Everything unanswered lands on the framework's own default.
    #[test]
    fn a_style_that_says_nothing_resolves_to_the_frameworks_own() {
        let r = TextStyle::NONE.resolved();
        assert_eq!(r.size, DEFAULT_TEXT_SIZE);
        assert_eq!(r.weight, FontWeight::Regular);
        assert!(!r.italic);
        assert_eq!(r.decoration, TextDecoration::NONE);
        // The colour is the one thing left open, its last word belonging to a theme this
        // type cannot see.
        assert_eq!(r.color, None);
    }

    /// The reader's font size lands on **the one place a size becomes a number**.
    #[test]
    fn the_readers_font_size_reaches_a_resolved_style() {
        assert_eq!(
            TextStyle::new(10.0).resolved().size,
            10.0,
            "unscaled by default"
        );
        with_text_scale(1.5, || {
            assert_eq!(TextStyle::new(10.0).resolved().size, 15.0);
            // A style that named no size scales the framework's own, not nothing.
            assert_eq!(
                TextStyle::NONE.resolved().size,
                DEFAULT_TEXT_SIZE * 1.5,
                "the default is a size like any other"
            );
        });
        assert_eq!(
            TextStyle::new(10.0).resolved().size,
            10.0,
            "and the scope put it back"
        );
    }

    /// Nothing but the size moves. A reader asking for larger text has not asked for a
    /// different typeface, and scaling a weight or a slant is not a thing to do.
    #[test]
    fn only_the_size_is_scaled() {
        with_text_scale(2.0, || {
            let r = TextStyle::new(10.0)
                .weight(FontWeight::Bold)
                .italic()
                .resolved();
            assert_eq!(r.size, 20.0);
            assert_eq!(r.weight, FontWeight::Bold);
            assert!(r.italic);
        });
    }

    /// A scale of zero is **ignored**, not obeyed. A font of no size is a screen with no
    /// words on it, and a platform reporting one is a platform to disbelieve rather than a
    /// user to accommodate.
    #[test]
    fn a_scale_of_nothing_is_disbelieved() {
        with_text_scale(0.0, || {
            assert_eq!(TextStyle::new(10.0).resolved().size, 10.0);
        });
        with_text_scale(-2.0, || {
            assert_eq!(TextStyle::new(10.0).resolved().size, 10.0);
        });
    }

    #[test]
    fn merge_overrides_type_but_inherits_missing_colour() {
        let base = TextStyle::new(16.0).color(Color::WHITE);
        // `over` changes size and weight but specifies no colour.
        let over = TextStyle::new(24.0).weight(FontWeight::Bold);
        let merged = base.merge(over);
        assert_eq!(merged.size, Some(24.0));
        assert_eq!(merged.weight, Some(FontWeight::Bold));
        assert_eq!(merged.color, Some(Color::WHITE), "colour inherited");

        // When `over` does specify a colour, it wins.
        let over2 = TextStyle::new(24.0).color(Color::BLACK);
        assert_eq!(base.merge(over2).color, Some(Color::BLACK));
    }

    #[test]
    fn span_children_inherit_unspecified_attributes() {
        // "Hello **bold** _red italic_": the bold part inherits size and colour, the
        // red italic inherits size and weight.
        let span = TextSpan::new("Hello ")
            .child(TextSpan::new("bold").bold())
            .child(
                TextSpan::new("red italic")
                    .italic()
                    .color(Color::rgb(1.0, 0.0, 0.0)),
            );
        let base = TextStyle::new(20.0).color(Color::WHITE);
        let runs = span.flatten(base);

        assert_eq!(runs.len(), 3);
        let (t0, s0) = &runs[0];
        assert_eq!(
            (t0.as_str(), s0.size, s0.weight),
            ("Hello ", 20.0, FontWeight::Regular)
        );
        let (t1, s1) = &runs[1];
        assert_eq!((t1.as_str(), s1.weight), ("bold", FontWeight::Bold));
        assert_eq!(s1.size, 20.0, "bold inherits the size");
        assert_eq!(s1.color, Some(Color::WHITE), "bold inherits the colour");
        let (t2, s2) = &runs[2];
        assert_eq!(t2, "red italic");
        assert!(s2.italic);
        assert_eq!(s2.weight, FontWeight::Regular, "italic inherits the weight");
        assert_eq!(s2.color, Some(Color::rgb(1.0, 0.0, 0.0)));
    }

    #[test]
    fn nested_spans_cascade_depth_first() {
        // A bold subtree in which a grandchild puts the size back to 12.
        let span = TextSpan::new("a").child(
            TextSpan::new("b")
                .bold()
                .child(TextSpan::new("c").size(12.0)),
        );
        let runs = span.flatten(TextStyle::new(16.0));
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[2].0, "c");
        assert_eq!(runs[2].1.size, 12.0);
        assert_eq!(
            runs[2].1.weight,
            FontWeight::Bold,
            "inherits bold from the parent"
        );
    }

    #[test]
    fn decorations_combine_and_cascade() {
        // Combining: underline plus strikethrough on the same style.
        let s = TextStyle::new(16.0).underline().strikethrough().resolved();
        assert!(s.decoration.underline && s.decoration.strikethrough);
        assert!(!s.decoration.overline);
        assert!(!TextDecoration::UNDERLINE.is_none());
        assert!(TextDecoration::NONE.is_none());

        // Cascade: the child inherits the parent's decoration when it specifies
        // nothing, and an explicit `decoration(NONE)` cancels it (Some(NONE) is not
        // the same as absent).
        let span = TextSpan::new("a")
            .underline()
            .decoration_color(Color::rgb(1.0, 0.0, 0.0))
            .child(TextSpan::new("b"))
            .child(TextSpan::new("c").decoration(TextDecoration::NONE));
        let runs = span.flatten(TextStyle::new(16.0));
        assert!(runs[0].1.decoration.underline);
        assert!(
            runs[1].1.decoration.underline,
            "the child inherits the underline"
        );
        assert_eq!(
            runs[1].1.decoration_color,
            Some(Color::rgb(1.0, 0.0, 0.0)),
            "the decoration colour cascades"
        );
        assert!(runs[2].1.decoration.is_none(), "explicitly cancelled");

        // merge: decoration is a typographic attribute, so `over`'s wins, while its
        // colour inherits the way the text colour does.
        let base = TextStyle::new(16.0)
            .underline()
            .decoration_color(Color::BLACK);
        let merged = base.merge(TextStyle::new(20.0).strikethrough()).resolved();
        assert!(merged.decoration.strikethrough && !merged.decoration.underline);
        assert_eq!(merged.decoration_color, Some(Color::BLACK));

        // And a style that names **no** decoration leaves the parent's alone, where it
        // used to erase it: `over.decoration` was a value the type could not withhold.
        let quiet = base.merge(TextStyle::new(20.0)).resolved();
        assert!(quiet.decoration.underline, "the underline survived");
    }

    #[test]
    fn empty_nodes_produce_no_run() {
        // A "group" node with no text of its own only exists to style its children.
        let span = TextSpan::new("").bold().child(TextSpan::new("x"));
        let runs = span.flatten(TextStyle::new(16.0));
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].1.weight, FontWeight::Bold);
    }

    /// A style that says nothing about its line height gets the framework's default, and
    /// one that names a ratio gets that ratio **of its own size**.
    #[test]
    fn a_line_height_is_a_ratio_of_the_size_that_asked_for_it() {
        assert_eq!(TextStyle::new(20.0).resolved().line_height(), 24.0);
        let open = TextStyle {
            height: Some(1.5),
            ..TextStyle::new(20.0)
        };
        assert_eq!(open.resolved().line_height(), 30.0);
        let packed = TextStyle {
            height: Some(1.0),
            ..TextStyle::new(20.0)
        };
        assert_eq!(packed.resolved().line_height(), 20.0);
    }

    /// **The leading grows with the letters.** A ratio rather than a length is what makes
    /// a paragraph keep its rhythm when the reader turns the type up: a `height` of 1.5 is
    /// 30 px at a size of 20 and 60 px when that 20 has been doubled for a reader who
    /// asked for larger text. A length would have stayed at 30 and closed the paragraph up
    /// exactly when it needed opening.
    #[test]
    fn the_leading_grows_with_the_reader() {
        let style = TextStyle {
            height: Some(1.5),
            ..TextStyle::new(20.0)
        };
        assert_eq!(style.resolved().line_height(), 30.0);
        with_text_scale(2.0, || {
            let resolved = style.resolved();
            assert_eq!(resolved.size, 40.0);
            assert_eq!(resolved.line_height(), 60.0);
        });
    }

    /// It inherits like every other field, and a nearer style still wins.
    #[test]
    fn a_line_height_is_inherited_and_overridable() {
        let handed_down = TextStyle {
            height: Some(1.8),
            ..TextStyle::NONE
        };
        let asks_only_a_size = TextStyle::new(10.0);
        assert_eq!(
            handed_down.merge(asks_only_a_size).resolved().line_height(),
            18.0,
            "the size is the near style's and the height the inherited one"
        );
        let asks_for_both = TextStyle {
            height: Some(1.0),
            ..TextStyle::new(10.0)
        };
        assert_eq!(
            handed_down.merge(asks_for_both).resolved().line_height(),
            10.0
        );
    }
}

//! [`TextStyle`]: the typographic attributes of a piece of text (size, weight,
//! italic, colour), independent of any widget or theme.
//!
//! A `TextStyle` is a pure `Copy` value. Its `color` is optional, and `None`
//! means "inherit" — the widget resolves it to the theme colour at paint time.
//! Named typographic scales, in the style of a Material `TextTheme`, are built
//! out of this type.

use crate::Color;

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

/// The typographic attributes of a single line of text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// Font size, in logical pixels.
    pub size: f32,
    /// Weight.
    pub weight: FontWeight,
    /// Italic.
    pub italic: bool,
    /// Explicit colour; `None` means inherited, resolved by the widget at paint.
    pub color: Option<Color>,
    /// Decoration lines (underline, strikethrough, and so on).
    pub decoration: TextDecoration,
    /// Decoration colour; `None` means the text's own colour.
    pub decoration_color: Option<Color>,
}

impl TextStyle {
    /// A style of size `size`, regular weight, inherited colour.
    pub const fn new(size: f32) -> Self {
        Self {
            size,
            weight: FontWeight::Regular,
            italic: false,
            color: None,
            decoration: TextDecoration::NONE,
            decoration_color: None,
        }
    }

    /// Sets the weight.
    pub const fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Switches to italic.
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Sets the size.
    pub const fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the colour.
    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the decoration lines.
    pub const fn decoration(mut self, decoration: TextDecoration) -> Self {
        self.decoration = decoration;
        self
    }

    /// Adds an underline, combining with any other decoration.
    pub const fn underline(mut self) -> Self {
        self.decoration = self.decoration.combine(TextDecoration::UNDERLINE);
        self
    }

    /// Adds a strikethrough, combining with any other decoration.
    pub const fn strikethrough(mut self) -> Self {
        self.decoration = self.decoration.combine(TextDecoration::STRIKETHROUGH);
        self
    }

    /// Sets the decoration colour; otherwise the text's colour is used.
    pub const fn decoration_color(mut self, color: Color) -> Self {
        self.decoration_color = Some(color);
        self
    }

    /// **Merges** `over` on top of `self`: `over`'s typographic attributes win,
    /// and its colour **inherits** from `self` when absent (`None`). This is the
    /// cascade: span style > default style > theme.
    pub fn merge(self, over: TextStyle) -> TextStyle {
        TextStyle {
            size: over.size,
            weight: over.weight,
            italic: over.italic,
            color: over.color.or(self.color),
            decoration: over.decoration,
            decoration_color: over.decoration_color.or(self.decoration_color),
        }
    }
}

/// **Partial** style overrides — every absent field inherits from the parent.
/// Internal: it is assembled through [`TextSpan`]'s builders.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Overrides {
    size: Option<f32>,
    weight: Option<FontWeight>,
    italic: Option<bool>,
    color: Option<Color>,
    decoration: Option<TextDecoration>,
    decoration_color: Option<Color>,
}

impl Overrides {
    /// Applies the overrides on top of an inherited, **resolved** style.
    fn apply(self, base: TextStyle) -> TextStyle {
        TextStyle {
            size: self.size.unwrap_or(base.size),
            weight: self.weight.unwrap_or(base.weight),
            italic: self.italic.unwrap_or(base.italic),
            color: self.color.or(base.color),
            decoration: self.decoration.unwrap_or(base.decoration),
            decoration_color: self.decoration_color.or(base.decoration_color),
        }
    }
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
    overrides: Overrides,
    children: Vec<TextSpan>,
}

impl TextSpan {
    /// A text fragment with no overrides — it inherits everything.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            overrides: Overrides::default(),
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
        let current = self.overrides.decoration.unwrap_or(TextDecoration::NONE);
        self.overrides.decoration = Some(current.combine(TextDecoration::UNDERLINE));
        self
    }

    /// Strikes through this subtree.
    pub fn strikethrough(mut self) -> Self {
        let current = self.overrides.decoration.unwrap_or(TextDecoration::NONE);
        self.overrides.decoration = Some(current.combine(TextDecoration::STRIKETHROUGH));
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

    /// Applies a complete [`TextStyle`] as an override; its colour only overrides
    /// when it is actually specified.
    pub fn style(mut self, style: TextStyle) -> Self {
        self.overrides = Overrides {
            size: Some(style.size),
            weight: Some(style.weight),
            italic: Some(style.italic),
            color: style.color,
            decoration: Some(style.decoration),
            decoration_color: style.decoration_color,
        };
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
    pub fn flatten(&self, base: TextStyle) -> Vec<(String, TextStyle)> {
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

    fn collect(&self, inherited: TextStyle, out: &mut Vec<(String, TextStyle)>) {
        let effective = self.overrides.apply(inherited);
        if !self.text.is_empty() {
            out.push((self.text.clone(), effective));
        }
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
        assert_eq!(s.size, 20.0);
        assert_eq!(s.weight, FontWeight::Bold);
        assert!(s.italic);
        assert_eq!(s.color, None);
    }

    #[test]
    fn merge_overrides_type_but_inherits_missing_colour() {
        let base = TextStyle::new(16.0).color(Color::WHITE);
        // `over` changes size and weight but specifies no colour.
        let over = TextStyle::new(24.0).weight(FontWeight::Bold);
        let merged = base.merge(over);
        assert_eq!(merged.size, 24.0);
        assert_eq!(merged.weight, FontWeight::Bold);
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
        let s = TextStyle::new(16.0).underline().strikethrough();
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
        let merged = base.merge(TextStyle::new(20.0).strikethrough());
        assert!(merged.decoration.strikethrough && !merged.decoration.underline);
        assert_eq!(merged.decoration_color, Some(Color::BLACK));
    }

    #[test]
    fn empty_nodes_produce_no_run() {
        // A "group" node with no text of its own only exists to style its children.
        let span = TextSpan::new("").bold().child(TextSpan::new("x"));
        let runs = span.flatten(TextStyle::new(16.0));
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].1.weight, FontWeight::Bold);
    }
}

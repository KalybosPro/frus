//! **Per-widget defaults**: the middle term of `what the caller said ?? what the theme
//! says ?? what the framework ships`.
//!
//! Three milestones in a row ended on the same missing piece. An application could
//! override one card's elevation, one divider's height, one splash's colour — and had
//! no way to say *every card in this application*, short of writing its own wrapper
//! around each widget. The reference resolves nearly every property through exactly
//! this chain, and only the middle term was absent here.
//!
//! Every field is an `Option`, and `None` means "the framework's own default" — so a
//! theme that sets nothing behaves exactly as no theme at all, and a theme that sets one
//! field changes one thing.
//!
//! ```ignore
//! let mut theme = Theme::dark();
//! theme.widgets.card.elevation = Some(0.0);       // a flat application
//! theme.widgets.divider.height = Some(1.0);       // hairlines, flush
//! theme.widgets.ink.color = Some(accent.fade(0.2));
//! ```
//!
//! **Layout properties are included** — a divider's height, a card's margin, a drawer's
//! width — which is why [`Widget::style_themed`](crate::Widget::style_themed) exists
//! beside `style`. A theme that could only reach paint would be able to recolour a
//! divider but not make one thin, which is the setting an application actually wants.

use frus_core::{BorderRadius, Color, TextAlign, TextOverflow, TextStyle};

use crate::card::CardVariant;

/// The per-widget defaults carried by a [`Theme`](crate::Theme).
///
/// Adding a widget here is adding a field: the pattern is one `Option` per builder the
/// widget already has, resolved in the same order everywhere.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WidgetThemes {
    pub app_bar: AppBarTheme,
    pub badge: BadgeTheme,
    pub button: ButtonTheme,
    pub card: CardTheme,
    pub checkbox: CheckboxTheme,
    pub chip: ChipTheme,
    pub divider: DividerTheme,
    pub drawer: DrawerTheme,
    pub icon: IconTheme,
    pub icon_button: IconButtonTheme,
    pub ink: InkTheme,
    pub radio: RadioTheme,
    pub segmented: SegmentedTheme,
    pub slider: SliderTheme,
    pub switch: SwitchTheme,
    pub tab_bar: TabBarTheme,
    pub text: DefaultTextStyle,
    pub text_field: TextFieldTheme,
}

/// Defaults for [`Badge`](crate::Badge).
///
/// A badge is an **alert** rather than an accent, which is why its untold fill is the
/// scheme's `error` and not its `primary`. The two sizes are the reference's: `small_size`
/// is the diameter of a badge with no label, `large_size` the height of one that carries
/// a count.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BadgeTheme {
    /// The pill's fill.
    pub background_color: Option<Color>,
    /// The label's colour on that fill.
    pub text_color: Option<Color>,
    /// The label's type.
    pub text_style: Option<TextStyle>,
    /// The diameter of a badge with no label.
    pub small_size: Option<f32>,
    /// The height of a badge that carries one.
    pub large_size: Option<f32>,
    /// The label's room either side.
    pub padding: Option<f32>,
}

/// Defaults for [`Checkbox`](crate::Checkbox).
///
/// A checkbox is two controls wearing one name: ticked it is a **filled box** with a
/// mark punched through it, unticked it is an **outline** and nothing else. The colours
/// do not carry over between the two, which is why they are named apart rather than as
/// one "active"/"inactive" pair.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CheckboxTheme {
    /// The box's fill, ticked.
    pub fill_color: Option<Color>,
    /// The tick drawn on that fill.
    pub check_color: Option<Color>,
    /// The outline, unticked and at rest.
    pub border_color: Option<Color>,
    /// The outline under a pointer, a finger or focus. The reference resolves this side
    /// per state, and an outline that did not answer would look inert.
    pub active_border_color: Option<Color>,
    /// The box's corner radius.
    pub radius: Option<f32>,
    /// The label beside it.
    pub label_color: Option<Color>,
}

/// Defaults for [`RadioGroup`](crate::RadioGroup).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RadioTheme {
    /// The ring and the dot of the chosen option.
    pub selected_color: Option<Color>,
    /// The ring of an option that is not chosen, at rest.
    pub border_color: Option<Color>,
    /// That ring under a pointer, a finger or focus.
    pub active_border_color: Option<Color>,
    /// The labels.
    pub label_color: Option<Color>,
}

/// Defaults for [`Slider`](crate::Slider) and [`RangeSlider`](crate::RangeSlider).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SliderTheme {
    /// The travelled part of the track.
    pub active_track_color: Option<Color>,
    /// The part still to travel.
    pub inactive_track_color: Option<Color>,
    /// The thumb's fill.
    pub thumb_color: Option<Color>,
    /// The ring around the thumb.
    pub thumb_border_color: Option<Color>,
}

/// Defaults for [`Switch`](crate::Switch).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwitchTheme {
    /// The track, on.
    pub track_color: Option<Color>,
    /// The track, off.
    pub inactive_track_color: Option<Color>,
    /// The thumb, on.
    pub thumb_color: Option<Color>,
    /// The thumb, off. Unset it follows the on colour, which is what the reference does
    /// and what a switch looks like: one thumb sliding, not two.
    pub inactive_thumb_color: Option<Color>,
}

/// The text style a **subtree** hands down — the reference's `DefaultTextStyle`, and an
/// inherited widget there for the reason it is a theme entry here: an app bar, a list
/// tile or a dialog wants every run of words inside it to read the same way without
/// reaching into each one, including the ones it never sees because a caller passed them
/// in already assembled.
///
/// It is the reference's shape exactly: a [`TextStyle`] plus the four questions that are
/// about the **box** rather than the type. It carried its own copy of every typographic
/// field until `TextStyle` learned to say *unset*; now the style is just a style.
///
/// A [`Text`](crate::Text) resolves `what the caller said ?? what this says ?? what the
/// framework ships`, field by field, and **a default the caller did not choose does not
/// count as having said something** — which is what an `Option` per field means and what
/// `TextStyle::new(16.0)` could not express.
///
/// [`DefaultTextStyle::around`] wraps a subtree with one, and that is how
/// [`AppBar::toolbar_text_style`](crate::AppBar::toolbar_text_style) delivers its own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefaultTextStyle {
    /// The type: size, weight, slant, colour, decoration — each answered or left open.
    pub style: TextStyle,
    /// Where the lines sit inside the box.
    pub align: Option<TextAlign>,
    /// Whether the text wraps at the width it is given.
    pub soft_wrap: Option<bool>,
    /// What becomes of text that does not fit.
    pub overflow: Option<TextOverflow>,
    /// At most this many lines.
    pub max_lines: Option<usize>,
}

impl DefaultTextStyle {
    /// Nothing said at all — what a theme carries until something sets a field.
    pub const NONE: Self = Self {
        style: TextStyle::NONE,
        align: None,
        soft_wrap: None,
        overflow: None,
        max_lines: None,
    };

    /// A handover of type and nothing else.
    pub const fn from_text_style(style: TextStyle) -> Self {
        Self {
            style,
            ..Self::NONE
        }
    }

    /// This style with `over`'s answers laid on top, **field by field**: where `over` said
    /// nothing, this one's answer survives.
    ///
    /// The operation an inherited style needs at every level it passes through — two
    /// nested subtrees each setting one field must leave a text wearing both.
    #[must_use]
    pub fn merge(self, over: Self) -> Self {
        Self {
            style: self.style.merge(over.style),
            align: over.align.or(self.align),
            soft_wrap: over.soft_wrap.or(self.soft_wrap),
            overflow: over.overflow.or(self.overflow),
            max_lines: over.max_lines.or(self.max_lines),
        }
    }

    /// Wraps `child` so that every [`Text`](crate::Text) in its subtree wears this style
    /// where it has not answered for itself — the reference's `DefaultTextStyle` widget.
    ///
    /// **Merged onto** whatever an enclosing subtree already handed down rather than
    /// replacing it, so two nested wrappers each setting one field leave a text wearing
    /// both:
    ///
    /// ```ignore
    /// DefaultTextStyle::from_text_style(TextStyle::NONE.color(muted))
    ///     .around(a_column_of_labels)
    /// ```
    pub fn around<Msg: 'static>(
        self,
        child: impl crate::Widget<Msg> + 'static,
    ) -> crate::Themed<Msg> {
        crate::Themed::tweak(move |t| t.widgets.text = t.widgets.text.merge(self), child)
    }
}

/// Defaults for [`Icon`](crate::Icon) — the reference's `IconTheme`, which is an
/// **inherited** widget there for the same reason this is scoped here: an app bar, a
/// list tile or a button bar wants every glyph inside it one colour without recolouring
/// the words beside them.
///
/// `Themed::tweak` is how a subtree sets it, and that is exactly how
/// [`AppBar::icon_theme`](crate::AppBar::icon_theme) delivers its own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IconTheme {
    /// The glyph's colour. Unset, the scheme's `on_surface` — an icon is text's peer,
    /// not a decoration.
    pub color: Option<Color>,
    /// The square's side, in logical pixels. Unset, 24: the grid the icons are drawn on.
    pub size: Option<f32>,
}

/// Defaults for [`AppBar`](crate::AppBar).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AppBarTheme {
    /// Whether the title is centred. Unset, the **platform**'s convention decides, which
    /// is the one place the framework's last word is a system convention rather than a
    /// taste of its own.
    pub center_title: Option<bool>,
    /// The title's type.
    pub title_style: Option<TextStyle>,
    /// The bar's surface.
    pub background: Option<Color>,
    /// What is drawn on it — the title's colour.
    pub foreground: Option<Color>,
    /// The shadow's depth. Unset, the bar is flat, as the reference's is.
    pub elevation: Option<f32>,
    /// The toolbar's height.
    pub height: Option<f32>,
    /// The colour of the shadow an elevated bar casts. Unset, the framework's near-black.
    pub shadow_color: Option<Color>,
    /// The colour laid over the surface in proportion to the elevation — Material 3's
    /// way of showing height, and the one that still reads on a dark background.
    pub surface_tint: Option<Color>,
    /// How far the bar's corners are rounded. Unset, square.
    pub shape: Option<BorderRadius>,
}

/// Defaults for [`TextField`](crate::TextField).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextFieldTheme {
    /// The container's fill. A filled field takes the theme's high surface container by
    /// default; an outlined one has none.
    pub fill: Option<Color>,
    /// The border, or the underline, at rest.
    pub border_color: Option<Color>,
    /// The border once focused.
    pub focused_border_color: Option<Color>,
    /// Border, label and helper colour while an error is showing.
    pub error_color: Option<Color>,
    /// The value's colour.
    pub text_color: Option<Color>,
    /// The label and the hint, at rest.
    pub label_color: Option<Color>,
    /// The label once focused.
    pub focused_label_color: Option<Color>,
    /// The helper line.
    pub helper_color: Option<Color>,
    /// The prefix and suffix icons.
    pub icon_color: Option<Color>,
    /// Corner radius.
    pub radius: Option<f32>,
    /// Border weight at rest.
    pub border_width: Option<f32>,
    /// Border weight once focused.
    pub focused_border_width: Option<f32>,
}

/// Defaults for [`Button`](crate::Button).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ButtonTheme {
    /// The surface under the label, whatever the variant would have used.
    pub color: Option<Color>,
    /// The label's colour.
    pub label_color: Option<Color>,
    /// The label's type.
    pub label_style: Option<TextStyle>,
    /// The outline's colour.
    pub border_color: Option<Color>,
    /// Its thickness; `0.0` removes it.
    pub border_width: Option<f32>,
    /// The corner radii. Unset, a button is a stadium.
    pub radius: Option<BorderRadius>,
    /// The room either side of the label.
    pub padding: Option<f32>,
    /// The button's height.
    pub height: Option<f32>,
    /// The narrowest it will be, however short its label.
    pub min_width: Option<f32>,
    /// How far it sits off the surface.
    pub elevation: Option<f32>,
}

/// Defaults for [`Card`](crate::Card).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CardTheme {
    /// Which of the three cards an untold `Card::new()` is.
    pub variant: Option<CardVariant>,
    /// How far off the surface it sits.
    pub elevation: Option<f32>,
    /// Its background, overriding the variant's tone.
    pub color: Option<Color>,
    /// Its corner radii.
    pub radius: Option<BorderRadius>,
    /// The room it leaves around itself.
    pub margin: Option<f32>,
    /// The room it leaves inside itself.
    pub padding: Option<f32>,
}

/// Defaults for [`Chip`](crate::Chip).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChipTheme {
    /// The surface under an unselected chip.
    pub color: Option<Color>,
    /// The surface under a selected one.
    pub selected_color: Option<Color>,
    /// The label's colour when unselected.
    pub label_color: Option<Color>,
    /// The label's colour when selected.
    pub selected_label_color: Option<Color>,
    /// The label's type.
    pub label_style: Option<TextStyle>,
    /// The outline's colour.
    pub border_color: Option<Color>,
    /// Its thickness; `0.0` removes it.
    pub border_width: Option<f32>,
    /// The corner radii.
    pub radius: Option<BorderRadius>,
    /// The room inside the outline.
    pub padding: Option<f32>,
    /// The room on either side of the label.
    pub label_padding: Option<f32>,
    /// The chip's height.
    pub height: Option<f32>,
    /// The size of a leading icon, a checkmark or a delete cross.
    pub icon_size: Option<f32>,
    /// Whether a selected chip shows a checkmark.
    pub show_checkmark: Option<bool>,
}

/// Defaults for [`Divider`](crate::Divider).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DividerTheme {
    /// The room the separator takes in the layout.
    pub height: Option<f32>,
    /// The thickness of the line drawn inside that room.
    pub thickness: Option<f32>,
    /// Its colour.
    pub color: Option<Color>,
    /// Its inset from the leading edge.
    pub indent: Option<f32>,
    /// Its inset from the trailing edge.
    pub end_indent: Option<f32>,
}

/// Defaults for [`Drawer`](crate::Drawer).
///
/// The rounding is on the **inner** edge only — the one that meets the content — and
/// the outer edge stays square against the window it is docked to, which is what the
/// reference's own shape resolves to. Which edge that is follows the direction: the
/// panel does not know which side of the screen it landed on until it paints.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawerTheme {
    /// The panel's width.
    pub width: Option<f32>,
    /// The panel's fill.
    pub background_color: Option<Color>,
    /// The hairline on the panel's inner edge.
    pub border_color: Option<Color>,
    /// That hairline's thickness; `0.0` removes it.
    pub border_width: Option<f32>,
    /// The rounding of the **inner** edge's two corners — the edge that meets the
    /// content. The outer one stays square against the window.
    pub radius: Option<f32>,
    /// How far off the surface the panel sits. `0.0` casts no shadow.
    pub elevation: Option<f32>,
    /// The scrim behind a modal panel, **alpha included**.
    pub scrim_color: Option<Color>,
}

/// Defaults for [`SegmentedButton`](crate::SegmentedButton).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SegmentedTheme {
    /// The fill under the chosen segment.
    pub selected_color: Option<Color>,
    /// The labels' colour.
    pub label_color: Option<Color>,
    /// The chosen segment's label colour, and its checkmark's.
    pub selected_label_color: Option<Color>,
    /// The outline's colour, and the hairlines' between segments.
    pub border_color: Option<Color>,
    /// Their thickness; `0.0` removes both.
    pub border_width: Option<f32>,
    /// The group's outer radius. Unset, the ends are stadium-rounded.
    pub radius: Option<f32>,
    /// The control's height.
    pub height: Option<f32>,
    /// The room either side of a label.
    pub padding: Option<f32>,
    /// The labels' type.
    pub label_style: Option<TextStyle>,
    /// The checkmark's size.
    pub icon_size: Option<f32>,
    /// Whether the chosen segment carries a checkmark.
    pub show_selected_icon: Option<bool>,
}

/// Defaults for [`TabBar`](crate::TabBar) — the bar, its labels and its indicator.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TabBarTheme {
    /// Which of the two bars an untold `TabBar::new()` is.
    pub variant: Option<crate::tabs::TabBarVariant>,
    /// The indicator's colour.
    pub indicator_color: Option<Color>,
    /// Its thickness — and, on a primary bar, the radius of its top corners.
    pub indicator_weight: Option<f32>,
    /// The **selected** label's colour.
    pub label_color: Option<Color>,
    /// Every other label's colour.
    pub unselected_label_color: Option<Color>,
    /// The labels' type.
    pub label_style: Option<TextStyle>,
    /// The colour of the hairline between the bar and what it labels.
    pub divider_color: Option<Color>,
    /// Its thickness; `0.0` removes it.
    pub divider_height: Option<f32>,
    /// The room on either side of a label.
    pub label_padding: Option<f32>,
    /// The tabs' height, the indicator excluded.
    pub tab_height: Option<f32>,
    /// Where the tabs sit when they do not fill the bar.
    pub alignment: Option<crate::tabs::TabAlignment>,
}

/// Defaults for [`IconButton`](crate::IconButton).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IconButtonTheme {
    /// The box's side.
    pub size: Option<f32>,
    /// The glyph's size inside it.
    pub icon_size: Option<f32>,
    /// The surface under the glyph.
    pub color: Option<Color>,
    /// The glyph's colour.
    pub icon_color: Option<Color>,
    /// The outline's colour.
    pub border_color: Option<Color>,
    /// Its thickness; `0.0` removes it.
    pub border_width: Option<f32>,
    /// The corner radii. Unset, an icon button is a circle.
    pub radius: Option<BorderRadius>,
}

/// Defaults for the **ink ripple** — every surface that splashes, including
/// [`InkWell`](crate::InkWell) and [`Button`](crate::Button).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InkTheme {
    /// The splash colour, **alpha included**. A widget that computes its own from its
    /// surface (a button splashing in its `on` colour) still yields to this when it is
    /// set: an application that has chosen an ink colour has chosen it everywhere.
    pub color: Option<Color>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardVariant, CARD_MARGIN};
    use crate::divider::{Divider, DIVIDER_SPACE};
    use crate::flex::Flex;
    use crate::runtime::Runtime;
    use crate::theme::Theme;
    use crate::ui::build_ui;
    use crate::widget::Widget;
    use frus_core::{Color, Primitive, Rect, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {}

    /// Builds the widget **inside a sized parent**, which is where a margin and an
    /// `Auto` width mean anything, and returns the frame's primitives.
    fn framed(widget: impl Widget<Msg> + 'static, theme: &Theme) -> Vec<Primitive> {
        let tree = Flex::column().width(200.0).height(100.0).child(widget);
        build_ui(&tree, Size::new(200.0, 100.0), &Runtime::default(), theme)
            .scene()
            .primitives()
            .to_vec()
    }

    /// The first crisp box the tree paints — the widget's own surface, past any shadow.
    fn painted_rect(widget: impl Widget<Msg> + 'static, theme: &Theme) -> Rect {
        framed(widget, theme)
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, blur, .. } if *blur == 0.0 => Some(*rect),
                _ => None,
            })
            .expect("the widget paints a box")
    }

    #[test]
    fn the_theme_reaches_layout_and_not_only_paint() {
        // The point of `style_themed`. A divider's *height* is a layout property, and a
        // theme that could not set it would be able to recolour a separator but not make
        // one thin — which is the setting an application actually asks for.
        let plain = Theme::default();
        let mut thin = Theme::default();
        thin.widgets.divider.height = Some(1.0);

        // The line is centred in its box, so a taller box puts it further down; the box
        // itself is what the theme moved.
        // The line is centred in its box: a 16 px box puts it at 7.5, a 1 px box at 0.
        let tall = painted_rect(Divider::new(), &plain);
        let thin_line = painted_rect(Divider::new(), &thin);
        assert!(
            tall.y > thin_line.y,
            "the themed divider's box is shorter: {tall:?} against {thin_line:?}"
        );
        assert_eq!(tall.height, 1.0, "the line itself is unchanged");
        assert_eq!(thin_line.height, 1.0);
    }

    #[test]
    fn the_caller_outranks_the_theme_and_the_theme_outranks_the_framework() {
        // The whole chain in one assertion per link, on a layout property so that both
        // halves of the resolution are exercised.
        let plain = Theme::default();
        let mut themed = Theme::default();
        themed.widgets.card.margin = Some(20.0);

        let x = |card: Card<Msg>, theme: &Theme| painted_rect(card, theme).x;

        assert_eq!(x(Card::new(), &plain), CARD_MARGIN, "the framework's");
        assert_eq!(x(Card::new(), &themed), 20.0, "the theme's");
        assert_eq!(
            x(Card::new().margin(2.0), &themed),
            2.0,
            "the caller's, over the theme's"
        );
    }

    #[test]
    fn a_theme_can_change_what_an_untold_widget_is() {
        // Not just a number: the *variant* is themeable, so an application can be flat
        // throughout without writing `.filled()` at every call site.
        let mut flat = Theme::default();
        flat.widgets.card.variant = Some(CardVariant::Filled);
        let shadows = |theme: &Theme| {
            framed(Card::<Msg>::new(), theme)
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { blur, .. } if *blur > 0.0))
                .count()
        };
        assert_eq!(shadows(&Theme::default()), 1, "elevated by default");
        assert_eq!(shadows(&flat), 0, "flat because the theme says so");
    }

    #[test]
    fn the_ink_colour_is_themeable_even_where_a_widget_computes_its_own() {
        // A `Button` derives its splash from its own `on` colour, which is the right
        // default and the wrong thing to insist on: an application that has chosen an
        // ink colour has chosen it for every surface.
        let mine = Color::rgb8(255, 0, 128);
        let mut theme = Theme::default();
        theme.widgets.ink.color = Some(mine);
        let button = crate::button::Button::<Msg>::new("Go");
        assert_eq!(
            Widget::<Msg>::ink(&button, &theme).map(|i| i.color),
            Some(mine)
        );
        // And a plain surface takes it too.
        assert_eq!(crate::ink::default_splash(&theme), mine);
    }

    #[test]
    fn changing_a_theme_invalidates_the_layout_cache() {
        // The cache keys on a fingerprint of the *effective* style, which now includes
        // the theme's say. If it did not, a theme swap would keep the old geometry and
        // the change would appear only after something else moved.
        let runtime = Runtime::default();
        let tree = Flex::<Msg>::column()
            .width(200.0)
            .height(100.0)
            .child(Divider::new());
        let plain = Theme::default();
        let mut thin = Theme::default();
        thin.widgets.divider.height = Some(1.0);

        let height = |theme: &Theme| {
            build_ui(&tree, Size::new(200.0, 100.0), &runtime, theme)
                .scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { rect, .. } => Some(rect.y),
                    _ => None,
                })
                .expect("a line")
        };
        // The same runtime, and therefore the same cache, across both builds.
        let first = height(&plain);
        let second = height(&thin);
        assert_ne!(
            first, second,
            "a themed size must not be served from the cache of an unthemed one"
        );
        assert_eq!(height(&plain), first, "and back again");
    }

    #[test]
    fn the_defaults_are_still_reachable_through_an_empty_theme() {
        let plain = Theme::default();
        assert_eq!(
            Widget::<Msg>::style_themed(&Divider::new(), &plain).height,
            frus_layout::Dimension::Length(DIVIDER_SPACE)
        );
    }

    #[test]
    fn a_theme_that_says_nothing_is_the_absence_of_a_theme() {
        // The whole contract of `None`: a fresh theme must carry no opinions at all, or
        // every widget's built-in default silently becomes unreachable.
        assert_eq!(Theme::default().widgets, WidgetThemes::default());
        assert_eq!(WidgetThemes::default().card.elevation, None);
        assert_eq!(WidgetThemes::default().divider.height, None);
        assert_eq!(WidgetThemes::default().drawer.width, None);
        assert_eq!(WidgetThemes::default().ink.color, None);
        assert_eq!(WidgetThemes::default().tab_bar.indicator_color, None);
        assert_eq!(WidgetThemes::default().chip.height, None);
        assert_eq!(WidgetThemes::default().button.height, None);
        assert_eq!(WidgetThemes::default().segmented.height, None);
        assert_eq!(WidgetThemes::default().icon_button.size, None);
    }
}

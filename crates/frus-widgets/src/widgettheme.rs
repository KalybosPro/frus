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

use frus_core::{BorderRadius, Color, Insets, ShapeBorder, TextAlign, TextOverflow, TextStyle};

use crate::card::CardVariant;

/// **The shape a box takes**, on the rungs everything here is resolved on
/// (`card.dart`, `button_style.dart`, and every other `shape` in the reference).
///
/// The caller's word first, then the theme's shape, then the theme's plain **radius** read
/// as a rounded rectangle, then the widget's own default.
///
/// The third rung is the one worth explaining. A radius was the only thing a theme could
/// say until milestone 450, and applications have written them; a theme that names a
/// `shape` outranks one that names only a `radius`, and naming both is naming the shape.
pub fn resolve_shape(
    own: Option<ShapeBorder>,
    themed: Option<ShapeBorder>,
    radius: Option<BorderRadius>,
    fallback: ShapeBorder,
) -> ShapeBorder {
    own.or(themed)
        .or_else(|| radius.map(ShapeBorder::rounded))
        .unwrap_or(fallback)
}

/// The per-widget defaults carried by a [`Theme`](crate::Theme).
///
/// Adding a widget here is adding a field: the pattern is one `Option` per builder the
/// widget already has, resolved in the same order everywhere.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetThemes {
    pub alert: AlertTheme,
    pub dialog: DialogTheme,
    pub app_bar: AppBarTheme,
    pub autocomplete: AutocompleteTheme,
    pub badge: BadgeTheme,
    pub banner: BannerTheme,
    pub bottom_app_bar: BottomAppBarTheme,
    pub bottom_sheet: BottomSheetTheme,
    pub breadcrumb: BreadcrumbTheme,
    pub button: ButtonTheme,
    pub card: CardTheme,
    pub checkbox: CheckboxTheme,
    pub chip: ChipTheme,
    pub date_picker: DatePickerTheme,
    pub divider: DividerTheme,
    pub drawer: DrawerTheme,
    /// Defaults for [`ListTile`](crate::ListTile).
    pub list_tile: ListTileTheme,
    pub dropdown: DropdownTheme,
    pub form: FormTheme,
    pub icon: IconTheme,
    pub icon_button: IconButtonTheme,
    pub ink: InkTheme,
    pub kanban: KanbanTheme,
    pub kbd: KbdTheme,
    pub menu: MenuTheme,
    pub nav_rail: NavRailTheme,
    pub radio: RadioTheme,
    pub segmented: SegmentedTheme,
    pub slider: SliderTheme,
    pub snack_bar: SnackBarTheme,
    pub steps: StepsTheme,
    pub switch: SwitchTheme,
    pub tab_bar: TabBarTheme,
    pub table: TableTheme,
    pub text: DefaultTextStyle,
    pub text_field: TextFieldTheme,
    pub time_picker: TimePickerTheme,
    pub timeline: TimelineTheme,
    pub tree: TreeTheme,
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
    /// **How much room it reserves for a finger** — the reference's per-widget say over
    /// the theme's (`checkbox.dart:512`). Unset, the theme's own answer.
    pub tap_target: Option<crate::theme::TapTarget>,
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
    /// **How much room it reserves for a finger** — the reference's per-widget say over
    /// the theme's (`radio.dart:734`). Unset, the theme's own answer.
    pub tap_target: Option<crate::theme::TapTarget>,
}

/// Defaults for [`Slider`](crate::Slider) and [`RangeSlider`](crate::RangeSlider).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SliderTheme {
    /// The value bubble's type. The reference calls it the *value indicator* and sets it in
    /// `labelLarge`.
    pub value_indicator_text_style: Option<TextStyle>,
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
    /// The glyph inside the thumb, on. Unset, the scheme's `on_primary_container`
    /// (`switch.dart:2338`).
    pub icon_color: Option<Color>,
    /// The glyph inside the thumb, off. Unset, the scheme's `surface_container_highest`
    /// (`switch.dart:2349`) — the track's own colour, so the glyph reads as a hole in the
    /// thumb rather than as a mark on it.
    pub inactive_icon_color: Option<Color>,
    /// **How much room it reserves for a finger** — the reference's per-widget say over
    /// the theme's (`switch.dart:603`). Unset, the theme's own answer.
    pub tap_target: Option<crate::theme::TapTarget>,
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
    /// **What shape the bar is** — the reference's `AppBarTheme.shape`. Unset, square.
    ///
    /// It was an `Option<BorderRadius>` under this name until milestone 455: the last
    /// theme field left carrying the reference's *word* with a corner radius behind it.
    pub shape: Option<ShapeBorder>,
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
    /// The same under the pointer: an errored field **deepens** on hover
    /// (`input_decorator.dart:5981`). Unset, the scheme's `on_error_container`.
    pub error_hover_color: Option<Color>,
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
    /// **What shape it is** (`shape_border.dart`), over the plain `radius` below:
    /// a button's, whose default is a **pill** (`button_style.dart`). Unset, the widget decides.
    pub shape: Option<ShapeBorder>,
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
    /// **What shape it is** (`shape_border.dart`), over the plain `radius` below:
    /// a card's. Unset, the widget decides.
    pub shape: Option<ShapeBorder>,
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
    /// **What shape it is** (`shape_border.dart`), over the plain `radius` below:
    /// a chip's. Unset, the widget decides.
    pub shape: Option<ShapeBorder>,
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

/// Defaults for [`ListTile`](crate::ListTile) — the reference's `ListTileThemeData`.
///
/// Every field is what the tile would otherwise decide for itself, on the usual rungs: the
/// tile's own word, then this, then the framework's. An application that wants all its
/// tiles rounded, or all its selected rows tinted, says it **once** here rather than on
/// every tile it ever builds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ListTileTheme {
    /// The tile's surface. Unset, transparent — a tile takes the colour of what it sits on.
    pub tile_color: Option<Color>,
    /// The surface while the tile is the chosen one.
    pub selected_tile_color: Option<Color>,
    /// The colour its words and slots take while it is the chosen one.
    pub selected_color: Option<Color>,
    /// The two slots' icon colour.
    pub icon_color: Option<Color>,
    /// The title's and subtitle's colour.
    pub text_color: Option<Color>,
    /// What shape the tile is — taken by its surface and by its ink.
    pub shape: Option<ShapeBorder>,
    /// The room kept inside the tile, round its content.
    pub content_padding: Option<Insets>,
    /// The title's type.
    pub title_style: Option<TextStyle>,
    /// The subtitle's.
    pub subtitle_style: Option<TextStyle>,
    /// The gap between the slots and the text column.
    pub title_gap: Option<f32>,
    /// How much room the leading slot is guaranteed, so the text lines up down a list.
    pub min_leading_width: Option<f32>,
    /// The tile's minimum height, over the one its line count asks for.
    pub min_height: Option<f32>,
    /// Whether tiles are the tighter kind.
    pub dense: Option<bool>,
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
    /// **What shape a leading panel is** — the reference's `DrawerThemeData.shape`
    /// (`drawer.dart:268`). Unset, the framework rounds the panel's *inner* edge by
    /// [`DRAWER_RADIUS`](crate::DRAWER_RADIUS).
    pub shape: Option<ShapeBorder>,
    /// **And a trailing one** — the reference's `endShape` (`drawer.dart:269`), which is a
    /// separate field rather than a mirror of the first because a panel's rounded edge is
    /// the one facing the page, and that is the opposite side.
    ///
    /// A trailing panel does **not** fall back to [`shape`](Self::shape): a theme that
    /// named only the leading panel's shape has said nothing about the other one, and the
    /// framework's own default is a better answer than a shape rounded on the wrong edge.
    pub end_shape: Option<ShapeBorder>,
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
    /// **How much room it reserves for a finger** — the reference's per-widget say over
    /// the theme's (`icon_button.dart:708`). Unset, the theme's own answer.
    pub tap_target: Option<crate::theme::TapTarget>,
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

/// Defaults for [`SnackBar`](crate::SnackBar).
///
/// The reference's Material 3 snackbar sets its content in `bodyMedium` and its action in
/// `labelLarge`, and both are read from the type scale rather than written down here.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SnackBarTheme {
    /// **What shape it is** (`shape_border.dart`), over the plain `radius` below:
    /// a snack bar's, which its behaviour already decides — see [`crate::SnackBarBehavior`]. Unset, the widget decides.
    pub shape: Option<ShapeBorder>,
    /// **Where its bars sit**, and therefore what they look like. Unset, the reference's
    /// `Fixed` (`snack_bar.dart:986`). See [`SnackBarBehavior`](crate::SnackBarBehavior).
    pub behavior: Option<crate::toast::SnackBarBehavior>,
    /// What a **floating** bar keeps clear of the page. Unset, the reference's
    /// `insetPadding` (`snack_bar.dart:989`). Silent under `Fixed`.
    pub inset_padding: Option<Insets>,
    /// A **floating** bar's width, instead of the room it is given. Unset, the room it is
    /// given. Silent under `Fixed`, where a width would contradict the behaviour.
    pub width: Option<f32>,
    /// The message's type.
    pub content_text_style: Option<TextStyle>,
    /// The action's type.
    pub action_text_style: Option<TextStyle>,
    /// The bar's surface. Unset, the scheme's `inverse_surface`.
    pub background_color: Option<Color>,
    /// The message's colour. Unset, the scheme's `on_inverse_surface`.
    pub text_color: Option<Color>,
    /// The action's colour. Unset, the scheme's `inverse_primary` — the one role in the
    /// scheme whose whole reason for existing is being legible on an inverted surface.
    pub action_text_color: Option<Color>,
    /// The close cross's colour. Unset, the scheme's `on_inverse_surface`
    /// (`snack_bar.dart:995`).
    pub close_icon_color: Option<Color>,
    /// The stripe down a notification's leading edge, whichever kind it is. Unset, the
    /// kind decides.
    pub accent_color: Option<Color>,
    /// The stripe for a **success**, the one kind Material 3 has no role for. Unset, a
    /// green of this crate's own choosing.
    pub success_color: Option<Color>,
    /// The corner. Unset, the reference's 4 for a **floating** bar and **nothing at all**
    /// for a fixed one, which is flush against the edges and has nothing to round.
    pub radius: Option<f32>,
    /// How far off the page it sits. Unset, the reference's 6.
    pub elevation: Option<f32>,
}

/// Defaults for [`PopupMenuButton`](crate::PopupMenuButton) and its items.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuTheme {
    /// The items' type.
    pub text_style: Option<TextStyle>,
}

/// Defaults for [`DropdownButton`](crate::DropdownButton) and its options.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DropdownTheme {
    /// The value's and the options' type.
    pub text_style: Option<TextStyle>,
}

/// Defaults for [`Autocomplete`](crate::Autocomplete) and its suggestions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AutocompleteTheme {
    /// The suggestions' type.
    pub text_style: Option<TextStyle>,
}

/// Defaults for [`Table`](crate::Table).
///
/// The reference names the two apart — a heading is `titleSmall`, a cell is `bodyMedium` —
/// which is why one `text_style` would be the wrong shape here.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TableTheme {
    /// The column headings' type.
    pub heading_text_style: Option<TextStyle>,
    /// The cells' type.
    pub data_text_style: Option<TextStyle>,
}

/// Defaults for [`DatePicker`](crate::DatePicker).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DatePickerTheme {
    /// The days' type.
    pub day_text_style: Option<TextStyle>,
    /// The weekday initials above them.
    pub weekday_text_style: Option<TextStyle>,
}

/// Defaults for [`NavigationRail`](crate::NavigationRail) and [`BottomBar`](crate::BottomBar).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NavRailTheme {
    /// **The shape of a selected destination's indicator** (`navigation_rail.dart:1148`).
    /// Unset, the reference's pill.
    pub indicator_shape: Option<frus_core::ShapeBorder>,
    /// **The highlight over a destination, per state** — over the framework's own state
    /// layer, and under the destination's own word (`navigation_bar.dart:232`).
    ///
    /// The first field here that is not a plain value but a
    /// [`WidgetStateProperty`](crate::WidgetStateProperty). It owns a `Vec`, which is why
    /// this struct and [`WidgetThemes`] stopped being `Copy` in milestone 448 — a theme
    /// that weighed eight kilobytes had no business being copied implicitly anyway.
    pub overlay_color: Option<crate::widgetstate::WidgetStateProperty<frus_core::Color>>,
    /// The destinations' labels.
    pub label_text_style: Option<TextStyle>,
    /// The count carried by a destination's badge.
    pub badge_text_style: Option<TextStyle>,
    /// The **rail's** surface. Unset, the scheme's `surface`.
    pub background_color: Option<Color>,
    /// The pill behind the selected destination. Unset, the scheme's
    /// `secondary_container` — **opaque**, as the reference's is.
    pub indicator_color: Option<Color>,
    /// The selected destination's glyph. Unset, `on_secondary_container`: it is drawn on
    /// the indicator, so it takes the indicator's content colour.
    pub selected_icon_color: Option<Color>,
    /// An unselected destination's glyph. Unset, `on_surface_variant`.
    pub unselected_icon_color: Option<Color>,
    /// The selected destination's label. Unset, `on_surface` — the label sits *below* the
    /// indicator rather than on it, which is why it is not the indicator's content colour.
    pub selected_label_color: Option<Color>,
    /// An unselected destination's label. Unset, `on_surface` on a rail and
    /// `on_surface_variant` on a bar, which is the one place the reference gives the two
    /// different answers.
    pub unselected_label_color: Option<Color>,
    /// The destinations' glyphs. Unset, the reference's 24.
    pub icon_size: Option<f32>,
    /// Whether a **rail's** selected destination gets an indicator behind its glyph.
    /// Unset, `true` (`navigation_rail.dart:1233`). `false` is the arrangement that
    /// predates it, where the selected destination says so in the accent instead.
    pub use_indicator: Option<bool>,
    /// How far off the page a **rail** sits. Unset, `0` — flat, as the reference's is
    /// (`navigation_rail.dart:1236`), the rail being separated by a rule and not by a
    /// shadow.
    pub elevation: Option<f32>,
    /// The **bottom bar's** surface, a rung higher than the rail's. Unset, the scheme's
    /// `surface_container`.
    ///
    /// The reference keeps a theme object per navigation widget and gives them different
    /// defaults; this crate keeps one for the two, so the two surfaces are two fields
    /// rather than two structs.
    pub bar_background_color: Option<Color>,
}

/// Defaults for [`Alert`](crate::Alert) — the message box, not the dialog. See
/// [`DialogTheme`] for [`AlertDialog`](crate::AlertDialog).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AlertTheme {
    /// The heading's type.
    pub title_text_style: Option<TextStyle>,
    /// The message's type.
    pub content_text_style: Option<TextStyle>,
}

/// Defaults for [`BottomAppBar`](crate::BottomAppBar).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BottomAppBarTheme {
    /// The bar's surface. Unset, the scheme's `surface_container`.
    pub color: Option<Color>,
}

/// Defaults for [`BottomSheet`](crate::BottomSheet).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BottomSheetTheme {
    /// **What shape the sheet is** — the reference's `BottomSheetThemeData.shape`. Unset,
    /// the framework rounds the **top** corners only: the bottom edge is flush against the
    /// window, and rounding it would cut two notches out of the screen.
    pub shape: Option<ShapeBorder>,
    /// The radius of those top two corners, for a theme that would rather give the number
    /// than the shape. Outranked by [`shape`](Self::shape).
    pub radius: Option<f32>,
    /// The sheet's surface. Unset, the scheme's `surface_container_low`.
    pub background_color: Option<Color>,
}

/// Defaults for [`MaterialBanner`](crate::MaterialBanner).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BannerTheme {
    /// The banner's colour. Unset, a low container tone.
    pub color: Option<Color>,
    /// What the surface is tinted towards for its elevation. Unset, nothing is.
    pub surface_tint: Option<Color>,
    /// The shadow's colour. Unset, there is none.
    pub shadow_color: Option<Color>,
    /// The rule along the bottom, drawn only where the banner is flat. Unset, the
    /// scheme's `outlineVariant`.
    pub divider_color: Option<Color>,
    /// How far off the page it sits. Unset, 1.
    pub elevation: Option<f32>,
    /// The padding around the message row. Unset, it depends on where the actions went.
    pub padding: Option<Insets>,
    /// The margin around the banner. Unset, it depends on the elevation.
    pub margin: Option<Insets>,
    /// What the leading slot is held off the message by. Unset, 16 at its trailing edge.
    pub leading_padding: Option<Insets>,
    /// What the message is set in. Unset, `bodyMedium`.
    pub content_text_style: Option<TextStyle>,
}

/// Defaults for [`Dialog`](crate::Dialog) and [`AlertDialog`](crate::AlertDialog).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DialogTheme {
    /// The surface's colour. Unset, the scheme's `surfaceContainerHigh`.
    pub color: Option<Color>,
    /// How far off the page it sits. Unset, 6.
    pub elevation: Option<f32>,
    /// The shadow's colour. Unset, **transparent**: the reference's Material 3 dialog
    /// shows its height by its container tone, not by a drop shadow.
    pub shadow_color: Option<Color>,
    /// What the surface is tinted towards for its elevation. Unset, nothing is — see
    /// [`Self::shadow_color`] for why.
    pub surface_tint: Option<Color>,
    /// **What shape it is** (`dialog.dart`). Unset, a 28-radius rounded rectangle.
    ///
    /// It carried the reference's *name* and a `BorderRadius` until milestone 451, which
    /// is the deviation this field is the clearest case of: `shape` in the reference is a
    /// `ShapeBorder`, and a corner radius is one of the things one can be.
    pub shape: Option<ShapeBorder>,
    /// How far the dialog is held off the window's edges. Unset, 40 across and 24 down.
    pub inset_padding: Option<Insets>,
    /// The glyph above the title. Unset, the scheme's `secondary`.
    pub icon_color: Option<Color>,
    /// The heading's type. Unset, `headlineSmall`.
    pub title_text_style: Option<TextStyle>,
    /// What the dialog says. Unset, `bodyMedium`.
    pub content_text_style: Option<TextStyle>,
}

/// Defaults for [`Steps`](crate::Steps).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepsTheme {
    /// The caption under each marker.
    pub label_text_style: Option<TextStyle>,
    /// The number inside the marker. It is a **glyph on a circle** rather than something
    /// read at length, so the framework's own default does not pass through the reader's
    /// font setting — a theme that sets this one takes that decision back.
    pub index_text_style: Option<TextStyle>,
}

/// Defaults for [`Tree`](crate::Tree).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TreeTheme {
    /// The rows' type.
    pub text_style: Option<TextStyle>,
}

/// Defaults for [`Timeline`](crate::Timeline).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimelineTheme {
    /// An entry's heading.
    pub title_text_style: Option<TextStyle>,
    /// The line under it.
    pub detail_text_style: Option<TextStyle>,
}

/// Defaults for [`Breadcrumb`](crate::Breadcrumb).
///
/// The reference has no breadcrumb, so its type is argued rather than read: a trail is a
/// **secondary** line of navigation above the page's own content, which is `bodyMedium`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BreadcrumbTheme {
    /// The segments' type, the separators included.
    pub text_style: Option<TextStyle>,
}

/// Defaults for [`Kanban`](crate::Kanban).
///
/// The reference has no board, so its type is argued: a card is a small piece of content
/// read on its own, which is `bodyLarge` — the step the reference gives a list tile's title
/// and this framework already gives a `Tree` row.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KanbanTheme {
    /// A text card's label.
    pub card_text_style: Option<TextStyle>,
    /// A column's heading over its cards.
    pub column_title_text_style: Option<TextStyle>,
}

/// Defaults for [`Form`](crate::Form)'s error summary.
///
/// The reference has no summary list, so its type is argued: a bullet restates an error
/// already shown under its field, which is `bodySmall` — the step the reference gives the
/// helper line those errors come from.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FormTheme {
    /// A summary bullet's type.
    pub bullet_text_style: Option<TextStyle>,
    /// The summary's heading over them.
    pub summary_title_text_style: Option<TextStyle>,
}

/// Defaults for [`TimePicker`](crate::TimePicker) and [`TimeRange`](crate::TimeRange).
///
/// The reference names three of these four. Its dial is `bodyLarge`, its help line
/// `labelMedium`, and its day period `titleMedium` — which is the same step as the dial and
/// is why the AM/PM cells here are simply cells. The **preview** is the fourth: the
/// reference puts an editable pair of fields at `displayMedium` where this widget shows one
/// read-only line above two grids, so it takes the heading step that line is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimePickerTheme {
    /// The hour, minute and AM/PM cells.
    pub dial_text_style: Option<TextStyle>,
    /// The `HH:MM` line above them.
    pub preview_text_style: Option<TextStyle>,
    /// The "Hour" and "Minute" lines over each grid.
    pub help_text_style: Option<TextStyle>,
    /// The "Start" and "End" headings of a [`TimeRange`](crate::TimeRange).
    pub range_label_text_style: Option<TextStyle>,
}

/// Defaults for [`Kbd`](crate::Kbd).
///
/// The reference has no key cap, so its type is argued rather than read: a shortcut hint is
/// a **label**, and a key cap is the one place in this framework where the glyphs stand for
/// what is printed on a keyboard, hence the monospaced default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KbdTheme {
    /// The cap's label.
    pub text_style: Option<TextStyle>,
}

#[cfg(test)]
mod tests {
    /// **The rungs a shape is resolved on**, and the third one in particular: a radius
    /// was all a theme could say until milestone 450, and applications have written them.
    /// A theme naming a `shape` outranks one naming only a `radius`; naming both is
    /// naming the shape.
    #[test]
    fn a_shape_outranks_a_radius_and_a_caller_outranks_both() {
        use super::{resolve_shape, BorderRadius, ShapeBorder};
        let fallback = ShapeBorder::stadium();
        let told = ShapeBorder::circle();
        let themed = ShapeBorder::beveled(3.0);

        assert_eq!(resolve_shape(None, None, None, fallback), fallback);
        assert_eq!(
            resolve_shape(None, None, Some(BorderRadius::uniform(4.0)), fallback),
            ShapeBorder::rounded(4.0),
            "a plain radius still speaks"
        );
        assert_eq!(
            resolve_shape(
                None,
                Some(themed),
                Some(BorderRadius::uniform(4.0)),
                fallback
            ),
            themed,
            "and a shape beside it outranks it"
        );
        assert_eq!(
            resolve_shape(
                Some(told),
                Some(themed),
                Some(BorderRadius::uniform(4.0)),
                fallback
            ),
            told,
            "and the caller outranks the theme"
        );
    }

    use super::*;
    use crate::card::{Card, CardVariant, CARD_MARGIN};
    use crate::divider::{Divider, DIVIDER_SPACE};
    use crate::flex::Flex;
    use crate::runtime::Runtime;
    use crate::theme::{TextTheme, Theme};
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

    /// The size every `Primitive::Text` in a frame was drawn at, in the order they were
    /// painted. A widget's type is not reachable any other way — these widgets paint their
    /// labels instead of laying them out.
    fn text_sizes(widget: impl Widget<Msg> + 'static, theme: &Theme) -> Vec<f32> {
        framed(widget, theme)
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { size, .. } => Some(*size),
                _ => None,
            })
            .collect()
    }

    /// The weight of the first text in a frame.
    fn first_weight(widget: impl Widget<Msg> + 'static, theme: &Theme) -> Option<u16> {
        framed(widget, theme).iter().find_map(|p| match p {
            Primitive::Text { weight, .. } => Some(weight.to_u16()),
            _ => None,
        })
    }

    #[test]
    fn a_widget_no_longer_decides_its_own_type() {
        // Milestone 413. Twelve widgets set their text in a private constant that no theme
        // and no caller could reach, and one of them had **drifted two pixels from the
        // reference** without anybody being able to see it. Each is checked here at the
        // step the reference names, and then moved by a theme — which is the part that was
        // impossible before, and the reason the drift went unseen.
        let plain = Theme::default();

        // A snackbar's content is `bodyMedium` — 14 px, not the 16 the constant said.
        assert_eq!(
            text_sizes(crate::toast::SnackBar::<Msg>::new("Saved"), &plain),
            vec![plain.text.body_medium.size.unwrap()]
        );
        let mut loud = Theme::default();
        loud.widgets.snack_bar.content_text_style = Some(TextStyle::new(30.0));
        assert_eq!(
            text_sizes(crate::toast::SnackBar::<Msg>::new("Saved"), &loud),
            vec![30.0],
            "the theme has a say"
        );
        assert_eq!(
            text_sizes(
                crate::toast::SnackBar::<Msg>::new("Saved").content_text_style(TextStyle::new(9.0)),
                &loud
            ),
            vec![9.0],
            "and the caller has the last word over it"
        );

        // A key cap is a label, and monospaced.
        let kbd = || crate::kbd::Kbd::new("Ctrl");
        assert_eq!(
            text_sizes(kbd(), &plain),
            vec![plain.text.label_medium.size.unwrap()]
        );
        assert_eq!(
            framed(kbd(), &plain).iter().find_map(|p| match p {
                Primitive::Text { family, .. } => Some(*family),
                _ => None,
            }),
            Some(Some(frus_core::FontFamily::Monospace)),
            "a cap stands for what is printed on a keyboard"
        );

        // A tree row is a list tile's title.
        let tree =
            || crate::tree::Tree::<Msg>::new(|_| unreachable!()).node(1, 0, "root", false, false);
        assert_eq!(
            text_sizes(tree(), &plain),
            vec![plain.text.body_large.size.unwrap()]
        );
        assert_eq!(
            text_sizes(tree().text_style(TextStyle::new(7.0)), &plain),
            vec![7.0],
            "said on the widget, whatever order the builders came in"
        );
    }

    #[test]
    fn the_reference_names_a_table_s_two_rows_apart() {
        // A data table's heading is `titleSmall` and its cells `bodyMedium` — **two
        // different steps**, which one `SIZE` constant for the whole widget could not say.
        let theme = Theme::default();
        let table = crate::table::Table::<Msg>::new(1)
            .header(&["Name"])
            .row(&["Ada"]);
        assert_eq!(
            text_sizes(table, &theme),
            vec![
                theme.text.title_small.size.unwrap(),
                theme.text.body_medium.size.unwrap(),
            ]
        );
        // And the heading carries the medium weight the step does.
        let heading = crate::table::Table::<Msg>::new(1).header(&["Name"]);
        assert_eq!(
            first_weight(heading, &theme),
            Some(frus_core::FontWeight::Medium.to_u16())
        );
    }

    #[test]
    fn a_table_carries_its_own_word_down_to_cells_it_did_not_build_directly() {
        // `Table` and `DatePicker` hand their override down the **theme**
        // (`Widget::theme_override`) rather than through the cells' fields: a table builds
        // cells in half a dozen places and a calendar has five constructors, and a value
        // carried down the theme reaches all of them without any of them being taught to
        // pass it on.
        let theme = Theme::default();
        let table = crate::table::Table::<Msg>::new(1)
            .header(&["Name"])
            .row(&["Ada"])
            .heading_text_style(TextStyle::new(21.0))
            .data_text_style(TextStyle::new(8.0));
        assert_eq!(text_sizes(table, &theme), vec![21.0, 8.0]);
        // Saying nothing costs nothing: no override, no theme to allocate.
        assert!(Widget::<Msg>::theme_override(
            &crate::table::Table::<Msg>::new(1).header(&["Name"]),
            &theme
        )
        .is_none());
    }

    #[test]
    fn the_un_themed_path_reads_the_same_scale_as_the_themed_one() {
        // `Widget::style` has no theme — it is what the transparent wrappers ask a child
        // when they want its size. It answers from `TextTheme::M3`, the same const
        // `Theme::default()` carries, rather than from a fallback constant beside it: two
        // numbers that have to agree is the shape the last five milestones were about.
        assert_eq!(TextTheme::M3, Theme::default().text);
        let kbd = crate::kbd::Kbd::new("Ctrl");
        assert_eq!(
            Widget::<Msg>::style(&kbd),
            Widget::<Msg>::style_themed(&kbd, &Theme::default()),
            "the same widget, sized the same way, with and without a theme in hand"
        );
    }

    #[test]
    fn the_last_seven_widgets_stop_deciding_their_own_type() {
        // Milestone 414, the tail of 413. Five of these seven turned out to be **read**
        // from the reference after all, and the note that sent them here said only two
        // were — which is the same mistake as the constants themselves, an estimate
        // standing in for a look.
        let plain = Theme::default();

        // Argued: a trail is a *secondary* line of navigation, `bodyMedium`.
        let trail = || crate::breadcrumb::Breadcrumb::<Msg>::new(|_| unreachable!()).crumb("Home");
        assert_eq!(
            text_sizes(trail(), &plain),
            vec![plain.text.body_medium.size.unwrap()]
        );
        assert_eq!(
            text_sizes(trail().text_style(TextStyle::new(9.0)), &plain),
            vec![9.0],
            "and a caller can still say otherwise"
        );
        let mut themed = Theme::default();
        themed.widgets.breadcrumb.text_style = Some(TextStyle::new(31.0));
        assert_eq!(text_sizes(trail(), &themed), vec![31.0]);
    }

    #[test]
    fn a_bar_titles_at_the_reference_s_weight() {
        // The app bar's title was a private `TextStyle::new(22.0).weight(Medium)`: the size
        // right and **the weight wrong**, because `titleLarge` is regular. Nothing tested it
        // — the one test that looked compared it against the constant it came from, which is
        // a tautology, not a check.
        let plain = Theme::default();
        assert_eq!(
            first_weight(crate::navbar::NavigationBar::<Msg>::new("Inbox"), &plain),
            Some(frus_core::FontWeight::Regular.to_u16())
        );
        assert_eq!(
            text_sizes(crate::navbar::NavigationBar::<Msg>::new("Inbox"), &plain),
            vec![plain.text.title_large.size.unwrap()],
            "and a navigation bar titles at 22, not at a private 20"
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
        assert_eq!(WidgetThemes::default().snack_bar.content_text_style, None);
        assert_eq!(WidgetThemes::default().table.heading_text_style, None);
        assert_eq!(WidgetThemes::default().date_picker.day_text_style, None);
        assert_eq!(WidgetThemes::default().kbd.text_style, None);
        assert_eq!(WidgetThemes::default().breadcrumb.text_style, None);
        assert_eq!(WidgetThemes::default().kanban.card_text_style, None);
        assert_eq!(WidgetThemes::default().form.bullet_text_style, None);
        assert_eq!(WidgetThemes::default().time_picker.dial_text_style, None);
        assert_eq!(
            WidgetThemes::default().slider.value_indicator_text_style,
            None
        );
    }
}

//! [`PopupMenuButton`]: a **floating** action menu — an anchor plus a list of items that opens
//! over it, through the overlay, and closes on an outside click.
//!
//! A row is a label and a message in the ordinary case, and [`MenuItem`] is what it is
//! when it is not: a picture in the leading column, a tick, the keys that work the action,
//! a row the caller drew, a rule between two groups, or a row that is unavailable while
//! the rest of the menu is not.
//!
//! **The leading column belongs to the menu.** If any row has a mark, every row keeps the
//! room for one, so the marks line up down a column and a tick that is off holds its place
//! rather than sliding its own label across. A menu with no marks at all is not indented at
//! all.

use std::rc::Rc;

use frus_core::{
    BorderRadius, Color, Insets, Point, Rect, ResolvedTextStyle, Scene, ShapeBorder, TextStyle,
};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::disabled::disabled_content;
use crate::divider::Divider;
use crate::flex::Flex;
use crate::icons::{IconData, Icons};
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

const WIDTH: f32 = 220.0;
/// **One row's height** — the smallest box a control worked by a finger reserves for
/// it, which is what the reference gives a menu item (`popup_menu.dart:279`). It was 38,
/// ten pixels under the number the accessibility scanners on both mobile platforms check
/// for, on a widget whose entire purpose is to be tapped.
const ROW_H: f32 = crate::theme::MIN_TAP_TARGET;
/// The room either side of a row's label (`popup_menu.dart:1876`).
const PAD_X: f32 = 12.0;
/// The room above and below the rows, inside the panel (`popup_menu.dart:1872`). It is
/// twice the panel's own corner, which is why a row's highlight can never reach a curve
/// and the panel needs no clip.
const PAD_Y: f32 = 8.0;
/// How far off the page the panel sits (`popup_menu.dart:1839`).
const ELEVATION: f32 = 3.0;
/// **The leading column**: the mark itself, and the room between it and the words.
/// Eighteen is the size of [`DropdownButton`](crate::DropdownButton)'s own tick — the
/// nearest control in the framework that puts a mark beside a label — and twelve is the
/// row's own padding again, so a marked row's words start two paddings in.
const LEAD: f32 = 18.0;
const LEAD_GAP: f32 = 12.0;
/// The least room between a label and the keys shown on its right. A shortcut close
/// enough to be read as the end of the label is worse than no shortcut.
const SHORTCUT_GAP: f32 = 24.0;

/// The panel's surface: the caller's word, then the theme's, then the reference's
/// — `surface_container`, a menu being a **distinct area within** the surface rather
/// than something floating above it in a colour of its own (`popup_menu.dart:1858`).
fn panel_background(own: Option<Color>, theme: &Theme) -> Color {
    own.or(theme.widgets.menu.background)
        .unwrap_or(theme.scheme.surface_container)
}

/// The style the items are drawn in: what the caller said, else what the theme says, else
/// the reference's — a popup menu's items are `labelLarge` (`popup_menu.dart:1849`).
///
/// **Resolved once**, so that the number the box is measured with is the number the glyphs
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403); a size that never passes through it is a size the reader cannot change.
fn label_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.menu.text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).label_large)
        .resolved()
}

/// **What sits in a row's leading column**, when the row has anything to put there.
#[derive(Clone, Copy)]
enum Lead {
    /// A tick, drawn only when the row is on — and the column is kept either way, so
    /// turning something off does not slide its own label across.
    Check(bool),
    /// A picture of the caller's choosing.
    Icon(IconData),
}

/// What one row needs **across**, kept where the panel can reach it.
///
/// The panel is what decides how wide a menu is, so the panel has to be able to measure
/// the rows — and it cannot ask them. By the time it holds them they are `dyn Widget`,
/// and a widget cannot be asked how wide it would like to be
/// ([#52](https://github.com/KalybosPro/frus/issues/52)). So the words are copied to
/// where the measuring happens.
#[derive(Clone)]
struct Measure {
    /// The row's own words, or `None` for a rule and for a row the caller drew.
    label: Option<String>,
    shortcut: Option<String>,
}

/// A row's child, held so the panel can be built **again** — which every builder on
/// [`PopupMenuButton`] does — without asking the caller's widget to be cloneable.
///
/// It is a transparent wrapper and nothing else: the sharing is the whole of it.
struct Shared<Msg> {
    inner: Rc<dyn Widget<Msg>>,
}

impl<Msg> Shared<Msg> {
    /// It changes nothing about the box: it *is* its child.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(Shared {
    /// Every one of these is **forwarded**: holding a widget by a shared pointer is not
    /// an identity, not a place, not a theme and not a surface.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

/// One menu action, a clickable row.
///
/// **Not a button.** It has no surface and no outline of its own: it is a strip of the
/// panel that lights up under a pointer, which is what a row in a list of actions is
/// everywhere it appears. It carries the panel's resolved colour down so its state layer
/// has the right thing to sit on.
struct Item<Msg> {
    /// The row's own words — `None` when the caller drew the row, in which case their
    /// widget is child 0.
    label: Option<String>,
    /// `[the caller's widget]`, or empty for a row of words.
    children: Vec<Box<dyn Widget<Msg>>>,
    /// This row's mark, if it has one.
    lead: Option<Lead>,
    /// Whether **the menu** keeps a leading column at all — a different question from
    /// whether this row has anything to put in it, and the whole reason the marks line up
    /// down a column instead of every row deciding for itself.
    lead_column: bool,
    /// The keys that work this action, shown on the right.
    shortcut: Option<String>,
    /// Whether this row can be used. The menu's own availability is folded in here, but
    /// only for tidiness: a disabled menu never opens, so there is no row to disable.
    enabled: bool,
    text_style: Option<TextStyle>,
    /// The caller's panel colour, so a row's highlight tints the surface it is drawn on
    /// and not the one the framework would have picked.
    background: Option<Color>,
    /// The caller's row padding and row height, if either was named.
    padding: Option<Insets>,
    height: Option<f32>,
    message: Msg,
}

impl<Msg> Item<Msg> {
    /// The room either side of the label: the caller's, then the theme's, then the
    /// reference's twelve.
    fn padding(&self, theme: Option<&Theme>) -> Insets {
        self.padding
            .or(theme.and_then(|t| t.widgets.menu.item_padding))
            .unwrap_or(Insets::new(0.0, PAD_X, 0.0, PAD_X))
    }

    /// What the leading column takes from the words — nought unless the menu keeps one.
    fn lead_room(&self) -> f32 {
        if self.lead_column {
            LEAD + LEAD_GAP
        } else {
            0.0
        }
    }

    /// The room kept clear on the right for the keys, their gap included.
    fn shortcut_room(&self, theme: Option<&Theme>) -> f32 {
        self.shortcut.as_deref().map_or(0.0, |keys| {
            SHORTCUT_GAP
                + frus_text::measure_resolved(keys, &label_style(self.text_style, theme)).width
        })
    }

    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let height = self
            .height
            .or(theme.and_then(|t| t.widgets.menu.item_height))
            .unwrap_or(ROW_H);
        // The row grows if the reader's type does not fit in it — the height is a
        // floor, not a promise.
        let line = frus_text::line_box(height, &label_style(self.text_style, theme), 0.0);
        // **Across, a row says nothing.** The panel decides how wide the menu is and
        // stretches every row to it; a row that named its own width would leave the
        // highlights ragged the moment one label was longer than the others.
        if self.children.is_empty() {
            return Style {
                width: Dimension::Auto,
                height: Dimension::Length(line),
                ..Default::default()
            };
        }
        let pad = self.padding(theme);
        Style {
            width: Dimension::Auto,
            // A row the caller drew **grows**. A two-line item is half of why the form
            // exists at all, and a fixed height would cut it in two.
            height: Dimension::Auto,
            min_height: Dimension::Length(line),
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            // The columns are kept clear by padding rather than by empty siblings, so a
            // caller's widget is laid out in exactly the room the words would have had.
            padding: Insets::new(
                pad.top,
                pad.right + self.shortcut_room(theme),
                pad.bottom,
                pad.left + self.lead_room(),
            ),
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Item<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // A row draws **nothing at rest**. The panel behind it is the surface; all this
        // adds is the state layer that says a pointer is over it. No state layer while
        // disabled: a hover tint promises that a press would do something.
        //
        // It used to draw a filled, outlined, rounded rectangle per row — which made a
        // menu a stack of buttons with two-pixel gutters showing the page through, where
        // the reference has one panel with rows inside it.
        let base = panel_background(self.background, theme);
        if self.enabled {
            let tinted = theme.state_layer(base, theme.on_surface, &status);
            if tinted != base {
                scene.fill_rect(bounds, tinted.fade(o));
            }
        }
        let ink = if self.enabled {
            theme.on_surface
        } else {
            disabled_content(theme)
        };
        let style = label_style(self.text_style, Some(theme));
        let pad = self.padding(Some(theme));
        // The mark, centred in its column. A tick that is off draws nothing and still
        // holds its place.
        let mark = match self.lead {
            Some(Lead::Check(on)) => on.then_some(Icons::CHECK),
            Some(Lead::Icon(icon)) => Some(icon),
            None => None,
        };
        if let Some(icon) = mark {
            let y = bounds.y + (bounds.height - LEAD) * 0.5;
            let path = icon.placed(LEAD, bounds.x + pad.left, y, theme.direction);
            scene.fill_path(&path, ink.fade(o));
        }
        let ty = bounds.y + (bounds.height - style.line_height()) * 0.5;
        if let Some(label) = &self.label {
            scene.text(
                Point::new(bounds.x + pad.left + self.lead_room(), ty),
                label.clone(),
                &style,
                ink.fade(o),
            );
        }
        if let Some(keys) = &self.shortcut {
            let width = frus_text::measure_resolved(keys, &style).width;
            // **Muted, never the label's ink.** The keys are a reminder of another way
            // in, not a second thing to read.
            let tint = if self.enabled {
                theme.muted
            } else {
                disabled_content(theme)
            };
            scene.text(
                Point::new(bounds.x + bounds.width - pad.right - width, ty),
                keys.clone(),
                &style,
                tint.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // A menu row said nothing to a reader before this.
        //
        // A row that is on or off is announced as a **checkbox**, with its state: there
        // is no `menuitemcheckbox` in this framework's vocabulary, and a checkbox is the
        // role that carries the one thing such a row has to say. A row the caller drew
        // announces nothing here and leaves that to whatever they drew.
        let (role, toggled) = match self.lead {
            Some(Lead::Check(on)) => (frus_core::Role::CheckBox, Some(on)),
            _ => (frus_core::Role::Button, None),
        };
        let mut spoken = self.label.clone().unwrap_or_default();
        if let Some(keys) = &self.shortcut {
            // The keys are on the row for someone who can see them, and the announcement
            // is the only other way anybody hears about them.
            if !spoken.is_empty() {
                spoken.push_str(", ");
            }
            spoken.push_str(keys);
        }
        let mut semantics = frus_core::SemanticsProperties::new(role).maybe_toggled(toggled);
        if !spoken.is_empty() {
            semantics = semantics.label(spoken);
        }
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// **The thing a menu actually is**: one surface, off the page, with the rows inside it.
///
/// This did not exist. `rebuild` handed the overlay a bare column of rows, so a menu was
/// a stack of outlined buttons with the page showing through two-pixel gutters — which
/// is not what a menu looks like anywhere, and is not what the reference draws
/// (`popup_menu.dart:1837`).
struct Panel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    /// What each row needs across, in order — see [`Measure`]. The panel is what decides
    /// the menu's width, so this is where the words have to be.
    rows: Vec<Measure>,
    /// Whether any row has a mark, and so whether every row keeps room for one.
    lead_column: bool,
    text_style: Option<TextStyle>,
    item_padding: Option<Insets>,
    background: Option<Color>,
    shape: Option<ShapeBorder>,
    elevation: Option<f32>,
    padding: Option<Insets>,
}

impl<Msg> Panel<Msg> {
    /// What shape the panel is: the caller's word, then the theme's shape, then the
    /// theme's plain radius, then the framework's own corner.
    ///
    /// The framework's is `theme.radius` and **not** the reference's four. The reference
    /// gives every component its own corner — four for a menu, twelve for a card,
    /// twenty-eight for a sheet — where this framework collapses them into one number an
    /// application sets once. Reaching into that collapse for one widget would make the
    /// menu the only thing on screen that ignores it; `MenuTheme::radius` is there for an
    /// application that wants the reference's number.
    fn shape_of(&self, theme: &Theme) -> ShapeBorder {
        crate::resolve_shape(
            self.shape,
            theme.widgets.menu.shape,
            theme.widgets.menu.radius.map(BorderRadius::uniform),
            ShapeBorder::rounded(theme.radius),
        )
    }

    /// The room above and below the rows: eight, which is twice the corner and so keeps a
    /// row's highlight clear of the curve without a clip.
    fn padding(&self, theme: &Theme) -> Insets {
        self.padding
            .or(theme.widgets.menu.padding)
            .unwrap_or(Insets::new(PAD_Y, 0.0, PAD_Y, 0.0))
    }

    /// **How wide a row is**: two hundred and twenty, or the widest row when a row wants
    /// more than that.
    ///
    /// It was a bare constant. That is fine for a list of one-word actions and wrong the
    /// moment a row carries a mark, a label and the keys that work it — and the way it
    /// fails is by overlapping, which no assertion about the tree catches and only a
    /// picture shows. The width is a **floor** so that no menu already drawn moves: a
    /// hundred per cent of the menus in this repository still measure under it.
    ///
    /// A row the caller drew contributes nothing, because it cannot be asked what it
    /// wants ([#52](https://github.com/KalybosPro/frus/issues/52)). It takes whatever
    /// the words decided, and that is the single place in the menu where that gap shows.
    fn row_width(&self, theme: Option<&Theme>) -> f32 {
        let style = label_style(self.text_style, theme);
        let pad = self
            .item_padding
            .or(theme.and_then(|t| t.widgets.menu.item_padding))
            .unwrap_or(Insets::new(0.0, PAD_X, 0.0, PAD_X));
        let lead = if self.lead_column {
            LEAD + LEAD_GAP
        } else {
            0.0
        };
        let widest = self.rows.iter().fold(0.0f32, |wide, row| {
            let label = row
                .label
                .as_deref()
                .map_or(0.0, |t| frus_text::measure_resolved(t, &style).width);
            let keys = row.shortcut.as_deref().map_or(0.0, |t| {
                SHORTCUT_GAP + frus_text::measure_resolved(t, &style).width
            });
            wide.max(pad.left + lead + label + keys + pad.right)
        });
        // Rounded up: half a pixel of a glyph past the edge is the whole of the bug.
        widest.max(WIDTH).ceil()
    }
}

impl<Msg: Clone> Widget<Msg> for Panel<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.row_width(None)),
            flex_direction: FlexDirection::Column,
            padding: Insets::new(PAD_Y, 0.0, PAD_Y, 0.0),
            ..Default::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let padding = self.padding(theme);
        Style {
            // The rows are the width above; the panel is that plus whatever room it was
            // told to keep at its own edges.
            width: Dimension::Length(self.row_width(Some(theme)) + padding.left + padding.right),
            flex_direction: FlexDirection::Column,
            padding,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let shape = self.shape_of(theme);
        let radius = shape
            .as_rounded(bounds)
            .map(|(_, r)| r)
            .unwrap_or(BorderRadius::ZERO);
        let depth = self
            .elevation
            .or(theme.widgets.menu.elevation)
            .unwrap_or(ELEVATION);
        if depth > 0.0 {
            let blur = depth * 4.0 + 8.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + depth * 2.0 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                theme.scheme.shadow.with_alpha(0.30).fade(o),
                radius.inflate(blur),
                blur,
            );
        }
        // Opaque, and **no outline**: a panel that is off the page says so with its
        // shadow. A shadow and a hairline together is the mash-up milestone 279 took out
        // of the card.
        scene.draw_shape(
            bounds,
            shape,
            panel_background(self.background, theme).fade(o),
        );
    }

    /// The panel itself answers nothing: the rows do.
    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// It **traps** the press all the same. A menu is a surface, and a press on the room
    /// above the first row, on the gap a rule leaves, or on a row that said it was
    /// unavailable, must not reach the page behind and dismiss the menu.
    ///
    /// The comment above this used to claim `on_click` did that. It did not: a widget
    /// with no message is not a target at all, so every one of those presses fell
    /// through, and the menu closed. Per-row availability is what found it — before
    /// that, every row in an open menu had a message and there was nothing to fall
    /// through.
    fn opaque(&self) -> bool {
        true
    }
}

/// **One entry of a [`PopupMenuButton`]'s list**: an action, or the rule between two
/// groups of them.
///
/// The shorthands on the button itself — [`item`](PopupMenuButton::item),
/// [`icon_item`](PopupMenuButton::icon_item),
/// [`checked_item`](PopupMenuButton::checked_item),
/// [`item_widget`](PopupMenuButton::item_widget) and
/// [`divider`](PopupMenuButton::divider) — each build one of these, and are what most
/// call sites should write. This type is the door to the two things a shorthand cannot
/// carry: a row that is **not available** while the rest of the menu is, and the **keys**
/// that work an action.
///
/// ```
/// use frus_widgets::{Container, Icons, MenuItem, PopupMenuButton};
///
/// #[derive(Clone)]
/// enum Msg {
///     Close,
///     Cut,
///     Paste,
///     Wrap,
/// }
///
/// let menu = PopupMenuButton::new(Container::<Msg>::new(), true, Msg::Close)
///     .entry(MenuItem::icon(Icons::CONTENT_CUT, "Cut", Msg::Cut).shortcut("Ctrl+X"))
///     .entry(MenuItem::new("Paste", Msg::Paste).enabled(false))
///     .divider()
///     .checked_item("Word wrap", true, Msg::Wrap);
/// ```
pub struct MenuItem<Msg> {
    label: Option<String>,
    /// The caller's own drawing, shared so the panel can be built again.
    child: Option<Rc<dyn Widget<Msg>>>,
    lead: Option<Lead>,
    shortcut: Option<String>,
    enabled: bool,
    /// **`None` is the rule** between two groups: the one entry that is not an action,
    /// and so the one entry with nothing to send.
    message: Option<Msg>,
}

impl<Msg> MenuItem<Msg> {
    /// An action with nothing on it yet.
    fn action(message: Msg) -> Self {
        Self {
            label: None,
            child: None,
            lead: None,
            shortcut: None,
            enabled: true,
            message: Some(message),
        }
    }

    /// A row of words: the ordinary item, and what
    /// [`PopupMenuButton::item`] builds.
    pub fn new(label: impl Into<String>, message: Msg) -> Self {
        Self {
            label: Some(label.into()),
            ..Self::action(message)
        }
    }

    /// A row **the caller draws**: two lines, a colour swatch, a badge, rich text —
    /// whatever the menu is for.
    ///
    /// It is laid out in exactly the room a label would have had, columns and all, and it
    /// **grows**: a row of words is a tap target tall, a row of anything else is at least
    /// that and as much more as it needs.
    pub fn widget(child: impl Widget<Msg> + 'static, message: Msg) -> Self {
        Self {
            child: Some(Rc::new(child)),
            ..Self::action(message)
        }
    }

    /// A row with a picture in the leading column.
    pub fn icon(icon: IconData, label: impl Into<String>, message: Msg) -> Self {
        Self {
            lead: Some(Lead::Icon(icon)),
            ..Self::new(label, message)
        }
    }

    /// A row that is **on or off**, ticked when it is on.
    ///
    /// It still takes a message, and turning it over is still the application's job: this
    /// says what the state *is*, not what it will be.
    pub fn checked(label: impl Into<String>, checked: bool, message: Msg) -> Self {
        Self {
            lead: Some(Lead::Check(checked)),
            ..Self::new(label, message)
        }
    }

    /// The **rule** between two groups of actions. It answers nothing, takes no focus,
    /// and is not a row's height: sixteen, the room a separator needs to read as one.
    pub fn divider() -> Self {
        Self {
            label: None,
            child: None,
            lead: None,
            shortcut: None,
            enabled: false,
            message: None,
        }
    }

    /// Whether **this row** can be used, over the menu's own availability.
    ///
    /// A disabled row is still drawn and still read out — greyed, announced as disabled,
    /// and returning no message. An action that is missing from the list altogether tells
    /// a reader nothing about why.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The **keys** that work this action, shown muted on the right.
    ///
    /// The text is the caller's, not a key combination this framework parses: the
    /// spelling differs by platform (`Ctrl+X`, `⌘X`) and nothing here knows which one an
    /// application means. It is announced after the label, since a reader who cannot see
    /// it has no other way to hear it.
    #[must_use]
    pub fn shortcut(mut self, keys: impl Into<String>) -> Self {
        self.shortcut = Some(keys.into());
        self
    }
}

/// A controlled action menu, opened and closed by the application.
pub struct PopupMenuButton<Msg> {
    open: bool,
    enabled: bool,
    /// `[anchor]`, or `[anchor, list]` when the menu is showing.
    children: Vec<Box<dyn Widget<Msg>>>,
    items: Vec<MenuItem<Msg>>,
    text_style: Option<TextStyle>,
    /// The panel's look, and the rows'. Every builder that writes one of these calls
    /// [`rebuild`](Self::rebuild), so **the order they are written in does not matter**
    /// — unlike [`BottomSheet`](crate::BottomSheet), where the panel is built by
    /// `body` and anything said after it is dropped.
    background: Option<Color>,
    shape: Option<ShapeBorder>,
    elevation: Option<f32>,
    menu_padding: Option<Insets>,
    item_padding: Option<Insets>,
    item_height: Option<f32>,
    dismiss: Option<Msg>,
    on_dismiss: Option<Msg>,
}

impl<Msg: Clone + 'static> PopupMenuButton<Msg> {
    /// Creates a menu around an anchor. When `open`, the list floats above it;
    /// `on_dismiss` is emitted on a click **outside** the menu.
    pub fn new(anchor: impl Widget<Msg> + 'static, open: bool, on_dismiss: Msg) -> Self {
        Self {
            open,
            enabled: true,
            children: vec![Box::new(anchor)],
            items: Vec::new(),
            text_style: None,
            background: None,
            shape: None,
            elevation: None,
            menu_padding: None,
            item_padding: None,
            item_height: None,
            dismiss: Some(on_dismiss.clone()),
            on_dismiss: if open { Some(on_dismiss) } else { None },
        }
    }

    /// The items' type, over the theme's and the reference's.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self.rebuild();
        self
    }

    /// **The panel's surface**, over the theme's and the reference's `surface_container`.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.rebuild();
        self
    }

    /// **What shape the panel is**, over the theme's and the framework's corner.
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(shape);
        self.rebuild();
        self
    }

    /// The shorthand for a rounded rectangle — `radius(4.0)` is the reference's own
    /// menu corner, which this framework does not use by default because it keeps **one**
    /// corner for the whole interface, in `Theme::radius`. Reaching past that for a single
    /// widget would make the menu the only thing on screen ignoring the number an
    /// application set.
    #[must_use]
    pub fn radius(self, radius: impl Into<BorderRadius>) -> Self {
        self.shape(ShapeBorder::rounded(radius.into()))
    }

    /// **How far off the page the panel sits**, in pixels. Three by default.
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self.rebuild();
        self
    }

    /// The room kept **above and below** the rows, inside the panel.
    #[must_use]
    pub fn menu_padding(mut self, padding: Insets) -> Self {
        self.menu_padding = Some(padding);
        self.rebuild();
        self
    }

    /// The room kept either side of a row's label.
    #[must_use]
    pub fn item_padding(mut self, padding: Insets) -> Self {
        self.item_padding = Some(padding);
        self.rebuild();
        self
    }

    /// **How tall one row is**, as a floor — a row still grows to fit the reader's
    /// type. The default is the smallest box a finger can be asked to hit.
    #[must_use]
    pub fn item_height(mut self, height: f32) -> Self {
        self.item_height = Some(height);
        self.rebuild();
        self
    }

    /// Whether the menu can be used. Disabled it is **inert** and, like a disabled
    /// [`DropdownButton`](crate::DropdownButton), **never open**: a floating panel over an anchor that
    /// answers nothing traps a press and returns no message.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.on_dismiss = if self.open && enabled {
            self.dismiss.clone()
        } else {
            None
        };
        self.rebuild();
        self
    }

    /// Adds an action: a label plus a message on click. Ignored when the menu is closed.
    pub fn item(self, label: impl Into<String>, message: Msg) -> Self {
        self.entry(MenuItem::new(label, message))
    }

    /// Adds a row **the caller draws** — two lines, a swatch, a badge, rich text — plus
    /// the message it sends.
    pub fn item_widget(self, child: impl Widget<Msg> + 'static, message: Msg) -> Self {
        self.entry(MenuItem::widget(child, message))
    }

    /// Adds an action with a picture in the leading column.
    pub fn icon_item(self, icon: IconData, label: impl Into<String>, message: Msg) -> Self {
        self.entry(MenuItem::icon(icon, label, message))
    }

    /// Adds an action that is **on or off**, ticked when it is on.
    pub fn checked_item(self, label: impl Into<String>, checked: bool, message: Msg) -> Self {
        self.entry(MenuItem::checked(label, checked, message))
    }

    /// Adds the **rule** between two groups of actions.
    pub fn divider(self) -> Self {
        self.entry(MenuItem::divider())
    }

    /// Adds an entry however it was built — the door to [`MenuItem::enabled`] and
    /// [`MenuItem::shortcut`], which no shorthand carries. Ignored when the menu is
    /// closed.
    pub fn entry(mut self, item: MenuItem<Msg>) -> Self {
        if self.open {
            self.items.push(item);
            self.rebuild();
        }
        self
    }

    /// (Re)builds the floating panel (child 1) from the items.
    ///
    /// A disabled menu keeps only its anchor, so there is no overlay to return and no
    /// panel to trap a press.
    fn rebuild(&mut self) {
        if !self.enabled {
            self.children.truncate(1);
            return;
        }
        // **Whether the menu keeps a leading column is decided once, for the menu.** The
        // marks line up down one column and a row with nothing to put there still keeps
        // the room — the alternative is labels that go ragged as things are turned on and
        // off, which is the version every desktop menu decided against.
        let lead_column = self.items.iter().any(|item| item.lead.is_some());
        // No gap. The rows are contiguous strips of one surface; the two-pixel gutter
        // this used to leave showed the page through the middle of the menu.
        let mut list = Flex::column();
        let mut rows = Vec::with_capacity(self.items.len());
        for item in &self.items {
            rows.push(Measure {
                label: item.label.clone(),
                shortcut: item.shortcut.clone(),
            });
            let Some(message) = &item.message else {
                // A rule, and not a row: it is not a tap target tall, it takes no focus
                // and it answers nothing.
                list = list.child(Divider::new());
                continue;
            };
            list = list.child(Item {
                label: item.label.clone(),
                children: item
                    .child
                    .clone()
                    .map(|inner| Box::new(Shared { inner }) as Box<dyn Widget<Msg>>)
                    .into_iter()
                    .collect(),
                lead: item.lead,
                lead_column,
                shortcut: item.shortcut.clone(),
                enabled: self.enabled && item.enabled,
                text_style: self.text_style,
                background: self.background,
                padding: self.item_padding,
                height: self.item_height,
                message: message.clone(),
            });
        }
        let panel: Box<dyn Widget<Msg>> = Box::new(Panel {
            children: vec![Box::new(list)],
            rows,
            lead_column,
            text_style: self.text_style,
            item_padding: self.item_padding,
            background: self.background,
            shape: self.shape,
            elevation: self.elevation,
            padding: self.menu_padding,
        });
        if self.children.len() > 1 {
            self.children[1] = panel;
        } else {
            self.children.push(panel);
        }
    }
}

impl<Msg: Clone> Widget<Msg> for PopupMenuButton<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.children
            .get(1)
            .map(|panel| (panel.as_ref(), Placement::Below))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn overlay_traps_focus(&self) -> bool {
        // An open menu **traps** keyboard focus in its items — Escape or an outside
        // click closes it through `on_dismiss` — the keyboard pattern menus are expected
        // to follow.
        self.open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Point as P, Runtime, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
        A,
        B,
    }

    fn anchor() -> Container<Msg> {
        Container::<Msg>::new().width(40.0).height(30.0)
    }

    /// Every filled rectangle in the frame, innermost layers included, as
    /// `(rect, colour, radius, border width)`.
    fn rects(scene: &frus_core::Scene) -> Vec<(Rect, frus_core::Color, BorderRadius, f32)> {
        fn walk(
            primitives: &[frus_core::Primitive],
            out: &mut Vec<(Rect, frus_core::Color, BorderRadius, f32)>,
        ) {
            for p in primitives {
                match p {
                    frus_core::Primitive::Rect {
                        rect,
                        color,
                        radius,
                        border_width,
                        ..
                    } => out.push((*rect, *color, *radius, *border_width)),
                    frus_core::Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(scene.primitives(), &mut out);
        out
    }

    fn open_menu() -> PopupMenuButton<Msg> {
        PopupMenuButton::new(anchor(), true, Msg::Close)
            .item("A", Msg::A)
            .item("B", Msg::B)
    }

    fn frame(menu: &PopupMenuButton<Msg>, theme: &Theme) -> crate::Ui<Msg> {
        build_ui(menu, Size::new(400.0, 400.0), &Runtime::default(), theme)
    }

    /// **A menu is one panel, not a stack of buttons.**
    ///
    /// It used to draw a filled, outlined, rounded rectangle **per row**, with a
    /// two-pixel gutter between them showing the page through the middle of the menu.
    /// That is not what a menu looks like anywhere, and the reference draws one surface
    /// with rows inside it (`popup_menu.dart:1837`).
    ///
    /// So: one opaque rectangle the width of the menu, in `surface_container`, and
    /// **not one border anywhere** — a panel that is off the page says so with its
    /// shadow, and a shadow with a hairline is the mash-up milestone 279 took out of the
    /// card.
    #[test]
    fn a_menu_is_one_panel_and_not_a_stack_of_buttons() {
        let theme = Theme::default();
        let ui = frame(&open_menu(), &theme);
        let painted = rects(ui.scene());

        let panels: Vec<_> = painted
            .iter()
            .filter(|(rect, color, ..)| {
                rect.width >= WIDTH && *color == theme.scheme.surface_container
            })
            .collect();
        assert_eq!(
            panels.len(),
            1,
            "one surface, not one per row: {painted:#?}"
        );
        assert!(
            panels[0].2 != BorderRadius::ZERO,
            "and it is rounded: {:?}",
            panels[0].2
        );

        assert!(
            painted.iter().all(|(.., border)| *border == 0.0),
            "nothing in an open menu draws an outline: {painted:#?}"
        );
    }

    /// **A row is at least a tap target.** It was 38 pixels tall — ten under the
    /// number the accessibility scanners on both mobile platforms check for, on a widget
    /// whose entire purpose is to be tapped. The reference gives a menu item
    /// `kMinInteractiveDimension` (`popup_menu.dart:279`).
    ///
    /// The panel is measured too: two rows of 48 plus 8 above and 8 below.
    #[test]
    fn a_row_is_at_least_a_tap_target() {
        let theme = Theme::default();
        let ui = frame(&open_menu(), &theme);
        let panel = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| find_panel(p, &theme))
            .expect("a panel");
        assert!(
            panel.height >= 2.0 * crate::theme::MIN_TAP_TARGET + 2.0 * PAD_Y,
            "two rows of a tap target each, plus the panel's own room: {panel:?}"
        );
    }

    fn find_panel(p: &frus_core::Primitive, theme: &Theme) -> Option<Rect> {
        match p {
            frus_core::Primitive::Rect { rect, color, .. }
                if rect.width >= WIDTH && *color == theme.scheme.surface_container =>
            {
                Some(*rect)
            }
            frus_core::Primitive::Layer { primitives, .. } => {
                primitives.iter().find_map(|p| find_panel(p, theme))
            }
            _ => None,
        }
    }

    /// **The panel answers to a theme, and to its caller over the theme** — the
    /// surface, the corner, the room inside it and how tall a row is. None of these were
    /// reachable at all: a menu picked every one of them for itself.
    #[test]
    fn a_menu_answers_to_its_theme_and_to_its_caller() {
        let mut theme = Theme::default();
        theme.widgets.menu.background = Some(frus_core::Color::rgb(0.2, 0.4, 0.6));
        theme.widgets.menu.radius = Some(3.0);
        theme.widgets.menu.item_height = Some(60.0);
        theme.widgets.menu.padding = Some(Insets::new(20.0, 0.0, 20.0, 0.0));

        let ui = frame(&open_menu(), &theme);
        let panel = rects(ui.scene())
            .into_iter()
            .find(|(rect, color, ..)| {
                rect.width >= WIDTH && *color == frus_core::Color::rgb(0.2, 0.4, 0.6)
            })
            .expect("the theme's surface");
        assert_eq!(panel.2, BorderRadius::uniform(3.0), "the theme's corner");
        assert_eq!(
            panel.0.height, 160.0,
            "two rows of 60 and 20 above and below"
        );

        // The caller outranks it, all four.
        let told = open_menu()
            .background(frus_core::Color::rgb(0.9, 0.1, 0.1))
            .radius(9.0)
            .item_height(50.0)
            .menu_padding(Insets::ZERO);
        let ui = frame(&told, &theme);
        let panel = rects(ui.scene())
            .into_iter()
            .find(|(rect, color, ..)| {
                rect.width >= WIDTH && *color == frus_core::Color::rgb(0.9, 0.1, 0.1)
            })
            .expect("the caller's surface");
        assert_eq!(panel.2, BorderRadius::uniform(9.0));
        assert_eq!(panel.0.height, 100.0);
    }

    /// **The order the builders are written in does not matter.** Every one of them
    /// rebuilds the panel, so `.item(..)` after `.background(..)` and `.background(..)`
    /// after `.item(..)` produce the same menu.
    ///
    /// This is the trap milestone 458 found in [`BottomSheet`](crate::BottomSheet), where
    /// the panel is built by `body` and anything said after it is silently dropped. It is
    /// worth a test here precisely because it is the kind of thing that is true when
    /// written and quietly stops being true later.
    #[test]
    fn the_builders_can_be_written_in_any_order() {
        let theme = Theme::default();
        let colour = frus_core::Color::rgb(0.9, 0.1, 0.1);
        let first = PopupMenuButton::new(anchor(), true, Msg::Close)
            .background(colour)
            .item("A", Msg::A)
            .item("B", Msg::B);
        let last = open_menu().background(colour);
        assert_eq!(
            rects(frame(&first, &theme).scene()),
            rects(frame(&last, &theme).scene())
        );
    }

    /// Every run of text in the frame, as `(where it starts, what it says)`.
    fn texts(scene: &frus_core::Scene) -> Vec<(Point, String)> {
        fn walk(primitives: &[frus_core::Primitive], out: &mut Vec<(Point, String)>) {
            for p in primitives {
                match p {
                    frus_core::Primitive::Text { position, text, .. } => {
                        out.push((*position, text.clone()))
                    }
                    frus_core::Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(scene.primitives(), &mut out);
        out
    }

    /// Where a given run of text starts.
    fn text_at(ui: &crate::Ui<Msg>, words: &str) -> Point {
        texts(ui.scene())
            .into_iter()
            .find(|(_, text)| text == words)
            .unwrap_or_else(|| panic!("{words:?} is drawn"))
            .0
    }

    /// The message a press at a point produces, if any.
    fn press(ui: &crate::Ui<Msg>, x: f32, y: f32) -> Option<Msg> {
        ui.hit(P::new(x, y)).and_then(|id| ui.msg_for(id))
    }

    /// The middle of row `n`, in a menu whose anchor is 30 tall and whose rows are the
    /// default height.
    fn row_middle(n: f32) -> f32 {
        30.0 + PAD_Y + ROW_H * (n + 0.5)
    }

    /// **The marks line up down one column**, and a row with nothing to put in it keeps
    /// the room all the same.
    ///
    /// The alternative — each row indenting itself only when it has a mark — is a menu
    /// whose labels move sideways as things are turned on and off, which is why no
    /// desktop menu has ever done it that way. It is decided once, for the menu, and a
    /// menu with no marks at all is not indented at all: every picture already in this
    /// repository is of one of those.
    #[test]
    fn the_marks_line_up_down_one_column() {
        let theme = Theme::default();
        let plain = frame(&open_menu(), &theme);
        let marked = frame(
            &PopupMenuButton::new(anchor(), true, Msg::Close)
                .checked_item("A", true, Msg::A)
                .item("B", Msg::B),
            &theme,
        );

        let unmarked_row = text_at(&marked, "B").x;
        assert_eq!(
            text_at(&marked, "A").x,
            unmarked_row,
            "the ticked row and the row with nothing start in the same place"
        );
        assert_eq!(
            unmarked_row - text_at(&plain, "B").x,
            LEAD + LEAD_GAP,
            "and that place is the leading column further in than in a menu with no marks"
        );
    }

    /// **A tick that is off still holds its place.** It draws nothing — there is one
    /// filled path in the menu when one of the two rows is on, and none when neither is
    /// — and the labels do not move between the two.
    #[test]
    fn a_tick_that_is_off_draws_nothing_and_moves_nothing() {
        let theme = Theme::default();
        let menu = |on: bool| {
            PopupMenuButton::new(anchor(), true, Msg::Close)
                .checked_item("A", on, Msg::A)
                .item("B", Msg::B)
        };
        let (off, on) = (frame(&menu(false), &theme), frame(&menu(true), &theme));
        assert_eq!(
            paths(off.scene()),
            0,
            "nothing is drawn for a tick that is off"
        );
        assert_eq!(paths(on.scene()), 1, "and one tick when it is on");
        assert_eq!(
            text_at(&off, "A").x,
            text_at(&on, "A").x,
            "the label does not move when the tick appears"
        );
    }

    /// How many filled paths the frame holds — the marks, since a menu draws nothing else
    /// with one.
    fn paths(scene: &frus_core::Scene) -> usize {
        fn walk(primitives: &[frus_core::Primitive]) -> usize {
            primitives
                .iter()
                .map(|p| match p {
                    frus_core::Primitive::Path { .. } => 1,
                    frus_core::Primitive::Layer { primitives, .. } => walk(primitives),
                    _ => 0,
                })
                .sum()
        }
        walk(scene.primitives())
    }

    /// **A row can be a widget the caller drew**, laid out in exactly the room the words
    /// would have had — and it **grows**, which a row of words does not.
    #[test]
    fn a_row_can_be_a_widget_the_caller_drew() {
        let theme = Theme::default();
        let mark = frus_core::Color::rgb(0.9, 0.1, 0.1);
        let menu = PopupMenuButton::new(anchor(), true, Msg::Close)
            .item_widget(
                Container::<Msg>::new().width(60.0).height(80.0).color(mark),
                Msg::A,
            )
            .item("B", Msg::B);
        let ui = frame(&menu, &theme);
        let drawn = rects(ui.scene())
            .into_iter()
            .find(|(_, color, ..)| *color == mark)
            .expect("the caller's own widget is drawn");
        assert_eq!(
            drawn.0.x,
            text_at(&ui, "B").x,
            "it starts where a label would have started"
        );
        assert_eq!(
            drawn.0.height, 80.0,
            "and it is given the height it asked for"
        );

        // The row grew with it: the panel is the tall row, the ordinary row, and its own
        // room above and below.
        let panel = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| find_panel(p, &theme))
            .expect("a panel");
        assert_eq!(panel.height, 80.0 + ROW_H + 2.0 * PAD_Y);
    }

    /// **A rule is not a row.** Sixteen tall rather than a tap target, no message, and no
    /// place in the tab order — a separator that could be focused is a separator that
    /// swallows a keystroke.
    #[test]
    fn a_rule_is_not_a_row() {
        let theme = Theme::default();
        let menu = PopupMenuButton::new(anchor(), true, Msg::Close)
            .item("A", Msg::A)
            .divider()
            .item("B", Msg::B);
        let ui = frame(&menu, &theme);
        let panel = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| find_panel(p, &theme))
            .expect("a panel");
        assert_eq!(
            panel.height,
            2.0 * ROW_H + crate::divider::DIVIDER_SPACE + 2.0 * PAD_Y,
            "two rows, a rule of sixteen, and the panel's own room"
        );
        assert_eq!(
            press(&ui, 110.0, row_middle(0.0)),
            Some(Msg::A),
            "the row above it still answers"
        );
    }

    /// **One row can be unavailable while the rest of the menu is not.** It was the whole
    /// menu or nothing, so an application with one action it could not offer had to leave
    /// it out — and an action that is missing tells a reader nothing about why.
    ///
    /// It is still drawn, in the greyed ink, and still announced.
    #[test]
    fn one_row_can_be_unavailable_while_the_rest_is_not() {
        let theme = Theme::default();
        let menu = PopupMenuButton::new(anchor(), true, Msg::Close)
            .item("A", Msg::A)
            .entry(MenuItem::new("B", Msg::B).enabled(false));
        let ui = frame(&menu, &theme);
        assert_eq!(press(&ui, 110.0, row_middle(0.0)), Some(Msg::A));
        assert_eq!(
            press(&ui, 110.0, row_middle(1.0)),
            None,
            "the row that said it was unavailable answers nothing"
        );
        let greyed = texts(ui.scene()).iter().any(|(_, text)| text == "B");
        assert!(greyed, "and is still drawn rather than left out");
    }

    /// **The keys are drawn on the right, muted, and read out after the label.** A
    /// shortcut is the only thing on a row that a reader who cannot see it has no other
    /// way to learn.
    #[test]
    fn the_keys_are_drawn_on_the_right_and_announced() {
        let theme = Theme::default();
        let menu = PopupMenuButton::new(anchor(), true, Msg::Close)
            .entry(MenuItem::new("Paste", Msg::A).shortcut("Ctrl+V"));
        let ui = frame(&menu, &theme);
        let keys = text_at(&ui, "Ctrl+V");
        assert!(
            keys.x > text_at(&ui, "Paste").x,
            "the keys are past the label, not before it"
        );

        let row = Item {
            label: Some("Paste".into()),
            children: Vec::new(),
            lead: None,
            lead_column: false,
            shortcut: Some("Ctrl+V".into()),
            enabled: true,
            text_style: None,
            background: None,
            padding: None,
            height: None,
            message: Msg::A,
        };
        assert_eq!(
            Widget::<Msg>::semantics(&row)
                .expect("announced")
                .label
                .as_deref(),
            Some("Paste, Ctrl+V")
        );
    }

    /// **A row that is on or off is announced as one**, with its state — there is no
    /// `menuitemcheckbox` in this framework's vocabulary and a checkbox is the role that
    /// carries the one thing such a row has to say.
    #[test]
    fn a_row_that_is_on_or_off_is_announced_as_one() {
        let row = |on: bool| Item {
            label: Some("Word wrap".into()),
            children: Vec::new(),
            lead: Some(Lead::Check(on)),
            lead_column: true,
            shortcut: None,
            enabled: true,
            text_style: None,
            background: None,
            padding: None,
            height: None,
            message: Msg::A,
        };
        let on = Widget::<Msg>::semantics(&row(true)).expect("announced");
        assert_eq!(on.role, frus_core::Role::CheckBox);
        assert_eq!(on.toggled, frus_core::Toggled::True);
        assert!(on.clickable, "and it is still an action");
        assert_eq!(
            Widget::<Msg>::semantics(&row(false))
                .expect("announced")
                .toggled,
            frus_core::Toggled::False,
            "off is a state, not the absence of one"
        );
    }

    /// **The menu grows to its widest row**, and does not shrink below the width it has
    /// always had.
    ///
    /// The width was a bare constant. That is fine for a list of one-word actions and
    /// wrong the moment a row carries a mark, a label and the keys that work it: what it
    /// does then is overlap, which no assertion about the tree catches. Two hundred and
    /// twenty is now a floor, which is why no picture in this repository moved.
    #[test]
    fn the_menu_grows_to_its_widest_row_and_never_shrinks() {
        let theme = Theme::default();
        let width = |menu: &PopupMenuButton<Msg>| {
            frame(menu, &theme)
                .scene()
                .primitives()
                .iter()
                .find_map(|p| find_panel(p, &theme))
                .expect("a panel")
                .width
        };
        assert_eq!(
            width(&open_menu()),
            WIDTH,
            "short labels keep the old width"
        );
        let long = PopupMenuButton::new(anchor(), true, Msg::Close)
            .item("Duplicate this record and everything under it", Msg::A);
        assert!(
            width(&long) > WIDTH,
            "a label that does not fit widens the menu instead of running off it"
        );
    }

    /// **A press on the menu's own surface does not close it**, and outside it still
    /// does.
    ///
    /// The room above the first row, the gap a rule leaves, a row that said it was
    /// unavailable — none of those is a target, and each of them used to fall through to
    /// the window-wide region whose press dismisses the menu. The comment above
    /// `Panel::on_click` claimed otherwise for three hundred milestones: a widget with no
    /// message is not registered at all, so there was nothing there to stop anything.
    ///
    /// Verified twice over. Taking `Panel::opaque` back out fails this and the test for a
    /// row that is unavailable, and nothing else in 1 356; so does collapsing
    /// `WidgetId::barrier` back to the identity it used to borrow, since a target the
    /// round trip through `msg_for` cannot tell from the barrier is no target at all.
    #[test]
    fn a_press_on_the_menus_own_surface_does_not_close_it() {
        let theme = Theme::default();
        let ui = frame(&open_menu(), &theme);
        assert_eq!(
            press(&ui, 110.0, 30.0 + PAD_Y * 0.5),
            None,
            "the panel's own room above the first row swallows the press"
        );
        assert_eq!(
            press(&ui, 390.0, 290.0),
            Some(Msg::Close),
            "and the page beyond it still closes the menu"
        );
    }

    #[test]
    fn closed_has_no_overlay() {
        let menu = PopupMenuButton::new(anchor(), false, Msg::Close).item("A", Msg::A);
        assert!(Widget::<Msg>::overlay(&menu).is_none());
        assert_eq!(Widget::<Msg>::children(&menu).len(), 1);
    }

    #[test]
    fn open_floats_items_and_dismisses_outside() {
        let menu = PopupMenuButton::new(anchor(), true, Msg::Close)
            .item("A", Msg::A)
            .item("B", Msg::B);
        assert!(Widget::<Msg>::overlay(&menu).is_some());
        assert_eq!(Widget::<Msg>::overlay_dismiss(&menu), Some(Msg::Close));

        let ui = build_ui(
            &menu,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // A click far from the anchor (the bottom-right corner) closes the menu.
        let corner = ui.hit(P::new(390.0, 290.0)).expect("hit de fermeture");
        assert_eq!(ui.msg_for(corner), Some(Msg::Close));
    }
}

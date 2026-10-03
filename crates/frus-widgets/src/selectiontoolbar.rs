//! The floating bar over a text selection: **Cut, Copy, Paste, Select all** (milestone 568,
//! towards [#23]).
//!
//! A phone has no Ctrl key, so a selection made with a finger needs something to act on it.
//! The bar is what a field shows over its selection after a hold, and after a handle has
//! been dragged; on a desktop, what a right-click opens.
//!
//! **The items are decided by conditions, not written out.** Cut and Copy want a selection,
//! Paste wants something on the clipboard, Select all wants something left unselected — and
//! a field that must not give its text away (a password) offers neither Cut nor Copy. That
//! is the [`ToolbarContext`], and the field turns it into a default list. An application
//! that wants another list says so with [`TextField::selection_toolbar`], which is handed the
//! context and the defaults and returns what to show: drop an item, add one of its own
//! ([`ToolbarItem::custom`]), or return nothing to have no bar at all.
//!
//! **A button does not send a message.** Cut, Copy, Paste and Select all are things the
//! *shell* does — it holds the clipboard and the editing state — so their buttons carry an
//! [`EditAction`] and no message, and the shell performs it on the focused field. Only an
//! application's own item carries a message.
//!
//! [#23]: https://github.com/KalybosPro/frus/issues/23
//! [`TextField::selection_toolbar`]: crate::TextField::selection_toolbar

use frus_core::{BorderRadius, Rect, Scene, ShapeBorder, Size};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// What the shell does to the focused field when a built-in item of the bar is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditAction {
    /// Copies the selection to the clipboard, then deletes it.
    Cut,
    /// Copies the selection to the clipboard.
    Copy,
    /// Types the clipboard's text where the selection or the caret is.
    Paste,
    /// Selects the whole text.
    SelectAll,
}

impl EditAction {
    /// The words on the button.
    pub fn label(self) -> &'static str {
        match self {
            EditAction::Cut => "Cut",
            EditAction::Copy => "Copy",
            EditAction::Paste => "Paste",
            EditAction::SelectAll => "Select all",
        }
    }
}

/// **What is true of the field and the clipboard** when the bar is asked for: the three
/// conditions the default list is built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ToolbarContext {
    /// Something is selected — Cut and Copy have something to take.
    pub has_selection: bool,
    /// The clipboard holds text — Paste has something to give. Where the platform cannot
    /// say without asking (the web's clipboard is asynchronous), this is `true`.
    pub can_paste: bool,
    /// Everything is selected already, or there is nothing to select — Select all would
    /// do nothing.
    pub all_selected: bool,
}

impl ToolbarContext {
    /// How many contexts there are: three conditions, two answers each.
    pub(crate) const VARIANTS: u8 = 8;

    /// A number for the context, `0..VARIANTS`: what a field caches its bars by, and what
    /// the identity of a bar is derived from, so that two different lists never share one.
    pub(crate) fn variant(self) -> u8 {
        u8::from(self.has_selection)
            | u8::from(self.can_paste) << 1
            | u8::from(self.all_selected) << 2
    }
}

/// One item of the bar: a built-in editing action, or the application's own.
#[derive(Clone)]
pub struct ToolbarItem<Msg = crate::callback::Callback> {
    kind: ItemKind<Msg>,
}

#[derive(Clone)]
enum ItemKind<Msg> {
    Action(EditAction),
    Custom { label: String, message: Msg },
}

impl<Msg> ToolbarItem<Msg> {
    /// A built-in item, with its own words.
    pub fn action(action: EditAction) -> Self {
        Self {
            kind: ItemKind::Action(action),
        }
    }

    /// **Cut**.
    pub fn cut() -> Self {
        Self::action(EditAction::Cut)
    }

    /// **Copy**.
    pub fn copy() -> Self {
        Self::action(EditAction::Copy)
    }

    /// **Paste**.
    pub fn paste() -> Self {
        Self::action(EditAction::Paste)
    }

    /// **Select all**.
    pub fn select_all() -> Self {
        Self::action(EditAction::SelectAll)
    }

    /// An item of the application's own: `message` is what pressing it sends, like any
    /// button. The bar closes when it is pressed.
    pub fn custom(label: impl Into<String>, message: Msg) -> Self {
        Self {
            kind: ItemKind::Custom {
                label: label.into(),
                message,
            },
        }
    }

    /// The words on the item.
    pub fn label(&self) -> &str {
        match &self.kind {
            ItemKind::Action(action) => action.label(),
            ItemKind::Custom { label, .. } => label,
        }
    }

    /// The built-in action this item performs, or `None` for an application's own item.
    pub fn edit_action(&self) -> Option<EditAction> {
        match &self.kind {
            ItemKind::Action(action) => Some(*action),
            ItemKind::Custom { .. } => None,
        }
    }
}

/// The bar's height, and so its ends' radius: half of it.
const HEIGHT: f32 = 44.0;
/// The room at either end of the bar, before the first label and after the last.
const PAD_END: f32 = 14.5;
/// The room either side of a label between two others.
const PAD_MIDDLE: f32 = 9.5;
/// A button's narrowest: a target a finger can hit.
const MIN_WIDTH: f32 = 48.0;
/// The bar's height above the surface: just off it, as the platform's own bar is
/// (milestone 606). It was a menu's three, with a shadow that spread four times as far.
const ELEVATION: f32 = 1.0;

/// The bar's surface and the words on it: **white and black** on the default light theme,
/// `#424242` and white on the default dark one — the platform's own bar, which the reference
/// matches by eye — and the scheme's surface and on-surface once a theme names its own
/// (milestone 606).
fn colours(theme: &Theme) -> (frus_core::Color, frus_core::Color) {
    let dark = theme.scheme.brightness == crate::Brightness::Dark;
    let default = if dark { Theme::dark() } else { Theme::light() };
    if theme.scheme.surface == default.scheme.surface
        && theme.scheme.on_surface == default.scheme.on_surface
    {
        if dark {
            (
                frus_core::Color::rgb(
                    0x42 as f32 / 255.0,
                    0x42 as f32 / 255.0,
                    0x42 as f32 / 255.0,
                ),
                frus_core::Color::WHITE,
            )
        } else {
            (frus_core::Color::WHITE, frus_core::Color::BLACK)
        }
    } else {
        (theme.scheme.surface, theme.scheme.on_surface)
    }
}

/// Whether a theme whose platform is `platform` shows the **desktop menu** rather than the
/// pill: Fuchsia, Linux and Windows, as the reference's adaptive toolbar does
/// (`adaptive_text_selection_toolbar.dart:325`). Android and the Apple platforms show the
/// pill; the reference's Apple bars have their own look, which frus does not have yet.
pub(crate) fn uses_menu(platform: frus_core::TargetPlatform) -> bool {
    use frus_core::TargetPlatform as P;
    match platform {
        P::Fuchsia | P::Linux | P::Windows => true,
        P::Android | P::Ios | P::MacOs => false,
    }
}

/// The desktop menu's width (`desktop_text_selection_toolbar.dart:17`).
const MENU_WIDTH: f32 = 222.0;
/// Its corners (`:56`).
const MENU_RADIUS: f32 = 7.0;
/// The room it keeps from the window's edges (`:16`).
const MENU_SCREEN_PADDING: f32 = 8.0;
/// A row's least height (`desktop_text_selection_toolbar_button.dart:69`).
const ROW_HEIGHT: f32 = 36.0;
/// The room before a row's words, after them and under them (`:19`).
const ROW_PAD_X: f32 = 20.0;
const ROW_PAD_BOTTOM: f32 = 3.0;

/// The surface the buttons sit on, in the form the theme's platform takes: a pill, opaque,
/// just lifted off the page, or on the desktops a menu of rows (milestone 618).
pub(crate) struct SelectionToolbar<Msg> {
    pill: Vec<Box<dyn Widget<Msg>>>,
    menu: Vec<Box<dyn Widget<Msg>>>,
    /// Which form the last layout chose, from the theme it was laid out under; read by the
    /// walks that follow it in the same frame.
    desktop: std::cell::Cell<bool>,
}

impl<Msg: Clone + 'static> SelectionToolbar<Msg> {
    /// A bar of `items`, in order.
    pub(crate) fn new(items: Vec<ToolbarItem<Msg>>) -> Self {
        let total = items.len();
        let pill = items
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, item)| {
                let button: Box<dyn Widget<Msg>> =
                    Box::new(ToolbarButton::new(item, index == 0, index + 1 == total));
                button
            })
            .collect();
        let menu = items
            .into_iter()
            .map(|item| Box::new(MenuButton::new(item)) as Box<dyn Widget<Msg>>)
            .collect();
        Self {
            pill,
            menu,
            desktop: std::cell::Cell::new(false),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for SelectionToolbar<Msg> {
    fn style_themed(&self, theme: &Theme) -> Style {
        let desktop = uses_menu(theme.platform);
        self.desktop.set(desktop);
        if desktop {
            Style {
                width: Dimension::Length(MENU_WIDTH),
                height: Dimension::Auto,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }
        } else {
            Widget::<Msg>::style(self)
        }
    }

    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            height: Dimension::Length(HEIGHT),
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        if self.desktop.get() {
            &self.menu
        } else {
            &self.pill
        }
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        if self.desktop.get() {
            // The desktop menu: a card, its corners rounded, one step off the page
            // (`desktop_text_selection_toolbar.dart:55`).
            let radius = BorderRadius::uniform(MENU_RADIUS);
            frus_core::paint_elevation(
                scene,
                bounds,
                radius,
                ELEVATION,
                theme.scheme.shadow.fade(o),
            );
            scene.draw_shape(
                bounds,
                ShapeBorder::rounded(radius),
                theme.scheme.surface.fade(o),
            );
            return;
        }
        // A pill: the ends are half circles, whatever the theme's corners are.
        let radius = BorderRadius::uniform(HEIGHT * 0.5);
        let shape = ShapeBorder::rounded(radius);
        // The reference's shadows for this height (milestone 606).
        frus_core::paint_elevation(
            scene,
            bounds,
            radius,
            ELEVATION,
            theme.scheme.shadow.fade(o),
        );
        scene.draw_shape(bounds, shape, colours(theme).0.fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// A press on the bar that lands on no button is still a press **on the bar**: it must
    /// not reach the page behind and put the selection away.
    fn opaque(&self) -> bool {
        true
    }
}

/// One word on the bar.
struct ToolbarButton<Msg> {
    label: String,
    action: Option<EditAction>,
    message: Option<Msg>,
    /// Whether it is the first on the bar, and the last: the ends keep more room, and
    /// their highlight follows the pill's round ends.
    first: bool,
    last: bool,
}

impl<Msg> ToolbarButton<Msg> {
    fn new(item: ToolbarItem<Msg>, first: bool, last: bool) -> Self {
        let (label, action, message) = match item.kind {
            ItemKind::Action(action) => (action.label().to_owned(), Some(action), None),
            ItemKind::Custom { label, message } => (label, None, Some(message)),
        };
        Self {
            label,
            action,
            message,
            first,
            last,
        }
    }

    /// The room before and after the label.
    fn padding(&self) -> (f32, f32) {
        (
            if self.first { PAD_END } else { PAD_MIDDLE },
            if self.last { PAD_END } else { PAD_MIDDLE },
        )
    }
}

/// The words' style: the scheme's `label_large` at the regular weight, as the platform's
/// own bar sets them.
fn label_style(theme: Option<&Theme>) -> frus_core::TextStyle {
    let mut style = crate::theme::type_scale(theme).label_large;
    style.weight = Some(frus_core::FontWeight::Regular);
    style
}

impl<Msg: Clone> Widget<Msg> for ToolbarButton<Msg> {
    fn style_themed(&self, theme: &Theme) -> Style {
        let measured = frus_text::measure_style(&self.label, label_style(Some(theme)));
        let (before, after) = self.padding();
        Style {
            width: Dimension::Length((measured.width + before + after).ceil().max(MIN_WIDTH)),
            height: Dimension::Length(HEIGHT),
            ..Default::default()
        }
    }

    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let (_, ink) = colours(theme);
        // Nothing at rest; the theme's state layer under a pointer or a finger. Square
        // where it meets another button, round where it meets the pill's end, so it never
        // spills past the bar.
        let layer = theme.state_layer(frus_core::Color::TRANSPARENT, ink, &status);
        let end = HEIGHT * 0.5;
        let (start_r, end_r) = (
            if self.first { end } else { 0.0 },
            if self.last { end } else { 0.0 },
        );
        let radius = BorderRadius {
            top_left: start_r,
            bottom_left: start_r,
            top_right: end_r,
            bottom_right: end_r,
        };
        scene.draw_shape(bounds, ShapeBorder::rounded(radius), layer.fade(o));
        let resolved = label_style(Some(theme)).resolved();
        let measured = frus_text::measure_resolved(&self.label, &resolved);
        let (before, after) = self.padding();
        // Centred in the room between the paddings, which differ at the ends.
        let x = bounds.x + before + (bounds.width - before - after - measured.width) / 2.0;
        scene.text(
            frus_core::Point::new(x, bounds.y + (bounds.height - measured.height) / 2.0),
            self.label.clone(),
            &resolved,
            ink.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    /// A built-in item has no message, so nothing would register it as a target: this is
    /// what makes a press on it a press **on it**, and so a press the shell can resolve
    /// to its [`EditAction`].
    fn opaque(&self) -> bool {
        self.action.is_some()
    }

    fn edit_action(&self) -> Option<EditAction> {
        self.action
    }

    /// The field keeps the focus while its bar is pressed: a bar that took it would end
    /// the selection it acts on.
    fn focusable(&self) -> bool {
        false
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        Some(
            frus_core::SemanticsProperties::new(frus_core::Role::Button)
                .label(self.label.clone())
                .clickable(),
        )
    }
}

/// One row of the desktop menu (`desktop_text_selection_toolbar_button.dart`): the words at
/// its start, white on a dark scheme and black at 87 % on a light one, in a square button
/// the width of the menu.
struct MenuButton<Msg> {
    label: String,
    action: Option<EditAction>,
    message: Option<Msg>,
}

impl<Msg> MenuButton<Msg> {
    fn new(item: ToolbarItem<Msg>) -> Self {
        let (label, action, message) = match item.kind {
            ItemKind::Action(action) => (action.label().to_owned(), Some(action), None),
            ItemKind::Custom { label, message } => (label, None, Some(message)),
        };
        Self {
            label,
            action,
            message,
        }
    }
}

/// A row's words: 14 px at the regular weight (`desktop_text_selection_toolbar_button.dart:12`).
fn menu_label_style() -> frus_core::TextStyle {
    frus_core::TextStyle::new(14.0).weight(frus_core::FontWeight::Regular)
}

/// A row's ink: white on a dark scheme, black at 87 % on a light one (`:42`).
fn menu_ink(theme: &Theme) -> frus_core::Color {
    if theme.scheme.brightness == crate::Brightness::Dark {
        frus_core::Color::WHITE
    } else {
        frus_core::Color::BLACK.fade(0.87)
    }
}

impl<Msg: Clone> Widget<Msg> for MenuButton<Msg> {
    fn style(&self) -> Style {
        let measured = frus_text::measure_style(&self.label, menu_label_style());
        Style {
            width: Dimension::Length(MENU_WIDTH),
            height: Dimension::Length(ROW_HEIGHT.max(measured.height + ROW_PAD_BOTTOM).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let ink = menu_ink(theme);
        // Square, the menu's width: the reference's `RoundedRectangleBorder()` with no
        // radius, so a row's highlight meets the next one edge to edge.
        let layer = theme.state_layer(frus_core::Color::TRANSPARENT, ink, &status);
        scene.draw_rect(
            bounds,
            layer.fade(o),
            0.0,
            0.0,
            frus_core::Color::TRANSPARENT,
        );
        let resolved = menu_label_style().resolved();
        let measured = frus_text::measure_resolved(&self.label, &resolved);
        // At the start, centred in the room above the bottom padding.
        let room = bounds.height - ROW_PAD_BOTTOM;
        scene.text(
            frus_core::Point::new(
                bounds.x + ROW_PAD_X,
                bounds.y + (room - measured.height) / 2.0,
            ),
            self.label.clone(),
            &resolved,
            ink.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    /// As on the pill: a built-in row is a target with no message, resolved to its action.
    fn opaque(&self) -> bool {
        self.action.is_some()
    }

    fn edit_action(&self) -> Option<EditAction> {
        self.action
    }

    /// The field keeps the focus while its menu is used.
    fn focusable(&self) -> bool {
        false
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        Some(
            frus_core::SemanticsProperties::new(frus_core::Role::Button)
                .label(self.label.clone())
                .clickable(),
        )
    }
}

/// **Where the desktop menu goes**, its top-left corner: at `anchor`, moved back by as much
/// as it would hang past the window's padded edge on the right or at the bottom
/// (`desktop_text_selection_toolbar_layout_delegate.dart:39`).
pub(crate) fn place_menu(anchor: frus_core::Point, size: Size, window: Rect) -> (f32, f32) {
    let right = window.x + window.width - MENU_SCREEN_PADDING;
    let bottom = window.y + window.height - MENU_SCREEN_PADDING;
    let over_x = anchor.x + size.width - right;
    let over_y = anchor.y + size.height - bottom;
    (
        if over_x > 0.0 {
            anchor.x - over_x
        } else {
            anchor.x
        },
        if over_y > 0.0 {
            anchor.y - over_y
        } else {
            anchor.y
        },
    )
}

/// The point a menu opened without a pointer hangs from: the top of the selection, at its
/// middle — the reference's primary anchor (`text_selection_toolbar_anchors.dart:45`).
pub(crate) fn primary_anchor(selection: Rect) -> frus_core::Point {
    frus_core::Point::new(selection.x + selection.width * 0.5, selection.y)
}

/// The gap between the bar and the selection it acts on, in px (the reference's
/// `_kToolbarContentDistance`).
pub(crate) const GAP: f32 = 8.0;
/// The room a bar keeps from the window's edges, in px.
pub(crate) const MARGIN: f32 = 8.0;

/// **Where the bar goes**, its top-left corner: centred on the selection and **above** it;
/// **below** it when there is no room above — clearing the handles that hang under the
/// selection, `below_clear` px — and held inside the window on either side.
///
/// `anchor` is the selection's box, `size` the bar's, `window` the room there is.
pub(crate) fn place(anchor: Rect, size: Size, window: Rect, below_clear: f32) -> (f32, f32) {
    let centre = anchor.x + anchor.width * 0.5;
    let max_x = (window.x + window.width - size.width - MARGIN).max(window.x + MARGIN);
    let x = (centre - size.width * 0.5).clamp(window.x + MARGIN, max_x);
    let above = anchor.y - size.height - GAP;
    let y = if above >= window.y + MARGIN {
        above
    } else {
        let below = anchor.y + anchor.height + below_clear + GAP;
        // Even below it might not fit; then it stays on the window rather than off it.
        below.min((window.y + window.height - size.height - MARGIN).max(window.y))
    };
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Rect = Rect::new(0.0, 0.0, 400.0, 800.0);

    #[test]
    fn every_context_has_its_own_number() {
        let mut seen = std::collections::HashSet::new();
        for has_selection in [false, true] {
            for can_paste in [false, true] {
                for all_selected in [false, true] {
                    let context = ToolbarContext {
                        has_selection,
                        can_paste,
                        all_selected,
                    };
                    let variant = context.variant();
                    assert!(variant < ToolbarContext::VARIANTS);
                    assert!(seen.insert(variant), "two contexts share {variant}");
                }
            }
        }
        assert_eq!(seen.len(), usize::from(ToolbarContext::VARIANTS));
    }

    /// **Which platforms get the menu** (`adaptive_text_selection_toolbar.dart:325`):
    /// Fuchsia, Linux and Windows; Android and the Apple platforms keep the pill.
    #[test]
    fn the_desktops_get_the_menu() {
        use frus_core::TargetPlatform as P;
        for platform in P::ALL {
            let menu = matches!(platform, P::Fuchsia | P::Linux | P::Windows);
            assert_eq!(uses_menu(platform), menu, "{platform}");
        }
    }

    /// **The menu hangs from its anchor**, and is pushed back by as much as it would hang
    /// past the window's padded right or bottom edge — no more.
    #[test]
    fn the_menu_hangs_from_its_anchor_inside_the_window() {
        let size = Size::new(MENU_WIDTH, 144.0);
        let at = |x: f32, y: f32| place_menu(frus_core::Point::new(x, y), size, WINDOW);
        assert_eq!(at(50.0, 60.0), (50.0, 60.0), "room: at the anchor");
        assert_eq!(
            at(300.0, 60.0).0,
            400.0 - 8.0 - MENU_WIDTH,
            "pushed back from the right"
        );
        assert_eq!(
            at(50.0, 750.0).1,
            800.0 - 8.0 - 144.0,
            "and up from the bottom"
        );
        assert_eq!(
            primary_anchor(Rect::new(100.0, 40.0, 60.0, 20.0)),
            frus_core::Point::new(130.0, 40.0),
            "a selection's anchor is the middle of its top"
        );
    }

    /// **On the desktops, a menu of rows** (`desktop_text_selection_toolbar.dart`): 222 px
    /// wide, a 36 px row per item, top to bottom in the items' order, the words at the
    /// start, on the scheme's surface with rounded corners — whatever this test runs on.
    #[test]
    fn on_the_desktops_it_is_a_menu_of_rows() {
        let theme = Theme::dark().with_platform(frus_core::TargetPlatform::Linux);
        let ui = crate::build_ui(
            &four(),
            Size::new(600.0, 400.0),
            &crate::Runtime::default(),
            &theme,
        );
        let words: Vec<(String, frus_core::Point)> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, position, .. } => {
                    Some((text.clone(), *position))
                }
                _ => None,
            })
            .collect();
        let labels: Vec<&str> = words.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(labels, ["Cut", "Copy", "Paste", "Select all"]);
        for (i, (_, origin)) in words.iter().enumerate() {
            assert_eq!(origin.x, ROW_PAD_X, "at the start");
            assert!(
                origin.y >= ROW_HEIGHT * i as f32 && origin.y < ROW_HEIGHT * (i + 1) as f32,
                "row {i} of 36 px: {origin:?}"
            );
        }
        let panel = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect {
                    rect,
                    color,
                    radius,
                    ..
                } if rect.width == MENU_WIDTH && *color == theme.scheme.surface => {
                    Some((*rect, radius.top_left))
                }
                _ => None,
            })
            .expect("the menu's surface");
        assert_eq!(panel.0.height, ROW_HEIGHT * 4.0);
        assert_eq!(panel.1, MENU_RADIUS);
    }

    #[test]
    fn the_bar_sits_above_the_selection_centred_on_it() {
        let anchor = Rect::new(150.0, 300.0, 100.0, 20.0);
        let size = Size::new(200.0, 44.0);
        let (x, y) = place(anchor, size, WINDOW, 22.0);
        assert_eq!(x, 100.0, "centred: 200 + 0 - 100");
        assert_eq!(y, 300.0 - 44.0 - GAP);
    }

    #[test]
    fn with_no_room_above_it_goes_below_and_clears_the_handles() {
        let anchor = Rect::new(150.0, 20.0, 100.0, 20.0);
        let size = Size::new(200.0, 44.0);
        let (_, y) = place(anchor, size, WINDOW, 22.0);
        assert_eq!(y, 20.0 + 20.0 + 22.0 + GAP);
    }

    #[test]
    fn against_an_edge_it_is_held_inside_the_window() {
        let size = Size::new(200.0, 44.0);
        // A selection at the far left: centring would put the bar off the left edge.
        let left = place(Rect::new(0.0, 300.0, 30.0, 20.0), size, WINDOW, 22.0);
        assert_eq!(left.0, MARGIN);
        // And at the far right.
        let right = place(Rect::new(380.0, 300.0, 20.0, 20.0), size, WINDOW, 22.0);
        assert_eq!(right.0, 400.0 - 200.0 - MARGIN);
    }

    #[test]
    fn a_window_too_narrow_for_the_bar_pins_it_to_the_left_margin() {
        let (x, _) = place(
            Rect::new(10.0, 300.0, 20.0, 20.0),
            Size::new(500.0, 44.0),
            WINDOW,
            22.0,
        );
        assert_eq!(x, MARGIN);
    }

    #[test]
    fn below_the_selection_it_still_stays_on_the_window() {
        // No room above, and the selection is so low that below does not fit either.
        let window = Rect::new(0.0, 0.0, 400.0, 100.0);
        let (_, y) = place(
            Rect::new(150.0, 30.0, 100.0, 20.0),
            Size::new(200.0, 44.0),
            window,
            22.0,
        );
        assert_eq!(y, 100.0 - 44.0 - MARGIN);
    }

    #[test]
    fn a_built_in_item_says_its_action_and_an_application_item_does_not() {
        assert_eq!(
            ToolbarItem::<()>::cut().edit_action(),
            Some(EditAction::Cut)
        );
        assert_eq!(ToolbarItem::<()>::select_all().label(), "Select all");
        let own = ToolbarItem::custom("Translate", 7);
        assert_eq!(own.edit_action(), None);
        assert_eq!(own.label(), "Translate");
    }

    #[test]
    fn a_built_in_button_is_a_target_without_a_message_and_never_takes_the_focus() {
        let button = ToolbarButton::<u8>::new(ToolbarItem::copy(), true, true);
        assert!(Widget::<u8>::opaque(&button));
        assert_eq!(Widget::<u8>::on_click(&button), None);
        assert_eq!(Widget::<u8>::edit_action(&button), Some(EditAction::Copy));
        assert!(!Widget::<u8>::focusable(&button));
    }

    #[test]
    fn an_application_button_sends_its_message_and_is_not_an_edit() {
        let button = ToolbarButton::new(ToolbarItem::custom("Translate", 7u8), true, true);
        assert_eq!(Widget::<u8>::on_click(&button), Some(7));
        assert_eq!(Widget::<u8>::edit_action(&button), None);
        assert!(!Widget::<u8>::opaque(&button));
    }

    /// The four built-in items, the way a field shows them.
    fn four() -> SelectionToolbar<u8> {
        SelectionToolbar::new(vec![
            ToolbarItem::cut(),
            ToolbarItem::copy(),
            ToolbarItem::paste(),
            ToolbarItem::select_all(),
        ])
    }

    /// The bar laid out as the **pill**: under the theme given, following Android, which is
    /// where the pill is the platform's (milestone 618) — whatever this test runs on.
    fn frame(theme: &Theme) -> crate::Ui<u8> {
        crate::build_ui(
            &four(),
            Size::new(600.0, 100.0),
            &crate::Runtime::default(),
            &theme
                .clone()
                .with_platform(frus_core::TargetPlatform::Android),
        )
    }

    /// **As the platform's own bar looks** (milestone 606): a white pill on the default
    /// light theme, `#424242` on the default dark one, just lifted off the page, with its
    /// words in black or white.
    #[test]
    fn it_is_a_pill_just_off_the_page_in_the_platforms_colours() {
        for (theme, surface, ink) in [
            (
                Theme::light(),
                frus_core::Color::WHITE,
                frus_core::Color::BLACK,
            ),
            (
                Theme::dark(),
                frus_core::Color::rgb(
                    0x42 as f32 / 255.0,
                    0x42 as f32 / 255.0,
                    0x42 as f32 / 255.0,
                ),
                frus_core::Color::WHITE,
            ),
        ] {
            let ui = frame(&theme);
            let bar = ui
                .scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect {
                        rect,
                        color,
                        radius,
                        blur,
                        ..
                    } if *blur == 0.0 && *color == surface => Some((*rect, *radius)),
                    _ => None,
                })
                .expect("the bar's surface");
            assert_eq!(bar.0.height, HEIGHT);
            assert_eq!(bar.1, BorderRadius::uniform(HEIGHT * 0.5), "round ends");
            assert_eq!(crate::shadowprobe::height(ui.scene().primitives()), 1.0);
            let words: Vec<_> = ui
                .scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Text { color, .. } => Some(*color),
                    _ => None,
                })
                .collect();
            assert_eq!(words, vec![ink; 4]);
        }
    }

    /// **A theme with its own surface** gets its surface and its on-surface.
    #[test]
    fn a_theme_of_its_own_colours_it() {
        let mut theme = Theme::light();
        theme.scheme.surface = frus_core::Color::rgb(0.9, 0.95, 1.0);
        theme.scheme.on_surface = frus_core::Color::rgb(0.1, 0.1, 0.3);
        assert_eq!(
            colours(&theme),
            (theme.scheme.surface, theme.scheme.on_surface)
        );
    }

    /// **More room at the ends than between**, fourteen and a half against nine and a half,
    /// so the first and last words sit clear of the round ends; and a short word still gets
    /// a target a finger can hit.
    #[test]
    fn the_ends_keep_more_room_and_a_short_word_a_wide_enough_target() {
        let theme = Theme::light();
        let style = label_style(Some(&theme));
        let width = |label: &str| frus_text::measure_style(label, style).width;
        let first = ToolbarButton::<u8>::new(ToolbarItem::cut(), true, false);
        let middle = ToolbarButton::<u8>::new(ToolbarItem::copy(), false, false);
        let last = ToolbarButton::<u8>::new(ToolbarItem::select_all(), false, true);
        let styled = |b: &ToolbarButton<u8>| match Widget::<u8>::style_themed(b, &theme).width {
            Dimension::Length(w) => w,
            _ => unreachable!(),
        };
        assert_eq!(
            styled(&first),
            (width("Cut") + PAD_END + PAD_MIDDLE).ceil().max(MIN_WIDTH)
        );
        assert_eq!(
            styled(&middle),
            (width("Copy") + 2.0 * PAD_MIDDLE).ceil().max(MIN_WIDTH)
        );
        assert_eq!(
            styled(&last),
            (width("Select all") + PAD_MIDDLE + PAD_END).ceil()
        );
        let tiny = ToolbarButton::<u8>::new(ToolbarItem::custom("A", 1), false, false);
        assert_eq!(styled(&tiny), MIN_WIDTH);
    }
}

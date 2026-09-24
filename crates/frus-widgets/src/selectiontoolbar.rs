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
pub struct ToolbarItem<Msg = crate::callback::Callback> {
    kind: ItemKind<Msg>,
}

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

/// The bar's height and its buttons' minimum width: a comfortable target for a finger.
const HEIGHT: f32 = 44.0;
/// The room on either side of a label.
const PAD_X: f32 = 16.0;
/// The bar's height above the surface: the same as a menu's.
const ELEVATION: f32 = 3.0;

/// The surface the buttons sit on: rounded, opaque, lifted by a shadow.
pub(crate) struct SelectionToolbar<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SelectionToolbar<Msg> {
    /// A bar of `items`, in order.
    pub(crate) fn new(items: Vec<ToolbarItem<Msg>>) -> Self {
        let children = items
            .into_iter()
            .map(|item| {
                let button: Box<dyn Widget<Msg>> = Box::new(ToolbarButton::new(item));
                button
            })
            .collect();
        Self { children }
    }
}

impl<Msg: Clone> Widget<Msg> for SelectionToolbar<Msg> {
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
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let shape = ShapeBorder::rounded(theme.radius);
        let radius = shape
            .as_rounded(bounds)
            .map(|(_, radius)| radius)
            .unwrap_or(BorderRadius::ZERO);
        let blur = ELEVATION * 4.0 + 8.0;
        scene.shadow(
            Rect::new(
                bounds.x - blur,
                bounds.y + ELEVATION * 2.0 - blur,
                bounds.width + 2.0 * blur,
                bounds.height + 2.0 * blur,
            ),
            theme.scheme.shadow.with_alpha(0.35).fade(o),
            radius.inflate(blur),
            blur,
        );
        scene.draw_shape(bounds, shape, theme.scheme.surface_container.fade(o));
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
}

impl<Msg> ToolbarButton<Msg> {
    fn new(item: ToolbarItem<Msg>) -> Self {
        match item.kind {
            ItemKind::Action(action) => Self {
                label: action.label().to_owned(),
                action: Some(action),
                message: None,
            },
            ItemKind::Custom { label, message } => Self {
                label,
                action: None,
                message: Some(message),
            },
        }
    }
}

/// The words' style: the scheme's `label_large`, as a menu's rows use.
fn label_style(theme: Option<&Theme>) -> frus_core::TextStyle {
    crate::theme::type_scale(theme).label_large
}

impl<Msg: Clone> Widget<Msg> for ToolbarButton<Msg> {
    fn style_themed(&self, theme: &Theme) -> Style {
        let measured = frus_text::measure_style(&self.label, label_style(Some(theme)));
        Style {
            width: Dimension::Length((measured.width + PAD_X * 2.0).ceil()),
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
        // Nothing at rest; the theme's state layer under a pointer or a finger.
        let layer = theme.state_layer(
            frus_core::Color::TRANSPARENT,
            theme.scheme.on_surface,
            &status,
        );
        let radius = theme.radius;
        scene.draw_shape(bounds, ShapeBorder::rounded(radius), layer.fade(o));
        let resolved = label_style(Some(theme)).resolved();
        let measured = frus_text::measure_resolved(&self.label, &resolved);
        scene.text(
            frus_core::Point::new(
                bounds.x + (bounds.width - measured.width) / 2.0,
                bounds.y + (bounds.height - measured.height) / 2.0,
            ),
            self.label.clone(),
            &resolved,
            theme.scheme.on_surface.fade(o),
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
        let button = ToolbarButton::<u8>::new(ToolbarItem::copy());
        assert!(Widget::<u8>::opaque(&button));
        assert_eq!(Widget::<u8>::on_click(&button), None);
        assert_eq!(Widget::<u8>::edit_action(&button), Some(EditAction::Copy));
        assert!(!Widget::<u8>::focusable(&button));
    }

    #[test]
    fn an_application_button_sends_its_message_and_is_not_an_edit() {
        let button = ToolbarButton::new(ToolbarItem::custom("Translate", 7u8));
        assert_eq!(Widget::<u8>::on_click(&button), Some(7));
        assert_eq!(Widget::<u8>::edit_action(&button), None);
        assert!(!Widget::<u8>::opaque(&button));
    }
}

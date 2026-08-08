//! [`Menu`]: a **floating** action menu — an anchor plus a list of items that opens
//! over it, through the overlay, and closes on an outside click.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

const WIDTH: f32 = 220.0;
const ROW_H: f32 = 38.0;
const PAD_X: f32 = 12.0;
const SIZE: f32 = 16.0;

/// One menu action, a clickable row.
struct Item<Msg> {
    label: String,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Item<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(WIDTH),
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // A floating panel is an **elevated** surface, the `surface_container_high` role.
        let bg = theme.state_layer(
            theme.scheme.surface_container_high,
            theme.on_surface,
            &status,
        );
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, theme.border.fade(o));
        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(
            Point::new(bounds.x + PAD_X, ty),
            self.label.clone(),
            SIZE,
            theme.on_surface.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// A controlled action menu, opened and closed by the application.
pub struct Menu<Msg> {
    open: bool,
    /// `[ancre]` ou `[ancre, liste]`.
    children: Vec<Box<dyn Widget<Msg>>>,
    items: Vec<(String, Msg)>,
    on_dismiss: Option<Msg>,
}

impl<Msg: Clone + 'static> Menu<Msg> {
    /// Creates a menu around an anchor. When `open`, the list floats above it;
    /// `on_dismiss` is emitted on a click **outside** the menu.
    pub fn new(anchor: impl Widget<Msg> + 'static, open: bool, on_dismiss: Msg) -> Self {
        Self {
            open,
            children: vec![Box::new(anchor)],
            items: Vec::new(),
            on_dismiss: if open { Some(on_dismiss) } else { None },
        }
    }

    /// Adds an action: a label plus a message on click. Ignored when the menu is closed.
    pub fn item(mut self, label: impl Into<String>, message: Msg) -> Self {
        if self.open {
            self.items.push((label.into(), message));
            self.rebuild();
        }
        self
    }

    /// (Re)builds the floating list (child 1) from the items.
    fn rebuild(&mut self) {
        let mut list = Flex::column().gap(2.0);
        for (label, message) in &self.items {
            list = list.child(Item {
                label: label.clone(),
                message: message.clone(),
            });
        }
        let list: Box<dyn Widget<Msg>> = Box::new(list);
        if self.children.len() > 1 {
            self.children[1] = list;
        } else {
            self.children.push(list);
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Menu<Msg> {
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
            .map(|list| (list.as_ref(), Placement::Below))
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

    #[test]
    fn closed_has_no_overlay() {
        let menu = Menu::new(anchor(), false, Msg::Close).item("A", Msg::A);
        assert!(Widget::<Msg>::overlay(&menu).is_none());
        assert_eq!(Widget::<Msg>::children(&menu).len(), 1);
    }

    #[test]
    fn open_floats_items_and_dismisses_outside() {
        let menu = Menu::new(anchor(), true, Msg::Close)
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

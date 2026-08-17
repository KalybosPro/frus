//! [`Portal`]: shows content **above** everything else (out of the layout flow,
//! not clipped by parents). The basis for floating menus, tooltips and modals.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Where to place the overlay relative to its anchor, or to the window.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Just below the anchor (a dropdown menu).
    Below,
    /// Centred in the window, with a scrim (a modal).
    Center,
    /// Above the anchor, **only while the anchor is hovered** (a tooltip).
    Tooltip,
    /// A full-height panel against the **left** edge of the window, with a
    /// scrim (a side drawer). See [`crate::Drawer`].
    Left,
    /// A full-height panel against the **right** edge of the window, with a
    /// scrim (a side drawer). See [`crate::Drawer`].
    Right,
    /// A full-width panel against the **bottom** edge of the window, sliding up,
    /// with a scrim (a modal sheet). See [`crate::BottomSheet`].
    Bottom,
}

/// A portal: an **anchor** (in the flow) and an optional floating **overlay**.
pub struct Portal<Msg> {
    /// `[anchor]` or `[anchor, overlay]`.
    children: Vec<Box<dyn Widget<Msg>>>,
    placement: Placement,
    /// Message emitted when the scrim is clicked (a modal), to close it.
    on_dismiss: Option<Msg>,
}

impl<Msg> Portal<Msg> {
    /// Creates a portal around an anchor, which is rendered normally.
    pub fn new(anchor: impl Widget<Msg> + 'static) -> Self {
        Self {
            children: vec![Box::new(anchor)],
            placement: Placement::Below,
            on_dismiss: None,
        }
    }

    /// Adds the floating content and its placement.
    pub fn overlay(mut self, content: impl Widget<Msg> + 'static, placement: Placement) -> Self {
        self.children.truncate(1);
        self.children.push(Box::new(content));
        self.placement = placement;
        self
    }

    /// Message emitted on a click **outside** the content (on the scrim) — to close.
    pub fn dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Portal<Msg> {
    fn style(&self) -> Style {
        Style::default()
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
            .map(|content| (content.as_ref(), self.placement))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    /// `Escape` closes the portal (if a dismiss message is configured) — consumed
    /// while bubbling leaf→root when the focus is inside.
    fn on_key(&self, key: &crate::Key) -> crate::KeyResponse<Msg> {
        match (key, &self.on_dismiss) {
            (crate::Key::Escape, Some(message)) => {
                crate::KeyResponse::Handled(Some(message.clone()))
            }
            _ => crate::KeyResponse::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Runtime, Size, Text};

    #[test]
    fn overlay_present_only_when_set() {
        let bare: Portal<()> = Portal::new(Container::<()>::new());
        assert!(Widget::<()>::overlay(&bare).is_none());

        let with: Portal<()> =
            Portal::new(Container::<()>::new()).overlay(Text::new("tip"), Placement::Center);
        assert!(Widget::<()>::overlay(&with).is_some());
    }

    #[test]
    fn center_overlay_draws_scrim_and_content() {
        // A Center overlay draws a full-screen scrim + the content on top.
        let portal: Portal<()> = Portal::new(Container::<()>::new().width(20.0).height(20.0))
            .overlay(
                Container::<()>::new()
                    .width(100.0)
                    .height(60.0)
                    .color(frus_core::Color::WHITE),
                Placement::Center,
            );
        let ui = crate::build_ui(
            &portal,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        // At least: the scrim (full screen) + the overlay's content.
        let full_screen =
            ui.scene().primitives().iter().any(
                |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 400.0),
            );
        assert!(full_screen, "the full-screen scrim must be present");
    }

    /// An anchored overlay is nudged back inside the window when it overflows an edge —
    /// that is for a menu opened near the right margin, and it assumes the window is
    /// showing the anchor. When the anchor has **left** the window the nudge does the
    /// opposite of its job: it pulls a menu into view that belongs to something nobody
    /// can see, and leaves a window-wide dismissal barrier behind it that swallows the
    /// next press anywhere.
    #[test]
    fn an_overlay_whose_anchor_left_the_window_goes_with_it() {
        // A row far wider than the window puts the portal's anchor past the right edge.
        let row: crate::Flex<()> = crate::Flex::row()
            .child(Container::<()>::new().width(900.0).height(20.0))
            .child(
                Portal::new(Container::<()>::new().width(20.0).height(20.0))
                    .overlay(
                        Container::<()>::new()
                            .width(120.0)
                            .height(40.0)
                            .color(frus_core::Color::WHITE),
                        Placement::Below,
                    )
                    .dismiss(()),
            );
        let ui = crate::build_ui(
            &row,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        // The overlay's own 120x40 panel must not have been dragged back on screen.
        let inside = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - 120.0).abs() < 0.5 && rect.x < 400.0)
        });
        assert!(!inside, "the overlay was pulled back into the window");
        // And no window-wide dismissal barrier was left behind — `top_dismiss` is what
        // Escape and an outside press both resolve through.
        assert!(
            ui.top_dismiss().is_none(),
            "an invisible barrier would eat the next press"
        );
    }
}

//! [`PopupMenuButton`]: a **floating** action menu — an anchor plus a list of items that opens
//! over it, through the overlay, and closes on an outside click.

use frus_core::{Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

const WIDTH: f32 = 220.0;
const ROW_H: f32 = 38.0;
const PAD_X: f32 = 12.0;

/// The style the items are drawn in: what the caller said, else what the theme says, else
/// the reference's — a popup menu's items are `titleMedium`.
///
/// **Resolved once**, so that the number the box is measured with is the number the glyphs
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403); a size that never passes through it is a size the reader cannot change.
fn label_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.menu.text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).title_medium)
        .resolved()
}

/// One menu action, a clickable row.
struct Item<Msg> {
    label: String,
    /// The menu's availability, handed down to every row.
    enabled: bool,
    text_style: Option<TextStyle>,
    message: Msg,
}

impl<Msg> Item<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        Style {
            width: Dimension::Length(WIDTH),
            height: Dimension::Length(frus_text::line_box(
                ROW_H,
                &label_style(self.text_style, theme),
                0.0,
            )),
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
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // A floating panel is an **elevated** surface, the `surface_container_high` role.
        // No state layer while disabled: a hover tint promises that a press would do
        // something. The row's outline is its container, so it takes the container opacity.
        let bg = if self.enabled {
            theme.state_layer(
                theme.scheme.surface_container_high,
                theme.on_surface,
                &status,
            )
        } else {
            theme.scheme.surface_container_high
        };
        let outline = if self.enabled {
            theme.border
        } else {
            disabled_container(theme)
        };
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, outline.fade(o));
        let ink = if self.enabled {
            theme.on_surface
        } else {
            disabled_content(theme)
        };
        let style = label_style(self.text_style, Some(theme));
        let ty = bounds.y + (bounds.height - style.line_height()) * 0.5;
        scene.text(
            Point::new(bounds.x + PAD_X, ty),
            self.label.clone(),
            &style,
            ink.fade(o),
        );
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
        let semantics =
            frus_core::SemanticsProperties::new(frus_core::Role::Button).label(self.label.clone());
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// A controlled action menu, opened and closed by the application.
pub struct PopupMenuButton<Msg> {
    open: bool,
    enabled: bool,
    /// `[anchor]`, or `[anchor, list]` when the menu is showing.
    children: Vec<Box<dyn Widget<Msg>>>,
    items: Vec<(String, Msg)>,
    text_style: Option<TextStyle>,
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
    pub fn item(mut self, label: impl Into<String>, message: Msg) -> Self {
        if self.open {
            self.items.push((label.into(), message));
            self.rebuild();
        }
        self
    }

    /// (Re)builds the floating list (child 1) from the items.
    ///
    /// A disabled menu keeps only its anchor, so there is no overlay to return and no
    /// panel to trap a press.
    fn rebuild(&mut self) {
        if !self.enabled {
            self.children.truncate(1);
            return;
        }
        let mut list = Flex::column().gap(2.0);
        for (label, message) in &self.items {
            list = list.child(Item {
                label: label.clone(),
                enabled: self.enabled,
                text_style: self.text_style,
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

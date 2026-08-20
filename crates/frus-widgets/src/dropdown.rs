//! [`DropdownButton`]: a **controlled** dropdown list whose options float above
//! everything else (through the overlay mechanism), below the header.
//!
//! Adjustable width ([`width`](DropdownButton::width)), the **selected** option highlighted
//! and ticked ([`selected`](DropdownButton::selected)), and **keyboard** navigation: the
//! header and the options take focus (Enter opens or picks, the arrows move through).

use frus_core::{Path, Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::flex::Flex;
use crate::icons::Icons;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

const DEFAULT_WIDTH: f32 = 240.0;
const ROW_H: f32 = 40.0;
const PAD_X: f32 = 12.0;
const SIZE: f32 = 18.0;

/// One row: the header, or an option.
struct Row<Msg> {
    label: String,
    width: f32,
    is_header: bool,
    /// The currently selected option (highlighted + ticked). Ignored for the header.
    selected: bool,
    /// The list's availability, handed down to the header and to every option.
    enabled: bool,
    on_click: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Row<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Selected option: a primary-tinted background; hover on top (the state layer).
        let base = if self.selected && self.enabled {
            theme.surface.lerp(theme.primary, 0.14)
        } else {
            theme.surface
        };
        // No state layer while disabled: a hover tint is a promise that a press would do
        // something. The outline is the row's **container**, so it takes the container
        // opacity rather than the content one.
        let bg = if self.enabled {
            theme.state_layer(base, theme.on_surface, &status)
        } else {
            base
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
        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(
            Point::new(bounds.x + PAD_X, ty),
            self.label.clone(),
            SIZE,
            ink.fade(o),
        );

        if self.is_header {
            // A vector "▾" chevron (a downward-pointing triangle), on the right.
            let cx = bounds.x + self.width - PAD_X - 4.0;
            let cy = bounds.y + ROW_H * 0.5;
            let (w, h) = (5.0, 3.0);
            let tri = Path::new()
                .move_to(Point::new(cx - w, cy - h))
                .line_to(Point::new(cx + w, cy - h))
                .line_to(Point::new(cx, cy + h))
                .close();
            let chevron = if self.enabled {
                theme.muted
            } else {
                disabled_content(theme)
            };
            scene.fill_path(&tri, chevron.fade(o));
        } else if self.selected {
            // The selected option's tick, on the right.
            let size = 18.0;
            let scale = size / 24.0;
            let x = bounds.x + self.width - PAD_X - size;
            let y = bounds.y + (ROW_H - size) * 0.5;
            let path = Icons::Check.path().scaled(scale).translated(x, y);
            // The tick stays: which option is chosen is still owed to a reader who cannot
            // choose another.
            let check = if self.enabled {
                theme.primary
            } else {
                disabled_content(theme)
            };
            scene.fill_path(&path, check.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        self.on_click.clone()
    }

    fn focusable(&self) -> bool {
        self.enabled && self.on_click.is_some()
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // A dropdown row said nothing to a reader before this. Announcing that a row is
        // unavailable without ever announcing the row would have been announcing an
        // absence, the same hole `RadioOption` had in milestone 322.
        let semantics =
            frus_core::Semantics::new(frus_core::Role::Button).label(self.label.clone());
        let semantics = if self.is_header {
            semantics
        } else {
            semantics.toggled(self.selected)
        };
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// A single-selection dropdown list (a floating menu).
pub struct DropdownButton<Msg> {
    header_label: String,
    on_toggle: Msg,
    width: f32,
    selected: Option<usize>,
    open: bool,
    enabled: bool,
    labels: Vec<String>,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> DropdownButton<Msg> {
    /// Creates a list: the current label + the toggle message (open/close).
    pub fn new(selected_label: impl Into<String>, on_toggle: Msg) -> Self {
        let mut dropdown = Self {
            header_label: selected_label.into(),
            on_toggle,
            width: DEFAULT_WIDTH,
            selected: None,
            open: false,
            enabled: true,
            labels: Vec::new(),
            on_select: None,
            children: Vec::new(),
        };
        dropdown.rebuild();
        dropdown
    }

    /// Whether the list can be opened or chosen from. Disabled it is **inert** - the
    /// header takes no press, no row takes focus - and it still shows the current choice.
    ///
    /// A disabled list is also **never open**: whatever `options` was told, the menu is
    /// not built, because a floating menu over a control that cannot be chosen from is a
    /// menu that traps a press and returns nothing.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Width of the header and the menu, in logical pixels (240 by default).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.rebuild();
        self
    }

    /// Index of the **selected** option (highlighted + ticked in the menu).
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self.rebuild();
        self
    }

    /// Sets the options; if `open`, they float below the header. `on_select` maps
    /// the chosen index to a message.
    pub fn options(
        mut self,
        open: bool,
        labels: &[&str],
        on_select: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        self.open = open;
        self.labels = labels.iter().map(|s| s.to_string()).collect();
        self.on_select = Some(Box::new(on_select));
        self.rebuild();
        self
    }

    /// Rebuilds the header (and the menu if open) from the current state.
    fn rebuild(&mut self) {
        let header = Row {
            label: self.header_label.clone(),
            width: self.width,
            is_header: true,
            selected: false,
            enabled: self.enabled,
            on_click: Some(self.on_toggle.clone()),
        };
        self.children = vec![Box::new(header)];

        if self.open && self.enabled && !self.labels.is_empty() {
            let mut menu = Flex::column().gap(4.0);
            for (index, label) in self.labels.iter().enumerate() {
                let on_click = self.on_select.as_ref().map(|f| f(index));
                menu = menu.child(Row {
                    label: label.clone(),
                    width: self.width,
                    is_header: false,
                    selected: self.selected == Some(index),
                    enabled: self.enabled,
                    on_click,
                });
            }
            self.children.push(Box::new(menu));
        }
    }
}

impl<Msg: Clone> Widget<Msg> for DropdownButton<Msg> {
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
            .map(|menu| (menu.as_ref(), Placement::Below))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle,
        Select(usize),
    }

    #[test]
    fn closed_has_no_overlay_open_floats_options() {
        let closed =
            DropdownButton::new("Pick one", Msg::Toggle).options(false, &["A", "B"], Msg::Select);
        assert!(
            Widget::<Msg>::overlay(&closed).is_none(),
            "closed: no overlay"
        );

        let open =
            DropdownButton::new("Pick one", Msg::Toggle).options(true, &["A", "B"], Msg::Select);
        assert!(
            Widget::<Msg>::overlay(&open).is_some(),
            "open: a floating menu"
        );
        let menu = &Widget::<Msg>::children(&open)[1];
        assert_eq!(menu.children().len(), 2);
        assert_eq!(menu.children()[1].on_click(), Some(Msg::Select(1)));
    }

    #[test]
    fn header_and_options_are_keyboard_focusable() {
        let open = DropdownButton::new("Pick", Msg::Toggle).options(true, &["A", "B"], Msg::Select);
        // A focusable header (opens from the keyboard) + 2 options.
        assert!(Widget::<Msg>::children(&open)[0].focusable());
        let menu = &Widget::<Msg>::children(&open)[1];
        assert!(menu.children()[0].focusable() && menu.children()[1].focusable());
    }

    #[test]
    fn selected_option_is_highlighted_and_checked() {
        let open = DropdownButton::new("Pick", Msg::Toggle)
            .selected(1)
            .options(true, &["A", "B"], Msg::Select)
            .width(200.0);
        // The menu is an overlay: render it on its own to read its primitives.
        let (menu, _) = Widget::<Msg>::overlay(&open).unwrap();
        let ui = build_ui(
            menu,
            Size::new(220.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let theme = Theme::default();
        // The selected option's tick (a filled path).
        let has_check = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Path { .. }));
        assert!(has_check, "the selected option is ticked");
        // The selected option's primary-tinted background.
        let sel = theme.surface.lerp(theme.primary, 0.14);
        let has_tint = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                Primitive::Rect { color, .. } if color.fade(1.0) == sel.fade(1.0)
            )
        });
        assert!(has_tint, "the selected option is highlighted");
    }

    #[test]
    fn a_disabled_list_is_inert_and_cannot_be_open() {
        let dead = DropdownButton::new("Option B", Msg::Toggle)
            .selected(1)
            .options(true, &["A", "B"], Msg::Select)
            .enabled(false);
        // Told to be open, and not open: a floating menu over a control that cannot be
        // chosen from would trap a press and return nothing.
        assert!(Widget::<Msg>::overlay(&dead).is_none(), "no floating menu");
        assert_eq!(Widget::<Msg>::children(&dead).len(), 1, "header only");

        let header = &Widget::<Msg>::children(&dead)[0];
        assert_eq!(header.on_click(), None, "the header takes no press");
        assert!(!header.focusable(), "and no focus");
        let semantics = header.semantics().expect("still announced");
        assert!(semantics.disabled, "announced as unavailable");
        assert_eq!(
            semantics.label.as_deref(),
            Some("Option B"),
            "and still says which option is current"
        );
    }

    #[test]
    fn a_live_list_is_untouched_by_it() {
        let live = DropdownButton::new("Pick one", Msg::Toggle)
            .options(true, &["A", "B"], Msg::Select)
            .enabled(true);
        assert!(Widget::<Msg>::overlay(&live).is_some());
        assert_eq!(
            Widget::<Msg>::children(&live)[0].on_click(),
            Some(Msg::Toggle)
        );
    }
}

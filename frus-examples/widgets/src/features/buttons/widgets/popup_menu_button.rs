//! `PopupMenuButton` and `MenuItem` — a list of actions that floats over a button and
//! closes on a click outside. For the "more" menu of a row or an app bar.

use frus::{
    Button, Callback, IconButton, Icons, MenuItem, PopupMenuButton, Row, State, StateContext,
    StatefulWidget, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(PopupDemo.into_widget())
}

struct PopupDemo;

/// Menus are **controlled**: the application keeps which one is open. Here at most one
/// is, so a single field says which.
struct PopupDemoState {
    open: Option<Menu>,
    last: String,
    show_grid: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Menu {
    Simple,
    Rich,
}

impl PopupDemoState {
    fn toggle(&mut self, menu: Menu) {
        self.open = if self.open == Some(menu) {
            None
        } else {
            Some(menu)
        };
    }

    /// An item was chosen: remember it, and close the menu.
    fn chose(&mut self, what: &str) {
        self.last = what.to_string();
        self.open = None;
    }
}

impl StatefulWidget for PopupDemo {
    type State = PopupDemoState;

    fn create_state(&self) -> PopupDemoState {
        PopupDemoState {
            open: None,
            last: "nothing yet".into(),
            show_grid: true,
        }
    }
}

impl State for PopupDemoState {
    type Widget = PopupDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let pick = |what: &'static str| -> Callback {
            cx.callback(move |s: &mut PopupDemoState| s.chose(what))
        };
        // A click outside the menu sends this.
        let close = cx.callback(|s: &mut PopupDemoState| s.open = None);

        // The anchor is any widget; here an icon button that opens and closes the menu.
        let simple = PopupMenuButton::new(
            IconButton::new(Icons::MORE_VERT)
                .label("More")
                .on_press(cx.callback(|s: &mut PopupDemoState| s.toggle(Menu::Simple))),
            self.open == Some(Menu::Simple),
            close.clone(),
        )
        .item("Rename", pick("Rename"))
        .item("Duplicate", pick("Duplicate"))
        .item("Delete", pick("Delete"));

        // `MenuItem` covers the rows that are not just words: an icon, a tick, a
        // keyboard shortcut, a rule between groups, a row that is unavailable.
        let rich = PopupMenuButton::new(
            Button::new("Edit")
                .variant(Variant::Outlined)
                .on_press(cx.callback(|s: &mut PopupDemoState| s.toggle(Menu::Rich))),
            self.open == Some(Menu::Rich),
            close,
        )
        .entry(MenuItem::icon(Icons::CONTENT_CUT, "Cut", pick("Cut")).shortcut("Ctrl+X"))
        .entry(MenuItem::icon(Icons::CONTENT_COPY, "Copy", pick("Copy")).shortcut("Ctrl+C"))
        .entry(MenuItem::icon(Icons::CONTENT_PASTE, "Paste", pick("Paste")).enabled(false))
        .divider()
        .entry(MenuItem::checked(
            "Show grid",
            self.show_grid,
            cx.callback(|s: &mut PopupDemoState| {
                s.show_grid = !s.show_grid;
                s.open = None;
            }),
        ));
        // The panel's look (shape, elevation, padding) comes from the theme's
        // `MenuTheme`, so every menu of the app agrees.

        sections(vec![
            Box::new(section(
                "Simple and rich menus",
                Row::new().spacing(24.0).child(simple).child(rich),
            )),
            Box::new(Text::new(format!(
                "Last action: {} · grid {}",
                self.last,
                if self.show_grid { "on" } else { "off" }
            ))),
        ])
    }
}

//! `MenuBar`, `SubmenuButton` and `MenuPath` — the File / Edit / View menus along the
//! top of a desktop window, with submenus inside them.

use frus::{
    Callback, MenuBar, MenuItem, MenuPath, State, StateContext, StatefulWidget, SubmenuButton,
    Text, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(MenuBarDemo.into_widget())
}

struct MenuBarDemo;

/// Which menu is open is a **path** — the menu on the bar, then the submenu inside it —
/// kept by the application like any other state.
struct MenuBarDemoState {
    path: MenuPath,
    last: String,
    word_wrap: bool,
}

impl MenuBarDemoState {
    fn chose(&mut self, what: &str) {
        self.last = what.to_string();
        self.path = MenuPath::closed();
    }
}

impl StatefulWidget for MenuBarDemo {
    type State = MenuBarDemoState;

    fn create_state(&self) -> MenuBarDemoState {
        MenuBarDemoState {
            path: MenuPath::closed(),
            last: "nothing yet".into(),
            word_wrap: true,
        }
    }
}

impl State for MenuBarDemoState {
    type Widget = MenuBarDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let pick = |what: &'static str| -> Callback {
            cx.callback(move |s: &mut MenuBarDemoState| s.chose(what))
        };

        let file = SubmenuButton::new("File")
            .item(MenuItem::new("New", pick("New")).shortcut("Ctrl+N"))
            .item(MenuItem::new("Open…", pick("Open")).shortcut("Ctrl+O"))
            // A row that opens a menu beside this one.
            .submenu(
                SubmenuButton::new("Open recent")
                    .item(MenuItem::new("notes.txt", pick("notes.txt")))
                    .item(MenuItem::new("todo.md", pick("todo.md"))),
            )
            .divider()
            .item(MenuItem::new("Quit", pick("Quit")));
        let edit = SubmenuButton::new("Edit")
            .item(MenuItem::new("Undo", pick("Undo")).shortcut("Ctrl+Z"))
            .item(MenuItem::new("Redo", pick("Redo")).enabled(false));
        let view = SubmenuButton::new("View").item(MenuItem::checked(
            "Word wrap",
            self.word_wrap,
            cx.callback(|s: &mut MenuBarDemoState| {
                s.word_wrap = !s.word_wrap;
                s.path = MenuPath::closed();
            }),
        ));

        // The bar reports the path it should have next — a menu opened, moved to,
        // or everything closed — and the handler stores it.
        let bar = MenuBar::new(
            &self.path,
            cx.handler(|s: &mut MenuBarDemoState, path: MenuPath| s.path = path),
        )
        .menu(file)
        .menu(edit)
        .menu(view);

        sections(vec![
            Box::new(section("A window's menus", bar)),
            Box::new(Text::new(format!(
                "Last command: {} · word wrap {}",
                self.last,
                if self.word_wrap { "on" } else { "off" }
            ))),
        ])
    }
}

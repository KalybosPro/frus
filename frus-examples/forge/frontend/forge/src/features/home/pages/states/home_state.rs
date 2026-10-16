use std::rc::Rc;

use frus::{Colors, MenuBar, MenuItem, MenuPath, StateContext, SubmenuButton};

use crate::common::shared::on;

pub(crate) struct HomeState {
    pub menu: MenuPath,
}

impl HomeState {
    pub fn menu_bar(&self, cx: &StateContext<Self>) -> MenuBar {
        let handle = cx.handle();
        let pick = |work: Rc<dyn Fn()>| {
            let handle = handle.clone();
            on(move || {
                work();
                handle.set_state(|s| s.menu = MenuPath::closed());
            })
        };

        let file = SubmenuButton::new("File")
            .item(
                MenuItem::new("Save", pick(Rc::new(move || println!("Save") )))
                    .shortcut("Ctrl+S"),
            )
            .item(MenuItem::new(
                "Load",
                pick(Rc::new(move || println!("Load") )),
            ))
            .divider()
            .item(
                MenuItem::new("Clear completed…", {
                    let handle = handle.clone();
                    on(move || {
                        handle.set_state(|s| {
                            s.menu = MenuPath::closed();
                        })
                    })
                })
                .shortcut("Ctrl+X"),
            ).submenu(SubmenuButton::new("Go"));

        let view = SubmenuButton::new("View");

        MenuBar::new(
            &self.menu,
            cx.handler(|s: &mut HomeState, path: MenuPath| s.menu = path),
        )
        .background(Colors::TRANSPARENT)
        .menu(file)
        .menu(view)
    }
}

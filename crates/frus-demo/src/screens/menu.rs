//! **The window's menu bar** (milestones 607, 642): on a wide window, above every screen, and
//! on the title bar's line where the system allows it.
//!
//! It is the window's, not a screen's. The screens take turns under it: it stays put while
//! one slides in, and a screen without a menu of its own does not hand the line back to the
//! system. So it is built around the router's pages, by the application's `builder`, and
//! what it acts on is what the window can reach — the shared [`Demo`] and the router.

use crate::l10n::LANGS;
use crate::prelude::*;
use frus_widgets::{MenuBar, MenuItem, MenuPath, SubmenuButton, UseState, WindowMenuBar};

/// The application's pages, under the window's menu bar on a wide window. A phone has no
/// bar: its home screen folds the same actions into its header's "⋯" menu.
///
/// The [`WindowMenuBar`] is there at every width, with no bar on a narrow window, so that the
/// pages keep their place in the tree — and their state — when the window is resized across
/// the line.
pub(crate) fn window(
    cx: &BuildContext,
    pages: Box<dyn Widget>,
    demo: &Rc<Demo>,
    router: &GoRouter,
) -> Box<dyn Widget> {
    let menus = cx.use_state(MenuPath::closed);
    let wide = MediaQuery::of().size_class() == SizeClass::Expanded;
    let bar = wide.then(|| menu_bar(&menus, demo, router));
    Box::new(WindowMenuBar::new(bar, pages))
}

/// The menus, in the order a desktop application lists them. A row does its work and closes
/// the menus, as a popup menu's row does: a press is one message, and it is the row's.
fn menu_bar(menus: &UseState<MenuPath>, demo: &Rc<Demo>, router: &GoRouter) -> MenuBar {
    let pick = |work: Rc<dyn Fn()>| {
        let menus = menus.clone();
        on(move || {
            work();
            menus.set(MenuPath::closed());
        })
    };
    let with = |change: fn(&Demo)| -> Rc<dyn Fn()> {
        let demo = demo.clone();
        Rc::new(move || change(&demo))
    };
    // What belongs to the home screen is done there: from another screen, the window goes
    // home first.
    let home = {
        let router = router.clone();
        move || {
            if router.location() != "/" {
                router.go("/");
            }
        }
    };
    let prefs = demo.prefs();

    let file = SubmenuButton::new("File")
        .item(MenuItem::new(
            "Save",
            pick(Rc::new({
                let demo = demo.clone();
                move || demo.save()
            })),
        ))
        .item(MenuItem::new(
            "Load",
            pick(Rc::new({
                let demo = demo.clone();
                move || demo.load()
            })),
        ))
        .divider()
        .item(MenuItem::new(
            "Clear completed…",
            pick(Rc::new({
                let (demo, home) = (demo.clone(), home.clone());
                // On the task list, whose footer the confirmation hangs from: from the Stats
                // section it was asked for and never shown.
                move || {
                    home();
                    demo.set_section(0);
                    demo.ask_clear(true);
                }
            })),
        ));

    let size = |step: f32| -> Rc<dyn Fn()> {
        let demo = demo.clone();
        Rc::new(move || demo.set_density(demo.prefs().density + step))
    };
    let mut language = SubmenuButton::new("Language").item(MenuItem::checked(
        "System",
        prefs.lang.is_none(),
        pick(Rc::new({
            let demo = demo.clone();
            move || demo.set_lang(None)
        })),
    ));
    for (index, (name, _)) in LANGS.iter().enumerate() {
        let demo = demo.clone();
        language = language.item(MenuItem::checked(
            *name,
            prefs.lang == Some(index),
            pick(Rc::new(move || demo.set_lang(Some(index)))),
        ));
    }
    let view = SubmenuButton::new("View")
        .item(MenuItem::checked(
            "Dark theme",
            !prefs.light,
            pick(with(Demo::toggle_theme)),
        ))
        .item(MenuItem::checked(
            "Right to left",
            prefs.rtl,
            pick(with(Demo::toggle_rtl)),
        ))
        .item(MenuItem::new("Next colour", pick(with(Demo::cycle_seed))))
        .divider()
        .submenu(
            SubmenuButton::new("Text size")
                .item(MenuItem::new("Larger", pick(size(0.1))))
                .item(MenuItem::new("Smaller", pick(size(-0.1)))),
        )
        .submenu(language);

    let mut go = SubmenuButton::new("Go");
    for (index, name) in ["Tasks", "Stats", "About"].into_iter().enumerate() {
        go = go.item(MenuItem::checked(
            name,
            demo.section() == index,
            pick(Rc::new({
                let (demo, home) = (demo.clone(), home.clone());
                move || {
                    home();
                    demo.set_section(index);
                }
            })),
        ));
    }
    // A screen is pushed over the one on show, so Back returns to it; asked for again from
    // itself, it is not pushed twice.
    let open = |location: &'static str| {
        let router = router.clone();
        pick(Rc::new(move || {
            if router.location() != location {
                router.push(location);
            }
        }))
    };
    let go = go
        .divider()
        .item(MenuItem::new("Log", open("/journal")))
        .item(MenuItem::new("Settings", open("/settings")));

    let set = menus.clone();
    MenuBar::new(&menus.get(), move |path: MenuPath| {
        let set = set.clone();
        Callback::new(move || set.set(path.clone()))
    })
    .menu(file)
    .menu(view)
    .menu(go)
}

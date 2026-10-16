// use std::rc::Rc;

// use frus::{
//     BuildContext, Callback, GoRouter, MenuBar, MenuItem, MenuPath, SubmenuButton, UseState, Widget,
//     WindowMenuBar,
// };

// use crate::common::{auth::Auth, routers::router, shared::on};

// pub(crate) fn window(cx: &BuildContext, child: Box<dyn Widget>) -> Box<dyn Widget> {
//     let menus = cx.use_state(MenuPath::closed);
//     let bar = cx
//         .theme()
//         .platform
//         .is_desktop()
//         .then(|| menu_bar(&menus, &router(Auth::new())));

//     Box::new(WindowMenuBar::new(bar, child))
// }

// fn menu_bar(state: &UseState<MenuPath>, router: &GoRouter) -> MenuBar {
//     let pick = |work: Rc<dyn Fn()>| {
//         let menus = state.clone();
//         on(move || {
//             work();
//             menus.set(MenuPath::closed());
//         })
//     };

//     let home = {
//         let router = router.clone();
//         move || {
//             if router.location() != "/" {
//                 router.go("/");
//             }
//         }
//     };

//     let task = SubmenuButton::new("Task")
//     .item(MenuItem::new("Home", pick(Rc::new(move || home()))))
//     .item(MenuItem::new(
//         "New task",
//         pick(Rc::new(move || println!("New Task pressed"))),
//     ));

//     let set = state.clone();
//     MenuBar::new(&state.get(), move |path: MenuPath| {
//         let set = set.clone();
//         Callback::new(move || set.set(path.clone()))
//     })
//     .menu(task)
// }


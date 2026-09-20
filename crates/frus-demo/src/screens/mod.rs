//! One module per screen, and the routes that say where each one lives.
//!
//! A screen is a widget — a `StatelessWidget` when it only shows what it is given, a
//! `StatefulWidget` when it keeps something — and a route is the address it answers to. The
//! router builds the widget for the address it was told, keeps the ones under it while
//! another is on top, and moves between them with the slide and the back gesture. Which route
//! is open, how deep the stack is and what each page was told when it was built are the
//! router's; nothing here keeps them.

mod board;
mod charts;
mod data;
mod grid;
mod journal;
mod licenses;
mod settings;
mod sheet;
mod task;
mod todo;
mod tour;
mod wizard;

pub(crate) use board::*;
pub(crate) use charts::*;
pub(crate) use data::*;
pub(crate) use grid::*;
pub(crate) use journal::*;
pub(crate) use licenses::*;
pub(crate) use settings::*;
pub(crate) use sheet::*;
pub(crate) use task::*;
pub(crate) use todo::*;
pub(crate) use tour::*;
pub(crate) use wizard::*;

use crate::prelude::*;

/// The demo's routes. Home is the root; every other screen is a sub-route of it, so going
/// back from any of them lands on the task list — and a task's own screen is `/task/:id`.
pub(crate) fn router(demo: Rc<Demo>) -> GoRouter {
    let with_demo = |path: &str, build: fn(Rc<Demo>) -> Component| {
        let demo = demo.clone();
        GoRoute::new(path.to_string(), move |_, _| build(demo.clone()))
    };
    let home = with_demo("/", |demo| HomePage { demo }.into_widget());
    let task = {
        let demo = demo.clone();
        GoRoute::new("task/:id", move |_, state| {
            TaskPage {
                demo: demo.clone(),
                id: state
                    .param("id")
                    .and_then(|id| id.parse().ok())
                    .unwrap_or(0),
                entering: state.entering(),
            }
            .into_widget()
        })
        .name("task")
    };
    GoRouter::new(vec![home.routes(vec![
        with_demo("settings", |demo| SettingsPage { demo }.into_widget()),
        with_demo("wizard", |demo| WizardPage { demo }.into_widget()),
        with_demo("grid", |demo| GridPage { demo }.into_widget()),
        GoRoute::new("journal", |_, _| JournalPage.into_widget()),
        GoRoute::new("charts", |_, _| ChartsPage.into_widget()),
        GoRoute::new("data", |_, _| DataPage.into_widget()),
        GoRoute::new("board", |_, _| BoardPage.into_widget()),
        GoRoute::new("tour", |_, _| TourPage.into_widget()),
        GoRoute::new("sheet", |_, _| SheetPage.into_widget()),
        GoRoute::new("licenses", |_, _| LicensesPage.into_widget()),
        task,
    ])])
}

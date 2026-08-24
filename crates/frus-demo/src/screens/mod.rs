//! One module per screen, and the routing that decides which one you are looking
//! at.

mod board;
mod charts;
mod data;
mod grid;
mod journal;
mod settings;
mod task;
mod todo;
mod tour;
mod wizard;

pub(crate) use board::*;
pub(crate) use charts::*;
pub(crate) use data::*;
pub(crate) use grid::*;
pub(crate) use journal::*;
pub(crate) use settings::*;
pub(crate) use task::*;
pub(crate) use todo::*;
pub(crate) use tour::*;
pub(crate) use wizard::*;

use crate::prelude::*;

/// The view's entry point: a `Navigator` around the current screen.
pub(crate) fn build_view(app: &TodoApp, theme: &Theme) -> Navigator<Msg> {
    // A back gesture in progress: it previews the pop, driven by the finger.
    if let Some(gesture) = &app.back {
        let progress = gesture.progress;
        let top = screen(current_route(app), app, theme);
        let below_route = app
            .routes
            .split_last()
            .and_then(|(_, rest)| rest.last().copied())
            .unwrap_or(Route::Home);
        let below = screen(below_route, app, theme);
        return Navigator::new(below).from(top, progress, false);
    }

    let current = screen(current_route(app), app, theme);
    match app.nav_from {
        Some(from) => {
            Navigator::new(current).from(screen(from, app, theme), app.nav.value(), app.nav_forward)
        }
        None => Navigator::new(current),
    }
}

/// Builds the screen matching a route.
pub(crate) fn screen(route: Route, app: &TodoApp, theme: &Theme) -> Box<dyn Widget<Msg>> {
    match route {
        Route::Home => todo_screen(app, theme),
        Route::Settings => Box::new(settings_screen(app, theme)),
        Route::Journal => Box::new(journal_screen(app, theme)),
        Route::Wizard => wizard_screen(app, theme),
        Route::GridView => grid_screen(app, theme),
        Route::Charts => charts_screen(app, theme),
        Route::Data => data_screen(app, theme),
        Route::Board => board_screen(app, theme),
        Route::Tour => tour_screen(app, theme),
        Route::Task(id) => task_screen(app, theme, id),
    }
}

//! One module per screen, and the routing that decides which one you are looking
//! at.

mod board;
mod charts;
mod data;
mod grid;
mod journal;
mod licenses;
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
pub(crate) use licenses::licenses_screen;
pub(crate) use settings::*;
pub(crate) use task::*;
pub(crate) use todo::*;
pub(crate) use tour::*;
pub(crate) use wizard::*;

use crate::prelude::*;

/// The view's entry point: a `Navigator` around the current screen.
///
/// Each screen is told **how far in it is** — `1.0` settled, `0.0` not arrived — which is
/// the number the navigator itself is driven by, read from the other end. It is the
/// reference's `ModalRoute.of(context).animation`: a screen that wants to move its own
/// contents as it arrives needs the route's progress, and the application is what has it.
/// A screen that does not care ignores it, which is all but one of them.
pub(crate) fn build_view(app: &TodoApp, theme: &Theme) -> Navigator<Msg> {
    // A back gesture in progress: it previews the pop, driven by the finger. The top
    // screen is the one leaving, so its own progress runs the other way.
    if let Some(gesture) = &app.back {
        let progress = gesture.progress;
        let top = screen(current_route(app), app, theme, 1.0 - progress);
        let below_route = app
            .routes
            .split_last()
            .and_then(|(_, rest)| rest.last().copied())
            .unwrap_or(Route::Home);
        let below = screen(below_route, app, theme, progress);
        return Navigator::new(below).from(top, progress, false);
    }

    match app.nav_from {
        Some(from) => {
            let progress = app.nav.value();
            let current = screen(current_route(app), app, theme, progress);
            let leaving = screen(from, app, theme, 1.0 - progress);
            Navigator::new(current).from(leaving, progress, app.nav_forward)
        }
        None => Navigator::new(screen(current_route(app), app, theme, 1.0)),
    }
}

/// Builds the screen matching a route.
pub(crate) fn screen(
    route: Route,
    app: &TodoApp,
    theme: &Theme,
    entering: f32,
) -> Box<dyn Widget<Msg>> {
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
        Route::Task(id) => task_screen(app, theme, id, entering),
        Route::Licenses => Box::new(licenses_screen(app, theme)),
    }
}

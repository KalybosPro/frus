//! One task's own screen, with a bottom app bar and a docked action.

use crate::prelude::*;
use frus_widgets::column;

/// A paged walkthrough: the finger and the picker drive **one** page number, held by
/// the application (milestone 283).
///
/// This is the whole point of the two-way binding: `on_page_changed` writes the page
/// the finger reached into the state, and `page` reads it back out. Neither side owns
/// it, so neither can drift from the other.
/// One task on its own screen (milestone 286).
///
/// The avatar carries the **same** `Hero` tag as the one on the row this screen was
/// opened from, so the two are understood to be one thing and the transition flies it
/// from the row into place instead of fading one out and the other in.
pub(crate) fn task_screen(app: &TodoApp, theme: &Theme, id: u64) -> Box<dyn Widget<Msg>> {
    let todo = app.todos.iter().find(|t| t.id == id);
    let (label, done) = match todo {
        Some(todo) => (todo.text.clone(), todo.done),
        // Deleted while its screen was open: say so rather than show an empty page.
        None => ("This task no longer exists.".to_string(), false),
    };
    let avatar = Hero::new(id, CircleAvatar::new(label.clone()).size(96.0));
    let state = if done { "Done" } else { "Still to do" };
    let body = column![
        avatar,
        text(label).size(24.0).weight(FontWeight::Bold).wrap(),
        text(state).size(15.0).color(theme.muted),
    ]
    .gap(18.0)
    .align(Align::Center)
    .justify(Justify::Center)
    .flex(1.0);

    // A bottom app bar and a **docked** button (milestone 291): the screen's own
    // actions along the bottom, and the one that matters most astride the bar's top
    // edge, in a notch cut to receive it.
    Scaffold::new()
        .background(theme.background)
        .app_bar(NavigationBar::new("Task").on_back(Msg::Pop))
        // No scroller: this screen's content is centred in whatever room it is given, so
        // it wants the **whole** of that room and nothing more. `flex(1.0)` is how a body
        // asks to fill, now that the Scaffold places it rather than expanding it
        // (milestone 321) — without it the centring would have nothing to centre within.
        .body(
            // No width: the body **is** the slot the Scaffold hands it, and a lone child
            // is bounded by the box it is given (milestone 392). `flex(1)` is the height
            // — the slot's, so that the centring below has something to centre within.
            Container::new().flex(1.0).padding(24.0).child(body),
        )
        .bottom_app_bar(
            // Unfilled actions, as a bottom app bar carries: icons and words, not
            // filled buttons. Also the only thing that works today — see the renderer
            // note in milestone 291: a filled button on a **notched** bar is painted
            // over by the bar's own outline.
            BottomAppBar::new().color(theme.surface).child(
                Flex::row()
                    .align(Align::Center)
                    .gap(16.0)
                    .child(
                        Container::new()
                            .padding(8.0)
                            .on_click(Msg::DeleteTodo(id))
                            .child(text("Delete").size(15.0).color(theme.error)),
                    )
                    .child(
                        Container::new()
                            .padding(8.0)
                            .on_click(Msg::Pop)
                            .child(text("Back").size(15.0).color(theme.muted)),
                    )
                    .child(bar_spacer()),
            ),
        )
        .fab_location(FabLocation::EndDocked)
        .fab(fab_button(
            if done { "↺" } else { "✓" },
            Msg::ToggleTodo(id),
        ))
        .build()
}

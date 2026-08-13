//! The journal screen: a virtualised list of 5000 rows.

use crate::prelude::*;
use frus_widgets::{column, row};

/// The "Journal" screen: a **virtualised list** of 5000 rows, and the place where
/// the two scroll behaviours can be compared by hand (milestone 277).
pub(crate) fn journal_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let t = *theme; // Theme is Copy — captured by the item factory.
    let mut list = List::new(5000, 44.0, move |i| {
        Container::<Msg>::new()
            .height(44.0)
            .radius(8.0)
            .color(if i % 2 == 0 { t.surface } else { t.background })
            .border(1.0, t.border)
            .padding_each(12.0, 14.0, 12.0, 14.0)
            .child(text(format!("Row {}", i + 1)).size(16.0))
    })
    .width((width - 48.0).max(200.0))
    .height((height - 152.0).max(160.0));
    // Unset, the list follows the platform. The toggle overrides it, which is the
    // point of the demonstration: fling to an end and feel the difference.
    if app.journal_bounces {
        list = list.physics(ScrollPhysics::Bouncing);
    }
    let label = if app.journal_bounces {
        "Edges: bounce"
    } else {
        "Edges: platform default"
    };
    // Pull the list past its top edge to reload it. The indicator spins for exactly as
    // long as the application says it is working — here, a countdown in `tick`.
    let pullable = Refresh::new(list)
        .on_refresh(Msg::ReloadJournal)
        .refreshing(app.journal_reloading > 0.0);
    let reloads = if app.journal_reloads == 0 {
        "Pull to reload".to_string()
    } else {
        format!("Reloaded {}×", app.journal_reloads)
    };
    let content = column![
        row![
            text(label).size(14.0).color(theme.muted),
            spacer(),
            text(reloads).size(14.0).color(theme.muted),
            button("Switch", Msg::ToggleScrollPhysics).variant(Variant::Secondary),
        ]
        .gap(12.0),
        pullable,
    ]
    .gap(12.0)
    .padding(24.0);
    let screen = column![NavBar::new("Log · 5000 rows").on_back(Msg::Pop), content]
        .width(width)
        .height(height);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(screen)
}

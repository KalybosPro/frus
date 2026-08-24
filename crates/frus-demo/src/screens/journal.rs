//! The journal screen: a virtualised list of 5000 rows.

use crate::prelude::*;
use frus_widgets::{column, row};

/// The "Journal" screen: a **virtualised list** of 5000 rows, and the place where
/// the two scroll behaviours can be compared by hand (milestone 277).
pub(crate) fn journal_screen(app: &TodoApp, theme: &Theme) -> Container<Msg> {
    // The window this screen fills, read from the surface description in force:
    // nothing hands it down any more.
    let Size { width, height } = surface();
    let t = *theme; // Theme is Copy — captured by the item factory.
    let mut list = ListView::new(5000, 44.0, move |i| {
        Container::<Msg>::new()
            .height(44.0)
            .radius(8.0)
            .color(if i % 2 == 0 { t.surface } else { t.background })
            .border(1.0, t.border)
            .padding_each(12.0, 14.0, 12.0, 14.0)
            .child(text(format!("Row {}", i + 1)).size(16.0))
    })
    .width((width - 48.0).max(200.0))
    // 56 for the bar, 24 + 24 for the padding, 40 for the header row and 12 for the gap
    // under it. It said 152 until milestone 349, and the four missing pixels were being
    // paid for by the list quietly giving way — which is exactly the kind of arithmetic
    // slip that no longer hides.
    .height((height - 156.0).max(160.0));
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
    let pullable = RefreshIndicator::new(list)
        .on_refresh(Msg::ReloadJournal)
        .refreshing(app.journal_reloading > 0.0);
    let reloads = if app.journal_reloads == 0 {
        "Pull to reload".to_string()
    } else {
        format!("Reloaded {}×", app.journal_reloads)
    };
    let content = column![
        row![
            // Expanding rather than a `spacer()` after it: it does the same pushing when
            // there is room, and when there is not — a phone is 25 px short of this
            // header — it is the one that gives way, with an ellipsis, instead of the
            // row running past the screen.
            Expanded::new(text(label).size(14.0).color(theme.muted).ellipsis()),
            text(reloads).size(14.0).color(theme.muted),
            button("Switch", Msg::ToggleScrollPhysics).variant(Variant::Outlined),
        ]
        .gap(12.0),
        pullable,
    ]
    .gap(12.0)
    .padding(24.0);
    let screen = column![
        NavigationBar::new("Log · 5000 rows").on_back(Msg::Pop),
        content
    ]
    .flex(1.0);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        // The background runs **under** the bars; the content does not. `SafeArea` reads
        // the intrusions from the surface description, so a screen with no `Scaffold` to
        // do it for it still keeps clear of the notch.
        .child(SafeArea::new(screen))
}

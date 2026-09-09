//! The journal screen: a virtualised list of 5000 rows.

use crate::prelude::*;
use frus_widgets::{column, row, LinearProgressIndicator};

/// The name the log list answers to. A scroll request addresses a region by key, the
/// way a focus request addresses a field, and the application is the one that knows both
/// ends of it — so the name is written once, here, rather than spelled out in the screen
/// that declares it and again in the `update` that commands it.
pub(crate) const JOURNAL_LIST: &str = "journal-list";

/// How tall one row is. The header divides by it to say which row is at the top, so it
/// is a constant rather than a number typed twice.
const ROW_HEIGHT: f32 = 44.0;

/// How many rows there are.
const ROWS: usize = 5000;

/// How far down the list has to be before the way back is worth offering. Under this,
/// the reader is a flick from the top and a button would be furniture.
const TOP_BUTTON_AT: f32 = 400.0;

/// The "Journal" screen: a **virtualised list** of 5000 rows, and the place where
/// the two scroll behaviours can be compared by hand (milestone 277). Since milestone
/// 493 it is also where the list says where it is and is told where to go.
pub(crate) fn journal_screen(app: &TodoApp, theme: &Theme) -> Container<Msg> {
    // The window this screen fills, read from the surface description in force:
    // nothing hands it down any more.
    let Size { width, height } = surface();
    // Owned, because the item factory outlives this call. Eight kilobytes, once per
    // build of this screen — `Theme` stopped being `Copy` in milestone 448, which is
    // what makes that visible here rather than silent.
    let t = theme.clone();
    let mut list = ListView::new(ROWS, ROW_HEIGHT, move |i| {
        Container::<Msg>::new()
            .height(ROW_HEIGHT)
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
    // slip that no longer hides. Milestone 493 added the position row: 28 more, and its
    // own 12 of gap.
    .height((height - 196.0).max(160.0))
    // Where it is, whenever it moves. Not every pixel: four is a tenth of a row, so the
    // number in the header is never a row out, and a slow drag says so ten times less
    // often. A fling still reports every frame — nothing under four pixels happens
    // during one — which is the honest cost of a view that answers the offset.
    .on_scroll(Msg::JournalScrolled)
    .notify_every(4.0);
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
    //
    // The list is **named** on its way in: `Msg::JournalToTop` returns a request
    // addressed to this key, and the frame turns the key back into the region.
    let pullable = RefreshIndicator::new(keyed(JOURNAL_LIST, list))
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
        position_row(app, theme),
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

/// The row that answers the list: which row is at the top, how far down it is, and the
/// way back once there is one.
///
/// **Its height is fixed**, and that is not a detail. The button comes and goes with the
/// offset; a row that grew when it appeared would shorten the list, which changes how far
/// the list can scroll, which moves the offset — a view driven by a measurement of itself
/// has to be built so that what it draws cannot change the measurement.
fn position_row(app: &TodoApp, theme: &Theme) -> Container<Msg> {
    let position = app.journal_scroll;
    let where_it_is = match position {
        // Nothing has moved yet, so nothing has been measured yet, and a row number
        // here would be this screen guessing rather than the list reporting.
        None => format!("{ROWS} rows"),
        Some(position) => {
            let first = (position.offset.1 / ROW_HEIGHT).floor().max(0.0) as usize + 1;
            format!("Row {} of {ROWS}", first.min(ROWS))
        }
    };
    let mut bar = row![
        Expanded::new(text(where_it_is).size(14.0).color(theme.muted).ellipsis()),
        LinearProgressIndicator::new(position.map_or(0.0, |p| p.fraction_y())).width(120.0),
    ]
    .gap(12.0);
    if position.is_some_and(|p| p.offset.1 > TOP_BUTTON_AT) {
        bar = bar.child(button("Top", Msg::JournalToTop).variant(Variant::Text));
    }
    Container::new().height(28.0).child(bar)
}

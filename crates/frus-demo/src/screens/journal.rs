//! The journal screen: a virtualised list of 5000 rows.

use crate::prelude::*;
use frus_widgets::host;
use frus_widgets::{column, row, LinearProgressIndicator};
use std::time::Duration;

/// The name the log list answers to. A scroll request addresses a region by key, the
/// way a focus request addresses a field, and the screen is the one that knows both
/// ends of it — so the name is written once, here, rather than spelled out in the list that
/// declares it and again in the button that commands it.
pub(crate) const JOURNAL_LIST: &str = "journal-list";

/// How tall one row is. The header divides by it to say which row is at the top, so it
/// is a constant rather than a number typed twice.
const ROW_HEIGHT: f32 = 44.0;

/// How many rows there are.
const ROWS: usize = 5000;

/// How far down the list has to be before the way back is worth offering. Under this,
/// the reader is a flick from the top and a button would be furniture.
const TOP_BUTTON_AT: f32 = 400.0;

/// How long the stand-in reload behind the list's pull-to-refresh takes.
const RELOAD: Duration = Duration::from_millis(1200);

/// The "Journal" screen: a **virtualised list** of 5000 rows, and the place where
/// the two scroll behaviours can be compared by hand (milestone 277). Since milestone
/// 493 it is also where the list says where it is and is told where to go.
pub(crate) struct JournalPage;

/// What the log screen keeps.
#[derive(Default)]
pub(crate) struct JournalState {
    /// Does the log list bounce at its ends rather than stop dead? `false` leaves it on the
    /// platform's own behaviour.
    pub(crate) bounces: bool,
    /// Is the log list reloading? The pull-to-refresh indicator spins for exactly as long as
    /// this is true — the screen owns the answer, not the framework.
    pub(crate) reloading: bool,
    /// How many times the log has been reloaded, so a completed pull leaves a trace.
    pub(crate) reloads: usize,
    /// **Where the log list is**, as the list itself last said. `None` until it has moved at
    /// all — which is not the same as resting at the top, and the header says so: a list that
    /// has never moved shows no row number, because nothing has been measured yet.
    pub(crate) scroll: Option<ScrollPosition>,
}

impl JournalState {
    /// A stand-in for the request a real application would fire: the indicator spins for as
    /// long as `reloading` is set, and the reload counts once it is done.
    fn start_reload(&mut self, handle: frus_widgets::StateHandle<Self>) -> bool {
        if self.reloading {
            return false;
        }
        self.reloading = true;
        host::after(RELOAD, move || {
            handle.set_state(|s| {
                s.reloading = false;
                s.reloads += 1;
            })
        });
        true
    }
}

impl StatefulWidget for JournalPage {
    type State = JournalState;

    fn create_state(&self) -> JournalState {
        JournalState::default()
    }
}

impl State for JournalState {
    type Widget = JournalPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme().clone();
        let theme = &theme;
        // The window this screen fills, read from the surface description in force:
        // nothing hands it down any more.
        let Size { width, height } = surface();
        // What the system's bars take from it, which the safe area keeps the content clear of:
        // the list is sized to what is left.
        let insets = MediaQuery::of().padding;
        let bars = insets.top + insets.bottom;
        // Owned, because the item factory outlives this call. Eight kilobytes, once per
        // build of this screen — `Theme` stopped being `Copy` in milestone 448, which is
        // what makes that visible here rather than silent.
        let t = theme.clone();
        let mut list = ListView::new(ROWS, ROW_HEIGHT, move |i| {
            Container::<Callback>::new()
                .height(ROW_HEIGHT)
                .radius(8.0)
                .color(if i % 2 == 0 { t.surface } else { t.background })
                .border(1.0, t.border)
                .padding_each(12.0, 14.0, 12.0, 14.0)
                .child(text(format!("Row {}", i + 1)).size(16.0))
        })
        .width((width - 48.0).max(200.0))
        // What is left of the window once the system's bars are taken off, less 56 for the bar,
        // 24 + 24 for the padding, 40 for the header row and 12 for the gap
        // under it. It said 152 until milestone 349, and the four missing pixels were being
        // paid for by the list quietly giving way — which is exactly the kind of arithmetic
        // slip that no longer hides. Milestone 493 added the position row: 28 more, and its
        // own 12 of gap.
        .height((height - bars - 196.0).max(160.0))
        // Where it is, whenever it moves. Not every pixel: four is a tenth of a row, so the
        // number in the header is never a row out, and a slow drag says so ten times less
        // often. A fling still reports every frame — nothing under four pixels happens
        // during one — which is the honest cost of a view that answers the offset.
        .on_scroll(cx.handler(|s, position: ScrollPosition| s.scroll = Some(position)))
        .notify_every(4.0);
        // Unset, the list follows the platform. The toggle overrides it, which is the
        // point of the demonstration: fling to an end and feel the difference.
        if self.bounces {
            list = list.physics(ScrollPhysics::BOUNCING);
        }
        let label = if self.bounces {
            "Edges: bounce"
        } else {
            "Edges: platform default"
        };
        // Pull the list past its top edge to reload it. The indicator spins for exactly as
        // long as the screen says it is working.
        //
        // The list is **named** on its way in: the "Top" button asks the host to scroll the
        // region with this key, and the frame turns the key back into the region.
        let handle = cx.handle();
        let pullable = RefreshIndicator::new(keyed(JOURNAL_LIST, list))
            .on_refresh(on(move || {
                let again = handle.clone();
                handle.set_state(|s| {
                    s.start_reload(again);
                });
            }))
            .refreshing(self.reloading);
        let reloads = if self.reloads == 0 {
            "Pull to reload".to_string()
        } else {
            format!("Reloaded {}×", self.reloads)
        };
        let content = column![
            row![
                // Expanding rather than a `spacer()` after it: it does the same pushing when
                // there is room, and when there is not — a phone is 25 px short of this
                // header — it is the one that gives way, with an ellipsis, instead of the
                // row running past the screen.
                Expanded::new(text(label).size(14.0).color(theme.muted).ellipsis()),
                text(reloads).size(14.0).color(theme.muted),
                button("Switch", cx.callback(|s| s.bounces = !s.bounces))
                    .variant(Variant::Outlined),
            ]
            .gap(12.0),
            self.position_row(theme),
            pullable,
        ]
        .gap(12.0)
        .padding(24.0);
        let router = cx.router();
        let screen = column![
            NavigationBar::new("Log · 5000 rows").on_back(on(move || {
                router.pop();
            })),
            content
        ]
        .flex(1.0);
        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(theme.background)
                // The background runs **under** the bars; the content does not. `SafeArea`
                // reads the intrusions from the surface description, so a screen with no
                // `Scaffold` to do it for it still keeps clear of the notch.
                .child(SafeArea::new(screen)),
        )
    }
}

impl JournalState {
    /// The row that answers the list: which row is at the top, how far down it is, and the
    /// way back once there is one.
    ///
    /// **Its height is fixed**, and that is not a detail. The button comes and goes with the
    /// offset; a row that grew when it appeared would shorten the list, which changes how far
    /// the list can scroll, which moves the offset — a view driven by a measurement of itself
    /// has to be built so that what it draws cannot change the measurement.
    fn position_row(&self, theme: &Theme) -> Container {
        let position = self.scroll;
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
            // Not a change of state: the same list at the same offset, and yet this happens
            // once. So it is a request to the host, and it names the region the way a focus
            // request names a field.
            bar = bar.child(
                button(
                    "Top",
                    on(|| host::scroll_to(JOURNAL_LIST, ScrollTo::start())),
                )
                .variant(Variant::Text),
            );
        }
        Container::new().height(28.0).child(bar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_measured_until_the_list_moves() {
        let state = JournalState::default();
        assert!(state.scroll.is_none());
        assert!(!state.reloading);
        assert_eq!(state.reloads, 0);
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! State restoration through the shell (milestone 753): an application is driven — a page
//! pushed, a list scrolled, a count raised, a draft typed — its restoration data taken as the
//! platform would take it, and a **second application**, made afresh as a new process makes
//! it, is handed that data and must open where the first one was.

use crate::app::testing::Driver;
use crate::{Application, FrusApp};
use frus_widgets::{
    column, component, restoration_scope, Button, Callback, ExpansionTile, GoRoute, GoRouter,
    GridView, ListView, ListWheel, PageView, Point, RestorationBucket, TabBar, Text, TextField,
    Widget,
};

const W: f32 = 400.0;
const H: f32 = 600.0;

/// A home page with a count and a long list, and a page to write a message on.
fn app(restoring: bool) -> FrusApp {
    let router = GoRouter::new(vec![
        GoRoute::new("/", |_, _| {
            component(|cx| {
                let taps = cx.use_restorable_state("taps", || 0u32);
                let tap = taps.clone();
                let router = cx.router();
                Box::new(column![
                    Text::new(format!("Taps: {}", taps.get())),
                    Button::new("Tap").on_press(move || tap.update(|n| *n += 1)),
                    Button::new("Open").on_press(move || router.push("/compose")),
                    ListView::new(200, 50.0, |i| Text::new(format!("Row {i}")))
                        .height(300.0)
                        .restoration_id("rows"),
                ]) as Box<dyn Widget>
            })
        }),
        GoRoute::new("/compose", |_, _| {
            component(|cx| {
                let draft = cx.use_restorable_text_controller("draft", "");
                Box::new(column![
                    TextField::new("")
                        .label("Message")
                        .controller(&draft)
                        .restoration_id("field"),
                    // A part of the page with ids of its own: the same id as the page's.
                    restoration_scope("options", |cx| {
                        let taps = cx.use_restorable_state("taps", || 0u32);
                        let tap = taps.clone();
                        Box::new(column![
                            Text::new(format!("Option taps: {}", taps.get())),
                            Button::new("Option").on_press(move || tap.update(|n| *n += 1)),
                        ]) as Box<dyn Widget>
                    }),
                ]) as Box<dyn Widget>
            })
        }),
    ])
    // Pages arrive at once: the frames of a test are not spent sliding.
    .animate(false);
    let app = FrusApp::router(router);
    if restoring {
        app.restoration_scope_id("app")
    } else {
        app
    }
}

fn settle(driver: &mut Driver<FrusApp>) {
    for _ in 0..90 {
        driver.frame(1.0 / 60.0);
    }
}

fn shows(driver: &Driver<FrusApp>, text: &str) -> bool {
    driver.texts().iter().any(|(t, _)| t == text)
}

fn tap(driver: &mut Driver<FrusApp>, label: &str) {
    assert!(
        driver.tap_text(label),
        "nothing reads {label:?}: {:?}",
        driver.texts()
    );
    settle(driver);
}

/// The vertical offset of the one scroll area that is scrolled — or 0.
fn list_offset(driver: &Driver<FrusApp>) -> f32 {
    driver
        .offsets()
        .iter()
        .map(|(_, (_, y))| *y)
        .fold(0.0, f32::max)
}

/// The application's bucket, out of the bytes.
fn bucket(data: &[u8]) -> RestorationBucket {
    RestorationBucket::decode(data)
        .and_then(|root| root.child("app").cloned())
        .expect("the data holds the application's bucket")
}

/// The whole round: a page pushed, a list scrolled, counts raised in two scopes, a draft
/// typed; then a new application opens on the same page, with the same draft, its caret
/// where it was, and going back finds the list scrolled and the count kept.
#[test]
fn a_new_process_opens_where_the_last_one_was() {
    let mut first = Driver::new(app(true), W, H);
    settle(&mut first);
    tap(&mut first, "Tap");
    tap(&mut first, "Tap");
    assert!(shows(&first, "Taps: 2"));

    // Scroll the list by dragging it up.
    let row = first
        .texts()
        .into_iter()
        .find(|(t, _)| t == "Row 3")
        .map(|(_, rect)| rect)
        .expect("the list's rows are on screen");
    let at = Point::new(row.x + 10.0, row.y + 5.0);
    first.press(at);
    for step in 1..=10 {
        first.move_to(Point::new(at.x, at.y - step as f32 * 15.0));
        first.frame(1.0 / 60.0);
    }
    first.release(Point::new(at.x, at.y - 150.0));
    settle(&mut first);
    let scrolled = list_offset(&first);
    assert!(scrolled > 50.0, "the list moved: {scrolled}");

    tap(&mut first, "Open");
    assert!(first.assistive_set_value("Message", "hello there"));
    settle(&mut first);
    tap(&mut first, "Option");
    assert!(shows(&first, "Option taps: 1"));

    let data = first.restoration_data().expect("restoration is on");
    let kept = bucket(&data);
    assert_eq!(kept.value("router"), Some("g/\np/compose"), "{kept:?}");

    // A new process: everything made afresh, then handed what the last one kept.
    let mut second = Driver::new(app(true), W, H);
    assert!(second.restore(&data));
    settle(&mut second);
    assert_eq!(second.app().location().as_deref(), Some("/compose"));
    assert_eq!(second.app().location_stack(), vec!["/", "/compose"]);
    assert!(
        shows(&second, "hello there"),
        "the draft is back: {:?}",
        second.texts()
    );
    assert!(
        shows(&second, "Option taps: 1"),
        "and the scope's own count"
    );
    // The caret came back to where typing left it, and is kept again from here.
    let again = bucket(&second.restoration_data().unwrap());
    let compose = again
        .children()
        .find(|(name, _)| name.starts_with("route 1 "))
        .map(|(_, b)| b.clone())
        .expect("the page on top has a scope");
    assert_eq!(
        compose.get(&["field".into(), "selection".into()]),
        Some("11")
    );
    assert_eq!(compose.value("draft"), Some("hello there"));
    assert_eq!(compose.get(&["options".into(), "taps".into()]), Some("1"));

    // Back to the page beneath: its count, and its list where it was scrolled to.
    assert!(second.system_back());
    settle(&mut second);
    assert_eq!(second.app().location().as_deref(), Some("/"));
    assert!(shows(&second, "Taps: 2"), "{:?}", second.texts());
    let restored = list_offset(&second);
    assert!(
        (restored - scrolled).abs() < 0.5,
        "the list opens where it was: {restored} against {scrolled}"
    );

    // The page that left the stack took its values with it.
    let after = bucket(&second.restoration_data().unwrap());
    assert!(
        after
            .children()
            .all(|(name, _)| !name.starts_with("route 1 ")),
        "{after:?}"
    );
}

/// Without a restoration scope id nothing is kept, and data handed over is ignored.
#[test]
fn nothing_is_kept_unless_the_application_asks() {
    let mut first = Driver::new(app(false), W, H);
    settle(&mut first);
    tap(&mut first, "Tap");
    assert!(first.restoration_data().is_none());

    let mut keeping = Driver::new(app(true), W, H);
    settle(&mut keeping);
    tap(&mut keeping, "Tap");
    let data = keeping.restoration_data().unwrap();

    let mut second = Driver::new(app(false), W, H);
    assert!(!second.restore(&data));
    settle(&mut second);
    assert!(shows(&second, "Taps: 0"));
}

/// Data from another application, or damaged, starts the application afresh.
#[test]
fn data_that_is_not_ours_starts_afresh() {
    let mut other = RestorationBucket::new();
    other
        .child_mut("another app")
        .set_value("router", "g/\np/compose");
    for data in [other.encode(), b"garbage".to_vec()] {
        let mut driver = Driver::new(app(true), W, H);
        assert!(!driver.restore(&data));
        settle(&mut driver);
        assert_eq!(driver.app().location().as_deref(), Some("/"));
        assert!(shows(&driver, "Taps: 0"));
    }
}

/// A count kept in a hook is a value like any other: a State that names a restoration id is
/// given back what it saved, before `init_state`.
#[test]
fn a_state_with_a_restoration_id_is_restored_before_init() {
    use frus_widgets::{State, StateContext, StatefulWidget};

    #[derive(Clone)]
    struct Counter;
    struct CounterState {
        count: u32,
        at_init: Option<u32>,
    }
    impl StatefulWidget for Counter {
        type State = CounterState;
        fn create_state(&self) -> CounterState {
            CounterState {
                count: 0,
                at_init: None,
            }
        }
    }
    impl State for CounterState {
        type Widget = Counter;
        fn init_state(&mut self, _cx: &StateContext<Self>) {
            self.at_init = Some(self.count);
        }
        fn save(&self) -> Option<String> {
            Some(self.count.to_string())
        }
        fn restore(&mut self, saved: &str) {
            self.count = saved.parse().unwrap_or(0);
        }
        fn restoration_id(&self) -> Option<String> {
            Some("counter".into())
        }
        fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
            Box::new(column![
                Text::new(format!(
                    "Count: {} (at init {})",
                    self.count,
                    self.at_init.unwrap_or(99)
                )),
                Button::new("Add").on_press(cx.callback(|s: &mut CounterState| s.count += 1)),
            ])
        }
    }

    let make = || FrusApp::stateful(Counter).restoration_scope_id("app");
    let mut first = Driver::new(make(), W, H);
    settle(&mut first);
    tap(&mut first, "Add");
    tap(&mut first, "Add");
    tap(&mut first, "Add");
    let data = first.restoration_data().unwrap();

    let mut second = Driver::new(make(), W, H);
    assert!(second.restore(&data));
    settle(&mut second);
    assert!(
        shows(&second, "Count: 3 (at init 3)"),
        "{:?}",
        second.texts()
    );
}

/// Values kept inside the rows of a list built on demand — keyed rows, whose state follows
/// their key (milestone 748) — come back to the same rows: each row has a scope of its own
/// below the list's, so the same id in every row does not collide.
#[test]
fn values_in_keyed_list_rows_come_back_to_their_rows() {
    let make = || {
        let router = GoRouter::new(vec![GoRoute::new("/", |_, _| {
            ListView::new(6, 60.0, |i| {
                frus_widgets::keyed(
                    ("item", i),
                    component(move |cx| {
                        let taps = cx.use_restorable_state("taps", || 0u32);
                        let tap = taps.clone();
                        Box::new(column![
                            Text::new(format!("Row {i}: {}", taps.get())),
                            Button::new(format!("Tap {i}"))
                                .on_press(move || tap.update(|n| *n += 1)),
                        ]) as Box<dyn Widget>
                    }),
                )
            })
            .height(400.0)
            .restoration_id("items")
        })])
        .animate(false);
        FrusApp::router(router).restoration_scope_id("app")
    };
    let mut first = Driver::new(make(), W, H);
    settle(&mut first);
    tap(&mut first, "Tap 2");
    tap(&mut first, "Tap 2");
    tap(&mut first, "Tap 0");
    assert!(shows(&first, "Row 2: 2"), "{:?}", first.texts());
    let data = first.restoration_data().unwrap();
    let kept = bucket(&data);
    let page = kept
        .children()
        .find(|(name, _)| name.starts_with("route 0 "))
        .map(|(_, b)| b.clone())
        .expect("the page has a scope");
    assert_eq!(
        page.get(&["items".into(), "row 2".into(), "taps".into()]),
        Some("2"),
        "{page:?}"
    );

    let mut second = Driver::new(make(), W, H);
    assert!(second.restore(&data));
    settle(&mut second);
    assert!(shows(&second, "Row 2: 2"), "{:?}", second.texts());
    assert!(shows(&second, "Row 0: 1"));
    assert!(shows(&second, "Row 1: 0"));
}

// ---------------------------------------------------------------------------------------
// Milestone 756: the widgets that keep a position of their own
// ---------------------------------------------------------------------------------------

/// Whether `text` is drawn inside the window, not merely somewhere in the scene.
fn shows_on_screen(driver: &Driver<FrusApp>, text: &str, width: f32) -> bool {
    driver
        .texts()
        .iter()
        .any(|(t, r)| t == text && r.x >= -0.5 && r.x + r.width <= width + 0.5)
}

/// A page view whose page the application follows in a plain `use_state` — not a
/// restorable one: what comes back is the view's own, and the application hears it.
fn paged_app(restore_id: bool) -> FrusApp {
    let router = GoRouter::new(vec![GoRoute::new("/", move |_, _| {
        component(move |cx| {
            let page = cx.use_state(|| 0usize);
            let (next, follow) = (page.clone(), page.clone());
            let mut view = PageView::new(6, |i| Text::new(format!("Page {i}")))
                .height(200.0)
                .page(page.get())
                .on_page_changed(move |p: usize| follow.set(p));
            if restore_id {
                view = view.restoration_id("pages");
            }
            Box::new(column![
                Text::new(format!("On page {}", page.get())),
                Button::new("Next").on_press(move || next.update(|p| *p += 1)),
                view,
            ]) as Box<dyn Widget>
        })
    })])
    .animate(false);
    FrusApp::router(router).restoration_scope_id("app")
}

/// A `PageView` with a restoration id opens on the page it was left on — on its very first
/// frame, with no page 0 flashing past — and tells the application so through
/// `on_page_changed`. Without the id it opens on page 0, as before.
#[test]
fn a_page_view_opens_on_the_page_it_was_left_on() {
    let mut first = Driver::new(paged_app(true), W, H);
    settle(&mut first);
    for _ in 0..3 {
        tap(&mut first, "Next");
    }
    assert!(shows_on_screen(&first, "Page 3", W), "{:?}", first.texts());
    let data = first.restoration_data().unwrap();
    let kept = bucket(&data);
    let page = kept
        .children()
        .find(|(name, _)| name.starts_with("route 0 "))
        .map(|(_, b)| b.clone())
        .expect("the page has a scope");
    assert_eq!(page.value("pages"), Some("3"), "{page:?}");

    let mut second = Driver::new(paged_app(true), W, H);
    assert!(second.restore(&data));
    second.frame(1.0 / 60.0);
    assert!(
        shows_on_screen(&second, "Page 3", W),
        "the first frame is already on page 3: {:?}",
        second.texts()
    );
    assert!(!shows_on_screen(&second, "Page 0", W));
    settle(&mut second);
    assert!(
        shows_on_screen(&second, "Page 3", W),
        "{:?}",
        second.texts()
    );
    assert!(
        shows(&second, "On page 3"),
        "the application heard the restored page: {:?}",
        second.texts()
    );
    // And it is the page from here on: the view still turns from it.
    tap(&mut second, "Next");
    assert!(
        shows_on_screen(&second, "Page 4", W),
        "{:?}",
        second.texts()
    );
    assert!(shows(&second, "On page 4"));

    // Without an id, nothing of the view's is kept.
    let mut plain = Driver::new(paged_app(false), W, H);
    settle(&mut plain);
    tap(&mut plain, "Next");
    let data = plain.restoration_data().unwrap();
    let mut again = Driver::new(paged_app(false), W, H);
    again.restore(&data);
    settle(&mut again);
    assert!(shows_on_screen(&again, "Page 0", W), "{:?}", again.texts());
}

/// What is kept is the **page**, not the offset: a process started again in a window of
/// another width — the phone turned on its side — opens on the same page.
#[test]
fn a_restored_page_survives_another_window_width() {
    let mut first = Driver::new(paged_app(true), W, H);
    settle(&mut first);
    tap(&mut first, "Next");
    tap(&mut first, "Next");
    let data = first.restoration_data().unwrap();

    let wide = 700.0;
    let mut second = Driver::new(paged_app(true), wide, H);
    assert!(second.restore(&data));
    settle(&mut second);
    assert!(
        shows_on_screen(&second, "Page 2", wide),
        "{:?}",
        second.texts()
    );
    assert!(shows(&second, "On page 2"));
    let (viewport, (x, _)) = second
        .offsets()
        .into_iter()
        .find(|(_, (x, _))| *x > 0.0)
        .expect("the view is scrolled");
    assert!(
        (x - 2.0 * viewport.width).abs() < 0.5,
        "two pages of {} in: {x}",
        viewport.width
    );
}

/// A `ListWheel` with a restoration id opens on the row it rested on, and says so through
/// `on_selected`.
#[test]
fn a_list_wheel_opens_on_its_row() {
    let make = || {
        let router = GoRouter::new(vec![GoRoute::new("/", |_, _| {
            component(|cx| {
                let chosen = cx.use_state(|| 0usize);
                let follow = chosen.clone();
                Box::new(column![
                    Text::new(format!("Chosen {}", chosen.get())),
                    ListWheel::new(60, 40.0, |i| Text::new(format!("{i} min")))
                        .height(200.0)
                        .selected(chosen.get())
                        .on_selected(move |i: usize| follow.set(i))
                        .restoration_id("minutes"),
                ]) as Box<dyn Widget>
            })
        })])
        .animate(false);
        FrusApp::router(router).restoration_scope_id("app")
    };
    let mut first = Driver::new(make(), W, H);
    settle(&mut first);
    // Turn the wheel with a finger: a slow drag up, released without a fling.
    let row = first
        .texts()
        .into_iter()
        .find(|(t, _)| t == "2 min")
        .map(|(_, rect)| rect)
        .expect("the wheel's rows are drawn");
    let at = Point::new(row.x + 10.0, row.y + 5.0);
    first.press(at);
    for step in 1..=30 {
        first.move_to(Point::new(at.x, at.y - step as f32 * 7.0));
        first.frame(1.0 / 60.0);
    }
    for _ in 0..10 {
        first.frame(1.0 / 60.0);
    }
    first.release(Point::new(at.x, at.y - 210.0));
    settle(&mut first);
    let rested = list_offset(&first);
    let row = (rested / 40.0).round() as usize;
    assert!(row > 0, "the wheel turned: {rested}");
    let chosen = format!("Chosen {row}");
    assert!(shows(&first, &chosen), "{:?}", first.texts());
    let data = first.restoration_data().unwrap();

    let mut second = Driver::new(make(), W, H);
    assert!(second.restore(&data));
    settle(&mut second);
    assert!(shows(&second, &chosen), "{:?}", second.texts());
    let offset = list_offset(&second);
    assert!(
        (offset - row as f32 * 40.0).abs() < 0.5,
        "{row} rows of 40 down: {offset}"
    );
}

/// A windowed `GridView` with a restoration id opens scrolled where it was.
#[test]
fn a_grid_opens_where_it_was_scrolled() {
    let make = || {
        let router = GoRouter::new(vec![GoRoute::new("/", |_, _| {
            GridView::builder(3, 300, |i| Text::new(format!("Tile {i}")))
                .width(W)
                .height(500.0)
                .restoration_id("tiles")
        })])
        .animate(false);
        FrusApp::router(router).restoration_scope_id("app")
    };
    let mut first = Driver::new(make(), W, H);
    settle(&mut first);
    let tile = first
        .texts()
        .into_iter()
        .find(|(t, _)| t == "Tile 4")
        .map(|(_, rect)| rect)
        .unwrap_or_else(|| panic!("the grid's tiles are on screen: {:?}", first.texts()));
    let at = Point::new(tile.x + 5.0, tile.y + 5.0);
    first.press(at);
    for step in 1..=10 {
        first.move_to(Point::new(at.x, at.y - step as f32 * 20.0));
        first.frame(1.0 / 60.0);
    }
    first.release(Point::new(at.x, at.y - 200.0));
    settle(&mut first);
    let scrolled = list_offset(&first);
    assert!(scrolled > 100.0, "the grid moved: {scrolled}");
    let data = first.restoration_data().unwrap();

    let mut second = Driver::new(make(), W, H);
    assert!(second.restore(&data));
    settle(&mut second);
    let restored = list_offset(&second);
    assert!(
        (restored - scrolled).abs() < 0.5,
        "the grid opens where it was: {restored} against {scrolled}"
    );
}

/// The widgets that are **told** their state — a tab bar's tab, a tile's open flag — keep
/// it where the application keeps it: a restorable hook brings them back as they were.
#[test]
fn told_widgets_come_back_through_the_application_values() {
    let make = || {
        let router = GoRouter::new(vec![GoRoute::new("/", |_, _| {
            component(|cx| {
                let tab = cx.use_restorable_state("tab", || 0usize);
                let open = cx.use_restorable_state("details open", || false);
                let (select, toggle) = (tab.clone(), open.clone());
                Box::new(column![
                    TabBar::new(tab.get(), move |i: usize| select.set(i))
                        .tab("Inbox", Text::new("Inbox panel"))
                        .tab("Sent", Text::new("Sent panel")),
                    ExpansionTile::new(
                        "Details",
                        open.get(),
                        Callback::new(move || toggle.update(|o| *o = !*o)),
                    )
                    .content(Text::new("The details")),
                ]) as Box<dyn Widget>
            })
        })])
        .animate(false);
        FrusApp::router(router).restoration_scope_id("app")
    };
    let mut first = Driver::new(make(), W, H);
    settle(&mut first);
    tap(&mut first, "Sent");
    tap(&mut first, "Details");
    assert!(shows(&first, "Sent panel") && shows(&first, "The details"));
    let data = first.restoration_data().unwrap();

    let mut second = Driver::new(make(), W, H);
    assert!(second.restore(&data));
    settle(&mut second);
    assert!(shows(&second, "Sent panel"), "{:?}", second.texts());
    assert!(shows(&second, "The details"));
}

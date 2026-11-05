// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! State restoration through the shell (milestone 753): an application is driven — a page
//! pushed, a list scrolled, a count raised, a draft typed — its restoration data taken as the
//! platform would take it, and a **second application**, made afresh as a new process makes
//! it, is handed that data and must open where the first one was.

use crate::app::testing::Driver;
use crate::{Application, FrusApp};
use frus_widgets::{
    column, component, restoration_scope, Button, GoRoute, GoRouter, ListView, Point,
    RestorationBucket, Text, TextField, Widget,
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

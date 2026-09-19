//! `frus-hello` — the smallest complete frus application: a counter.
//!
//! This is the framework's "Hello, world!" and the source of the `cargo generate`
//! template (see `templates/app/`). A widget that has something to remember is a
//! `StatefulWidget`: the widget is its configuration, its `State` is what stays between
//! rebuilds, and a button runs a closure that changes it.
//!
//! Run it on the desktop with `cargo run -p frus-hello`.

#![deny(missing_docs)]

use std::time::Duration;

// A **single** dependency: the `frus` facade supplies everything — framework layer,
// widgets and DSL.
use frus::{
    button, column, row, text, Align, Container, FrusApp, Justify, MediaQuery, Size, State,
    StateContext, StatefulWidget, Variant, Widget,
};

/// The counter: a widget with nothing to configure.
#[derive(Clone)]
struct Counter;

/// What the counter keeps between rebuilds: a plain struct.
#[derive(Default)]
struct CounterState {
    count: i32,
    /// Automatic counting: while it is on, a one-second timer increments the counter. The
    /// same timer runs on the desktop (a thread), on Android and on the Web (`setInterval`) —
    /// the widget only asks for it.
    auto: bool,
}

impl CounterState {
    fn increment(&mut self) {
        self.count += 1;
    }

    fn decrement(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        }
    }

    fn toggle_auto(&mut self) {
        self.auto = !self.auto;
    }
}

impl StatefulWidget for Counter {
    type State = CounterState;

    fn create_state(&self) -> CounterState {
        CounterState::default()
    }
}

impl State for CounterState {
    type Widget = Counter;

    /// `build` describes the interface for the current state. A handler is a closure:
    /// `cx.callback(..)` is `set_state` wrapped up as one.
    ///
    /// It is **not** given the size. The framework installs a description of the surface
    /// around the build, and `MediaQuery::of()` is how anything here asks about it: a
    /// `Scaffold`, an `AppBar` or a `SafeArea` reads it without being told.
    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // A timer is asked for by the build that wants it, and stops as soon as a build stops
        // asking: automatic mode off, no timer left.
        if self.auto {
            cx.use_interval(Duration::from_secs(1), cx.callback(CounterState::increment));
        }

        let Size { width, height } = MediaQuery::of().size;
        let content = column![
            text(format!("{}", self.count)).size(48.0),
            column![
                row![
                    button("+", cx.callback(CounterState::increment)).variant(Variant::Filled),
                    button("−", cx.callback(CounterState::decrement)).variant(Variant::Outlined)
                ]
                .gap(20.0),
                button(
                    if self.auto { "Stop auto" } else { "Start auto" },
                    cx.callback(CounterState::toggle_auto),
                )
                .variant(Variant::Outlined),
            ]
            .gap(8.0)
            .align(Align::Center),
        ]
        .gap(16.0)
        .align(Align::Center);

        // Centred on screen: a full-window Flex that centres its only child on both
        // axes, laid on the theme's background.
        let centered = column![content]
            .width(width)
            .height(height)
            .justify(Justify::Center)
            .align(Align::Center);

        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(cx.theme().background)
                .child(centered),
        )
    }
}

// **A single entry point**: one declaration generates the desktop, Android and Web
// entry points (see `frus::main!`). The thin `src/bin/frus-hello.rs` binary calls the
// `run()` it produces, for the desktop.
frus::main!(FrusApp::stateful(Counter).title("frus — counter"));

#[cfg(test)]
mod tests {
    use super::*;
    use frus::{build_deferred, Runtime, Theme};

    /// The counter's rules are plain Rust: testable with no widget, no GPU and no window.
    #[test]
    fn counting_is_plain_state() {
        let mut state = CounterState::default();
        state.increment();
        state.increment();
        state.decrement();
        assert_eq!(state.count, 1);
        state.decrement();
        state.decrement();
        assert_eq!(state.count, 0, "it does not go below zero");
        state.toggle_auto();
        assert!(state.auto);
    }

    /// The whole widget builds headless, and at rest asks for no timer: only automatic mode
    /// does, and a build that does not ask stops it.
    #[test]
    fn the_widget_builds_and_at_rest_wants_no_timer() {
        let runtime = Runtime::default();
        let counter = Counter.into_widget();
        MediaQuery::new(Size::new(400.0, 800.0)).scope(|| {
            runtime.states.begin_build();
            build_deferred(&counter, &Theme::default(), &runtime);
            runtime.states.end_frame();
        });
        assert!(
            frus::intervals().is_empty(),
            "automatic mode is off: no timer"
        );
        assert_eq!(runtime.states.len(), 1, "and it keeps one state");
    }
}

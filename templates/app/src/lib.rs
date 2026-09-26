//! `{{project-name}}` — a frus application: a counter, built from a component.
//!
//! - Desktop: `cargo run`
//! - Android: `cargo apk run`   (see https://… the getting-started guide)

// A **single** dependency: the `frus` facade provides everything (framework layer +
// widgets + DSL).
use frus::{
    button, column, text, Align, Container, FrusApp, Justify, MediaQuery, Size, State,
    StateContext, StatefulWidget, Variant, Widget,
};

/// The counter: a widget with nothing to configure.
#[derive(Clone)]
struct Counter;

/// What the counter keeps between rebuilds: a plain struct.
#[derive(Default)]
struct CounterState {
    count: i32,
}

impl CounterState {
    fn increment(&mut self) {
        self.count += 1;
    }

    fn decrement(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        } else {
            self.count = 0;
        }
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
        let Size { width, height } = MediaQuery::of().size;
        let content = column![
            text(format!("{}", self.count)).size(48.0),
            row![
                button("+", cx.callback(CounterState::increment)).variant(Variant::Filled),
                button("−", cx.callback(CounterState::decrement)).variant(Variant::Outlined),
            ]
            .gap(8.0)
            .align(Align::Center),
        ]
        .gap(16.0)
        .align(Align::Center);

        // Centred on screen, over the theme's background.
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

// **One entry point** — a single declaration generates the desktop / Android / Web
// entries (see `frus::main!`). The thin `src/bin/{{project-name}}.rs` binary calls the
// `run()` it produces for the desktop.
frus::main!(FrusApp::stateful(Counter).title("{{project-name}}"));

#[cfg(test)]
mod tests {
    use super::*;

    /// The counter's rules are plain Rust: testable with no widget, no GPU and no window.
    #[test]
    fn counting_is_plain_state() {
        let mut state = CounterState::default();
        state.increment();
        state.increment();
        state.decrement();
        assert_eq!(state.count, 1);
    }
}

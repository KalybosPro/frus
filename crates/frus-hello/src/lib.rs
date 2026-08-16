//! `frus-hello` — the smallest complete frus application: a counter.
//!
//! This is the framework's "Hello, world!" and the source of the `cargo generate`
//! template (see `templates/app/`). It fits in one state struct, a pure `update` and a
//! `view` — the whole Elm model.
//!
//! Run it on the desktop with `cargo run -p frus-hello`.

use std::time::Duration;

// A **single** dependency: the `frus` facade supplies everything — framework layer,
// widgets and DSL.
use frus::{
    button, column, row, text, Align, Application, Command, Container, Justify, Subscription,
    Theme, Variant, Widget,
};

/// The application's state: a plain counter.
#[derive(Default)]
struct Counter {
    count: i32,
    /// Automatic counting: when it is on, an `every(1s)` **subscription** increments
    /// the counter. The same continuous source runs on the desktop (a thread), on
    /// Android and on the Web (`setInterval`) — the app knows only
    /// `Subscription::every`.
    auto: bool,
}

/// The messages the interface emits.
#[derive(Clone)]
enum Msg {
    Increment,
    Decrement,
    /// Turns automatic counting on or off.
    ToggleAuto,
    /// A tick from the subscription: increment.
    Tick,
}

impl Application for Counter {
    type Message = Msg;

    /// `update` is **pure**: it advances the state and returns whatever effects there
    /// are — none here. Testable with neither GPU nor window.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Increment | Msg::Tick => self.count += 1,
            Msg::Decrement => {
                if self.count > 0 {
                    self.count -= 1;
                }
            }
            Msg::ToggleAuto => self.auto = !self.auto,
        }
        Command::none()
    }

    /// Subscriptions **from the state**: in automatic mode it emits one `Tick` a
    /// second, otherwise none. The framework starts and stops the source by diffing.
    fn subscription(&self) -> Subscription<Msg> {
        if self.auto {
            Subscription::every(Duration::from_secs(1), |_| Msg::Tick)
        } else {
            Subscription::none()
        }
    }

    /// `view` describes the interface for the current state — a pure function of
    /// `(state, theme, size)`. The framework (re)builds it as needed.
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        let content = column![
            text(format!("{}", self.count)).size(48.0),
            column![
                row![
                    button("+", Msg::Increment).variant(Variant::Filled),
                    button("−", Msg::Decrement).variant(Variant::Outlined)
                ]
                .gap(20.0),
                button(
                    if self.auto { "Stop auto" } else { "Start auto" },
                    Msg::ToggleAuto,
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
                .color(theme.background)
                .child(centered),
        )
    }

    fn title(&self) -> String {
        "frus — counter".to_string()
    }
}

// **A single entry point**: one declaration generates the desktop, Android and Web
// entry points (see `frus::main!`). The thin `src/bin/frus-hello.rs` binary calls the
// `run()` it produces, for the desktop.
frus::main!(Counter::default());

#[cfg(test)]
mod tests {
    use super::*;

    /// The Elm advantage: `update` is tested with neither GPU nor window.
    #[test]
    fn counting_is_pure() {
        let mut app = Counter::default();
        app.update(Msg::Increment);
        app.update(Msg::Increment);
        app.update(Msg::Decrement);
        assert_eq!(app.count, 1);
    }

    /// Automatic mode drives the subscription: absent at rest, present once turned on
    /// — the framework starts and stops it from that diff — and a `Tick` counts.
    #[test]
    fn auto_mode_drives_the_subscription() {
        let mut app = Counter::default();
        assert!(app.subscription().is_empty(), "at rest: no subscription");
        app.update(Msg::ToggleAuto);
        assert!(
            !app.subscription().is_empty(),
            "automatic: one every(1s) source"
        );
        app.update(Msg::Tick);
        assert_eq!(app.count, 1, "a tick increments");
        app.update(Msg::ToggleAuto);
        assert!(
            app.subscription().is_empty(),
            "automatic off: no subscription left"
        );
    }
}

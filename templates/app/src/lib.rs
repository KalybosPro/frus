//! `{{project-name}}` — a frus application (the Elm model: state, `update`, `view`).
//!
//! - Desktop: `cargo run`
//! - Android: `cargo apk run`   (see https://… the getting-started guide)

// A **single** dependency: the `frus` facade provides everything (framework layer +
// widgets + DSL).
use frus::{button, column, text, Align, Application, Command, Container, Justify, Theme, Variant, Widget};

/// The application state: a simple counter.
#[derive(Default)]
struct App {
    count: i32,
}

/// The messages the interface emits.
#[derive(Clone)]
enum Msg {
    Increment,
    Decrement,
}

impl Application for App {
    type Message = Msg;

    /// `update` is **pure**: it advances the state and returns any effects
    /// (none here). Testable with no GPU and no window.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Increment => self.count += 1,
            Msg::Decrement => self.count -= 1,
        }
        Command::none()
    }

    /// `view` describes the interface for the current state — a pure function of
    /// `(state, theme, size)`.
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        let content = column![
            text(format!("{}", self.count)).size(48.0),
            column![
                button("+", Msg::Increment).variant(Variant::Primary),
                button("−", Msg::Decrement).variant(Variant::Secondary),
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
                .color(theme.background)
                .child(centered),
        )
    }

    fn title(&self) -> String {
        "{{project-name}}".to_string()
    }
}

// **One entry point** — a single declaration generates the desktop / Android / Web
// entries (see `frus::main!`). The thin `src/bin/{{project-name}}.rs` binary calls the
// `run()` it produces for the desktop.
frus::main!(App::default());

#[cfg(test)]
mod tests {
    use super::*;

    /// The Elm advantage: `update` is testable with no GPU and no window.
    #[test]
    fn counting_is_pure() {
        let mut app = App::default();
        app.update(Msg::Increment);
        app.update(Msg::Increment);
        app.update(Msg::Decrement);
        assert_eq!(app.count, 1);
    }
}

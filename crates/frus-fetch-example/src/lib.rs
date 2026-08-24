//! `frus-fetch-example` — the **end-to-end networking chain** in one screen: a button
//! starts a GET and the screen moves through **loading → data**, or **error**.
//!
//! It is the way frus renders an in-flight future, and the demonstration of the stack
//! added in milestones 270–272:
//!
//! - [`frus::Command::perform_async`] drives a future to completion and its value
//!   becomes a message;
//! - [`frus::fetch`] and [`frus::Request`] make the HTTP round trip, here with headers
//!   and a timeout.
//!
//! It all fits in the Elm model: a **pure** `update` — the one impurity, the network, is
//! banished into a `Command` — and a `view` that displays nothing but the current state.
//!
//! Run it on the desktop with `cargo run -p frus-fetch-example`; add `RUST_LOG=info` for
//! logs.

use std::time::Duration;

// A **single** dependency: the `frus` facade, with the `net` feature for `fetch` and
// `Request`.
use frus::{
    button, column, text, Align, Application, Color, Command, Container, Justify, MediaQuery,
    RemoteData, Request, Size, Theme, Variant, Widget,
};

/// The API queried: a joke returned as **plain text**, through an `Accept: text/plain`
/// header. It allows browser requests (CORS), so the example works on the Web too.
const JOKE_URL: &str = "https://icanhazdadjoke.com/";

/// The state: the request's status, expressed through the framework's [`RemoteData`]
/// idiom (`NotAsked → Loading → Success | Failure`) rather than a hand-rolled state
/// machine.
#[derive(Default)]
struct FetchDemo {
    joke: RemoteData<String>,
}

/// The messages the interface and the network effect emit.
#[derive(Clone)]
enum Msg {
    /// The user asked for a load.
    Fetch,
    /// The network effect finished: `Ok(body)` or `Err(message)`.
    Got(Result<String, String>),
}

impl Application for FetchDemo {
    type Message = Msg;

    /// `update` stays **pure**: it advances the state and, for `Fetch`, returns the
    /// network **effect**, the one impurity. When the future resolves, the shell calls
    /// `update` again with `Got(...)`. No `await` and no GPU here, so it is testable as
    /// it stands.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Fetch => {
                self.joke = RemoteData::Loading;
                // A GET with a header, asking for plain text, and a timeout: if the API
                // does not answer within 5 s we get a `FetchError::Network`, which lands
                // in the `Failure` branch.
                return Command::perform_async(async {
                    let res = Request::get(JOKE_URL)
                        .header("Accept", "text/plain")
                        .header("User-Agent", "frus-fetch-example (github.com/frus)")
                        .timeout(Duration::from_secs(5))
                        .send()
                        .await;
                    Msg::Got(res.map_err(|err| err.to_string()))
                });
            }
            // The effect's `Result` becomes a `RemoteData` directly, the body trimmed on the way.
            Msg::Got(res) => {
                self.joke = RemoteData::from_result(res.map(|body| body.trim().to_string()))
            }
        }
        Command::none()
    }

    /// `view` does nothing but paint the state: a button, then the current status.
    fn view(&self, theme: &Theme) -> Box<dyn Widget<Msg>> {
        // The window, from the description the framework installed around this call.
        let Size { width, height } = MediaQuery::of().size;
        // The button's label follows the state; it can be fired again afterwards.
        let label = match self.joke {
            RemoteData::NotAsked => "Get a joke",
            RemoteData::Loading => "Loading…",
            _ => "Get another joke",
        };

        // The result area: we **fold** over `RemoteData`'s four cases.
        let result: Box<dyn Widget<Msg>> = match self.joke.as_ref() {
            RemoteData::NotAsked => Box::new(text("Press the button to fetch a joke.").size(18.0)),
            RemoteData::Loading => Box::new(text("Loading…").size(18.0)),
            RemoteData::Success(body) => Box::new(text(body.clone()).size(22.0)),
            RemoteData::Failure(err) => Box::new(
                text(format!("Failed: {err}"))
                    .size(18.0)
                    .color(Color::rgb(0.85, 0.2, 0.2)),
            ),
        };

        let content = column![
            text("frus · fetch").size(14.0).color(theme.muted),
            button(label, Msg::Fetch).variant(Variant::Filled),
            Container::new().width(width.min(420.0)).child(result),
        ]
        .gap(20.0)
        .align(Align::Center);

        // Centred full-screen, on the theme's background.
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
        "frus — fetch".to_string()
    }
}

// **A single entry point**: it generates the desktop, Android and Web entries.
frus::main!(FetchDemo::default());

#[cfg(test)]
mod tests {
    use super::*;

    /// `Fetch` switches to loading **and** returns an effect, the network future. With
    /// neither network nor GPU in the test, only the intent is observed.
    #[test]
    fn fetch_enters_loading_and_emits_an_effect() {
        let mut app = FetchDemo::default();
        assert_eq!(app.joke, RemoteData::NotAsked);
        let cmd = app.update(Msg::Fetch);
        assert!(app.joke.is_loading());
        assert!(!cmd.is_empty(), "Fetch must produce a network effect");
    }

    /// Resolving the effect paints the state: success → the trimmed data, failure → the error.
    #[test]
    fn result_messages_drive_the_state() {
        let mut app = FetchDemo::default();

        app.update(Msg::Got(Ok("  a good joke  ".to_string())));
        assert_eq!(app.joke.value().map(String::as_str), Some("a good joke"));

        app.update(Msg::Got(Err("HTTP status 500".to_string())));
        assert_eq!(
            app.joke.error().map(String::as_str),
            Some("HTTP status 500")
        );
    }
}

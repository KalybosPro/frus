//! Tests for `websocket::features::chat::controllers::socket`, moved out of `src/` (milestone 662).

use std::sync::mpsc;
use std::time::Duration;
use websocket::features::chat::controllers::connection::{Event, Outgoing};
use websocket::features::chat::controllers::socket::*;

const WAIT: Duration = Duration::from_secs(15);

/// The whole round trip against the public echo server. Ignored by default because
/// it needs the network: `cargo test -- --ignored`.
#[test]
#[ignore = "needs the network"]
fn the_echo_server_greets_echoes_and_closes() {
    let (requests_out, requests) = mpsc::channel();
    let (events, events_in) = mpsc::channel();
    let thread = std::thread::spawn(move || run("wss://echo.websocket.org", requests, events));

    assert_eq!(events_in.recv_timeout(WAIT), Ok(Event::Connected));
    let greeting = events_in.recv_timeout(WAIT);
    assert!(matches!(greeting, Ok(Event::Received(_))), "{greeting:?}");

    requests_out
        .send(Outgoing::Text("hello from frus".into()))
        .expect("the socket thread is listening");
    assert_eq!(
        events_in.recv_timeout(WAIT),
        Ok(Event::Received("hello from frus".into()))
    );

    requests_out
        .send(Outgoing::Close)
        .expect("the socket thread is listening");
    assert_eq!(events_in.recv_timeout(WAIT), Ok(Event::Closed(None)));
    thread.join().expect("the socket thread ends after closing");
}

#[test]
fn an_address_that_is_not_a_websocket_fails_without_the_network() {
    let (_requests_out, requests) = mpsc::channel();
    let (events, events_in) = mpsc::channel();
    run("https://example.test", requests, events);
    assert!(matches!(events_in.recv(), Ok(Event::Failed(_))));
}

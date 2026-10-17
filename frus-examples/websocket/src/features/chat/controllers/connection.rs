//! [`Connection`]: one WebSocket, as the page sees it.
//!
//! The page never touches the socket. It holds a `Connection`, which it can only
//! [`send`](Connection::send) through or [`close`](Connection::close), and it is told
//! what happens through the `on_event` function it gave to [`Connection::open`]. That
//! function runs **on the interface's thread**, so it may change state like any handler.
//!
//! Natively (desktop, Android), underneath, three pieces cooperate (on the Web, see
//! `browser.rs`):
//!
//! ```text
//!   page ──send/close──▶ mpsc ──▶ socket thread ──▶ mpsc ──▶ pump (host::spawn) ──▶ on_event
//!                                (blocking socket)                (interface thread)
//! ```
//!
//! - the **socket thread** (`socket.rs`) owns the socket and blocks on it, which is what a
//!   synchronous client does; nothing on the interface's thread ever waits for the network;
//! - two **channels** carry text out and events back: `std::sync::mpsc` is enough, both
//!   ends are `Send`;
//! - the **pump** brings events back to the interface's thread with `frus::host::spawn`:
//!   its work (on another thread) waits for the next events, and its answer (on the
//!   interface's thread) hands them to `on_event`, then starts the next wait. The shell
//!   wakes the window when an answer arrives, so there is no timer polling for messages.
//!
//! Closing is the page's job, and it is one call: [`Connection::close`], or dropping the
//! `Connection` — the socket thread sees its channel close either way, says goodbye to the
//! server, and ends; then the pump finds no one left to listen to, and ends too.

#[cfg(not(target_arch = "wasm32"))]
use std::rc::Rc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver, Sender};

#[cfg(not(target_arch = "wasm32"))]
use frus::host;

// On the Web the same API is the browser's own `WebSocket`: see `browser.rs`.
#[cfg(target_arch = "wasm32")]
pub use super::browser::{Connection, Watch};

/// What a connection reports, in the order it happens.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The handshake succeeded: messages can go both ways.
    Connected,
    /// The server sent a message.
    Received(String),
    /// The connection ended cleanly — the server closed it, or we did. Carries the
    /// server's reason, if it gave one.
    Closed(Option<String>),
    /// The connection could not be made, or broke. Nothing more will come from it.
    Failed(String),
}

#[cfg(not(target_arch = "wasm32"))]
/// What the page asks of the socket thread.
pub enum Outgoing {
    Text(String),
    Close,
}

#[cfg(not(target_arch = "wasm32"))]
/// A live connection: a way to talk to the socket thread. Cheap to hold in state.
pub struct Connection {
    outgoing: Sender<Outgoing>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Connection {
    /// Starts connecting to `url` (`ws://` or `wss://`) and returns at once; the outcome
    /// arrives as an [`Event`] through `on_event`, on the interface's thread.
    pub fn open(url: &str, on_event: impl Fn(Event) + 'static) -> Self {
        let (outgoing, requests) = mpsc::channel();
        let (events_out, events_in) = mpsc::channel();
        start(url.to_owned(), requests, events_out);
        pump(events_in, Rc::new(on_event));
        Self { outgoing }
    }

    /// Queues `text` for the server. If the socket is already gone this does nothing: its
    /// `Failed` or `Closed` event is on its way and says why.
    pub fn send(&self, text: String) {
        let _ = self.outgoing.send(Outgoing::Text(text));
    }

    /// Asks the socket thread to close the connection politely and stop. Dropping the
    /// `Connection` does the same.
    pub fn close(&self) {
        let _ = self.outgoing.send(Outgoing::Close);
    }
}

/// Starts the socket thread.
#[cfg(not(target_arch = "wasm32"))]
fn start(url: String, requests: Receiver<Outgoing>, events: Sender<Event>) {
    let report = events.clone();
    let spawned = std::thread::Builder::new()
        .name("websocket".into())
        .spawn(move || super::socket::run(&url, requests, events));
    if let Err(error) = spawned {
        let _ = report.send(Event::Failed(format!(
            "Could not start the socket thread: {error}"
        )));
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Brings events from the socket thread to the interface's thread, as long as the socket
/// thread is there to send them.
///
/// `host::spawn(work, then)` runs `work` on a thread of its own and `then` with its result
/// on the interface's thread. Here `work` blocks until at least one event is ready and
/// takes every other one already waiting; `then` hands them over in order, and starts the
/// next wait. The receiver travels with the result, from one wait to the next.
fn pump(events: Receiver<Event>, on_event: Rc<dyn Fn(Event)>) {
    host::spawn(
        move || {
            // `recv` fails only when the socket thread has ended and everything it sent
            // has been taken: an empty batch, which ends the pump.
            let batch: Vec<Event> = events.recv().into_iter().chain(events.try_iter()).collect();
            (events, batch)
        },
        move |(events, batch): (Receiver<Event>, Vec<Event>)| {
            if batch.is_empty() {
                return;
            }
            for event in batch {
                on_event(event);
            }
            pump(events, on_event.clone());
        },
    );
}

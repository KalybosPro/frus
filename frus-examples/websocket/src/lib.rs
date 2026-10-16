//! `websocket` — a live, long-lived connection: an echo chat over a WebSocket.
//!
//! - Desktop: `cargo run` (or `cargo run -- wss://another.server` to start elsewhere)
//! - Android: `cargo apk run --lib`
//! - Web: see README.md ("The Web")
//!
//! frus draws the interface; the network is any Rust crate you like. Natively this example
//! brings in `tungstenite` and runs the socket on a thread of its own; on the Web the same
//! `Connection` is the browser's `WebSocket` (`controllers/browser.rs`). It shows the three
//! things every long-lived connection needs:
//!
//! 1. **values arriving over time** reach the page —
//!    `features/chat/controllers/connection.rs` (the pump, with `frus::host::spawn`);
//! 2. **the page sends** — the same file, `Connection::send`, an `mpsc` channel to the
//!    socket thread (`controllers/socket.rs`);
//! 3. **cleanup** — the connection opens in the page's `init_state` and closes in its
//!    `dispose` (`features/chat/pages/chatpage.rs`).
//!
//! And around them: a status that is always visible, and automatic reconnection with
//! backoff (`controllers/reconnect.rs`, `controllers/chat_controller.rs`), all driven by
//! a state machine with no network in it (`features/chat/states/chatstate.rs`), tested
//! without one.

use frus::FrusApp;

use crate::features::chat::pages::chatpage::ChatPage;

mod common;
mod features;
mod models;

/// A public echo server: it greets each new connection, then sends back every message.
const DEFAULT_SERVER: &str = "wss://echo.websocket.org";

/// The server to start with: the first `ws://` or `wss://` argument, else the default.
fn start_server() -> String {
    std::env::args()
        .skip(1)
        .find(|arg| arg.starts_with("ws://") || arg.starts_with("wss://"))
        .unwrap_or_else(|| DEFAULT_SERVER.to_string())
}

frus::main!(FrusApp::stateful(ChatPage::new(start_server())).title("WebSocket"));

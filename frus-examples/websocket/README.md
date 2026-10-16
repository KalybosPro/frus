# websocket

A live, long-lived connection: an echo chat over a WebSocket. Values arrive over time and
land in the page, the page sends, the connection retries by itself after an error, and it
closes when the page goes away.

frus has no WebSocket client of its own; it does not need one. This example brings in
[`tungstenite`](https://crates.io/crates/tungstenite) (synchronous, with `rustls` and the
Mozilla root certificates compiled in, so `wss://` works on Android with no system
OpenSSL) and runs it on a thread of its own. Any other client fits the same shape.

## Run it

```sh
cargo run                                  # desktop, against wss://echo.websocket.org
cargo run -- wss://another.server/path     # start with another server
cargo apk run --lib                        # Android
# Web: see below
cargo test                                 # the state machine, the backoff, the model
cargo test -- --ignored                    # a real round trip to the echo server (network)
```

The echo server greets each new connection ("Request served by …") and sends back every
message. The address can be changed from the gear icon; a new address reconnects at once.


## The Web

On the Web the same `Connection` API is the browser's own `WebSocket`
(`src/features/chat/controllers/browser.rs`, through `web-sys`): no thread, no TLS crate.
The browser calls back on the page's only thread, so `onmessage` hands each event straight
to the page.

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg     target/wasm32-unknown-unknown/web-release/websocket.wasm
# then serve web/ with any static server, e.g.:
python -m http.server --directory web 8000
```

The `wasm-bindgen` CLI must be the exact version `Cargo.lock` names for the
`wasm-bindgen` crate (0.2.127 here): `cargo install wasm-bindgen-cli --version 0.2.127`.
The native-only crates (`tungstenite`, `rustls`) are under
`[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` and never reach the wasm build.

## What to read

| File | What it shows |
|---|---|
| `src/features/chat/controllers/connection.rs` | `Connection`: `open(url, on_event)`, `send`, `close`; how events cross from the socket thread to the interface's thread (`host::spawn`, no polling timer) |
| `src/features/chat/controllers/browser.rs` | the Web's `Connection`: the browser's `WebSocket`, its handlers, and a `Watch` future that keeps frames coming while it is open |
| `src/features/chat/controllers/socket.rs` | the socket thread: connect with a timeout, one loop that sends and reads, a polite close |
| `src/features/chat/controllers/chat_controller.rs` | the glue: opening a connection from the state, routing its events in through a `StateHandle`, scheduling a retry with `host::after` |
| `src/features/chat/controllers/reconnect.rs` | the backoff schedule: 1 s, 2 s, 4 s … up to 30 s |
| `src/features/chat/states/chatstate.rs` | every rule: status transitions, sending, ignoring a closed socket's late events, when to retry — unit-tested with made-up events |
| `src/features/chat/pages/chatpage.rs` | the page: opens the connection in `init_state`, closes it in `dispose`, builds the strip, the conversation and the composer |
| `src/features/chat/widgets/` | the status pill, a message bubble, the composer, the server settings |
| `src/models/message.rs` | a message: who said it, what, and when |

## The ideas

- **The network never runs on the interface's thread.** The socket thread owns the socket
  and blocks on it. The page holds a `Connection`: two channels, one each way.
- **Events come back as handler calls.** `host::spawn(work, then)` runs `work` on another
  thread and `then` on the interface's thread. The pump's `work` waits for events, its `then`
  hands them to the page and starts the next wait. The shell wakes the window when they
  arrive, so nothing polls.
- **State changes from outside `build` go through a `StateHandle`.** `cx.handle()` gives
  one; `set_state` changes the state and asks for a rebuild, and does nothing once the page
  has left the tree, so a late event is harmless.
- **The connection lives as long as the page.** `init_state` opens it, `dispose` closes
  it. Dropping a `Connection` closes it too.
- **Sessions guard against stale events.** Every connect and disconnect bumps a number;
  events and retries carry the number they started under, and an old number is ignored.
  A retry timer cannot be cancelled, so it checks when it fires.
- **Backoff after a failure, not after a clean close.** Failures in a row double the wait
  up to a ceiling; a successful connection resets it. Disconnect stops retrying.

## Building against the published crate

Outside the frus checkout, replace the path dependency with `frus = "0.3"`.

pub mod chat_controller;
pub mod connection;
// The Web's `Connection`: the browser's own WebSocket, behind the same API.
#[cfg(target_arch = "wasm32")]
pub mod browser;
pub mod reconnect;
// The socket itself needs threads and TCP, which the Web does not give a wasm module.
#[cfg(not(target_arch = "wasm32"))]
pub mod socket;

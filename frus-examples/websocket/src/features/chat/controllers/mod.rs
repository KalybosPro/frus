pub(crate) mod chat_controller;
pub(crate) mod connection;
// The Web's `Connection`: the browser's own WebSocket, behind the same API.
#[cfg(target_arch = "wasm32")]
pub(crate) mod browser;
pub(crate) mod reconnect;
// The socket itself needs threads and TCP, which the Web does not give a wasm module.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod socket;

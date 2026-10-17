//! `http` — talking to an HTTP API from a frus app, the same code on the desktop, Android
//! and the Web.
//!
//! - Desktop: `cargo run` (or `cargo run -- /posts/1` to open a post directly)
//! - Android: `cargo apk run --lib`
//!
//! A client for the public test API at <https://jsonplaceholder.typicode.com>: a list of
//! posts, a post with its author and comments, and a form that creates a post.
//!
//! What to look at:
//!
//! - `common/api.rs` — the client: base URL, headers, timeout, one typed `async fn` per
//!   call (`frus::Request` + `send_json` / `json_body`), and `ApiError`, the failures in
//!   words a user can act on;
//! - `common/remote.rs` — `use_api`, the hook that runs a call from a `build` with
//!   `cx.use_future` and hands the page a `RemoteData` (loading / data / error);
//! - `features/posts/` — loading, error + Retry, pull-to-refresh;
//! - `features/post/` — two requests in parallel, and a third (`FutureBuilder`) that
//!   waits for the first;
//! - `features/new_post/` — a POST the user starts: a send in progress, its answer, its
//!   failure;
//! - `models/` — the JSON payloads as plain structs, with serde.

use frus::FrusApp;

use crate::common::routers::router;

pub mod common;
pub mod features;
pub mod models;

// A router app: each page is a route, and `cx.router()` reaches the router from any
// page.
frus::main!(FrusApp::router(router()).title("Posts"));

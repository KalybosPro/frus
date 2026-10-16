//! `authentication` — an application with sign-in: an auth service shared by the pages,
//! route guards, and a session that survives a restart.
//!
//! - Desktop: `cargo run`  (or `cargo run -- /profile` to watch the guard at work)
//! - Android: `cargo apk run --lib`
//!
//! What to look at:
//!
//! - `common/auth.rs` — the service: who is signed in (a `ValueNotifier`), and the
//!   requests that change it, answered a moment later like a network would;
//! - `common/routers.rs` — the router: `redirect` runs the guard on every navigation,
//!   `refresh_listenable` runs it again when the service changes, and `?from=` brings the
//!   person back where they were going;
//! - `features/auth/` — the login, register and password-reset pages; their states hold
//!   the forms' rules (`frus::form`), tested without a window; `controllers/user_store.rs`
//!   is the pretend server;
//! - `common/session.rs` — remembering the session on disk.

use frus::{FrusApp, Theme};

use crate::common::{auth::Auth, colors::brand, routers::router};

mod common;
mod features;
mod models;

frus::main!({
    // One service for the whole application: the router and every page get a handle on it.
    let auth = Auth::new();
    FrusApp::router(router(auth))
        .title("frus auth")
        .theme(Theme::from_seed(brand(), false))
        .dark_theme(Theme::from_seed(brand(), true))
        .window_size(440.0, 760.0)
});

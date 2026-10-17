//! `navigation` — moving between screens: a small shop driven by a router.
//!
//! - Desktop: `cargo run`, or `cargo run -- /products/3` to open it at a location
//! - Android: `cargo apk run --lib`
//!
//! What to look at:
//!
//! - `common/routes.rs` — every path and route name, in one place;
//! - `common/routers.rs` — the routes and their pages, nested routes with a path
//!   parameter, the page for an unknown location, and the checkout guard (a redirect,
//!   rerun by a refresh listenable);
//! - `common/shell.rs` — the sections, switched with `go`: a bottom bar on a phone, a
//!   rail on a wide window, and a drawer of links;
//! - `common/cart.rs` — the state every page shares, made once below;
//! - the pages under `features/` — `push` vs `go` vs `replace` vs `pop`, named routes,
//!   query parameters, `extra`, and a page that returns a result.

use frus::FrusApp;

use crate::common::{cart::Cart, routers::router};

pub mod common;
pub mod features;
pub mod models;

frus::main!({
    // **One cart for the whole app**, handed to the router, which hands it to the pages.
    let cart = Cart::new();
    // `FrusApp::router` makes the router the root, and reachable from every page with
    // `cx.router()`. On the web it also ties the router to the address bar.
    FrusApp::router(router(cart)).title("Corner Shop")
});

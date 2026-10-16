//! `hello_world` — the smallest frus application, laid out the way a real one is.
//!
//! - Desktop: `cargo run`
//! - Android: `cargo apk run --lib`
//!
//! A frus application is a root widget handed to `frus::main!`. Here it is
//! `HomePage`, a `StatelessWidget`: it has nothing to remember, so it only says what it
//! builds.

// A **single** dependency: the `frus` facade provides everything (framework layer +
// widgets).
use frus::FrusApp;

use crate::features::home::pages::homepage::HomePage;

mod common;
mod features;

// **One entry point** — a single declaration generates the desktop / Android / Web
// entries. The thin `src/bin/hello_world.rs` binary calls the `run()` it produces.
frus::main!(FrusApp::new(HomePage).title("Hello, world"));

//! `{{project-name}}` — a frus application: a counter, built from a component.
//!
//! The same code runs on every platform; README.md says how to start it on each one.
//!
//! - Desktop: `cargo run`
//! - Android: `cargo apk run --lib`
//! - Web: see "Web" in README.md
//!
//! The application's code lives in modules here, and its tests in `tests/`, beside `src/`.

// A **single** dependency: the `frus` facade provides everything (framework layer +
// widgets + DSL).
use frus::FrusApp;

pub mod counter;

pub use counter::{Counter, CounterState};

// **One entry point** — a single declaration generates the desktop / Android / Web
// entries (see `frus::main!`). The thin `src/bin/{{project-name}}.rs` binary calls the
// `run()` it produces for the desktop.
frus::main!(FrusApp::stateful(Counter).title("{{project-name}}"));

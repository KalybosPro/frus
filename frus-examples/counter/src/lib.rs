//! `counter` — a page that remembers something: a `StatefulWidget` and its `State`.
//!
//! - Desktop: `cargo run`
//! - Android: `cargo apk run --lib`
//!
//! What to look at:
//!
//! - `features/counter/pages/counterpage.rs` — the widget (its configuration) and the
//!   `build` of its state;
//! - `features/counter/states/counterstate.rs` — the state: a plain struct whose methods
//!   are the only way it changes, tested without a window;
//! - `features/counter/widgets/` — a piece of the page pulled out into its own
//!   `StatelessWidget`, configured by its parent.

use frus::FrusApp;

use crate::features::counter::pages::counterpage::CounterPage;

mod common;
mod features;

// `FrusApp::stateful` takes a root that has state. The widget is cloned for every
// rebuild, which is why `CounterPage` derives `Clone`.
frus::main!(FrusApp::stateful(CounterPage::new(1)).title("Counter"));

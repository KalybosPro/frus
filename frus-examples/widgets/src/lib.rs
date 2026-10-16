//! `widgets` — a gallery of frus widgets: every widget with a live example and the code
//! that builds it.
//!
//! - Desktop: `cargo run`
//! - Android: `cargo apk run --lib`
//!
//! Each family of widgets is a feature folder (`src/features/<family>/widgets/`), and each
//! widget is **one file** there: a `pub(crate) fn example() -> Box<dyn Widget>` that is
//! the whole demo. The gallery shows that same file in the demo's "Code" tab, so what
//! you read is exactly what runs.

use frus::FrusApp;

use crate::common::routers::router;

mod common;
mod features;
mod models;

frus::main!(FrusApp::router(router()).title("frus widgets"));

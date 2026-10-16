//! `forge` — a frus application: a counter, built from a component.
//!
//! - Desktop: `cargo run`
//! - Android: `cargo apk run`   (see https://… the getting-started guide)

// A **single** dependency: the `frus` facade provides everything (framework layer +
// widgets + DSL).
use frus::FrusApp;

use crate::common::{auth::Auth, routers::router};

mod common;
mod features;
mod models;

// **One entry point** — a single declaration generates the desktop / Android / Web
// entries (see `frus::main!`). The thin `src/bin/forge.rs` binary calls the
// `run()` it produces for the desktop.

frus::main!({
    frus::fonts::add_font(include_bytes!("../assets/fonts/Inter-Bold.ttf").to_vec());
    frus::fonts::add_font(include_bytes!("../assets/fonts/Inter-Medium.ttf").to_vec());
    frus::fonts::add_font(include_bytes!("../assets/fonts/Inter-Regular.ttf").to_vec());
    frus::fonts::add_font(include_bytes!("../assets/fonts/Inter-SemiBold.ttf").to_vec());
    // The family name stored *inside* these TTFs (name ID 16), not the file name.
    frus::fonts::set_default_family("Inter");
    FrusApp::router(router(Auth::new())).title("Forge")
    //     .builder(move |cx, child| window(cx, child)
    // )
});

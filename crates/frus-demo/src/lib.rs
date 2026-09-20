//! Sample application: a **to-do list** written with frus, as an **external consumer** of the
//! framework — with the framework's own vocabulary: `StatelessWidget`, `StatefulWidget` and
//! `State`, a router that maps a location to a screen, and `FrusApp` to run it.
//!
//! Two entry points for the same code:
//! - desktop: `cargo run -p frus-demo` → the `src/bin/frus-demo.rs` binary → `run()`;
//! - Android: the `cdylib` library exposes `android_main`, called by the native activity.
//!
//! Where things are kept:
//! - **a screen's own state** lives in its `State` — the filter of the list, the value of a
//!   setting, the step of the wizard;
//! - **what more than one screen needs** — the tasks, how the demo is dressed, the
//!   notifications — lives in `Demo` (the `demo` module), handed to the screens that ask for it;
//! - **where the reader is** belongs to the router.

#![deny(missing_docs)]

mod assets;
mod demo;
mod l10n;
mod model;
mod parts;
mod prelude;
mod screens;
/// The tool that renders the README's pictures; see the module's own documentation.
#[cfg(feature = "shots")]
pub mod shots;
mod storage;
#[cfg(test)]
mod tests;
mod theme;

use crate::l10n::LANGS;
use crate::prelude::*;
use frus_shell::FrusApp;
use frus_widgets::Locale;

/// The demo application: its routes, and how it is dressed at the start.
///
/// Public so that the tool that renders the README's pictures and the tests can build the same
/// application the entry point runs.
pub fn app() -> FrusApp {
    build().0
}

/// The application, and the two handles a test or a tool wants beside it: the router it moves
/// with, and the state its screens share.
pub(crate) fn build() -> (FrusApp, GoRouter, Rc<Demo>) {
    let demo = Rc::new(Demo::default());
    let router = screens::router(demo.clone());
    let (save, restore, start) = (demo.clone(), demo.clone(), demo.clone());
    let app = FrusApp::router(router.clone())
        .title("frus — Todo")
        .window_size(900.0, 680.0)
        // **The languages this demonstration has**, best first — the three it embeds as
        // Fluent resources. The framework resolves the device's list against these.
        .supported_locales(LANGS.iter().map(|(_, tag)| Locale::new(*tag)).collect())
        // Here rather than in `main`, because there are three entry points (desktop,
        // Android, web) and only one of them is a `main`.
        .on_start(move || {
            // **The licences of everything this binary links**, generated from its own
            // dependency graph by `scripts/gen_licenses.py` and embedded. One call, and the
            // list cannot drift from what is linked without the file changing — which is the
            // one failure mode that matters for a licence list.
            frus_widgets::licenses::add_all(include_str!("../assets/licenses.txt"));
            // Loads the persisted tasks at start-up.
            start.start();
        })
        // Live-reload: the essentials of the state survive a recompilation — the tasks, the
        // theme (light/dark + seed).
        .persist(
            move || Some(save.snapshot()),
            move |bytes| restore.restore(bytes),
        );
    // `FrusApp` starts from the defaults; how the demo is dressed is `Demo`'s to say, and it
    // says it after, so the builders above cannot undo it.
    demo.apply();
    (app, router, demo)
}

// A **single** entry point: one declaration generates both the desktop entry (`run()`,
// called by the binary) and the Android one (`android_main`). See `frus_shell::main!`.
frus_shell::main!(app());

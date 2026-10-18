//! **Hot reload**: the application's code patched while it runs, its state kept.
//!
//! `frus run` builds a patch each time a source file changes (the subsecond engine) and sends
//! it to the application, which connects to the patch server at start-up when it was built
//! with the `hot-reload` feature. A patch replaces the functions that changed; each
//! component's `build` is called through the patch table (see `frus_widgets::component`), so
//! the next build of the interface runs the new code, with every component's state where it
//! was.
//!
//! What a patch cannot do is change the **shape** of a state — a field added to a struct that
//! is alive in memory. That needs a restart (`R`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use winit::window::Window;

/// A patch arrived since the last frame looked.
static PATCHED: AtomicBool = AtomicBool::new(false);
/// The window to wake when one does: a patch arrives on another thread, while the event loop
/// may be asleep.
static WINDOW: Mutex<Option<Arc<Window>>> = Mutex::new(None);

/// Connects to the patch server `frus run` started, if it did. Once, at start-up.
pub(crate) fn connect() {
    dioxus_devtools::subsecond::register_handler(Arc::new(|| {
        PATCHED.store(true, Ordering::SeqCst);
        log::info!("hot reload: patch applied");
        if let Ok(window) = WINDOW.lock() {
            if let Some(window) = window.as_ref() {
                window.request_redraw();
            }
        }
    }));
    dioxus_devtools::connect_subsecond();
}

/// The window a patch wakes.
pub(crate) fn set_window(window: Arc<Window>) {
    if let Ok(mut slot) = WINDOW.lock() {
        *slot = Some(window);
    }
}

/// Whether a patch arrived since the last call: the interface is then built again.
pub(crate) fn take_patched() -> bool {
    PATCHED.swap(false, Ordering::SeqCst)
}

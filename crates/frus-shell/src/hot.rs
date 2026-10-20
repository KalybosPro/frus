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
    #[cfg(target_os = "android")]
    engine_settings();
    // A patch is linked against the process it was made for — its calls into the rest of the
    // program are that process's addresses — so the engine's own client, which applies only a
    // patch addressed to this process, is the one used.
    #[cfg(not(target_arch = "wasm32"))]
    dioxus_devtools::connect_subsecond();
    #[cfg(target_arch = "wasm32")]
    connect_page();
}

/// On the web, `frus run --hot` hands each patch to the page (milestone 679), which calls
/// `window.__frusPatch` with it: defined here. Each patch is a module the page fetches, compiles
/// and links into the running one.
#[cfg(target_arch = "wasm32")]
fn connect_page() {
    use wasm_bindgen::closure::Closure;

    let Some(window) = web_sys::window() else {
        return;
    };
    let apply = Closure::<dyn FnMut(String)>::new(apply_message);
    let _ = js_sys::Reflect::set(&window, &"__frusPatch".into(), apply.as_ref());
    apply.forget();
}

/// One message of the engine's, as `frus run` passed it on: a patch is applied, anything else
/// left alone.
#[cfg(target_arch = "wasm32")]
fn apply_message(text: String) {
    use dioxus_devtools::DevserverMsg;

    // The messages borrow `&'static str`s from the text they are read from.
    let text: &'static str = Box::leak(text.into_boxed_str());
    let Ok(DevserverMsg::HotReload(message)) = serde_json::from_str::<DevserverMsg>(text) else {
        return;
    };
    // The page's build is the engine's first (a wasm module has no build id of its own).
    if message.for_build_id.is_some_and(|id| id != 0) {
        return;
    }
    if let Some(table) = message.jump_table {
        // A patch already linked into this page is not linked again.
        static APPLIED: Mutex<Vec<std::path::PathBuf>> = Mutex::new(Vec::new());
        if let Ok(mut applied) = APPLIED.lock() {
            if applied.contains(&table.lib) {
                return;
            }
            applied.push(table.lib.clone());
        }
        // SAFETY: the table was built by the engine for this very module, which it linked;
        // applying it is what the engine's own web client does.
        if let Err(error) = unsafe { dioxus_devtools::subsecond::apply_patch(table) } {
            log::warn!("hot reload: the patch could not be applied: {error:?}");
        }
    }
}

/// On Android, the engine cannot hand the application its settings — the patch server's port
/// among them — as a process's environment: it pushes them to a file on the device, read here
/// into the environment before connecting (milestone 680).
#[cfg(target_os = "android")]
const ENGINE_SETTINGS: &str = "/data/local/tmp/dx/.env";

#[cfg(target_os = "android")]
fn engine_settings() {
    let Ok(text) = std::fs::read_to_string(ENGINE_SETTINGS) else {
        return;
    };
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            std::env::set_var(key.trim(), value.trim());
        }
    }
}

/// Whether the hot-reload engine started this application: it left its settings on the device.
#[cfg(target_os = "android")]
pub(crate) fn under_engine() -> bool {
    std::path::Path::new(ENGINE_SETTINGS).is_file()
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

//! The Web side of a reload under `frus run` (see [`crate::dev`] for the other platforms).
//!
//! On the Web a reload is the page reloading itself: the script `frus run` adds to the page asks
//! which build is served, and reloads when there is a new one. The address survives that — it is
//! the application's location — and so do the component states that keep themselves
//! (`State::save`). While the page runs under `frus run`, the shell keeps a snapshot of them,
//! written into the tab's session storage as the page goes away and read back, once, when the
//! new build starts. A restart afresh (`R`) leaves a mark in the same storage, and nothing is
//! kept.
//!
//! The page says it is served by `frus run` by setting `window.__frusRun`; nothing of this runs
//! on any other page. On such a page the shell also defines `window.__frusCommand(command)`,
//! which the script calls with what the keys of `frus run` ask (milestone 668).

use std::cell::{Cell, RefCell};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Where the snapshot waits for the next build, in the tab's session storage.
const STATES: &str = "frus-run-states";
/// Left by the page before a restart afresh: nothing is kept.
const FRESH: &str = "frus-run-fresh";

thread_local! {
    /// The states as of the last frame, written out when the page goes away.
    static LATEST: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Whether the page is served by `frus run`, read once.
    static WATCHED: Cell<bool> = const { Cell::new(false) };
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok()?
}

/// At start-up: if `frus run` serves the page, the states a reload handed over (consumed), and
/// the page made to keep its states as it goes away.
#[inline(never)]
pub(crate) fn start() -> Option<String> {
    let window = web_sys::window()?;
    let watched = js_sys::Reflect::get(&window, &"__frusRun".into())
        .map(|v| v.is_truthy())
        .unwrap_or(false);
    WATCHED.with(|w| w.set(watched));
    if !watched {
        return None;
    }
    let keep = Closure::<dyn FnMut()>::new(|| {
        let Some(storage) = storage() else { return };
        if storage.get_item(FRESH).ok().flatten().is_some() {
            let _ = storage.remove_item(FRESH);
            let _ = storage.remove_item(STATES);
            return;
        }
        LATEST.with(|latest| {
            if let Some(states) = latest.borrow().as_deref() {
                let _ = storage.set_item(STATES, states);
            }
        });
    });
    let _ = window.add_event_listener_with_callback("pagehide", keep.as_ref().unchecked_ref());
    keep.forget();
    let command = Closure::<dyn FnMut(String)>::new(|command: String| {
        crate::dev_commands::push_command(&command);
    });
    let _ = js_sys::Reflect::set(&window, &"__frusCommand".into(), command.as_ref());
    command.forget();

    let storage = storage()?;
    let states = storage.get_item(STATES).ok().flatten();
    let _ = storage.remove_item(STATES);
    states
}

/// After a frame: the states as they are now, for the reload that may come.
pub(crate) fn keep(snapshot: impl FnOnce() -> String) {
    if watched() {
        set_latest(snapshot());
    }
}

/// Whether the page is served by `frus run`. Not generic, and not inlined, like every access to
/// these thread-locals: see `frus-widgets/src/component.rs` on hot patches.
#[inline(never)]
fn watched() -> bool {
    WATCHED.with(Cell::get)
}

#[inline(never)]
fn set_latest(states: String) {
    LATEST.with(|latest| *latest.borrow_mut() = Some(states));
}

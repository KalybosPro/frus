//! The browser's address bar and history, as the shell reads and moves them. Web only.
//!
//! What is decided — which move the history should make for a location, what an address
//! means — is in [`crate::history`], with no browser in it. This is only the calls.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use winit::window::Window;

use crate::application::LocationStrategy;
use crate::history::{address_of, location_of, Step};

thread_local! {
    /// What the browser said since the shell last asked: the location and the `state` of the
    /// entry it moved to.
    static POPPED: RefCell<VecDeque<(String, Option<usize>)>> = const { RefCell::new(VecDeque::new()) };
}

/// The document's base path: the `<base href>`'s if it has one, `/` otherwise.
pub(crate) fn base() -> String {
    let document = web_sys::window().and_then(|window| window.document());
    let explicit = document
        .as_ref()
        .is_some_and(|document| matches!(document.query_selector("base[href]"), Ok(Some(_))));
    if !explicit {
        return "/".to_string();
    }
    document
        .and_then(|document| document.base_uri().ok().flatten())
        .and_then(|uri| web_sys::Url::new(&uri).ok())
        .map(|url| url.pathname())
        .unwrap_or_else(|| "/".to_string())
}

/// The location the address carries, and the number the current entry carries in its `state`
/// — `None` for either if there is none.
pub(crate) fn current(strategy: LocationStrategy, base: &str) -> (Option<String>, Option<usize>) {
    match web_sys::window() {
        Some(window) => (read_location(&window, strategy, base), read_state(&window)),
        None => (None, None),
    }
}

fn read_location(
    window: &web_sys::Window,
    strategy: LocationStrategy,
    base: &str,
) -> Option<String> {
    let location = window.location();
    location_of(
        strategy,
        base,
        &location.pathname().unwrap_or_default(),
        &location.search().unwrap_or_default(),
        &location.hash().unwrap_or_default(),
    )
}

fn read_state(window: &web_sys::Window) -> Option<usize> {
    let state = window.history().ok()?.state().ok()?;
    state.as_f64().map(|index| index as usize)
}

/// Makes the browser do `step`. `index` is the number the current entry is to carry.
pub(crate) fn apply(step: Step, strategy: LocationStrategy, base: &str, index: usize) {
    let Some(history) = web_sys::window().and_then(|window| window.history().ok()) else {
        return;
    };
    let state = JsValue::from_f64(index as f64);
    let result = match step {
        Step::Push(location) => {
            history.push_state_with_url(&state, "", Some(&address_of(strategy, base, &location)))
        }
        Step::Replace(location) => {
            history.replace_state_with_url(&state, "", Some(&address_of(strategy, base, &location)))
        }
        Step::Go(delta) => history.go_with_delta(delta as i32),
    };
    if let Err(err) = result {
        log::warn!("the browser refused a history move: {err:?}");
    }
}

/// Starts listening to the browser's own moves — its back and forward buttons, an address
/// typed over the old one. Each is queued for [`take_popped`] and asks `window` for a frame,
/// which is where the shell looks.
pub(crate) fn listen(window: Arc<Window>, strategy: LocationStrategy, base: String) {
    let Some(browser) = web_sys::window() else {
        return;
    };
    let handler = {
        let browser = browser.clone();
        Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            let location = read_location(&browser, strategy, &base).unwrap_or_else(|| "/".into());
            let state = read_state(&browser);
            POPPED.with(|queue| queue.borrow_mut().push_back((location, state)));
            window.request_redraw();
        })
    };
    if browser
        .add_event_listener_with_callback("popstate", handler.as_ref().unchecked_ref())
        .is_ok()
    {
        // The listener lives as long as the page.
        handler.forget();
    }
}

/// What the browser has said since the last call, oldest first.
pub(crate) fn take_popped() -> Vec<(String, Option<usize>)> {
    POPPED.with(|queue| queue.borrow_mut().drain(..).collect())
}

//! The **clipboard** on the Web (#17): the browser's asynchronous Clipboard API,
//! `navigator.clipboard`.
//!
//! Until this, the clipboard was a no-op in the browser: a copy dropped its text and a
//! paste found none, and nothing said so. The desktop's interface is kept for the copy;
//! the paste is the one thing that differs, because the browser's read is a promise — see
//! `clip::Clipboard::paste` in the driver for how its answer comes back.
//!
//! **Best-effort, like the other bridges.** The API is missing outside a secure context
//! and in older browsers, a read may be refused by the user or by the browser's
//! permission policy, and a write may be refused when the page does not have the focus.
//! Each of those is a log line and nothing else: nothing on this path can panic, and no
//! method is called on an object that is not there — which in the browser is a thrown
//! `TypeError`, not a `None`.

use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

/// The page's clipboard, if it has one that can do `method`.
///
/// `navigator.clipboard` is `undefined` outside a secure context, and a browser may have
/// the object without every method on it: the bindings would call through regardless,
/// so both are asked first.
fn clipboard(method: &str) -> Option<web_sys::Clipboard> {
    let clipboard = web_sys::window()?.navigator().clipboard();
    let value: &JsValue = clipboard.as_ref();
    if value.is_undefined() || value.is_null() {
        return None;
    }
    let callable = js_sys::Reflect::get(value, &JsValue::from_str(method))
        .map(|member| member.is_function())
        .unwrap_or(false);
    callable.then_some(clipboard)
}

/// Puts `text` on the clipboard. The write finishes later; a refusal is logged.
pub(crate) fn set_text(text: &str) {
    let Some(clipboard) = clipboard("writeText") else {
        log::warn!("clipboard: unavailable (not a secure context?) — the copy is dropped");
        return;
    };
    let written = JsFuture::from(clipboard.write_text(text));
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(err) = written.await {
            log::warn!("clipboard: write refused ({err:?})");
        }
    });
}

/// Reads the clipboard's text and hands it to `answer` once the browser has it.
///
/// `answer` is not called when the read is refused, or when the API is missing. The read
/// is **started here**, before the first `await`: a browser may allow it only while the
/// key press that asked is being handled, and a read started from a later task is no
/// longer part of that gesture.
pub(crate) fn get_text(answer: impl FnOnce(String) + 'static) {
    let Some(clipboard) = clipboard("readText") else {
        log::warn!("clipboard: unavailable (not a secure context?) — nothing to paste");
        return;
    };
    let read = JsFuture::from(clipboard.read_text());
    wasm_bindgen_futures::spawn_local(async move {
        match read.await {
            Ok(value) => match value.as_string() {
                Some(text) => answer(text),
                None => log::warn!("clipboard: the read did not answer with text"),
            },
            Err(err) => log::warn!("clipboard: read refused ({err:?})"),
        }
    });
}

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **A page that cannot draw says so** (milestone 745): when no renderer can be made — the
//! browser offers neither WebGPU nor WebGL 2, or both failed — the reader sees why, instead of
//! an empty, dark page whose only trace is a line in the developer console.
//!
//! A panic, which ends a web application for good, is shown the same way: a page that stopped
//! says so too.
//!
//! The shell dispatches a cancelable `frus-error` event on `window`, its `detail` the message.
//! A page that shows the message itself — the generated `index.html` does, in its status
//! box — calls `preventDefault()`. When no page did, the shell lays its own box over the
//! canvas.

/// The `id` of the box the shell lays over the page when the page did not show the message.
#[cfg(web)]
pub(crate) const OVERLAY_ID: &str = "frus-error";

/// The event a page listens to for the message.
#[cfg(web)]
pub(crate) const EVENT: &str = "frus-error";

/// What the reader is told when no renderer could be made, `why` being the renderer's error.
pub(crate) fn message(why: &str) -> String {
    format!(
        "This application cannot draw in this browser: it needs WebGPU or WebGL 2, and \
         neither could be used here. Try a recent version of Chrome, Edge, Firefox or Safari, \
         with hardware acceleration turned on.\n\nDetails: {why}"
    )
}

/// What the reader is told when the application stopped on a panic, `what` being the panic.
pub(crate) fn stopped(what: &str) -> String {
    format!(
        "This application stopped because of an error. Reloading the page starts it again.\n\n\
         Details: {what}"
    )
}

/// The panic hook on the Web: the panic in the console, as before, and on the page.
#[cfg(web)]
pub(crate) fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        console_error_panic_hook::hook(info);
        show(&stopped(&info.to_string()));
    }));
}

/// Tells the page — or, when it does not listen, the reader directly — `text`: that nothing
/// can be drawn, or that the application stopped, and why.
#[cfg(web)]
pub(crate) fn show(text: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let init = web_sys::CustomEventInit::new();
    init.set_cancelable(true);
    init.set_detail(&wasm_bindgen::JsValue::from_str(text));
    if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict(EVENT, &init) {
        // `false`: a listener called `preventDefault()`, and the page shows the message.
        if window.dispatch_event(&event) == Ok(false) {
            return;
        }
    }
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else {
        return;
    };
    let overlay = match document.get_element_by_id(OVERLAY_ID) {
        Some(existing) => existing,
        None => {
            let Ok(div) = document.create_element("div") else {
                return;
            };
            div.set_id(OVERLAY_ID);
            let _ = div.set_attribute("role", "alert");
            let _ = div.set_attribute(
                "style",
                "position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;\
                 justify-content:center;padding:24px;box-sizing:border-box;\
                 background:#121418;color:#e6e6ef;font:15px/1.5 system-ui,sans-serif;\
                 text-align:center;white-space:pre-line;",
            );
            let _ = body.append_child(&div);
            div
        }
    };
    overlay.set_text_content(Some(text));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_names_both_interfaces_and_keeps_the_details() {
        let text = message("no adapter");
        assert!(text.contains("WebGPU or WebGL 2"), "{text}");
        assert!(text.ends_with("Details: no adapter"), "{text}");
    }

    #[test]
    fn a_panic_is_told_as_a_stop() {
        let text = stopped("index out of bounds");
        assert!(text.starts_with("This application stopped"), "{text}");
        assert!(text.ends_with("Details: index out of bounds"), "{text}");
    }
}

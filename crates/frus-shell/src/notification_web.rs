//! **Notifications on the Web** (milestone 704): [`crate::SystemNotification`] through the
//! browser's `Notification`.
//!
//! - **The permission.** A page must be allowed to show notifications. The first one asks the
//!   person, once; a browser that wants the question to follow a click of theirs answers *no*
//!   to a page that asks on its own, so the first notification is best shown from a press.
//! - **The answer.** A click focuses the page's window and closes the notification; a close
//!   is a close. Both are events on the notification, turned into one awaited promise.
//! - **What a page's notification cannot carry.** Buttons and a large picture belong to the
//!   notifications a service worker shows, which a page alone has not got: they are left out,
//!   and the notification is shown with its title and body. Its icon is the page's own — the
//!   tab's icon, which is the application's unless the page names another.

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Notification, NotificationOptions, NotificationPermission};

use crate::notification::{NotificationError, NotificationResponse, SystemNotification};

/// Shows `notification` and waits for what the person does with it.
pub(crate) async fn show(
    notification: &SystemNotification,
) -> Result<NotificationResponse, NotificationError> {
    if !js_sys::Reflect::has(&js_sys::global(), &JsValue::from_str("Notification")).unwrap_or(false)
    {
        return Err(failed("this browser has no notifications"));
    }
    allowed().await?;
    if !notification.buttons.is_empty() || notification.image.is_some() {
        log::debug!(
            "notification {:?}: a page's notification has no buttons and no picture; shown \
             without them",
            notification.title
        );
    }

    let options = NotificationOptions::new();
    if !notification.body.is_empty() {
        options.set_body(&notification.body);
    }
    options.set_silent(Some(notification.silent));
    if let Some(icon) = page_icon() {
        options.set_icon(&icon);
    }
    let shown = Notification::new_with_options(&notification.title, &options)
        .map_err(|err| failed(&describe(&err)))?;

    // One promise for the three ways it ends. Each handler is a closure the browser frees once
    // it has run; the ones that never run are a few bytes kept for the page's life.
    let ended = js_sys::Promise::new(&mut |resolve, _reject| {
        let answer = |resolve: js_sys::Function, what: &'static str| {
            move || {
                let _ = resolve.call1(&JsValue::NULL, &JsValue::from_str(what));
            }
        };
        let clicked = {
            let shown = shown.clone();
            let answer = answer(resolve.clone(), "clicked");
            Closure::once_into_js(move || {
                if let Some(window) = web_sys::window() {
                    let _ = window.focus();
                }
                shown.close();
                answer();
            })
        };
        shown.set_onclick(Some(clicked.unchecked_ref()));
        let closed = Closure::once_into_js(answer(resolve.clone(), "closed"));
        shown.set_onclose(Some(closed.unchecked_ref()));
        let refused = Closure::once_into_js(answer(resolve, "error"));
        shown.set_onerror(Some(refused.unchecked_ref()));
    });
    let what = JsFuture::from(ended)
        .await
        .ok()
        .and_then(|value| value.as_string());
    match what.as_deref() {
        Some("clicked") => Ok(NotificationResponse::Clicked),
        Some("closed") => Ok(NotificationResponse::Closed),
        _ => Err(failed("the browser could not show it")),
    }
}

/// Whether the page may show notifications, asking the person the first time.
async fn allowed() -> Result<(), NotificationError> {
    match Notification::permission() {
        NotificationPermission::Granted => return Ok(()),
        NotificationPermission::Denied => {
            return Err(failed("notifications are blocked for this site"))
        }
        _ => {}
    }
    let asked = Notification::request_permission().map_err(|err| failed(&describe(&err)))?;
    let answer = JsFuture::from(asked)
        .await
        .map_err(|err| failed(&describe(&err)))?;
    match answer.as_string().as_deref() {
        Some("granted") => Ok(()),
        Some("denied") => Err(failed("the person did not allow notifications")),
        _ => Err(failed(
            "the person did not answer whether notifications are allowed; some browsers only \
             ask after a click of theirs",
        )),
    }
}

/// The page's icon: the tab's, which is the application's unless the page names its own.
fn page_icon() -> Option<String> {
    web_sys::window()?
        .document()?
        .query_selector("link[rel~='icon']")
        .ok()
        .flatten()?
        .get_attribute("href")
}

fn describe(err: &JsValue) -> String {
    err.as_string()
        .or_else(|| {
            js_sys::Reflect::get(err, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{err:?}"))
}

fn failed(why: &str) -> NotificationError {
    NotificationError(format!("the notification could not be shown: {why}"))
}

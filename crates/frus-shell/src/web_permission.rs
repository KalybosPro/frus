//! **Permissions on the web** (milestone 730): the browser's.
//!
//! The status is `navigator.permissions.query`; a request is what the browser asks on — the
//! camera or the microphone opened for a moment (`getUserMedia`, whose tracks are stopped at
//! once), the position read once (`geolocation.getCurrentPosition`), notifications asked for
//! (`Notification.requestPermission`). Reached through `js_sys::Reflect` rather than typed
//! bindings, because the browsers disagree on what they support: Firefox will not query the
//! camera's permission at all, and a page is told [`PermissionStatus::Ask`] there, which a
//! request then settles.

use js_sys::{Array, Function, Object, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::permission::from_browser;
use crate::{Permission, PermissionStatus};

/// The name the browser's permissions know `permission` by.
fn browser_name(permission: Permission) -> &'static str {
    match permission {
        Permission::Camera => "camera",
        Permission::Microphone => "microphone",
        Permission::Location => "geolocation",
        Permission::Notifications => "notifications",
    }
}

fn get(target: &JsValue, key: &str) -> Option<JsValue> {
    Reflect::get(target, &JsValue::from_str(key))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null())
}

/// Calls `target.method(args…)`.
fn call(target: &JsValue, method: &str, args: &[JsValue]) -> Option<JsValue> {
    let function: Function = get(target, method)?.dyn_into().ok()?;
    function
        .apply(target, &args.iter().cloned().collect::<Array>())
        .ok()
}

/// Whether the page may use powerful features at all: a secure page, or `localhost`.
fn secure() -> bool {
    get(&js_sys::global(), "isSecureContext")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

/// Where `permission` stands, from `navigator.permissions`.
pub(crate) async fn status(permission: Permission) -> PermissionStatus {
    if permission != Permission::Notifications && !secure() {
        return PermissionStatus::Unavailable;
    }
    let Some(navigator) = get(&js_sys::global(), "navigator") else {
        return PermissionStatus::Unavailable;
    };
    let Some(permissions) = get(&navigator, "permissions") else {
        return PermissionStatus::Ask;
    };
    let descriptor = Object::new();
    let _ = Reflect::set(
        &descriptor,
        &JsValue::from_str("name"),
        &JsValue::from_str(browser_name(permission)),
    );
    let Some(asked) = call(&permissions, "query", &[descriptor.into()]) else {
        return PermissionStatus::Ask;
    };
    let Ok(promise) = asked.dyn_into::<Promise>() else {
        return PermissionStatus::Ask;
    };
    match JsFuture::from(promise).await {
        Ok(answer) => get(&answer, "state")
            .and_then(|state| state.as_string())
            .map_or(PermissionStatus::Ask, |state| from_browser(&state)),
        // A name this browser does not know: whether it will ask is not known until it does.
        Err(_) => PermissionStatus::Ask,
    }
}

/// Asks for `permission` the way the browser asks: by using it once.
pub(crate) async fn request(permission: Permission) -> PermissionStatus {
    let before = status(permission).await;
    if before != PermissionStatus::Ask {
        return before;
    }
    let answered = match permission {
        Permission::Camera => media("video").await,
        Permission::Microphone => media("audio").await,
        Permission::Location => position().await,
        Permission::Notifications => notifications().await,
    };
    match answered {
        Some(status) => status,
        None => status(permission).await,
    }
}

/// Opens the camera or the microphone for a moment, which is what the browser asks on.
async fn media(kind: &str) -> Option<PermissionStatus> {
    let navigator = get(&js_sys::global(), "navigator")?;
    let devices = get(&navigator, "mediaDevices")?;
    let constraints = Object::new();
    let _ = Reflect::set(&constraints, &JsValue::from_str(kind), &JsValue::TRUE);
    let promise: Promise = call(&devices, "getUserMedia", &[constraints.into()])?
        .dyn_into()
        .ok()?;
    Some(match JsFuture::from(promise).await {
        Ok(stream) => {
            // It was only opened to ask: let go of it at once, so the light goes off.
            if let Some(tracks) = call(&stream, "getTracks", &[]) {
                for track in Array::from(&tracks).iter() {
                    let _ = call(&track, "stop", &[]);
                }
            }
            PermissionStatus::Granted
        }
        Err(error) => match get(&error, "name")
            .and_then(|name| name.as_string())
            .as_deref()
        {
            Some("NotFoundError") | Some("OverconstrainedError") => PermissionStatus::Unavailable,
            Some("NotAllowedError") | Some("SecurityError") => PermissionStatus::Blocked,
            _ => PermissionStatus::Denied,
        },
    })
}

/// Reads the position once, which is what the browser asks on.
async fn position() -> Option<PermissionStatus> {
    let navigator = get(&js_sys::global(), "navigator")?;
    let geolocation = get(&navigator, "geolocation")?;
    let mut ask = |resolve: Function, reject: Function| {
        let _ = call(
            &geolocation,
            "getCurrentPosition",
            &[resolve.into(), reject.into()],
        );
    };
    let promise = Promise::new(&mut ask);
    Some(match JsFuture::from(promise).await {
        Ok(_) => PermissionStatus::Granted,
        // `GeolocationPositionError.PERMISSION_DENIED` is 1; anything else is a position that
        // could not be had, which says nothing against the permission.
        Err(error) => match get(&error, "code").and_then(|code| code.as_f64()) {
            Some(1.0) => PermissionStatus::Blocked,
            _ => return None,
        },
    })
}

/// Asks for notifications.
async fn notifications() -> Option<PermissionStatus> {
    let notification = get(&js_sys::global(), "Notification")?;
    let promise: Promise = call(&notification, "requestPermission", &[])?
        .dyn_into()
        .ok()?;
    let answer = JsFuture::from(promise).await.ok()?.as_string()?;
    Some(match answer.as_str() {
        "granted" => PermissionStatus::Granted,
        "denied" => PermissionStatus::Blocked,
        _ => PermissionStatus::Ask,
    })
}

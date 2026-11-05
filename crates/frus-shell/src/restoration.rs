// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! Where each platform keeps the application's restoration data (milestone 753): the bytes
//! [`frus_widgets::Restoration::data`] returns, handed to the system while the application
//! runs and given back to the next process.
//!
//! - **Android**: the activity's saved instance state. The bytes of the last frame are kept
//!   here; `FrusRestoration` (in the bundled dex) asks for them once the activity has saved
//!   its own state and puts them under the key NativeActivity reads its native state from, so
//!   the next `onCreate` hands them to [`android_on_create`] with the rest of its creation.
//!   Android 10 and later: before, there is no moment after NativeActivity has written that
//!   key, and nothing is kept.
//! - **Web**: the tab's session storage, one entry per entry of the history, so that a reload
//!   — or a tab the browser discarded and opened again — finds the state of the page it shows.
//! - **Desktop**: nothing. A desktop does not end an application to take its memory back, and
//!   a window that was closed was closed on purpose.
//! - **iOS**: not yet. TODO(ios): UIKit's state restoration (`NSUserActivity` /
//!   `stateRestorationActivity` of the scene) once the shell has a UIKit side to put it on;
//!   until then an application the system ended starts afresh there.

#[cfg(android)]
pub(crate) use android::*;
#[cfg(web)]
pub(crate) use web::*;

#[cfg(android)]
mod android {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    use jni::objects::{JClass, JObject, JValue};
    use jni::sys::jbyteArray;
    use jni::JNIEnv;
    use winit::platform::android::activity::OnCreateState;

    /// What the activity was created with: the last process's data, until the shell takes it.
    static RESTORED: Mutex<Option<Vec<u8>>> = Mutex::new(None);
    /// What the last frame said is to be kept, for the activity to save.
    static PUBLISHED: Mutex<Option<Vec<u8>>> = Mutex::new(None);
    /// Whether the application asked for restoration: nothing is installed otherwise.
    static ENABLED: AtomicBool = AtomicBool::new(false);

    /// Called by the native activity in `onCreate`, on the Java main thread, before the
    /// application's thread starts: the saved state it was created with, if any. Found by
    /// name by `android-activity`, with the signature it documents.
    #[no_mangle]
    pub fn android_on_create(state: &OnCreateState) {
        let saved = state.saved_state();
        if !saved.is_empty() {
            *RESTORED.lock().unwrap_or_else(|e| e.into_inner()) = Some(saved.to_vec());
        }
    }

    /// The data the activity was created with, once.
    pub(crate) fn take_restored() -> Option<Vec<u8>> {
        ENABLED.store(true, Ordering::Relaxed);
        RESTORED.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// The data to save if the activity is stopped now.
    pub(crate) fn publish(data: Option<Vec<u8>>) {
        *PUBLISHED.lock().unwrap_or_else(|e| e.into_inner()) = data;
    }

    /// Loads `FrusRestoration` from the bundled dex and registers it with the activity —
    /// when the application asked for restoration. A failure costs restoration only.
    pub(crate) fn install(env: &mut JNIEnv, loader: &JObject, activity: &JObject) {
        if !ENABLED.load(Ordering::Relaxed) {
            return;
        }
        match try_install(env, loader, activity) {
            Ok(true) => log::info!("state restoration installed"),
            Ok(false) => log::info!(
                "state restoration needs Android 10 or later: the application starts afresh \
                 after the system ends it"
            ),
            Err(err) => {
                if env.exception_check().unwrap_or(false) {
                    let _ = env.exception_clear();
                }
                log::warn!("state restoration unavailable ({err})");
            }
        }
    }

    fn try_install(
        env: &mut JNIEnv,
        loader: &JObject,
        activity: &JObject,
    ) -> Result<bool, jni::errors::Error> {
        let name = env.new_string("dev.frus.input.FrusRestoration")?;
        let class: JClass = env
            .call_method(
                loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&name)],
            )?
            .l()?
            .into();
        env.register_native_methods(
            &class,
            &[jni::NativeMethod {
                name: "nativeSnapshot".into(),
                sig: "()[B".into(),
                fn_ptr: native_snapshot as *mut _,
            }],
        )?;
        env.call_static_method(
            &class,
            "install",
            "(Landroid/app/Activity;)Z",
            &[JValue::Object(activity)],
        )?
        .z()
    }

    /// `FrusRestoration.nativeSnapshot`: what the last frame published, or `null`.
    extern "system" fn native_snapshot(env: JNIEnv, _class: JClass) -> jbyteArray {
        let data = PUBLISHED.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let Some(data) = data else {
            return std::ptr::null_mut();
        };
        if data.len() > 256 * 1024 {
            // The whole saved state crosses a binder transaction, which fails past 1 MB.
            log::warn!(
                "the restoration data is {} KB: keep less in it (scroll positions, drafts, ids — \
                 not the data itself)",
                data.len() / 1024
            );
        }
        match env.byte_array_from_slice(&data) {
            Ok(array) => array.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[cfg(web)]
mod web {
    /// One entry of the session storage per entry of the history.
    fn key(index: usize) -> String {
        format!("frus-restoration:{index}")
    }

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.session_storage().ok()?
    }

    /// What the history entry `index` was left with, if the tab kept anything for it.
    pub(crate) fn load(index: usize) -> Option<Vec<u8>> {
        let text = storage()?.get_item(&key(index)).ok()??;
        Some(text.into_bytes())
    }

    /// Keeps `data` for the history entry `index` — or forgets what it had, for `None`.
    pub(crate) fn publish(index: usize, data: Option<&[u8]>) {
        let Some(storage) = storage() else {
            return;
        };
        match data.and_then(|bytes| std::str::from_utf8(bytes).ok()) {
            Some(text) => {
                if storage.set_item(&key(index), text).is_err() {
                    log::warn!("the tab's storage refused the restoration data");
                }
            }
            None => {
                let _ = storage.remove_item(&key(index));
            }
        }
    }
}

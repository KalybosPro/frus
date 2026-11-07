// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! Where each platform keeps the application's restoration data (milestone 753): the bytes
//! [`frus_widgets::Restoration::data`] returns, handed to the system while the application
//! runs and given back to the next process.
//!
//! - **Android**: the activity's saved instance state. The bytes of the last frame are kept
//!   here, and go under the key NativeActivity reads its native state from, so the next
//!   `onCreate` hands them to `android_on_create` with the rest of its creation. Two ways to
//!   put them there:
//!   - when the manifest names `frus_native_activity_on_create` as the activity's entry
//!     (`android.app.func_name`; milestone 762), frus answers NativeActivity's own
//!     `onSaveInstanceState` callback with them, on every Android version;
//!   - otherwise `FrusRestoration` (in the bundled dex) writes them once the activity has
//!     saved its own state, a moment that exists from Android 10 (milestone 753); before,
//!     nothing is kept.
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

    use std::ffi::c_void;
    use winit::platform::android::activity::ndk_sys::ANativeActivity;

    /// NativeActivity's save callback: the bytes to keep, `malloc`ed, and their length.
    type SaveInstanceState =
        unsafe extern "C" fn(activity: *mut ANativeActivity, out_len: *mut usize) -> *mut c_void;

    extern "C" {
        /// android-activity's entry point, which the native activity calls by default.
        fn ANativeActivity_onCreate(
            activity: *mut ANativeActivity,
            saved_state: *mut c_void,
            saved_state_size: usize,
        );
        fn malloc(size: usize) -> *mut c_void;
    }

    /// Whether the activity was entered through `frus_native_activity_on_create`: frus then
    /// answers the activity's save callback itself.
    static HOOKED: AtomicBool = AtomicBool::new(false);
    /// The save callback android-activity installed, for an application that keeps nothing.
    static THEIRS: Mutex<Option<SaveInstanceState>> = Mutex::new(None);

    /// The native activity's entry point when the manifest names it
    /// (`android.app.func_name`, milestone 762): android-activity's, after which the save
    /// callback is frus's.
    ///
    /// NativeActivity asks the native side for the state to save in `onSaveInstanceState`,
    /// on every Android version, and stores what it gets under the key the next `onCreate`
    /// reads. android-activity answers with the state the activity was *created* with
    /// (winit never hands the application the request), so the answer is replaced: the
    /// bytes of the last frame, or nothing at all, never a stale copy.
    ///
    /// # Safety
    ///
    /// Called by NativeActivity only, with the arguments it gives `ANativeActivity_onCreate`.
    #[no_mangle]
    pub unsafe extern "C" fn frus_native_activity_on_create(
        activity: *mut ANativeActivity,
        saved_state: *mut c_void,
        saved_state_size: usize,
    ) {
        // SAFETY: the arguments are NativeActivity's, handed on unchanged.
        unsafe { ANativeActivity_onCreate(activity, saved_state, saved_state_size) };
        // SAFETY: NativeActivity's struct and its callback table live as long as the
        // activity, and this runs on the Java main thread, the only one that reads the table.
        unsafe {
            let callbacks = (*activity).callbacks;
            if callbacks.is_null() {
                return;
            }
            *THEIRS.lock().unwrap_or_else(|e| e.into_inner()) = (*callbacks).onSaveInstanceState;
            (*callbacks).onSaveInstanceState = Some(save_instance_state);
        }
        HOOKED.store(true, Ordering::Relaxed);
    }

    /// NativeActivity's `onSaveInstanceState`, answered by frus: the last frame's bytes.
    unsafe extern "C" fn save_instance_state(
        activity: *mut ANativeActivity,
        out_len: *mut usize,
    ) -> *mut c_void {
        // SAFETY: NativeActivity passes a valid pointer for the length.
        unsafe { *out_len = 0 };
        if !ENABLED.load(Ordering::Relaxed) {
            // The application keeps nothing: android-activity answers, as it did before.
            let theirs = *THEIRS.lock().unwrap_or_else(|e| e.into_inner());
            return match theirs {
                // SAFETY: android-activity's own callback, with NativeActivity's arguments.
                Some(theirs) => unsafe { theirs(activity, out_len) },
                None => std::ptr::null_mut(),
            };
        }
        let Some(data) = snapshot() else {
            // Nothing to keep: NativeActivity then writes no state at all.
            return std::ptr::null_mut();
        };
        // SAFETY: NativeActivity copies the bytes and `free`s them, which is why they are
        // `malloc`ed; a failed allocation keeps nothing.
        unsafe {
            let bytes = malloc(data.len());
            if bytes.is_null() {
                return std::ptr::null_mut();
            }
            std::ptr::copy_nonoverlapping(data.as_ptr(), bytes.cast::<u8>(), data.len());
            *out_len = data.len();
            log::info!(
                "state restoration: {} bytes kept for the system",
                data.len()
            );
            bytes
        }
    }

    /// What the last frame published, warned about when it is large.
    fn snapshot() -> Option<Vec<u8>> {
        let data = PUBLISHED
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()?;
        if data.len() > 256 * 1024 {
            // The whole saved state crosses a binder transaction, which fails past 1 MB.
            log::warn!(
                "the restoration data is {} KB: keep less in it (scroll positions, drafts, ids, \
                 not the data itself)",
                data.len() / 1024
            );
        }
        Some(data)
    }

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
    /// when the application asked for restoration and the activity was not entered through
    /// `frus_native_activity_on_create`, which keeps the state by itself. A failure costs
    /// restoration only.
    pub(crate) fn install(env: &mut JNIEnv, loader: &JObject, activity: &JObject) {
        if !ENABLED.load(Ordering::Relaxed) {
            return;
        }
        if HOOKED.load(Ordering::Relaxed) {
            log::info!("state restoration installed (the activity's own save)");
            return;
        }
        match try_install(env, loader, activity) {
            Ok(true) => log::info!("state restoration installed (after the activity's save)"),
            Ok(false) => log::info!(
                "state restoration before Android 10 needs the activity entered through \
                 `frus_native_activity_on_create` (`android.app.func_name` in the \
                 manifest): the application starts afresh after the system ends it"
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
        let Some(data) = snapshot() else {
            return std::ptr::null_mut();
        };
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

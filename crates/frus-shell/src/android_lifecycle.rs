// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! The end of an **Android** activity (milestone 755).
//!
//! Android destroys an activity without ending its process: the person leaves it with Back,
//! the developer setting "Don't keep activities" is on, or a configuration change the
//! manifest does not declare recreates it. The native activity then calls `onDestroy`, which
//! **waits for `android_main` to return** — and winit receives the destruction, logs that it
//! does not forward it, and keeps its loop running. The activity's main thread stayed blocked
//! until the system reported the application as not responding, and no later launch in that
//! process could work either: winit builds one event loop per process.
//!
//! So the shell asks the activity itself. `Activity.isDestroyed()` turns true as the
//! destruction begins, before `onDestroy` runs, and the native activity takes its surface
//! away before it destroys anything: the question only needs asking while there is no window,
//! and the loop ends as soon as the answer is yes. [`crate::run_android`] then ends the
//! process, and the next launch starts a new one — from the state the system kept, when the
//! application asked for restoration (milestone 753).
//!
//! As everywhere on this side, **every failed JNI call clears its exception**: a pending
//! Java exception left behind takes the process down on the next, unrelated call.

use jni::objects::JObject;
use jni::{JNIEnv, JavaVM};
use winit::platform::android::activity::AndroidApp;

/// Clears any Java exception left pending on this thread.
fn clear_pending(env: &mut JNIEnv) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

/// Whether the activity is being destroyed, or has been: `Activity.isDestroyed()`. `false`
/// when it cannot be asked.
pub(crate) fn activity_destroyed(app: &AndroidApp) -> bool {
    let Ok(vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }) else {
        return false;
    };
    let Ok(mut env) = vm.attach_current_thread_permanently() else {
        return false;
    };
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };
    match env
        .call_method(&activity, "isDestroyed", "()Z", &[])
        .and_then(|value| value.z())
    {
        Ok(destroyed) => destroyed,
        Err(_) => {
            clear_pending(&mut env);
            false
        }
    }
}

/// Back with nothing left to go back from: what the platform does then, on its main thread —
/// `Activity.onBackPressed()`, which finishes the activity, or from Android 12 moves a root
/// activity's task to the background. A finished activity is destroyed, and the loop ends
/// then, through [`activity_destroyed`].
pub(crate) fn back_at_root(app: &AndroidApp) {
    let handle = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let Ok(vm) = (unsafe { JavaVM::from_raw(handle.vm_as_ptr() as *mut jni::sys::JavaVM) })
        else {
            return;
        };
        let Ok(mut env) = vm.attach_current_thread_permanently() else {
            return;
        };
        let activity = unsafe { JObject::from_raw(handle.activity_as_ptr() as jni::sys::jobject) };
        if env
            .call_method(&activity, "onBackPressed", "()V", &[])
            .is_err()
        {
            clear_pending(&mut env);
            log::warn!("the activity refused Back: finishing it");
            if env.call_method(&activity, "finish", "()V", &[]).is_err() {
                clear_pending(&mut env);
            }
        }
    }));
}

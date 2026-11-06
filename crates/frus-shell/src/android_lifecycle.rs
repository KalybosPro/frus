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
//! **A second activity in the process** (milestone 763). Android starts a new activity of the
//! application's in its existing process when an intent asks for another instance — a link
//! opened from another application, a notification or a shortcut whose intent lacks
//! `FLAG_ACTIVITY_SINGLE_TOP` — and the native activity runs `android_main` again for it, on a
//! thread of its own, while the first activity's loop still runs. winit builds one event loop
//! per process, so the second cannot run: [`hand_over`] gives the link it was opened with to
//! the running loop, brings the running activity's task to the front when the new one is in
//! another, and returns, after which the native activity finishes the new one. The person
//! sees the application they had, at the link's location. Ending the process instead would
//! end the running activity too, with whatever it had not saved.
//!
//! As everywhere on this side, **every failed JNI call clears its exception**: a pending
//! Java exception left behind takes the process down on the next, unrelated call.

use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use jni::objects::{JObject, JValue};
use jni::{JNIEnv, JavaVM};
use winit::platform::android::activity::AndroidApp;
use winit::window::Window;

/// Whether an activity of this process runs the event loop.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// The activity that runs the loop: a second activity brings its task forward.
static RUNNING_ACTIVITY: Mutex<Option<AndroidApp>> = Mutex::new(None);

/// The links second activities handed over, for the loop's next frame.
static LINKS: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// The running activity's window, asked for the frame that reads the links.
static WINDOW: Mutex<Option<Arc<Window>>> = Mutex::new(None);

/// Whether `app`'s activity is the first of the process, which runs the loop; it is kept as
/// the one later activities hand over to. Every later call is `false`.
pub(crate) fn first_in_process(app: &AndroidApp) -> bool {
    let first = crate::app::first_loop_of_process(&RUNNING);
    if first {
        *RUNNING_ACTIVITY.lock().unwrap_or_else(|e| e.into_inner()) = Some(app.clone());
    }
    first
}

/// Hands the window links are woken through — or takes it away, with the surface.
pub(crate) fn set_window(window: Option<Arc<Window>>) {
    *WINDOW.lock().unwrap_or_else(|e| e.into_inner()) = window;
}

/// The links handed over since the last call, oldest first.
pub(crate) fn take_links() -> Vec<String> {
    LINKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .drain(..)
        .collect()
}

/// A second activity of this process, `app`, which cannot run a loop of its own: its link, if
/// it was opened with one, goes to the running loop, and the running activity comes to the
/// front if `app` is in another task. The caller then returns from `android_main`, and the
/// native activity finishes `app`.
pub(crate) fn hand_over(app: &AndroidApp) {
    let link = crate::android_link::launch_link(app);
    log::warn!(
        "a second activity started in this process{}: it hands over to the running one and \
         finishes (an intent that opens the application keeps to one activity with \
         FLAG_ACTIVITY_SINGLE_TOP | FLAG_ACTIVITY_NEW_TASK)",
        link.as_deref()
            .map(|link| format!(" with {link}"))
            .unwrap_or_default()
    );
    if let Some(link) = link {
        LINKS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(link);
    }
    let running = RUNNING_ACTIVITY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(running) = running {
        bring_forward(app, &running);
    }
    if let Some(window) = WINDOW.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        window.request_redraw();
    }
}

/// When `new` is in another task than `running` — a link opened from another application's
/// task, or an intent that asked for a task of its own — `running`'s task is brought to the
/// front; `new` has just started, which allows it from the background. In the same task,
/// finishing `new` is enough.
///
/// - First the application's own task list: `ActivityManager.getAppTasks()`, the task whose id
///   is `running`'s, `moveToFront()`. No permission is needed. It works on Android 16; on
///   Android 10 the list held only `new`'s task.
/// - Otherwise `new` finishes **first**, then starts `running`'s activity with
///   `FLAG_ACTIVITY_NEW_TASK | FLAG_ACTIVITY_SINGLE_TOP`. The system looks for the task by
///   affinity and passes over a task whose activities are all finishing, so it finds
///   `running`'s, brings it forward, and hands the intent to the activity on top instead of
///   making another. The same intent sent while `new` was alive went to `new`'s own task (the
///   same affinity, more recent), and the person was left on the home screen.
fn bring_forward(new: &AndroidApp, running: &AndroidApp) {
    let Ok(vm) = (unsafe { JavaVM::from_raw(new.vm_as_ptr() as *mut jni::sys::JavaVM) }) else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread_permanently() else {
        return;
    };
    let new = unsafe { JObject::from_raw(new.activity_as_ptr() as jni::sys::jobject) };
    let running = unsafe { JObject::from_raw(running.activity_as_ptr() as jni::sys::jobject) };
    let result = (|| -> jni::errors::Result<bool> {
        let wanted = env.call_method(&running, "getTaskId", "()I", &[])?.i()?;
        if env.call_method(&new, "getTaskId", "()I", &[])?.i()? == wanted {
            return Ok(false);
        }
        let name = env.new_string("activity")?;
        let manager = env
            .call_method(
                &new,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        let tasks = env
            .call_method(&manager, "getAppTasks", "()Ljava/util/List;", &[])?
            .l()?;
        let count = env.call_method(&tasks, "size", "()I", &[])?.i()?;
        let mut seen = Vec::new();
        for at in 0..count {
            let task = env
                .call_method(&tasks, "get", "(I)Ljava/lang/Object;", &[JValue::Int(at)])?
                .l()?;
            let info = env
                .call_method(
                    &task,
                    "getTaskInfo",
                    "()Landroid/app/ActivityManager$RecentTaskInfo;",
                    &[],
                )?
                .l()?;
            // `persistentId`: the task's id on every API level (`taskId` arrived in 29).
            let id = env.get_field(&info, "persistentId", "I")?.i()?;
            seen.push(id);
            if id == wanted {
                env.call_method(&task, "moveToFront", "()V", &[])?;
                return Ok(true);
            }
        }
        log::info!(
            "the running activity's task ({wanted}) is not among the application's tasks \
             ({seen:?}): the new activity finishes first, then asks for the running one"
        );
        env.call_method(&new, "finish", "()V", &[])?;
        let component = env
            .call_method(
                &running,
                "getComponentName",
                "()Landroid/content/ComponentName;",
                &[],
            )?
            .l()?;
        let intent = env.new_object("android/content/Intent", "()V", &[])?;
        env.call_method(
            &intent,
            "setComponent",
            "(Landroid/content/ComponentName;)Landroid/content/Intent;",
            &[JValue::Object(&component)],
        )?;
        // FLAG_ACTIVITY_NEW_TASK | FLAG_ACTIVITY_SINGLE_TOP.
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(0x1000_0000 | 0x2000_0000)],
        )?;
        env.call_method(
            &new,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(true)
    })();
    match result {
        Ok(true) => log::info!("the running activity's task comes to the front"),
        Ok(false) => {}
        Err(err) => {
            clear_pending(&mut env);
            log::warn!("the running activity could not be brought to the front ({err})");
        }
    }
}

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

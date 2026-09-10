//! The **clipboard** on Android (#22): `android.content.ClipboardManager`, reached through
//! `FrusClipboard` in the bundled dex.
//!
//! Until this, the clipboard was a no-op everywhere but the desktop: a copy dropped its
//! text and a paste found none, and nothing said so. The desktop's `arboard` interface is
//! kept, so no caller changes.
//!
//! **Best-effort, like the other bridges.** A read that fails, or that finds nothing that
//! reads as text, is `None`; a write that fails is dropped with a log line. Either way the
//! Java exception it left is cleared — one left pending takes the process down on the next
//! JNI call anybody makes (see `android_settings`). Nothing on this path can panic.

use std::sync::OnceLock;

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};

struct Handle {
    vm: JavaVM,
    class: GlobalRef,
    activity: GlobalRef,
}

// SAFETY: JavaVM and the GlobalRefs are shareable across threads, per the JNI contract.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

static HANDLE: OnceLock<Handle> = OnceLock::new();

/// Loads `FrusClipboard` from the bundled dex, through the loader the input bridge made.
///
/// A failure here costs the clipboard only: the input bridge is installed either way.
pub(crate) fn install(env: &mut JNIEnv, loader: &JObject, activity: &JObject) {
    match try_install(env, loader, activity) {
        Ok(handle) => {
            let _ = HANDLE.set(handle);
        }
        Err(err) => {
            clear_exception(env);
            log::warn!("clipboard unavailable ({err}) — copy and paste do nothing");
        }
    }
}

fn try_install(
    env: &mut JNIEnv,
    loader: &JObject,
    activity: &JObject,
) -> Result<Handle, jni::errors::Error> {
    let name = env.new_string("dev.frus.input.FrusClipboard")?;
    let class = env
        .call_method(
            loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&name)],
        )?
        .l()?;
    Ok(Handle {
        vm: env.get_java_vm()?,
        class: env.new_global_ref(&class)?,
        activity: env.new_global_ref(activity)?,
    })
}

fn clear_exception(env: &mut JNIEnv) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

/// The clipboard's text — `None` when it is empty, holds nothing that reads as text, or
/// cannot be reached.
pub(crate) fn get_text() -> Option<String> {
    let handle = HANDLE.get()?;
    let mut env = handle.vm.attach_current_thread_permanently().ok()?;
    let class: &JClass = handle.class.as_obj().into();
    let read = env.call_static_method(
        class,
        "getText",
        "(Landroid/app/Activity;)Ljava/lang/String;",
        &[JValue::Object(handle.activity.as_obj())],
    );
    let object = match read.and_then(|value| value.l()) {
        Ok(object) => object,
        Err(err) => {
            clear_exception(&mut env);
            log::warn!("clipboard: read failed ({err})");
            return None;
        }
    };
    // `null` is the Java side saying "nothing that reads as text" — not an error.
    if object.is_null() {
        return None;
    }
    let text = JString::from(object);
    let copied = env.get_string(&text).map(String::from);
    match copied {
        Ok(text) => Some(text),
        Err(err) => {
            clear_exception(&mut env);
            log::warn!("clipboard: the text could not be read back ({err})");
            None
        }
    }
}

/// Puts `text` on the clipboard. Returns whether the request reached the Java side.
pub(crate) fn set_text(text: &str) -> bool {
    let Some(handle) = HANDLE.get() else {
        return false;
    };
    let Ok(mut env) = handle.vm.attach_current_thread_permanently() else {
        return false;
    };
    let Ok(text) = env.new_string(text) else {
        clear_exception(&mut env);
        return false;
    };
    let class: &JClass = handle.class.as_obj().into();
    let written = env.call_static_method(
        class,
        "setText",
        "(Landroid/app/Activity;Ljava/lang/String;)V",
        &[
            JValue::Object(handle.activity.as_obj()),
            JValue::Object(&text),
        ],
    );
    match written {
        Ok(_) => true,
        Err(err) => {
            clear_exception(&mut env);
            log::warn!("clipboard: write failed ({err})");
            false
        }
    }
}

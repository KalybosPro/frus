//! The link an **Android** activity was opened with.
//!
//! Tapping `https://example.com/orders/42` in a mail — when the application declared an
//! intent filter for it — or `myapp://orders/42` starts the activity with an `Intent` whose
//! data is that string. It is one read from the activity, `getIntent().getDataString()`, and
//! [`crate::link::location_of_link`] turns it into a location.
//!
//! **Only the launch is covered.** A link opened while the application is already running
//! reaches the activity as `onNewIntent`, which the native activity does not forward and a
//! Rust library cannot override without a class of its own to carry it; that is a bridge of
//! the kind [`crate::android_ime`] is, and is not made here. With `launchMode="singleTask"`
//! the running activity comes to the front and the link is lost; without it, a second activity
//! starts and this reads its own.
//!
//! As in [`crate::android_settings`], the read is best-effort and **every failure clears its
//! exception**: a pending Java exception left behind takes the process down on the next,
//! unrelated JNI call.

use jni::objects::{JObject, JString};
use jni::{JNIEnv, JavaVM};
use winit::platform::android::activity::AndroidApp;

/// The link the activity was started with, if it was started with one.
pub(crate) fn launch_link(app: &AndroidApp) -> Option<String> {
    with_intent(app, |env, intent| {
        let data = env
            .call_method(intent, "getDataString", "()Ljava/lang/String;", &[])?
            .l()?;
        string(env, data)
    })
}

/// A text extra of the launch intent: `am start --es <name> <value>`. How `frus run` hands an
/// application the address it reports to (see [`crate::dev`]).
pub(crate) fn launch_extra(app: &AndroidApp, name: &str) -> Option<String> {
    with_intent(app, |env, intent| {
        let key = env.new_string(name)?;
        let value = env
            .call_method(
                intent,
                "getStringExtra",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[(&key).into()],
            )?
            .l()?;
        string(env, value)
    })
}

/// Runs `read` on the activity's intent; any failure is a warning and `None`.
fn with_intent(
    app: &AndroidApp,
    read: impl FnOnce(&mut JNIEnv, &JObject) -> Result<Option<String>, jni::errors::Error>,
) -> Option<String> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
    let mut env = vm.attach_current_thread_permanently().ok()?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };
    let result = env
        .call_method(&activity, "getIntent", "()Landroid/content/Intent;", &[])
        .and_then(|intent| intent.l())
        .and_then(|intent| {
            if intent.is_null() {
                Ok(None)
            } else {
                read(&mut env, &intent)
            }
        });
    match result {
        Ok(value) => value,
        Err(err) => {
            log::warn!("the launch intent could not be read ({err})");
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            None
        }
    }
}

/// A Java string, or `None` for `null`.
fn string(env: &mut JNIEnv, value: JObject) -> Result<Option<String>, jni::errors::Error> {
    if value.is_null() {
        return Ok(None);
    }
    let value = JString::from(value);
    let text: String = env.get_string(&value)?.into();
    Ok(Some(text))
}

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
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
    let mut env = vm.attach_current_thread_permanently().ok()?;
    match read(&mut env, app) {
        Ok(link) => link,
        Err(err) => {
            log::warn!("the launch intent could not be read ({err}); starting at the root");
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            None
        }
    }
}

fn read(env: &mut JNIEnv, app: &AndroidApp) -> Result<Option<String>, jni::errors::Error> {
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };
    let intent = env
        .call_method(&activity, "getIntent", "()Landroid/content/Intent;", &[])?
        .l()?;
    if intent.is_null() {
        return Ok(None);
    }
    let data = env
        .call_method(&intent, "getDataString", "()Ljava/lang/String;", &[])?
        .l()?;
    if data.is_null() {
        return Ok(None);
    }
    let data = JString::from(data);
    let link: String = env.get_string(&data)?.into();
    Ok(Some(link))
}

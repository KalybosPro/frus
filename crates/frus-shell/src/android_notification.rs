//! **Notifications on Android** (milestone 703): [`crate::SystemNotification`] posted through
//! `FrusNotifications` in the bundled dex.
//!
//! - **The channel.** Android 8 and later file every notification under a channel the person
//!   can tune in the settings: one for notifications, one for silent ones, made the first time.
//! - **The permission.** From Android 13 the person must allow notifications. The first one asks
//!   them, and waits for their answer; the application's manifest must ask for the permission
//!   (`android.permission.POST_NOTIFICATIONS`), or the system answers no without asking.
//! - **The answer.** A click opens the application through its launch intent, which brings the
//!   running one forward. A button or a dismissal is a broadcast the Java side records. So a
//!   notification that left the shade with nothing recorded was clicked: this polls for that,
//!   as the call is waiting anyway.
//!
//! As with the other bridges, every failure clears the Java exception it left.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};

use crate::notification::{
    left_the_shade, NotificationError, NotificationResponse, SystemNotification,
};

struct Handle {
    vm: JavaVM,
    class: GlobalRef,
    activity: GlobalRef,
}

// SAFETY: JavaVM and the GlobalRefs are shareable across threads, per the JNI contract.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

static HANDLE: OnceLock<Handle> = OnceLock::new();

/// How often the shade is looked at while a notification waits for its answer.
const POLL: Duration = Duration::from_millis(250);

/// How long the person is given to answer the question about notifications.
const ASKING: Duration = Duration::from_secs(60);

/// Loads `FrusNotifications` from the bundled dex, through the loader the input bridge made. A
/// failure costs the notifications only.
pub(crate) fn install(env: &mut JNIEnv, loader: &JObject, activity: &JObject) {
    match try_install(env, loader, activity) {
        Ok(handle) => {
            let _ = HANDLE.set(handle);
        }
        Err(err) => {
            clear_exception(env);
            log::warn!("notifications unavailable ({err})");
        }
    }
}

fn try_install(
    env: &mut JNIEnv,
    loader: &JObject,
    activity: &JObject,
) -> Result<Handle, jni::errors::Error> {
    let name = env.new_string("dev.frus.input.FrusNotifications")?;
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

/// Shows `notification` and waits for its answer.
pub(crate) fn show(
    notification: &SystemNotification,
) -> Result<NotificationResponse, NotificationError> {
    let handle = HANDLE
        .get()
        .ok_or_else(|| failed("the notification bridge was not loaded"))?;
    let mut env = handle
        .vm
        .attach_current_thread_permanently()
        .map_err(|err| failed(&format!("no Java thread ({err})")))?;
    let result = run(&mut env, handle, notification);
    if result.is_err() {
        clear_exception(&mut env);
    }
    result
}

fn failed(why: &str) -> NotificationError {
    NotificationError(format!("the notification could not be shown: {why}"))
}

fn run(
    env: &mut JNIEnv,
    handle: &Handle,
    notification: &SystemNotification,
) -> Result<NotificationResponse, NotificationError> {
    let jni = |err: jni::errors::Error| failed(&err.to_string());
    let class: &JClass = handle.class.as_obj().into();
    let activity = handle.activity.as_obj();
    let allowed = |env: &mut JNIEnv| {
        env.call_static_method(
            class,
            "allowed",
            "(Landroid/app/Activity;)I",
            &[JValue::Object(activity)],
        )
        .and_then(|value| value.i())
    };
    // Android 13 and later: ask, then wait for the person's answer.
    if allowed(env).map_err(jni)? == 0 {
        env.call_static_method(
            class,
            "ask",
            "(Landroid/app/Activity;)V",
            &[JValue::Object(activity)],
        )
        .map_err(jni)?;
        let asked = Instant::now();
        while allowed(env).map_err(jni)? == 0 && asked.elapsed() < ASKING {
            std::thread::sleep(POLL);
        }
    }
    match allowed(env).map_err(jni)? {
        1 => {}
        0 => return Err(failed("the person did not answer whether it may")),
        _ => {
            return Err(failed(
                "notifications are turned off for the application, or its manifest does not \
                 ask for android.permission.POST_NOTIFICATIONS",
            ))
        }
    }

    let title = string(env, &notification.title).map_err(jni)?;
    let body = string(env, &notification.body).map_err(jni)?;
    let ids: Vec<&str> = notification
        .buttons
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();
    let labels: Vec<&str> = notification
        .buttons
        .iter()
        .map(|(_, label)| label.as_str())
        .collect();
    let ids = strings(env, &ids).map_err(jni)?;
    let labels = strings(env, &labels).map_err(jni)?;
    let image = match &notification.image {
        Some(path) => string(env, &path.to_string_lossy()).map_err(jni)?,
        None => JObject::null(),
    };
    let id = env
        .call_static_method(
            class,
            "post",
            "(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;\
             [Ljava/lang/String;Ljava/lang/String;Z)I",
            &[
                JValue::Object(activity),
                JValue::Object(&title),
                JValue::Object(&body),
                JValue::Object(&ids),
                JValue::Object(&labels),
                JValue::Object(&image),
                JValue::Bool(notification.silent.into()),
            ],
        )
        .and_then(|value| value.i())
        .map_err(jni)?;
    if id == 0 {
        return Err(failed("the system has no notification service"));
    }

    let answer = |env: &mut JNIEnv| -> Result<Option<String>, jni::errors::Error> {
        let object = env
            .call_static_method(class, "answer", "(I)Ljava/lang/String;", &[JValue::Int(id)])?
            .l()?;
        if object.is_null() {
            return Ok(None);
        }
        Ok(Some(env.get_string(&JString::from(object))?.into()))
    };
    let active = |env: &mut JNIEnv| {
        env.call_static_method(
            class,
            "active",
            "(Landroid/app/Activity;I)Z",
            &[JValue::Object(activity), JValue::Int(id)],
        )
        .and_then(|value| value.z())
    };
    loop {
        if let Some(recorded) = answer(env).map_err(jni)? {
            return Ok(left_the_shade(Some(&recorded)));
        }
        if !active(env).map_err(jni)? {
            // Gone from the shade: a dismissal's broadcast may still be on its way.
            std::thread::sleep(POLL * 2);
            return Ok(left_the_shade(answer(env).map_err(jni)?.as_deref()));
        }
        std::thread::sleep(POLL);
    }
}

/// A Java string.
fn string<'local>(
    env: &mut JNIEnv<'local>,
    text: &str,
) -> Result<JObject<'local>, jni::errors::Error> {
    env.new_string(text).map(JObject::from)
}

/// A Java array of strings.
fn strings<'local>(
    env: &mut JNIEnv<'local>,
    texts: &[&str],
) -> Result<JObject<'local>, jni::errors::Error> {
    let array = env.new_object_array(texts.len() as i32, "java/lang/String", JObject::null())?;
    for (i, text) in texts.iter().enumerate() {
        let text = env.new_string(text)?;
        env.set_object_array_element(&array, i as i32, text)?;
    }
    Ok(array.into())
}

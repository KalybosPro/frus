//! The Android half of **autofill** (milestone 512): the form shown to the platform's
//! autofill service, and the values it hands back.
//!
//! What is shown, under which ids, and where a returned value goes are all decided in
//! `crate::autofill`, where they can be tested; this file carries the answers across the
//! bridge. The Java side lives in `FrusTextBridge`, the same view that captures the
//! keyboard: it declares the form as a virtual structure, one child per field, which is
//! how a service meets fields that are drawn rather than made of views.

use std::sync::Mutex;

use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jint;
use jni::JNIEnv;

use crate::autofill::AutofillField;

/// The values a service chose, `(virtual id, value)`: filled on the Java UI thread and
/// drained by the shell.
static FILLS: Mutex<Vec<(i32, String)>> = Mutex::new(Vec::new());

/// Shows the service `fields` as the form being edited, in place of the last one.
///
/// Built aside on the Java side and published whole, so a service asking half-way
/// through sees the previous form rather than half of this one. Every field's strings
/// are made in a local frame of their own: the thread is attached for good, so a local
/// reference nobody frees stays for as long as the process does.
pub(crate) fn publish(fields: &[AutofillField]) {
    crate::android_ime::with_bridge("publishing the autofill form", |env, class, _activity| {
        for field in fields {
            env.with_local_frame(8 + field.hints.len() as i32, |env| {
                let hints = env.new_object_array(
                    field.hints.len() as i32,
                    "java/lang/String",
                    JObject::null(),
                )?;
                for (i, hint) in field.hints.iter().enumerate() {
                    let hint = env.new_string(hint)?;
                    env.set_object_array_element(&hints, i as i32, &hint)?;
                }
                let value = env.new_string(&field.value)?;
                let [left, top, width, height] = field.bounds;
                env.call_static_method(
                    class,
                    "addField",
                    "(I[Ljava/lang/String;IIIILjava/lang/String;Z)V",
                    &[
                        JValue::Int(field.virtual_id),
                        JValue::Object(&hints),
                        JValue::Int(left),
                        JValue::Int(top),
                        JValue::Int(width),
                        JValue::Int(height),
                        JValue::Object(&value),
                        JValue::Bool(field.sensitive as u8),
                    ],
                )
                .map(|_| ())
            })?;
        }
        env.call_static_method(class, "publishFields", "()V", &[])
            .map(|_| ())
    });
}

/// A field of the published form takes focus: the service is told where it is.
pub(crate) fn enter(virtual_id: i32) {
    call_with_id("autofillEnter", virtual_id);
}

/// Focus leaves a field of the form.
pub(crate) fn exit(virtual_id: i32) {
    call_with_id("autofillExit", virtual_id);
}

fn call_with_id(method: &'static str, virtual_id: i32) {
    crate::android_ime::with_bridge(method, |env, class, activity| {
        env.call_static_method(
            class,
            method,
            "(Landroid/app/Activity;I)V",
            &[JValue::Object(activity), JValue::Int(virtual_id)],
        )
        .map(|_| ())
    });
}

/// A field's value is not what the service was last told.
pub(crate) fn value_changed(virtual_id: i32, value: &str) {
    crate::android_ime::with_bridge("autofillValueChanged", |env, class, activity| {
        env.with_local_frame(4, |env| {
            let value = env.new_string(value)?;
            env.call_static_method(
                class,
                "autofillValueChanged",
                "(Landroid/app/Activity;ILjava/lang/String;)V",
                &[
                    JValue::Object(activity),
                    JValue::Int(virtual_id),
                    JValue::Object(&value),
                ],
            )
            .map(|_| ())
        })
    });
}

/// The form is finished with: the service may offer to save what was typed in it.
pub(crate) fn commit() {
    crate::android_ime::with_bridge("autofillCommit", |env, class, activity| {
        env.call_static_method(
            class,
            "autofillCommit",
            "(Landroid/app/Activity;)V",
            &[JValue::Object(activity)],
        )
        .map(|_| ())
    });
}

/// The values chosen since the last call, in arrival order.
pub(crate) fn drain() -> Vec<(i32, String)> {
    std::mem::take(&mut FILLS.lock().unwrap())
}

/// A value the service chose for the field it knows by `virtual_id`, called on the Java
/// UI thread. Queued, and the loop woken: fields are edited on the shell's thread only.
pub(crate) extern "system" fn native_autofill(
    mut env: JNIEnv,
    _class: JClass,
    virtual_id: jint,
    value: JString,
) {
    let value: String = env.get_string(&value).map(Into::into).unwrap_or_default();
    FILLS.lock().unwrap().push((virtual_id, value));
    crate::android_ime::wake();
}

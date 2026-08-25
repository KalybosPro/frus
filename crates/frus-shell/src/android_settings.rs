//! What **Android** says about the person using the device.
//!
//! The font-size slider, the night setting, reduce-motion, the screen reader, the clock
//! format. Milestone 403 made the framework scale text by the reader's setting and
//! milestone 406 made every widget obey it — and both were inert on a device, because
//! nothing carried the number from the platform. This is that wire.
//!
//! It needs **no dex and no bridge class**, unlike [`crate::android_ime`]: everything here
//! is a public field or a public method on `Configuration`, `AccessibilityManager`,
//! `DateFormat` or `Settings`. It is one JNI walk from the activity, done on the main
//! thread when the surface is described.
//!
//! **Every read is best-effort.** A setting that cannot be read leaves its default rather
//! than failing the frame: a phone whose OEM removed a table is a phone that still has to
//! run. What is *not* acceptable is reading a wrong value silently, so each read either
//! produces the platform's answer or produces nothing at all.
//!
//! # Every failure must be cleared, not merely ignored
//!
//! A failed JNI call returns an `Err` on the Rust side **and leaves the Java exception
//! pending on the thread**. Dropping the `Err` does not drop the exception: the next JNI
//! call, however unrelated, aborts the entire runtime with *"No pending exception
//! expected"*. Not a missing setting — a crash on launch.
//!
//! That is not hypothetical. The first run of this module on a device died instantly:
//! `Configuration.fontWeightAdjustment` arrived in API 31, the test phone is API 29, and
//! the `NoSuchFieldError` it threw took the process down two calls later. It compiled
//! perfectly, because JNI resolves names at runtime and a compiler has nothing to check.
//!
//! So every fallible read here goes through a pair: an inner function returning a `Result`
//! that holds no borrow of the environment, and an outer one that clears the exception
//! before handing back `None`.

use frus_widgets::{Accessibility, Brightness};

use crate::app::PlatformSettings;
use jni::objects::{JObject, JValue};
use jni::{JNIEnv, JavaVM};
use winit::platform::android::activity::AndroidApp;

/// `UI_MODE_NIGHT_MASK` and `UI_MODE_NIGHT_YES` from `android.content.res.Configuration`.
const UI_MODE_NIGHT_MASK: i32 = 0x30;
const UI_MODE_NIGHT_YES: i32 = 0x20;

/// Clears any Java exception left pending on this thread.
///
/// Call it after **every** swallowed JNI failure. See the module documentation: an
/// exception that is ignored rather than cleared takes the process down on the next call.
fn clear_pending(env: &mut JNIEnv) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

/// Reads the current settings, or the defaults if the walk fails at any point.
pub(crate) fn read(app: &AndroidApp) -> PlatformSettings {
    match try_read(app) {
        Ok(settings) => settings,
        Err(err) => {
            log::warn!("platform settings unavailable ({err}) — using the neutral defaults");
            // The walk gave up part-way, which means a call threw and its exception is
            // still pending. Leaving it would abort the runtime on the next JNI call
            // anybody makes — the IME bridge's, for instance.
            if let Ok(vm) = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) } {
                if let Ok(mut env) = vm.attach_current_thread_permanently() {
                    clear_pending(&mut env);
                }
            }
            PlatformSettings::default()
        }
    }
}

/// The device's API level, or `0` when even that cannot be read.
///
/// Used to **not ask** for a field that does not exist yet, rather than to ask and recover:
/// recovering works, but every throw is a chance to leave an exception behind, and the
/// version is one cheap read that removes the whole class of them.
fn sdk_int(env: &mut JNIEnv) -> i32 {
    match env
        .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
        .and_then(|v| v.i())
    {
        Ok(level) => level,
        Err(_) => {
            clear_pending(env);
            0
        }
    }
}

fn try_read(app: &AndroidApp) -> Result<PlatformSettings, jni::errors::Error> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }?;
    let mut env = vm.attach_current_thread_permanently()?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };

    // `activity.getResources().getConfiguration()` — the one object that answers the
    // font size, the night setting and the bold-text weight.
    let resources = env
        .call_method(
            &activity,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        )?
        .l()?;
    let config = env
        .call_method(
            &resources,
            "getConfiguration",
            "()Landroid/content/res/Configuration;",
            &[],
        )?
        .l()?;

    // A public `float` field, not a getter. Disbelieved if it is zero or negative: a
    // scale of nothing would make every line of text vanish, and a device reporting one
    // is a device to ignore rather than obey.
    let text_scaler = match env.get_field(&config, "fontScale", "F").and_then(|v| v.f()) {
        Ok(scale) if scale > 0.0 => scale,
        Ok(_) => 1.0,
        Err(_) => {
            clear_pending(&mut env);
            1.0
        }
    };

    let ui_mode = env.get_field(&config, "uiMode", "I").and_then(|v| v.i())?;
    let brightness = if ui_mode & UI_MODE_NIGHT_MASK == UI_MODE_NIGHT_YES {
        Brightness::Dark
    } else {
        Brightness::Light
    };

    // `fontWeightAdjustment` arrived in **API 31**. Below that the field does not exist and
    // asking for it throws `NoSuchFieldError` — which is how the first device run of this
    // module killed the process. The version is checked instead of the throw being caught;
    // `UNDEFINED_FONT_WEIGHT_ADJUSTMENT` is `Integer.MIN_VALUE` and means the same "not
    // asked for" as zero does.
    let bold_text = if sdk_int(&mut env) >= 31 {
        match env
            .get_field(&config, "fontWeightAdjustment", "I")
            .and_then(|v| v.i())
        {
            Ok(adjustment) => adjustment != 0 && adjustment != i32::MIN,
            Err(_) => {
                clear_pending(&mut env);
                false
            }
        }
    } else {
        false
    };

    let accessibility = Accessibility {
        bold_text,
        high_contrast: secure_flag(&mut env, &activity, "high_text_contrast_enabled")
            .unwrap_or(false),
        disable_animations: animations_off(&mut env, &activity).unwrap_or(false),
        invert_colors: secure_flag(
            &mut env,
            &activity,
            "accessibility_display_inversion_enabled",
        )
        .unwrap_or(false),
        accessible_navigation: touch_exploration(&mut env, &activity).unwrap_or(false),
        always_use_24_hour_format: is_24_hour(&mut env, &activity).unwrap_or(false),
    };

    Ok(PlatformSettings {
        text_scaler,
        brightness,
        accessibility,
    })
}

/// `activity.getContentResolver()`.
fn content_resolver<'a>(
    env: &mut JNIEnv<'a>,
    activity: &JObject,
) -> Result<JObject<'a>, jni::errors::Error> {
    env.call_method(
        activity,
        "getContentResolver",
        "()Landroid/content/ContentResolver;",
        &[],
    )?
    .l()
}

/// One `Settings.Secure` integer flag, `true` when it is non-zero.
///
/// The two this reads — high-contrast text and colour inversion — have no public constant
/// naming them, so the key is a literal. That is why the result is an `Option` and why a
/// failure is not an error: the framework reports what it can read and says false for the
/// rest, rather than claiming a setting is off when it simply could not look.
fn secure_flag(env: &mut JNIEnv, activity: &JObject, key: &str) -> Option<bool> {
    let read = try_secure_flag(env, activity, key);
    if read.is_err() {
        clear_pending(env);
    }
    read.ok()
}

fn try_secure_flag(
    env: &mut JNIEnv,
    activity: &JObject,
    key: &str,
) -> Result<bool, jni::errors::Error> {
    let resolver = content_resolver(env, activity)?;
    let name = env.new_string(key)?;
    let value = env
        .call_static_method(
            "android/provider/Settings$Secure",
            "getInt",
            "(Landroid/content/ContentResolver;Ljava/lang/String;I)I",
            &[
                JValue::Object(&resolver),
                JValue::Object(&name),
                JValue::Int(0),
            ],
        )?
        .i()?;
    Ok(value != 0)
}

/// Whether the user has turned animations off, which Android expresses as an animator
/// duration **scale** of zero rather than as a boolean.
fn animations_off(env: &mut JNIEnv, activity: &JObject) -> Option<bool> {
    let read = try_animations_off(env, activity);
    if read.is_err() {
        clear_pending(env);
    }
    read.ok()
}

fn try_animations_off(env: &mut JNIEnv, activity: &JObject) -> Result<bool, jni::errors::Error> {
    let resolver = content_resolver(env, activity)?;
    let name = env.new_string("animator_duration_scale")?;
    let scale = env
        .call_static_method(
            "android/provider/Settings$Global",
            "getFloat",
            "(Landroid/content/ContentResolver;Ljava/lang/String;F)F",
            &[
                JValue::Object(&resolver),
                JValue::Object(&name),
                JValue::Float(1.0),
            ],
        )?
        .f()?;
    Ok(scale == 0.0)
}

/// Whether something is driving the screen without a pointer — TalkBack and its kin.
///
/// `isTouchExplorationEnabled`, not `isEnabled`: the second is true for services that
/// merely observe, and an interface that hid its hover-only controls for one of those
/// would be hiding them from a user who can still hover.
fn touch_exploration(env: &mut JNIEnv, activity: &JObject) -> Option<bool> {
    let read = try_touch_exploration(env, activity);
    if read.is_err() {
        clear_pending(env);
    }
    read.ok().flatten()
}

fn try_touch_exploration(
    env: &mut JNIEnv,
    activity: &JObject,
) -> Result<Option<bool>, jni::errors::Error> {
    let name = env.new_string("accessibility")?;
    let manager = env
        .call_method(
            activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&name)],
        )?
        .l()?;
    if manager.is_null() {
        return Ok(None);
    }
    Ok(Some(
        env.call_method(&manager, "isTouchExplorationEnabled", "()Z", &[])?
            .z()?,
    ))
}

/// The user's clock format, which `DateFormat` answers for the whole device.
fn is_24_hour(env: &mut JNIEnv, activity: &JObject) -> Option<bool> {
    let read = env
        .call_static_method(
            "android/text/format/DateFormat",
            "is24HourFormat",
            "(Landroid/content/Context;)Z",
            &[JValue::Object(activity)],
        )
        .and_then(|v| v.z());
    match read {
        Ok(value) => Some(value),
        Err(_) => {
            clear_pending(env);
            None
        }
    }
}

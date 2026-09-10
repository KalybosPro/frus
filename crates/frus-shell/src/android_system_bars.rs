//! The **system bars** on Android (#46): their colour and the brightness of their icons.
//!
//! What to ask for is decided on the Rust side — the regions under each bar, then the
//! theme (`frus_widgets::SystemUiOverlayStyle`). This only carries the answer across, to
//! `FrusSystemBars.apply` in the bundled dex, which makes the calls on the Java UI thread:
//! the window and decor view calls check the thread they are made on, and the native code
//! runs on its own.
//!
//! **Which calls, for which versions.** The colours are `Window.setStatusBarColor` and
//! `setNavigationBarColor` everywhere this builds for (API 24 and up), with the window told
//! to draw the bars' backgrounds. The icons are the `WindowInsetsController` appearance
//! flags from API 30, and the decor view's light-bar flags before it (the navigation bar's
//! from API 26). From API 35 an application targeting it is drawn edge to edge and the
//! colour setters do nothing; this one targets 34, and the icon half is the part that
//! survives that change.
//!
//! **Best-effort, like the other bridges.** A call that fails leaves the bars as they were,
//! with a log line, and clears the Java exception it left — an exception left pending takes
//! the process down on the next JNI call anybody makes (see `android_settings`).

use std::sync::OnceLock;

use frus_widgets::{Brightness, Color, SystemBars};
use jni::objects::{GlobalRef, JClass, JObject, JValue};
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

/// Loads `FrusSystemBars` from the bundled dex, through the loader the input bridge made.
///
/// A failure here costs the system bars only: the input bridge is installed either way.
pub(crate) fn install(env: &mut JNIEnv, loader: &JObject, activity: &JObject) {
    match try_install(env, loader, activity) {
        Ok(handle) => {
            let _ = HANDLE.set(handle);
        }
        Err(err) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            log::warn!("system bars unavailable ({err}) — the platform's defaults stay");
        }
    }
}

fn try_install(
    env: &mut JNIEnv,
    loader: &JObject,
    activity: &JObject,
) -> Result<Handle, jni::errors::Error> {
    let name = env.new_string("dev.frus.input.FrusSystemBars")?;
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

/// A colour the way Android spells one: `0xAARRGGBB`, in sRGB, which is what `Color` holds.
fn argb(color: Color) -> i32 {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    ((byte(color.a) << 24) | (byte(color.r) << 16) | (byte(color.g) << 8) | byte(color.b)) as i32
}

/// Asks for `bars`. Returns whether the request reached the Java side, so that the shell
/// asks again next frame rather than believing a request that was never made.
pub(crate) fn apply(bars: SystemBars) -> bool {
    let Some(handle) = HANDLE.get() else {
        return false;
    };
    let Ok(mut env) = handle.vm.attach_current_thread_permanently() else {
        return false;
    };
    let class: &JClass = handle.class.as_obj().into();
    let called = env.call_static_method(
        class,
        "apply",
        "(Landroid/app/Activity;IIZZ)V",
        &[
            JValue::Object(handle.activity.as_obj()),
            JValue::Int(argb(bars.status_bar_color)),
            JValue::Int(argb(bars.navigation_bar_color)),
            JValue::Bool((bars.status_bar_icons == Brightness::Dark) as u8),
            JValue::Bool((bars.navigation_bar_icons == Brightness::Dark) as u8),
        ],
    );
    match called {
        Ok(_) => true,
        Err(err) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            log::warn!("system bars: apply failed ({err})");
            false
        }
    }
}

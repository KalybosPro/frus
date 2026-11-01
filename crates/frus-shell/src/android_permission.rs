//! **Permissions on Android** (milestone 730): [`crate::Permission`] through `FrusPermissions`
//! in the bundled dex.
//!
//! The status is the system's own (`checkSelfPermission`), told apart further by whether the
//! system would explain itself before asking again and whether this application asked before.
//! A request asks with `requestPermissions` and waits for the answer the way notifications do
//! (milestone 705): a NativeActivity is not told it, so a yes is read from the permission and a
//! no from the question's dialog going away — the window getting its focus back. Waiting
//! happens on a thread made for it, never on the interface's.
//!
//! As with the other bridges, every failure clears the Java exception it left, and a bridge
//! that did not load answers [`PermissionStatus::Unavailable`].

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use jni::objects::{GlobalRef, JClass, JObject, JObjectArray, JValue};
use jni::{JNIEnv, JavaVM};

use crate::permission::{android_names, combine, from_android, question, Question};
use crate::{Permission, PermissionStatus};

struct Handle {
    vm: JavaVM,
    class: GlobalRef,
    activity: GlobalRef,
}

// SAFETY: JavaVM and the GlobalRefs are shareable across threads, per the JNI contract.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

static HANDLE: OnceLock<Handle> = OnceLock::new();

/// How often the permission is looked at while its question is up.
const POLL: Duration = Duration::from_millis(200);

/// Loads `FrusPermissions` from the bundled dex, through the loader the input bridge made. A
/// failure costs the permissions only.
pub(crate) fn install(env: &mut JNIEnv, loader: &JObject, activity: &JObject) {
    match try_install(env, loader, activity) {
        Ok(handle) => {
            let _ = HANDLE.set(handle);
        }
        Err(err) => {
            clear_exception(env);
            log::warn!("permissions unavailable ({err})");
        }
    }
}

fn try_install(
    env: &mut JNIEnv,
    loader: &JObject,
    activity: &JObject,
) -> Result<Handle, jni::errors::Error> {
    let name = env.new_string("dev.frus.input.FrusPermissions")?;
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

/// Runs `work` with the bridge, on the calling thread; `None` if the bridge did not load or JNI
/// failed.
fn with_bridge<T>(
    work: impl FnOnce(&mut JNIEnv, &JClass, &JObject) -> Result<T, jni::errors::Error>,
) -> Option<T> {
    let handle = HANDLE.get()?;
    let mut env = handle.vm.attach_current_thread_permanently().ok()?;
    let class: &JClass = handle.class.as_obj().into();
    let result = work(&mut env, class, handle.activity.as_obj());
    if result.is_err() {
        clear_exception(&mut env);
    }
    result.ok()
}

fn names<'a>(
    env: &mut JNIEnv<'a>,
    permission: Permission,
) -> Result<JObjectArray<'a>, jni::errors::Error> {
    let wanted = android_names(permission);
    let array = env.new_object_array(wanted.len() as i32, "java/lang/String", JObject::null())?;
    for (index, name) in wanted.iter().enumerate() {
        let name = env.new_string(name)?;
        env.set_object_array_element(&array, index as i32, name)?;
    }
    Ok(array)
}

fn code(
    env: &mut JNIEnv,
    class: &JClass,
    activity: &JObject,
    permission: Permission,
) -> Result<i32, jni::errors::Error> {
    // Each Android permission on its own: the location is granted by either of its two, so
    // the codes are combined by `combine` rather than by the Java side's "every one".
    let mut codes = Vec::new();
    for name in android_names(permission) {
        let one = env.new_object_array(1, "java/lang/String", JObject::null())?;
        let name = env.new_string(name)?;
        env.set_object_array_element(&one, 0, name)?;
        codes.push(
            env.call_static_method(
                class,
                "status",
                "(Landroid/app/Activity;[Ljava/lang/String;)I",
                &[JValue::Object(activity), JValue::Object(&one)],
            )?
            .i()?,
        );
    }
    Ok(combine(permission, &codes))
}

/// Where `permission` stands.
pub(crate) fn status(permission: Permission) -> PermissionStatus {
    // Notifications before Android 13 need no permission: the manifest does not name it, and
    // they are on unless the person turned them off for the application.
    if permission == Permission::Notifications && sdk() < 33 {
        return PermissionStatus::Granted;
    }
    let answer = with_bridge(|env, class, activity| code(env, class, activity, permission));
    match answer {
        Some(-3) => {
            log::warn!(
                "{permission} is refused without asking: the application's manifest does not ask \
                 for {}",
                android_names(permission).join(" and ")
            );
            PermissionStatus::Blocked
        }
        Some(code) => from_android(code),
        None => PermissionStatus::Unavailable,
    }
}

/// Asks for `permission` when there is something to ask, and waits for the answer — on a
/// thread made for waiting.
pub(crate) async fn request(permission: Permission) -> PermissionStatus {
    blocking::unblock(move || request_blocking(permission)).await
}

fn request_blocking(permission: Permission) -> PermissionStatus {
    let before = status(permission);
    if !matches!(before, PermissionStatus::Ask | PermissionStatus::Denied) {
        return before;
    }
    let asked = with_bridge(|env, class, activity| {
        let names = names(env, permission)?;
        env.call_static_method(
            class,
            "ask",
            "(Landroid/app/Activity;[Ljava/lang/String;)V",
            &[JValue::Object(activity), JValue::Object(&names)],
        )
        .map(|_| ())
    });
    if asked.is_none() {
        return PermissionStatus::Unavailable;
    }
    let started = Instant::now();
    let mut was_away = false;
    loop {
        let seen = with_bridge(|env, class, activity| {
            let granted = code(env, class, activity, permission)? == 1;
            let focused = env
                .call_static_method(
                    class,
                    "focused",
                    "(Landroid/app/Activity;)Z",
                    &[JValue::Object(activity)],
                )?
                .z()?;
            Ok((granted, focused))
        });
        let Some((granted, focused)) = seen else {
            return PermissionStatus::Unavailable;
        };
        was_away |= !focused;
        match question(i32::from(granted), focused, was_away, started.elapsed()) {
            Question::Open => std::thread::sleep(POLL),
            Question::Settled | Question::Unanswered => break,
        }
    }
    status(permission)
}

/// Opens the application's page in the system's settings.
pub(crate) fn open_settings() {
    let opened = with_bridge(|env, class, activity| {
        env.call_static_method(
            class,
            "openSettings",
            "(Landroid/app/Activity;)V",
            &[JValue::Object(activity)],
        )
        .map(|_| ())
    });
    if opened.is_none() {
        log::warn!("the application's settings could not be opened");
    }
}

/// The device's Android version, as its API level.
fn sdk() -> i32 {
    with_bridge(|env, _, _| {
        env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
            .i()
    })
    .unwrap_or(0)
}

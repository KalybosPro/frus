//! The **Android input bridge** (§6, tier 2) — the `InputConnection` NativeActivity
//! lacks.
//!
//! Without one, IMEs run in a degraded mode (`TYPE_NULL`): Latin keys only, with no
//! composition, no swipe, no suggestions and no CJK. The way out is a Java `View`
//! that supplies a real `InputConnection` wired to the engine, and we take that route
//! without Gradle:
//!
//! 1. `FrusTextBridge.java` — a 1×1 View plus a `BaseInputConnection` — is
//!    precompiled into a **bundled dex** (`assets/frus_input.dex`, built by
//!    scripts/build-input-dex.sh);
//! 2. at startup the dex is loaded through `InMemoryDexClassLoader`, the `native*`
//!    methods are wired up by `RegisterNatives`, and the view is added to the
//!    activity;
//! 3. every IME operation — commit, composition, deletion, action — arrives on the
//!    Java UI thread → is pushed onto a shared **queue** and the winit loop is woken
//!    (`AndroidAppWaker`) → the shell drains it in `new_events` and applies it to the
//!    focused field.
//!
//! IME focus, meaning opening and closing the keyboard, goes through
//! [`start_input`]/[`stop_input`]: the bridge view takes Java focus, the IME talks to
//! it, and the native content keeps receiving touch.

use std::sync::{Mutex, OnceLock};

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jint, JNI_TRUE};
use jni::{JNIEnv, JavaVM};
use winit::platform::android::activity::AndroidApp;

/// An input operation relayed by the IME; arrival order is preserved.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ImeEvent {
    /// **Final** text: a plain keystroke, a swipe, a chosen suggestion, an emoji.
    Commit(String),
    /// Text **being composed**, replacing the previous composition.
    Composing(String),
    /// The current composition becomes final, as it stands.
    FinishComposing,
    /// Deletes `before` characters ahead of the cursor and `after` behind it.
    Delete { before: u32, after: u32 },
    /// An editor action: the keyboard's Enter, OK or Search.
    Action,
    /// A key relayed by the IME (`sendKeyEvent`), already filtered on the Java side.
    Key { code: i32, unicode: u32 },
}

/// The operation queue, filled on the Java UI thread and drained by the shell.
static QUEUE: Mutex<Vec<ImeEvent>> = Mutex::new(Vec::new());
/// Wakes the winit loop when the queue fills.
static WAKER: OnceLock<Mutex<winit::platform::android::activity::AndroidAppWaker>> =
    OnceLock::new();
/// What it takes to cross over: the VM and the bridge class, as global refs, for
/// start and stop.
static BRIDGE: OnceLock<Bridge> = OnceLock::new();

struct Bridge {
    vm: JavaVM,
    class: GlobalRef,
    activity: GlobalRef,
}

// SAFETY: JavaVM and the GlobalRefs are shareable across threads, per the JNI contract.
unsafe impl Send for Bridge {}
unsafe impl Sync for Bridge {}

/// A snapshot of the focused field's editing state — the context the IME queries:
/// the text before and after the cursor, and the selection. Written by the shell,
/// read by the natives on the Java UI thread.
#[derive(Default)]
struct EditorState {
    /// The field's characters; indices count **characters**, not bytes.
    chars: Vec<char>,
    cursor: usize,
    selection: Option<(usize, usize)>,
}

static EDITOR: Mutex<Option<EditorState>> = Mutex::new(None);

/// Pushes the focused field's editing state, called by the shell on every change.
/// It is the context the IME's suggestions draw on.
pub(crate) fn set_editor_state(text: &str, cursor: usize, selection: Option<(usize, usize)>) {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    *EDITOR.lock().unwrap() = Some(EditorState {
        chars,
        cursor,
        selection,
    });
}

/// Clears the context, focus having left the text fields.
pub(crate) fn clear_editor_state() {
    *EDITOR.lock().unwrap() = None;
}

/// The bridge's dex, built by `scripts/build-input-dex.sh` and kept in the repo.
static BRIDGE_DEX: &[u8] = include_bytes!("../assets/frus_input.dex");

/// Installs the bridge: loads the dex, wires the natives, adds the view. A failing
/// step leaves everything untouched, with a log line, and the shell then falls back
/// to `TYPE_NULL` mode, Latin keys only.
pub(crate) fn install(app: &AndroidApp) {
    if BRIDGE.get().is_some() {
        return;
    }
    match try_install(app) {
        Ok(bridge) => {
            let _ = BRIDGE.set(bridge);
            let _ = WAKER.set(Mutex::new(app.create_waker()));
            log::info!("input bridge installed (a real InputConnection)");
        }
        Err(err) => log::warn!("input bridge unavailable ({err}) — falling back to key mode"),
    }
}

fn try_install(app: &AndroidApp) -> Result<Bridge, jni::errors::Error> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }?;
    let mut env = vm.attach_current_thread_permanently()?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };

    // 1. Load the bundled dex into an in-memory class loader.
    let buffer =
        unsafe { env.new_direct_byte_buffer(BRIDGE_DEX.as_ptr() as *mut u8, BRIDGE_DEX.len()) }?;
    let parent = env
        .call_method(
            &activity,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )?
        .l()?;
    let loader = env.new_object(
        "dalvik/system/InMemoryDexClassLoader",
        "(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V",
        &[JValue::Object(&buffer), JValue::Object(&parent)],
    )?;
    let name = env.new_string("dev.frus.input.FrusTextBridge")?;
    let class_obj = env
        .call_method(
            &loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&name)],
        )?
        .l()?;
    let class: JClass = class_obj.into();

    // 2. Wire up the bridge class's native methods.
    use jni::NativeMethod;
    let method = |name: &str, sig: &str, ptr: *mut std::ffi::c_void| NativeMethod {
        name: name.into(),
        sig: sig.into(),
        fn_ptr: ptr,
    };
    env.register_native_methods(
        &class,
        &[
            method(
                "nativeCommit",
                "(Ljava/lang/String;)V",
                native_commit as *mut _,
            ),
            method(
                "nativeSetComposing",
                "(Ljava/lang/String;)V",
                native_set_composing as *mut _,
            ),
            method(
                "nativeFinishComposing",
                "()V",
                native_finish_composing as *mut _,
            ),
            method("nativeDelete", "(II)V", native_delete as *mut _),
            method("nativeEditorAction", "(I)V", native_editor_action as *mut _),
            method("nativeKey", "(IZII)Z", native_key as *mut _),
            method(
                "nativeTextBeforeCursor",
                "(I)Ljava/lang/String;",
                native_text_before as *mut _,
            ),
            method(
                "nativeTextAfterCursor",
                "(I)Ljava/lang/String;",
                native_text_after as *mut _,
            ),
            method(
                "nativeSelectedText",
                "()Ljava/lang/String;",
                native_selected_text as *mut _,
            ),
        ],
    )?;

    // 3. Add the bridge view to the activity, posted on the Java UI thread.
    env.call_static_method(
        &class,
        "install",
        "(Landroid/app/Activity;)V",
        &[JValue::Object(&activity)],
    )?;

    let class_ref = env.new_global_ref(&class)?;
    let activity_ref = env.new_global_ref(&activity)?;
    Ok(Bridge {
        vm,
        class: class_ref,
        activity: activity_ref,
    })
}

/// Is the bridge operational?
pub(crate) fn installed() -> bool {
    BRIDGE.get().is_some()
}

/// Native focus enters a text field: the bridge view captures the IME, told what
/// kind of field it is.
///
/// The two numbers are computed on this side rather than chosen on the Java one, and
/// that is the point: the bridge's dex is **checked in**, so rebuilding it needs the
/// Android SDK. A Java file that only ever sets two integers on the `EditorInfo` never
/// has to be rebuilt again for a keyboard type we have not thought of yet.
pub(crate) fn start_input(ime: frus_widgets::Ime) {
    let Some(bridge) = BRIDGE.get() else {
        return;
    };
    let input_type = ime.keyboard.android_input_type();
    let ime_options = ime.action.android_ime_options();
    let result = bridge
        .vm
        .attach_current_thread_permanently()
        .and_then(|mut env| {
            let class: &JClass = bridge.class.as_obj().into();
            env.call_static_method(
                class,
                "startInput",
                "(Landroid/app/Activity;II)V",
                &[
                    JValue::Object(bridge.activity.as_obj()),
                    JValue::Int(input_type),
                    JValue::Int(ime_options),
                ],
            )
            .map(|_| ())
        });
    if let Err(err) = result {
        log::warn!("input bridge: startInput failed ({err})");
    }
}

/// Native focus leaves the text fields, and the IME closes.
pub(crate) fn stop_input() {
    call_bridge("stopInput");
}

fn call_bridge(method: &str) {
    let Some(bridge) = BRIDGE.get() else {
        return;
    };
    let result = bridge
        .vm
        .attach_current_thread_permanently()
        .and_then(|mut env| {
            let class: &JClass = bridge.class.as_obj().into();
            env.call_static_method(
                class,
                method,
                "(Landroid/app/Activity;)V",
                &[JValue::Object(bridge.activity.as_obj())],
            )
            .map(|_| ())
        });
    if let Err(err) = result {
        log::warn!("input bridge: {method} failed ({err})");
    }
}

/// Drains the pending operations, in arrival order.
pub(crate) fn drain() -> Vec<ImeEvent> {
    std::mem::take(&mut QUEUE.lock().unwrap())
}

fn push(event: ImeEvent) {
    QUEUE.lock().unwrap().push(event);
    if let Some(waker) = WAKER.get() {
        waker.lock().unwrap().wake();
    }
}

// --- Natives, called on the Java UI thread -----------------------------------

fn jstring_to_string(env: &mut JNIEnv, text: &JString) -> String {
    env.get_string(text).map(Into::into).unwrap_or_default()
}

extern "system" fn native_commit(mut env: JNIEnv, _class: JClass, text: JString) {
    let text = jstring_to_string(&mut env, &text);
    push(ImeEvent::Commit(text));
}

extern "system" fn native_set_composing(mut env: JNIEnv, _class: JClass, text: JString) {
    let text = jstring_to_string(&mut env, &text);
    push(ImeEvent::Composing(text));
}

extern "system" fn native_finish_composing(_env: JNIEnv, _class: JClass) {
    push(ImeEvent::FinishComposing);
}

extern "system" fn native_delete(_env: JNIEnv, _class: JClass, before: jint, after: jint) {
    push(ImeEvent::Delete {
        before: before.max(0) as u32,
        after: after.max(0) as u32,
    });
}

extern "system" fn native_editor_action(_env: JNIEnv, _class: JClass, _action: jint) {
    push(ImeEvent::Action);
}

/// A synchronous filter: `true` means an editing key was consumed and queued,
/// `false` means the key follows the default path — Back, navigation, and so on.
extern "system" fn native_key(
    _env: JNIEnv,
    _class: JClass,
    code: jint,
    down: jboolean,
    unicode: jint,
    _meta: jint,
) -> jboolean {
    const KEYCODE_ENTER: jint = 66;
    const KEYCODE_DEL: jint = 67;
    if down != JNI_TRUE {
        // The releases of consumed keys are consumed too, for symmetry.
        return matches!(code, KEYCODE_ENTER | KEYCODE_DEL) as jboolean | (unicode > 0) as jboolean;
    }
    let handled = matches!(code, KEYCODE_ENTER | KEYCODE_DEL) || unicode > 0;
    if handled {
        push(ImeEvent::Key {
            code,
            unicode: unicode.max(0) as u32,
        });
    }
    handled as jboolean
}

// --- The input context, read by the IME through the InputConnection ----------

/// Returns a Java `jstring`, or an empty string, without propagating an error.
fn to_jstring(env: &JNIEnv, text: &str) -> jni::sys::jstring {
    env.new_string(text)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

extern "system" fn native_text_before(env: JNIEnv, _class: JClass, n: jint) -> jni::sys::jstring {
    let guard = EDITOR.lock().unwrap();
    let text = guard
        .as_ref()
        .map(|s| {
            let n = (n.max(0) as usize).min(s.cursor);
            s.chars[s.cursor - n..s.cursor].iter().collect::<String>()
        })
        .unwrap_or_default();
    to_jstring(&env, &text)
}

extern "system" fn native_text_after(env: JNIEnv, _class: JClass, n: jint) -> jni::sys::jstring {
    let guard = EDITOR.lock().unwrap();
    let text = guard
        .as_ref()
        .map(|s| {
            let end = (s.cursor + n.max(0) as usize).min(s.chars.len());
            s.chars[s.cursor..end].iter().collect::<String>()
        })
        .unwrap_or_default();
    to_jstring(&env, &text)
}

extern "system" fn native_selected_text(env: JNIEnv, _class: JClass) -> jni::sys::jstring {
    let guard = EDITOR.lock().unwrap();
    let text = guard
        .as_ref()
        .and_then(|s| {
            let (a, b) = s.selection?;
            let (a, b) = (a.min(s.chars.len()), b.min(s.chars.len()));
            (a < b).then(|| s.chars[a..b].iter().collect::<String>())
        })
        .unwrap_or_default();
    to_jstring(&env, &text)
}

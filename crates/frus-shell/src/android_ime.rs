//! **Pont de saisie Android** (§6, palier 2) — l'`InputConnection` qui manque
//! à NativeActivity.
//!
//! Sans elle, les IME tournent en mode dégradé (`TYPE_NULL`) : touches
//! latines seulement — ni composition, ni swipe, ni suggestions, ni CJK.
//! L'embedding Flutter résout ça avec une `View` Java qui fournit une vraie
//! `InputConnection` reliée au moteur ; on suit le même chemin, sans Gradle :
//!
//! 1. `FrusTextBridge.java` (View 1×1 + `BaseInputConnection`) est précompilée
//!    en **dex embarqué** (`assets/frus_input.dex`, scripts/build-input-dex.sh) ;
//! 2. au démarrage, le dex est chargé via `InMemoryDexClassLoader`, les
//!    méthodes `native*` sont branchées par `RegisterNatives`, et la view est
//!    ajoutée à l'activité ;
//! 3. chaque opération de l'IME (commit, composition, suppression, action)
//!    arrive sur le thread UI Java → poussée dans une **file** partagée +
//!    réveil de la boucle winit (`AndroidAppWaker`) → le shell la draine dans
//!    `new_events` et l'applique au champ focalisé.
//!
//! Le focus IME (ouverture/fermeture du clavier) passe par
//! [`start_input`]/[`stop_input`] — la view pont prend le focus Java, l'IME
//! lui parle, le contenu natif continue de recevoir le tactile.

use std::sync::{Mutex, OnceLock};

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jint, JNI_TRUE};
use jni::{JNIEnv, JavaVM};
use winit::platform::android::activity::AndroidApp;

/// Une opération de saisie relayée par l'IME (ordre d'arrivée préservé).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ImeEvent {
    /// Texte **définitif** (frappe simple, swipe, suggestion choisie, émoji).
    Commit(String),
    /// Texte **en composition** (remplace la composition précédente).
    Composing(String),
    /// La composition courante devient définitive (telle quelle).
    FinishComposing,
    /// Suppression de `before` caractères avant le curseur, `after` après.
    Delete { before: u32, after: u32 },
    /// Action d'éditeur (Entrée/OK/Recherche du clavier).
    Action,
    /// Touche relayée par l'IME (`sendKeyEvent`) : déjà filtrée côté Java.
    Key { code: i32, unicode: u32 },
}

/// File des opérations (remplie sur le thread UI Java, drainée par le shell).
static QUEUE: Mutex<Vec<ImeEvent>> = Mutex::new(Vec::new());
/// Réveil de la boucle winit quand la file se remplit.
static WAKER: OnceLock<Mutex<winit::platform::android::activity::AndroidAppWaker>> =
    OnceLock::new();
/// La machine à voyager : VM + classe pont (refs globales), pour start/stop.
static BRIDGE: OnceLock<Bridge> = OnceLock::new();

struct Bridge {
    vm: JavaVM,
    class: GlobalRef,
    activity: GlobalRef,
}

// SÛRETÉ : JavaVM et les GlobalRef sont partageables entre threads (contrat JNI).
unsafe impl Send for Bridge {}
unsafe impl Sync for Bridge {}

/// Instantané de l'état d'édition du champ focalisé — le contexte que l'IME
/// interroge (texte avant/après le curseur, sélection). Mis à jour par le
/// shell, lu par les natives sur le thread UI Java.
#[derive(Default)]
struct EditorState {
    /// Caractères du champ (indices en **caractères**, pas en octets).
    chars: Vec<char>,
    cursor: usize,
    selection: Option<(usize, usize)>,
}

static EDITOR: Mutex<Option<EditorState>> = Mutex::new(None);

/// Pousse l'état d'édition du champ focalisé (appelé par le shell à chaque
/// changement) — sert de contexte aux suggestions de l'IME.
pub(crate) fn set_editor_state(text: &str, cursor: usize, selection: Option<(usize, usize)>) {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    *EDITOR.lock().unwrap() = Some(EditorState {
        chars,
        cursor,
        selection,
    });
}

/// Efface le contexte (le focus quitte les champs texte).
pub(crate) fn clear_editor_state() {
    *EDITOR.lock().unwrap() = None;
}

/// Le dex du pont, compilé par `scripts/build-input-dex.sh` et versionné.
static BRIDGE_DEX: &[u8] = include_bytes!("../assets/frus_input.dex");

/// Installe le pont : charge le dex, branche les natives, ajoute la view.
/// Sans effet (avec log) si une étape échoue — le shell retombe alors sur le
/// mode `TYPE_NULL` (touches latines).
pub(crate) fn install(app: &AndroidApp) {
    if BRIDGE.get().is_some() {
        return;
    }
    match try_install(app) {
        Ok(bridge) => {
            let _ = BRIDGE.set(bridge);
            let _ = WAKER.set(Mutex::new(app.create_waker()));
            log::info!("pont de saisie installé (InputConnection réelle)");
        }
        Err(err) => log::warn!("pont de saisie indisponible ({err}) — mode touches"),
    }
}

fn try_install(app: &AndroidApp) -> Result<Bridge, jni::errors::Error> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }?;
    let mut env = vm.attach_current_thread_permanently()?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };

    // 1. Charge le dex embarqué dans un class loader en mémoire.
    let buffer = unsafe {
        env.new_direct_byte_buffer(BRIDGE_DEX.as_ptr() as *mut u8, BRIDGE_DEX.len())
    }?;
    let parent = env
        .call_method(&activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
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

    // 2. Branche les méthodes natives de la classe pont.
    use jni::NativeMethod;
    let method = |name: &str, sig: &str, ptr: *mut std::ffi::c_void| NativeMethod {
        name: name.into(),
        sig: sig.into(),
        fn_ptr: ptr,
    };
    env.register_native_methods(
        &class,
        &[
            method("nativeCommit", "(Ljava/lang/String;)V", native_commit as *mut _),
            method("nativeSetComposing", "(Ljava/lang/String;)V", native_set_composing as *mut _),
            method("nativeFinishComposing", "()V", native_finish_composing as *mut _),
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

    // 3. Ajoute la view pont à l'activité (posté sur le thread UI Java).
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

/// Le pont est-il opérationnel ?
pub(crate) fn installed() -> bool {
    BRIDGE.get().is_some()
}

/// Le focus natif entre dans un champ texte : la view pont capte l'IME.
pub(crate) fn start_input() {
    call_bridge("startInput");
}

/// Le focus natif quitte les champs texte : IME refermé.
pub(crate) fn stop_input() {
    call_bridge("stopInput");
}

fn call_bridge(method: &str) {
    let Some(bridge) = BRIDGE.get() else {
        return;
    };
    let result = bridge.vm.attach_current_thread_permanently().and_then(|mut env| {
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
        log::warn!("pont de saisie : {method} a échoué ({err})");
    }
}

/// Draine les opérations en attente (ordre d'arrivée).
pub(crate) fn drain() -> Vec<ImeEvent> {
    std::mem::take(&mut QUEUE.lock().unwrap())
}

fn push(event: ImeEvent) {
    QUEUE.lock().unwrap().push(event);
    if let Some(waker) = WAKER.get() {
        waker.lock().unwrap().wake();
    }
}

// --- Natives (appelées sur le thread UI Java) --------------------------------

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

/// Filtre synchrone : `true` = touche d'édition consommée (et mise en file) ;
/// `false` = la touche suit le chemin par défaut (Retour, navigation…).
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
        // Les relâchements des touches consommées le sont aussi (symétrie).
        return matches!(code, KEYCODE_ENTER | KEYCODE_DEL) as jboolean
            | (unicode > 0) as jboolean;
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

// --- Contexte de saisie (lu par l'IME via l'InputConnection) -----------------

/// Renvoie une `jstring` Java (ou une chaîne vide) sans propager d'erreur.
fn to_jstring(env: &JNIEnv, text: &str) -> jni::sys::jstring {
    env.new_string(text)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

extern "system" fn native_text_before(
    env: JNIEnv,
    _class: JClass,
    n: jint,
) -> jni::sys::jstring {
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

extern "system" fn native_text_after(
    env: JNIEnv,
    _class: JClass,
    n: jint,
) -> jni::sys::jstring {
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

//! **Global shortcuts** (milestone 709, part of #219): a key combination that reaches the
//! application whatever window has the keyboard — the clipboard manager's, the quick-add of a
//! task list, the push-to-talk of a call.
//!
//! Declared from the state, like the tray: [`Application::global_shortcuts`](crate::Application::global_shortcuts),
//! or `FrusApp::global_shortcuts` for an application of components. The shell registers the
//! ones declared and gives back the ones withdrawn.
//!
//! ```
//! # use frus_shell::GlobalShortcut;
//! # #[derive(Clone)] enum Msg { QuickAdd }
//! let shortcuts: Vec<GlobalShortcut<Msg>> = vec![
//!     GlobalShortcut::new("Primary+Shift+Space", Msg::QuickAdd),
//!     GlobalShortcut::show_window("Primary+Alt+T"),
//! ];
//! ```
//!
//! A combination is written as its modifiers and one key, joined by `+`: `Ctrl`, `Alt`
//! (`Option`), `Shift`, `Super` (`Cmd`, `Meta`), and `Primary` for the platform's own — `Cmd`
//! on macOS, `Ctrl` elsewhere; the key a letter, a figure, `F1`…`F24`, `Space`, `Enter`, `Tab`,
//! the arrows (`ArrowUp`…), and the names of the others (`Comma`, `Slash`, `Backquote`…).
//!
//! **What a global shortcut can and cannot do** — the position #219 asked to be written down:
//!
//! - It is **one combination, registered with the system**, and the application is told when
//!   that combination is pressed and of nothing else. It reads no other key: it is not a hook on
//!   the keyboard, and on macOS needs none of the permissions a keyboard hook does.
//! - While registered, the combination is **taken from every other application**, which no
//!   longer receives it. So a combination must have a modifier other than `Shift`: a bare key,
//!   or `Shift` and a key, would take typing away from the whole system, and is refused (with a
//!   message in the log).
//! - A combination another application holds **cannot be taken**: the system refuses it, and
//!   the log says which. The application keeps its other shortcuts.
//! - It is given back when the description no longer has it, and when the application ends.
//!
//! Windows, macOS, and Linux under **X11**. Wayland lets no application grab a combination for
//! itself; its desktop portal asks the person instead, which is not done here. Behind the
//! `global-shortcuts` feature, on the desktop.

// Without the system's shortcuts, the description is read by nobody; it is tested either way.
#![cfg_attr(not(all(desktop, feature = "global-shortcuts")), allow(dead_code))]

/// What pressing a global shortcut does.
pub(crate) enum ShortcutAction<Msg> {
    /// Sends a message to the application.
    Message(Msg),
    /// Brings the window forward, showing it if it was hidden.
    ShowWindow,
}

/// A key combination that reaches the application from anywhere, and what it does.
pub struct GlobalShortcut<Msg> {
    /// As written by the application.
    pub(crate) keys: String,
    pub(crate) action: ShortcutAction<Msg>,
}

impl<Msg> GlobalShortcut<Msg> {
    /// `keys` — `"Primary+Shift+Space"` — sending `message` when pressed.
    pub fn new(keys: impl Into<String>, message: Msg) -> Self {
        Self {
            keys: keys.into(),
            action: ShortcutAction::Message(message),
        }
    }

    /// `keys` bringing the application's window forward, showing it if it was hidden — what a
    /// shortcut to an application most often is.
    pub fn show_window(keys: impl Into<String>) -> Self {
        Self {
            keys: keys.into(),
            action: ShortcutAction::ShowWindow,
        }
    }
}

/// `keys` as one canonical spelling — the modifiers in a fixed order, then the key, upper case
/// — so that `"shift+ctrl+k"` and `"Ctrl+Shift+K"` are one shortcut; or why it is refused.
/// `Primary` becomes `macos`'s `Cmd` and everyone else's `Ctrl`.
pub(crate) fn canonical(keys: &str, macos: bool) -> Result<String, String> {
    let (mut ctrl, mut alt, mut shift, mut sup) = (false, false, false, false);
    let mut key: Option<String> = None;
    for token in keys.split('+').map(str::trim) {
        if token.is_empty() {
            return Err(format!("{keys:?}: an empty part"));
        }
        match token.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => ctrl = true,
            "ALT" | "OPTION" => alt = true,
            "SHIFT" => shift = true,
            "SUPER" | "CMD" | "COMMAND" | "META" => sup = true,
            "PRIMARY" | "CMDORCTRL" | "COMMANDORCONTROL" => {
                if macos {
                    sup = true;
                } else {
                    ctrl = true;
                }
            }
            other => {
                if key.is_some() {
                    return Err(format!("{keys:?}: more than one key"));
                }
                key = Some(other.to_string());
            }
        }
    }
    let Some(key) = key else {
        return Err(format!("{keys:?}: no key"));
    };
    if !(ctrl || alt || sup) {
        return Err(format!(
            "{keys:?}: a global shortcut needs Ctrl, Alt or Super, or it takes typing from every \
             other application"
        ));
    }
    let mut parts = Vec::new();
    for (on, name) in [
        (ctrl, "CONTROL"),
        (alt, "ALT"),
        (shift, "SHIFT"),
        (sup, "SUPER"),
    ] {
        if on {
            parts.push(name.to_string());
        }
    }
    parts.push(key);
    Ok(parts.join("+"))
}

/// The global shortcuts as the system has them (desktop, `global-shortcuts` feature).
#[cfg(all(desktop, feature = "global-shortcuts"))]
pub(crate) mod native {
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::sync::{Arc, Mutex, Once};

    use global_hotkey::hotkey::HotKey;
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
    use winit::window::Window;

    /// What the system holds for the application: the manager, and each registered
    /// combination by the id its presses carry.
    #[derive(Default)]
    pub(crate) struct Registered {
        manager: Option<GlobalHotKeyManager>,
        /// Id → (canonical keys, the system's hot key).
        held: HashMap<u32, (String, HotKey)>,
        /// Combinations already refused, so a refusal is told once, not at every change.
        refused: Vec<String>,
    }

    /// The ids of the combinations pressed, waiting for the event loop.
    static PRESSED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    /// The window a press wakes, as the tray's events do.
    static WINDOW: Mutex<Option<Arc<Window>>> = Mutex::new(None);

    fn listen() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
                if event.state() != HotKeyState::Pressed {
                    return;
                }
                if let Ok(mut pressed) = PRESSED.lock() {
                    pressed.push(event.id());
                }
                if let Some(window) = WINDOW.lock().ok().and_then(|window| window.clone()) {
                    window.request_redraw();
                }
            }));
        });
    }

    /// The canonical keys of the combinations pressed since the last call.
    pub(crate) fn take_pressed(registered: &Registered) -> Vec<String> {
        let ids = PRESSED
            .lock()
            .map(|mut pressed| std::mem::take(&mut *pressed))
            .unwrap_or_default();
        ids.into_iter()
            .filter_map(|id| registered.held.get(&id).map(|(keys, _)| keys.clone()))
            .collect()
    }

    /// Makes the system hold exactly `wanted` (canonical keys): registers what is new, gives
    /// back what was withdrawn.
    pub(crate) fn sync(registered: &mut Registered, wanted: &[String], window: &Arc<Window>) {
        if let Ok(mut slot) = WINDOW.lock() {
            *slot = Some(window.clone());
        }
        if wanted.is_empty() && registered.held.is_empty() {
            return;
        }
        if registered.manager.is_none() {
            match GlobalHotKeyManager::new() {
                Ok(manager) => registered.manager = Some(manager),
                Err(err) => {
                    log::warn!("global shortcuts: unavailable ({err})");
                    return;
                }
            }
            listen();
        }
        let Some(manager) = registered.manager.as_ref() else {
            return;
        };
        registered.held.retain(|_, (keys, hotkey)| {
            let keep = wanted.contains(keys);
            if !keep {
                let _ = manager.unregister(*hotkey);
            }
            keep
        });
        for keys in wanted {
            if registered.held.values().any(|(held, _)| held == keys)
                || registered.refused.contains(keys)
            {
                continue;
            }
            let hotkey = match HotKey::from_str(keys) {
                Ok(hotkey) => hotkey,
                Err(err) => {
                    log::warn!("global shortcuts: {keys} is not a combination ({err})");
                    registered.refused.push(keys.clone());
                    continue;
                }
            };
            match manager.register(hotkey) {
                Ok(()) => {
                    log::debug!("global shortcuts: {keys} registered");
                    registered.held.insert(hotkey.id(), (keys.clone(), hotkey));
                }
                Err(err) => {
                    log::warn!(
                        "global shortcuts: {keys} could not be registered — another \
                         application may hold it ({err})"
                    );
                    registered.refused.push(keys.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **One combination, one spelling**: the modifiers in a fixed order and the key upper case,
    /// whatever order and case they were written in.
    #[test]
    fn a_combination_has_one_spelling() {
        assert_eq!(
            canonical("Ctrl+Shift+K", false).as_deref(),
            Ok("CONTROL+SHIFT+K")
        );
        assert_eq!(
            canonical("shift + ctrl + k", false).as_deref(),
            Ok("CONTROL+SHIFT+K")
        );
        assert_eq!(canonical("Alt+Option+F5", false).as_deref(), Ok("ALT+F5"));
        assert_eq!(canonical("Cmd+Space", true).as_deref(), Ok("SUPER+SPACE"));
    }

    /// **`Primary` is the platform's own**: `Cmd` on macOS, `Ctrl` elsewhere.
    #[test]
    fn primary_is_the_platforms_own() {
        assert_eq!(
            canonical("Primary+Shift+Space", true).as_deref(),
            Ok("SHIFT+SUPER+SPACE")
        );
        assert_eq!(
            canonical("Primary+Shift+Space", false).as_deref(),
            Ok("CONTROL+SHIFT+SPACE")
        );
    }

    /// **A combination that would take typing is refused**: a bare key, `Shift` and a key —
    /// and so is one that is not a combination.
    #[test]
    fn what_would_take_typing_is_refused() {
        assert!(canonical("K", false).is_err());
        assert!(canonical("Shift+K", false).is_err());
        assert!(canonical("Ctrl+Shift", false).is_err(), "no key");
        assert!(canonical("Ctrl+K+L", false).is_err(), "two keys");
        assert!(canonical("Ctrl++K", false).is_err(), "an empty part");
        assert!(canonical("Ctrl+", false).is_err(), "an empty key");
        assert!(canonical("Alt+K", false).is_ok());
        assert!(canonical("Super+K", false).is_ok());
    }

    /// **The spelling is the system's**: every canonical spelling of a supported combination
    /// parses as the library's hot key.
    #[cfg(all(desktop, feature = "global-shortcuts"))]
    #[test]
    fn the_spelling_is_the_systems() {
        use std::str::FromStr;
        for keys in [
            "Ctrl+Shift+K",
            "Primary+Alt+Space",
            "Alt+F5",
            "Ctrl+Alt+Shift+ArrowUp",
            "Super+1",
        ] {
            let spelled = canonical(keys, cfg!(target_os = "macos")).unwrap();
            assert!(
                global_hotkey::hotkey::HotKey::from_str(&spelled).is_ok(),
                "{keys} -> {spelled}"
            );
        }
    }
}

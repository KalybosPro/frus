//! **The browser's own shortcuts stay the browser's** (found using the web demo: Ctrl+F5 did
//! not reload the page).
//!
//! While the canvas has the focus, winit cancels the default action of *every* key it is
//! given — that is how an application owns its keyboard, and what makes Tab, the arrows and
//! Space belong to it rather than move the page. It also cancelled **F5, Ctrl+R, F12,
//! Ctrl+L** and the rest: a page the application had focused could not be reloaded, have its
//! developer tools opened or its address bar reached from the keyboard, and a reader who
//! pressed the keys every other page answers to concluded the page was hung.
//!
//! An application has no business owning those. So a listener on the **window**, in the
//! capture phase — ahead of the canvas, and so ahead of winit — recognises them and stops
//! the event there: winit never sees the key, nothing cancels it, and the browser does what
//! it does. Every other key goes on to the application exactly as before.

/// Whether `key`, with these modifiers held, is the **browser's** and not the application's:
/// reload, the developer tools, the address bar, tabs and windows, zoom, find, print.
///
/// `key` is the DOM's `KeyboardEvent.key`. `cmd` is Ctrl or the platform's Command key,
/// whichever the platform's shortcuts are made with. What is *not* here on purpose: Ctrl+C,
/// X, V, A, Z and Y — the application handles those itself, in its own fields — and every key
/// without a modifier that is not a function key.
#[cfg(any(web, test))]
pub(crate) fn browser_owns(key: &str, cmd: bool, alt: bool, shift: bool) -> bool {
    // The function keys nobody builds a screen on: reload, full screen, developer tools.
    if matches!(key, "F5" | "F11" | "F12") {
        return true;
    }
    // Alt with an arrow is Back and Forward, Alt with Home is the start page.
    if alt && !cmd && matches!(key, "ArrowLeft" | "ArrowRight" | "Home") {
        return true;
    }
    if !cmd {
        return false;
    }
    let letter = key.to_ascii_lowercase();
    match letter.as_str() {
        // Reload, the address bar, a new tab / window / private window, close, print, find,
        // history, downloads, bookmarks, view source. Not Save or Open: an application may
        // well want Ctrl+S for itself.
        "r" | "l" | "t" | "n" | "w" | "p" | "f" | "h" | "j" | "d" | "u" => true,
        // Zoom: in, out, back to the default.
        "+" | "-" | "=" | "0" => true,
        // Next and previous tab, by Tab or by the page keys.
        "tab" | "pageup" | "pagedown" => true,
        // The developer tools and the shortcuts that only exist with Shift held.
        "i" | "c" => shift, // Ctrl+Shift+I / Ctrl+Shift+C — plain Ctrl+C is a copy, the app's
        _ => false,
    }
}

/// Installs the window-level listeners that let the browser keep its shortcuts.
///
/// Best-effort like the other bridges: if there is no window, or the listener cannot be added,
/// the application keeps the keyboard it had — a page whose shortcuts do not work, and nothing
/// worse.
#[cfg(web)]
pub(crate) fn let_the_browser_keep_its_shortcuts() {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    for kind in ["keydown", "keyup"] {
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let Some(key) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };
            let cmd = key.ctrl_key() || key.meta_key();
            if browser_owns(&key.key(), cmd, key.alt_key(), key.shift_key()) {
                // Not `prevent_default`: the whole point is that the default happens.
                event.stop_immediate_propagation();
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        // Capture phase (`true`): before the canvas, and so before winit.
        let _ = (window.as_ref() as &web_sys::EventTarget)
            .add_event_listener_with_callback_and_bool(
                kind,
                closure.as_ref().unchecked_ref(),
                true,
            );
        // Kept for the life of the page.
        closure.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::browser_owns;

    /// Reload and the developer tools, however the keys are held.
    #[test]
    fn reload_and_the_developer_tools_are_the_browsers() {
        assert!(browser_owns("F5", false, false, false));
        assert!(
            browser_owns("F5", true, false, false),
            "Ctrl+F5, a hard reload"
        );
        assert!(browser_owns("F12", false, false, false));
        assert!(browser_owns("F11", false, false, false));
        assert!(browser_owns("r", true, false, false), "Ctrl+R");
        assert!(
            browser_owns("R", true, false, true),
            "Ctrl+Shift+R, as the browser reports it"
        );
        assert!(browser_owns("i", true, false, true), "Ctrl+Shift+I");
        assert!(browser_owns("l", true, false, false), "the address bar");
    }

    /// Save and Open are left to the application, which may want them.
    #[test]
    fn save_and_open_are_left_to_the_application() {
        assert!(!browser_owns("s", true, false, false));
        assert!(!browser_owns("o", true, false, false));
    }

    /// What the application handles itself is not the browser's: the clipboard and editing
    /// keys, and everything typed.
    #[test]
    fn the_applications_own_keys_are_not() {
        for key in ["c", "x", "v", "a", "z", "y"] {
            assert!(!browser_owns(key, true, false, false), "Ctrl+{key}");
        }
        // Plain Ctrl+C is a copy; with Shift it is the inspector.
        assert!(!browser_owns("c", true, false, false));
        assert!(browser_owns("c", true, false, true));
        for key in [
            "a",
            "Enter",
            "Tab",
            "Escape",
            "ArrowLeft",
            "Backspace",
            " ",
            "F1",
            "F2",
            "F10",
        ] {
            assert!(!browser_owns(key, false, false, false), "{key}");
        }
        assert!(
            !browser_owns("F10", false, false, true),
            "Shift+F10 is the bar's"
        );
        assert!(!browser_owns("Tab", false, false, true));
    }

    /// Zoom, tabs and history: browser-level, with the platform's modifier.
    #[test]
    fn zoom_tabs_and_history_are_the_browsers() {
        for key in ["+", "-", "=", "0"] {
            assert!(browser_owns(key, true, false, false), "zoom {key}");
        }
        assert!(browser_owns("Tab", true, false, false), "Ctrl+Tab");
        assert!(browser_owns("PageDown", true, false, false));
        assert!(
            browser_owns("ArrowLeft", false, true, false),
            "Alt+Left, Back"
        );
        assert!(
            browser_owns("ArrowRight", false, true, false),
            "Alt+Right, Forward"
        );
        // An arrow with Alt *and* Ctrl is not a history key.
        assert!(!browser_owns("ArrowLeft", true, true, false));
    }
}

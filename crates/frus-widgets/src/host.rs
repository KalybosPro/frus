//! **What a component asks of the application around it.**
//!
//! A component builds widgets. Some of what an interface does is not a widget: the theme
//! the whole window is dressed in, the language, the zoom; a field that should take the
//! focus, a list that should scroll to its top; a file to read on another thread, and what
//! to do with it when it is read. Those belong to the application and the shell, and a
//! component reaches them from here, from a handler or a build alike:
//!
//! ```
//! use frus_widgets::{host, ThemeMode};
//!
//! host::app().set_theme_mode(ThemeMode::Dark);
//! host::focus("email");
//! host::spawn(|| 6 * 7, |answer| assert_eq!(answer, 42));
//! # let _ = host::take_effects();
//! ```
//!
//! Nothing here runs anything. [`App`] keeps what the application is dressed in, and the
//! shell reads it every frame; an *effect* is a request, kept until the shell takes it
//! with [`take_effects`] after the frame and runs it as it runs any other effect.

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::callback::Callback;
use crate::component::request_rebuild;
use crate::locale::Locale;
use crate::localizations::Localizations;
use crate::media::Brightness;
use crate::scrollposition::ScrollTo;
use crate::sheet::SheetTo;
use crate::theme::{Theme, ThemeMode};

// ---------------------------------------------------------------------------------------
// The application's own settings
// ---------------------------------------------------------------------------------------

struct Settings {
    title: String,
    theme: Theme,
    dark_theme: Option<Theme>,
    theme_mode: ThemeMode,
    density: f32,
    locale: Option<Locale>,
    supported_locales: Vec<Locale>,
    localizations: Option<Rc<dyn Localizations>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: "frus".to_string(),
            theme: Theme::light(),
            dark_theme: None,
            theme_mode: ThemeMode::System,
            density: 1.0,
            locale: None,
            supported_locales: Vec::new(),
            localizations: None,
        }
    }
}

thread_local! {
    static SETTINGS: RefCell<Settings> = RefCell::new(Settings::default());
    static EFFECTS: RefCell<Vec<Effect>> = const { RefCell::new(Vec::new()) };
}

/// The application the interface is part of, as a component sees it: a handle on the
/// settings the shell reads every frame — the theme, the language, the zoom, the title.
///
/// Changing one asks for a rebuild, so the next frame is built with it. It is the
/// interface's own thread that owns them: there is one set per thread, and a handle is
/// only a way to reach it.
#[derive(Clone, Copy, Default, Debug)]
pub struct App;

/// The handle on the application's settings.
pub fn app() -> App {
    App
}

macro_rules! setting {
    ($(#[$doc:meta])* $get:ident, $set:ident: $ty:ty, $field:ident) => {
        $(#[$doc])*
        pub fn $get(&self) -> $ty {
            SETTINGS.with(|s| s.borrow().$field.clone())
        }

        #[doc = concat!("Changes [`App::", stringify!($get), "`].")]
        pub fn $set(&self, value: $ty) {
            SETTINGS.with(|s| s.borrow_mut().$field = value);
            request_rebuild();
        }
    };
}

impl App {
    setting!(
        /// The window's title.
        title, set_title: String, title
    );
    setting!(
        /// The theme — the light one, where the application has two.
        theme, set_theme: Theme, theme
    );
    setting!(
        /// The theme for a dark interface, if the application has one.
        dark_theme, set_dark_theme: Option<Theme>, dark_theme
    );
    setting!(
        /// Which of the two themes is on display: the system's choice by default.
        theme_mode, set_theme_mode: ThemeMode, theme_mode
    );
    setting!(
        /// A zoom over the whole interface, on top of the system's scale.
        density, set_density: f32, density
    );
    setting!(
        /// The language the reader picked in the application; `None` follows the device.
        locale, set_locale: Option<Locale>, locale
    );
    setting!(
        /// The languages the application has, best first.
        supported_locales, set_supported_locales: Vec<Locale>, supported_locales
    );
    setting!(
        /// The words the framework itself says on the application's behalf, when they are
        /// not the English ones.
        localizations, set_localizations: Option<Rc<dyn Localizations>>, localizations
    );

    /// The theme to dress the window in for a system that is `brightness` — what
    /// [`theme_mode`](Self::theme_mode) chooses between the two.
    pub fn resolved_theme(&self, brightness: Brightness) -> Theme {
        let dark = match self.theme_mode() {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => brightness == Brightness::Dark,
        };
        match (dark, self.dark_theme()) {
            (true, Some(theme)) => theme,
            _ => self.theme(),
        }
    }
}

// ---------------------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------------------

/// A request to the shell that is not a widget: something to do once.
pub enum Effect {
    /// Focus the widget wrapped in `keyed(key, …)` — the hash of its key.
    Focus(u64),
    /// Move the scroll region named by the hash of a key.
    Scroll(u64, ScrollTo),
    /// Move the sheet named by the hash of a key.
    Sheet(u64, SheetTo),
    /// Run this on another thread; if it answers with a callback, that callback runs on the
    /// interface's thread.
    Spawn(Box<dyn FnOnce() -> Option<Callback> + Send>),
    /// Run this callback after a delay.
    After(Duration, Callback),
}

/// The hash a key is known by, the same one `keyed(key, …)` and the shell's commands use.
pub fn key_hash(key: impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

fn push(effect: Effect) {
    EFFECTS.with(|effects| effects.borrow_mut().push(effect));
    // Something has to be there to take it: a frame.
    request_rebuild();
}

/// Takes the effects asked for since the last time, oldest first. The shell calls this after
/// each frame.
pub fn take_effects() -> Vec<Effect> {
    EFFECTS.with(|effects| std::mem::take(&mut *effects.borrow_mut()))
}

/// Asks for the focus to go to the widget wrapped in `keyed(key, …)`. Resolved against the
/// frame that follows.
pub fn focus(key: impl Hash) {
    push(Effect::Focus(key_hash(key)));
}

/// Asks for the scroll region wrapped in `keyed(key, …)` to go where `to` says.
pub fn scroll_to(key: impl Hash, to: ScrollTo) {
    push(Effect::Scroll(key_hash(key), to));
}

/// Asks for the sheet wrapped in `keyed(key, …)` to go where `to` says.
pub fn sheet_to(key: impl Hash, to: SheetTo) {
    push(Effect::Sheet(key_hash(key), to));
}

/// Runs `work` on another thread — a file to read, something slow — and then `then` with
/// what it made, on the interface's thread, where it may change state.
///
/// `then` runs once. If the work is still going when the component that asked has left the
/// tree, `then` still runs, so what it touches should say whether it is still there
/// ([`StateHandle::is_mounted`](crate::StateHandle::is_mounted)).
pub fn spawn<R: Send + 'static>(
    work: impl FnOnce() -> R + Send + 'static,
    then: impl Fn(R) + 'static,
) {
    let answer: Arc<Mutex<Option<R>>> = Arc::default();
    let deliver = {
        let answer = answer.clone();
        Callback::new(move || {
            let made = answer.lock().ok().and_then(|mut slot| slot.take());
            if let Some(made) = made {
                then(made);
            }
        })
    };
    push(Effect::Spawn(Box::new(move || {
        let made = work();
        *answer.lock().ok()? = Some(made);
        Some(deliver)
    })));
}

/// Runs `then` on the interface's thread once `delay` has passed.
pub fn after(delay: Duration, then: impl Fn() + 'static) {
    push(Effect::After(delay, Callback::new(then)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_is_read_back_and_asks_for_a_rebuild() {
        crate::take_rebuild_request();
        app().set_theme_mode(ThemeMode::Dark);
        assert_eq!(app().theme_mode(), ThemeMode::Dark);
        assert!(crate::rebuild_requested());
        app().set_title("hello".into());
        assert_eq!(app().title(), "hello");
    }

    #[test]
    fn the_mode_chooses_between_the_two_themes() {
        app().set_theme(Theme::light());
        app().set_dark_theme(Some(Theme::dark()));
        let light = Theme::light().background;
        app().set_theme_mode(ThemeMode::Light);
        assert_eq!(app().resolved_theme(Brightness::Dark).background, light);
        app().set_theme_mode(ThemeMode::Dark);
        assert_ne!(app().resolved_theme(Brightness::Light).background, light);
        app().set_theme_mode(ThemeMode::System);
        assert_eq!(app().resolved_theme(Brightness::Light).background, light);
        assert_ne!(app().resolved_theme(Brightness::Dark).background, light);
    }

    #[test]
    fn a_dark_mode_with_no_dark_theme_stays_light() {
        app().set_theme(Theme::light());
        app().set_dark_theme(None);
        app().set_theme_mode(ThemeMode::Dark);
        assert_eq!(
            app().resolved_theme(Brightness::Dark).background,
            Theme::light().background
        );
    }

    #[test]
    fn effects_are_kept_in_order_and_taken_once() {
        let _ = take_effects();
        focus("name");
        scroll_to("list", ScrollTo::start());
        after(Duration::from_millis(5), || {});
        let effects = take_effects();
        assert_eq!(effects.len(), 3);
        assert!(matches!(effects[0], Effect::Focus(h) if h == key_hash("name")));
        assert!(matches!(effects[1], Effect::Scroll(h, _) if h == key_hash("list")));
        assert!(matches!(effects[2], Effect::After(d, _) if d == Duration::from_millis(5)));
        assert!(take_effects().is_empty(), "taken once");
    }

    #[test]
    fn spawned_work_comes_back_as_a_callback_that_hands_over_the_answer() {
        let _ = take_effects();
        let seen = Rc::new(RefCell::new(None));
        let put = seen.clone();
        spawn(|| 6 * 7, move |answer| *put.borrow_mut() = Some(answer));
        let mut effects = take_effects();
        let Some(Effect::Spawn(task)) = effects.pop() else {
            panic!("a spawn is an effect");
        };
        // The shell runs it on another thread; the callback it returns is a Send handle.
        let callback = std::thread::spawn(move || task())
            .join()
            .expect("the work ran")
            .expect("and answered");
        assert_eq!(*seen.borrow(), None, "nothing runs until it is delivered");
        callback.call();
        assert_eq!(*seen.borrow(), Some(42));
        callback.call();
        assert_eq!(
            *seen.borrow(),
            Some(42),
            "and it hands the answer over once"
        );
    }
}

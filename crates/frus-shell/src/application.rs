//! The [`Application`] trait: the contract between an **application** and the
//! framework. The shell — window, renderer, input, runtime, animations — is generic
//! over this trait, and the application supplies only its logic.
//!
//! A minimal app implements `update` and `view` and nothing else. Every other method
//! has a default: a fixed dark theme, no animation, no navigation.

use frus_widgets::{ScrollPhysics, Theme, Widget, WindowInsets};

use crate::command::Command;
use crate::subscription::Subscription;

/// The application's **lifecycle** state. The framework hands it to
/// [`Application::on_lifecycle`] at every transition so the app can **react**: pause a
/// timer or a camera in the background, write a save before being destroyed, and so on.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    /// In the **foreground**, visible and **interactive**.
    Resumed,
    /// Visible but **not focused** — a notification shade, a system dialog, a
    /// multi-window layout. The app should not react to input but stays on screen.
    Inactive,
    /// In the **background**, **not visible**; on Android the render surface is lost.
    /// A good moment to release resources and suspend non-essential work.
    Paused,
    /// Being **destroyed** — the last chance to persist state, since the process may
    /// be killed afterwards with no further notice.
    Detached,
}

/// What an application supplies to the framework: an Elm-style message model.
pub trait Application {
    /// The message type the interface emits and [`Application::update`] consumes.
    ///
    /// `Send + 'static`, because effects ([`Command`]) cross threads.
    type Message: Clone + Send + 'static;

    /// Advances the state in response to a message, and returns the **effects** to
    /// run: I/O, background tasks and the like. Use [`Command::none`] for none.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message>;

    /// The effect to run **at startup**, such as an initial load.
    fn init(&mut self) -> Command<Self::Message> {
        Command::none()
    }

    /// **Continuous** sources of messages — timers and so on — declared from the
    /// state. The framework starts and stops them by diffing every cycle.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// Builds the widget tree for the current theme.
    ///
    /// **It is not told the size.** The framework installs a description of the surface
    /// — its size, its pixel ratio, the intrusions the platform last reported — around
    /// every call to this, and any widget built here reads it with `MediaQuery::of()`.
    /// A `Scaffold` takes no size because of that, and neither does an `AppBar` or a
    /// `Navigator`.
    ///
    /// That is the reference's arrangement, and it is not a convenience: a size passed
    /// as an argument gets carried down by hand, arithmetic gets done on it, and one of
    /// those subtractions is eventually wrong. Milestone 392 is what that looks like
    /// when it happens — a whole screen laid out to the width of its widest line.
    fn view(&self, theme: &Theme) -> Box<dyn Widget<Self::Message>>;

    /// The theme on display, possibly an animated blend; dark by default.
    fn theme(&self) -> Theme {
        Theme::dark()
    }

    /// Advances the app's **own** animations — a theme fade, a screen transition, a
    /// gesture settling — by `dt` seconds. Returns `true` while something is still
    /// moving, in which case the framework asks for another frame.
    fn tick(&mut self, _dt: f32) -> bool {
        false
    }

    /// The window's title.
    fn title(&self) -> String {
        "frus".to_string()
    }

    /// The window's desired initial **logical** size; `None` for the system default.
    fn window_size(&self) -> Option<(f32, f32)> {
        None
    }

    /// What this application wants to say about the **accessibility settings** *instead
    /// of the platform*.
    ///
    /// The platform is asked first, because the settings belong to the person using the
    /// device and not to the program. This hook lays an application's own answers over
    /// that, one setting at a time — a settings screen with a *reduce motion* switch of
    /// its own speaks for that and stays quiet about the rest.
    ///
    /// The default is [`AccessibilityOverrides::NONE`]: nothing said, every answer the
    /// user's. `None` per field is what makes that expressible; a plain
    /// [`Accessibility`](frus_widgets::Accessibility) could not, a `false` in it being
    /// indistinguishable from silence.
    ///
    /// The framework honours [`disable_animations`] itself, by completing implicit
    /// animations at once rather than over time. The rest reach the widgets through
    /// [`MediaQuery::of`](frus_widgets::MediaQuery::of).
    ///
    /// [`AccessibilityOverrides::NONE`]: frus_widgets::AccessibilityOverrides::NONE
    /// [`disable_animations`]: frus_widgets::Accessibility::disable_animations
    fn accessibility(&self) -> frus_widgets::AccessibilityOverrides {
        frus_widgets::AccessibilityOverrides::NONE
    }

    /// The interface's **density**: an **application-level** zoom factor, `1.0` by
    /// default, applied on top of the system DPI scale. `1.2` grows the whole UI by
    /// 20%, `0.9` tightens it. It can change at runtime through the state.
    fn density(&self) -> f32 {
        1.0
    }

    /// How the app's scrollables behave at their edges and after a fling.
    ///
    /// The default is what the running platform does — bouncing where the system
    /// scroll views bounce, clamping elsewhere — so an app that says nothing feels
    /// native on each target. Override it to pin one behaviour everywhere; an
    /// individual [`frus_widgets::SingleChildScrollView`] can still ask for its own.
    fn scroll_physics(&self) -> ScrollPhysics {
        ScrollPhysics::platform_default()
    }

    /// Called when the surface's **logical** size changes, whether the window was
    /// resized **or** the density changed. It lets the app react to a **breakpoint
    /// change** in its own logic — closing a drawer as things narrow, say — beyond
    /// simply re-rendering.
    fn on_resize(&mut self, _width: f32, _height: f32) {}

    /// Called at every **lifecycle transition** (see [`Lifecycle`]): `Resumed` in the
    /// foreground, `Inactive` when the window loses focus, `Paused` in the background
    /// with the surface lost, `Detached` just before closing. The default does
    /// nothing. This is where an app suspends and resumes its work — timers, sensors —
    /// or persists its state.
    fn on_lifecycle(&mut self, _state: Lifecycle) {}

    /// Called when the **window insets** change, split by nature: `padding` is the
    /// system bars and the notch, which are static, while `view_insets` is the
    /// software keyboard, which is dynamic — avoiding the keyboard means pushing the
    /// content away by `insets.safe()`. In **logical** px. Always zero on desktop.
    fn on_insets(&mut self, _insets: WindowInsets) {}

    /// **Live reload**, in development: a serialised snapshot of the state, captured
    /// just before a recompiled binary relaunches the application (`FRUS_WATCH=1`).
    /// `None`, the default, means the app opts out, and a reload then starts from a
    /// fresh state. The byte format belongs to the app.
    fn save_state(&self) -> Option<Vec<u8>> {
        None
    }

    /// Rehydrates the state from an [`Application::save_state`] snapshot taken by the
    /// **previous** binary; called before [`Application::init`]. The bytes come from
    /// another version of the code, so tolerate unexpected formats — ignoring beats
    /// panicking.
    fn restore_state(&mut self, _bytes: &[u8]) {}

    /// Can the app go back? This enables the edge back gesture.
    fn can_go_back(&self) -> bool {
        false
    }

    /// The back gesture is progressing (`0 → 1`), so `view` can preview it.
    fn back_gesture(&mut self, _progress: f32) {}

    /// The back gesture was released, with the finger's velocity in fractions per
    /// second. It is up to the app to commit or cancel, usually through an animated
    /// settle in `tick`.
    fn back_gesture_end(&mut self, _velocity: f32) {}
}

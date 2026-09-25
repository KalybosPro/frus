//! The [`Application`] trait: the contract between an **application** and the
//! framework. The shell — window, renderer, input, runtime, animations — is generic
//! over this trait, and the application supplies only its logic.
//!
//! A minimal app implements `update` and `view` and nothing else. Every other method
//! has a default: one theme, no animation, no navigation.

use frus_widgets::localizations::Localizations;
use frus_widgets::{
    Brightness, Curve, Locale, ScrollPhysics, Scrollbars, Theme, ThemeMode, Widget, WindowInsets,
};

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

/// How a location is written in a browser's address bar — see [`Application::location`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum LocationStrategy {
    /// After a `#`: `https://host/app/#/users/42`. Any static host serves it, because the
    /// server only ever sees the page's own address; a reload comes back to the same screen
    /// with nothing configured. The default.
    #[default]
    Hash,
    /// As a path under the document's base: `https://host/app/users/42`. The addresses read
    /// as an ordinary site's would, and the server has to answer every one of them with the
    /// application's page, which a static host does not do without a rule for it. The base
    /// is the document's `<base href>`, or `/` without one.
    Path,
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

    /// Effects the application asked for **outside** an [`update`](Application::update) — from
    /// a build, say, or from a component that has no message to return them with. The shell
    /// asks after every frame and after every message, and runs what comes back like any
    /// other command. An application that returns its effects from `update` never needs it.
    fn effects(&mut self) -> Command<Self::Message> {
        Command::none()
    }

    /// **Where the application is**, as an address: `/users/42?tab=posts`. `None` — the
    /// default — for an application with no addresses, which the shell then leaves alone.
    ///
    /// On the **web** the shell asks after every message and every frame, and shows the
    /// answer in the address bar: a new location becomes a new entry of the browser's history,
    /// going back to a location the history already holds goes back in it rather than adding
    /// one, and the very first answer replaces the address the page was opened at instead of
    /// adding to it — so a redirect on the way in leaves no entry to be trapped by. Other
    /// platforms have nowhere to show it yet.
    fn location(&self) -> Option<String> {
        None
    }

    /// Whether the move that led to [`location`](Self::location) **swapped** the page — a
    /// router's `replace` — rather than adding one. The browser's entry is then swapped too,
    /// instead of a new one being added behind it. `false` by default.
    fn location_replaces(&self) -> bool {
        false
    }

    /// The locations of the pages beneath the one shown, the lowest first, and
    /// [`location`](Self::location) last. Used once, when the page is opened at an address
    /// deep in the application: the browser's history is made to hold the pages beneath too,
    /// so that going back — the application's own or the browser's — has somewhere to land
    /// inside the application. The default is the location alone.
    fn location_stack(&self) -> Vec<String> {
        self.location().into_iter().collect()
    }

    /// **A location was asked for from outside**: the address the page was opened at, or the
    /// one the browser's back and forward buttons landed on. Move to it — or to wherever it
    /// redirects, and the shell corrects the address bar to match. Called with a location
    /// the application may already be at; that is a no-op to do, not to report.
    ///
    /// The launch address is delivered once, before the first frame, and only when the
    /// address carried one: a bare page keeps the application's own start.
    fn open_location(&mut self, location: &str) -> Command<Self::Message> {
        let _ = location;
        Command::none()
    }

    /// Whether the application is **one window**, and its name: on a desktop, a second process
    /// started with a link — the operating system's way of opening `myapp://orders/42` — hands
    /// the link to the first and ends, instead of opening a second window. `None`, the default,
    /// lets every process be its own.
    ///
    /// The name tells this application from the others on the machine; a reverse domain
    /// (`com.example.myapp`) is what one is. Links reach [`open_location`](Self::open_location)
    /// as locations — see [`location_of_link`](crate::location_of_link).
    fn instance_id(&self) -> Option<String> {
        None
    }

    /// How [`location`](Self::location) is written in the address bar. See
    /// [`LocationStrategy`].
    fn location_strategy(&self) -> LocationStrategy {
        LocationStrategy::Hash
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

    /// **The application's theme** — its light one, where it has two (`app.dart:425`).
    ///
    /// This is the whole answer for an application with a single theme. One that follows
    /// the system supplies a [`dark_theme`](Application::dark_theme) beside it and lets
    /// [`theme_mode`](Application::theme_mode) choose; the framework resolves the pair
    /// every frame and **fades** between them, so nothing here has to be an animated
    /// blend and no application needs to hold a fade's progress in its own state.
    ///
    /// **The default is the light theme**, which is the reference's (`app.dart:415`). It
    /// was dark here until milestone 452 named this slot the light one: a `theme()` that
    /// answers dark and a `dark_theme()` beside it is a contradiction an application
    /// would have had to work around.
    fn theme(&self) -> Theme {
        Theme::light()
    }

    /// **The theme to use when a dark interface is asked for** (`app.dart:447`).
    ///
    /// `None` — the default — means the application has only the one theme, and
    /// [`Application::theme`] is used whatever the platform reports. Answer with a dark
    /// theme and the framework switches to it by itself, following
    /// [`Application::theme_mode`].
    ///
    /// **This is the hook that makes the platform's brightness usable.** The shell has
    /// reported it since milestone 380 — `MediaQuery::platform_brightness`, read from the
    /// window manager on desktop and from the system settings on Android — but until an
    /// application could name a second theme there was nothing to switch *to*, and every
    /// application that wanted to follow the system had to read the brightness and write
    /// the crossfade itself.
    fn dark_theme(&self) -> Option<Theme> {
        None
    }

    /// **Which of the two themes is on display** (`app.dart:502`): the platform's choice
    /// by default, or one of them pinned.
    ///
    /// [`ThemeMode::System`] follows [`MediaQuery::platform_brightness`], which is what
    /// an application with a *System / Light / Dark* setting leaves it on until the
    /// reader picks one of the other two.
    ///
    /// [`MediaQuery::platform_brightness`]: frus_widgets::MediaQuery::platform_brightness
    fn theme_mode(&self) -> ThemeMode {
        ThemeMode::System
    }

    /// **The theme to use when the platform asks for higher contrast** (`app.dart:460`);
    /// `None` falls back to [`Application::theme`].
    ///
    /// The setting is the reader's, reported by the platform and readable at
    /// [`Accessibility::high_contrast`](frus_widgets::Accessibility::high_contrast). A
    /// design that already meets the contrast ratios has nothing to add here, which is
    /// why the default is to add nothing.
    fn high_contrast_theme(&self) -> Option<Theme> {
        None
    }

    /// **The theme for a dark, high-contrast interface** (`app.dart:476`); `None` falls
    /// back to [`Application::dark_theme`], then to [`Application::high_contrast_theme`],
    /// then to [`Application::theme`] — the reference's own order, rung for rung.
    fn high_contrast_dark_theme(&self) -> Option<Theme> {
        None
    }

    /// **Which of this application's themes is on display**, given what the platform
    /// reports — the reference's `_themeBuilder` (`app.dart:995`), rung for rung.
    ///
    /// A dark, high-contrast interface takes
    /// [`high_contrast_dark_theme`](Application::high_contrast_dark_theme), then
    /// [`dark_theme`](Application::dark_theme), then
    /// [`high_contrast_theme`](Application::high_contrast_theme); a dark one takes
    /// `dark_theme`; a high-contrast one takes `high_contrast_theme`; and anything still
    /// unanswered takes [`theme`](Application::theme), which every application has.
    ///
    /// The fallbacks are the point. An application supplying only a `dark_theme` is right
    /// about dark mode and silent about contrast, and gets its dark theme on a
    /// high-contrast device rather than being sent back to its light one.
    ///
    /// **The framework calls this every frame**; an application rarely does. It is public
    /// because there is no `BuildContext` here to ask, so anything outside the frame loop
    /// that needs the theme an application would be showing — an off-screen renderer, a
    /// golden harness, a test — has to be able to ask `self` for it. Overriding it
    /// replaces the rungs above wholesale.
    fn resolved_theme(&self, brightness: Brightness, high_contrast: bool) -> Theme {
        let dark = self.theme_mode().wants_dark(brightness);
        let picked = if dark && high_contrast {
            self.high_contrast_dark_theme()
                .or_else(|| self.dark_theme())
                .or_else(|| self.high_contrast_theme())
        } else if dark {
            self.dark_theme()
        } else if high_contrast {
            self.high_contrast_theme()
        } else {
            None
        };
        picked.unwrap_or_else(|| self.theme())
    }

    /// **How long the framework takes to cross from one theme to the next**, in seconds
    /// (`app.dart:516`); `0` switches at once. The default is the reference's 200 ms.
    ///
    /// Every change of theme is animated, not only a light/dark switch: a new seed
    /// colour, a different density of tokens, anything at all that makes the resolved
    /// theme a different value. The application does not run the fade and does not hold
    /// its progress — it names the two themes and the mode, and the framework blends.
    ///
    /// A reader who has asked for **reduced motion** gets the change at once whatever
    /// this says, as with every implicit animation the framework runs.
    fn theme_animation_duration(&self) -> f32 {
        0.2
    }

    /// **The shape of that crossing** (`app.dart:526`); linear by default, as in the
    /// reference. Ignored when [`Application::theme_animation_duration`] is `0`.
    fn theme_animation_curve(&self) -> Curve {
        Curve::Linear
    }

    /// Advances the app's **own** animations — a screen transition, a gesture settling —
    /// by `dt` seconds. Returns `true` while something is still moving, in which case the
    /// framework asks for another frame.
    ///
    /// The theme fade is **not** one of them: since milestone 452 the framework runs it.
    fn tick(&mut self, _dt: f32) -> bool {
        false
    }

    /// The window's title.
    fn title(&self) -> String {
        "frus".to_string()
    }

    /// The application's icon: the frus logo unless it says otherwise. On the desktop it is the
    /// window's, on the Web the tab's; see [`AppIcon`](crate::AppIcon) for what it does on each platform, and
    /// for how to change or remove it.
    fn icon(&self) -> crate::AppIcon {
        crate::AppIcon::Frus
    }

    /// The window's desired initial **logical** size; `None` for the system default. Ignored on
    /// the Web, where the page decides how big the canvas is.
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

    /// **Whether the app's scrollables draw a scrollbar.**
    ///
    /// The default is what the running platform does: none on a touch screen, one down
    /// the inner edge on a desktop — which is the reference's own answer, resolved the
    /// same way and for the same reason (`app.dart:857`). A finger already knows where it
    /// is on the page. Override it to pin one behaviour everywhere; an individual
    /// [`frus_widgets::SingleChildScrollView`] can still ask for its own.
    fn scrollbars(&self) -> Scrollbars {
        Scrollbars::platform_default()
    }

    /// **The languages this application has** (`app.dart`'s `supportedLocales`), best
    /// first — the answer to *what can I actually show this reader?*
    ///
    /// The default is `["en"]`, which is what every string in the framework was written in
    /// before any of them could be translated. **The order matters**: it decides ties, so
    /// an application listing `en-US` before `en-GB` has said which English a reader who
    /// asked only for `en` should get, and the first entry is what a reader whose
    /// languages are all unavailable ends up with.
    ///
    /// An application that translates nothing can leave this alone and lose nothing: the
    /// resolution then always answers `en`, which is where it was.
    fn supported_locales(&self) -> Vec<Locale> {
        vec![Locale::default()]
    }

    /// **A language pinned by the application**, over the reader's own (`app.dart`'s
    /// `locale`); `None` — the default — follows the device.
    ///
    /// This is what an application's *Language* setting writes to. It is still resolved
    /// against [`supported_locales`](Application::supported_locales), so pinning one the
    /// application does not have gives the nearest thing it does have rather than nothing.
    fn locale(&self) -> Option<Locale> {
        None
    }

    /// **Which language the interface ends up in**, given what the reader prefers.
    ///
    /// `preferred` is the platform's list, best first. An application that pinned a
    /// [`locale`](Application::locale) replaces that list with the one it named — the
    /// reference does the same, resolving the pin rather than obeying it blindly — and
    /// everything then goes through [`locale::resolve`](frus_widgets::locale::resolve),
    /// which is the reference's `basicLocaleListResolution` (`app.dart:146`).
    ///
    /// **The framework calls this every frame** and installs the answer, so a language
    /// setting changed while the application is running is obeyed on the next one.
    /// Override it to negotiate differently — the reference's `localeListResolutionCallback`.
    fn resolved_locale(&self, preferred: &[Locale]) -> Locale {
        let supported = self.supported_locales();
        match self.locale() {
            Some(pinned) => frus_widgets::locale::resolve(&[pinned], &supported),
            None => frus_widgets::locale::resolve(preferred, &supported),
        }
    }

    /// **The words the framework says on this application's behalf**, in the reader's
    /// language: the label on a back arrow, the word on a notification's cross, a
    /// calendar's month names and *which day its weeks start on*.
    ///
    /// `None` — the default — leaves them in English, which is what they were before
    /// any of them could be translated. Answer with a table and the shell installs it
    /// **every frame**, so an application that changes language while it is running is
    /// obeyed on the next one.
    ///
    /// See [`frus_widgets::localizations`].
    fn localizations(&self) -> Option<std::rc::Rc<dyn Localizations>> {
        None
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

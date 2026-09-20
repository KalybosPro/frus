//! [`FrusApp`]: an application made of components.
//!
//! An application here is a root widget, and whatever it builds. Nothing else is
//! declared: no message type, no `update`, no `view`. State lives in the components that
//! use it, and an interaction runs the closure the widget was given.
//!
//! ```ignore
//! use frus::{button, column, text, FrusApp};
//!
//! frus::main!(FrusApp::from_fn(|cx| {
//!     let count = cx.use_state(|| 0);
//!     let add = count.clone();
//!     Box::new(column![
//!         text(format!("{}", count.get())),
//!         button("+", move || add.update(|n| *n += 1)),
//!     ])
//! }));
//! ```
//!
//! It is an [`Application`] like any other — the shell drives it through the same
//! contract — whose message is a [`Callback`] and whose `update` runs it.

use std::rc::Rc;

use frus_widgets::host::{self, Effect};
use frus_widgets::{
    BuildContext, Callback, Component, GoRouter, Locale, Localizations, StatefulWidget,
    StatelessWidget, Theme, ThemeMode, Widget,
};

use crate::application::Application;
use crate::command::Command;
use crate::subscription::Subscription;

/// What builds the root: called on every rebuild.
type Root = Rc<dyn Fn(&BuildContext) -> Box<dyn Widget>>;

/// An application: a root component, and how it is dressed.
///
/// ```
/// use frus_shell::FrusApp;
/// use frus_widgets::{text, Theme, ThemeMode};
///
/// let app = FrusApp::from_fn(|_| Box::new(text("hello")))
///     .title("Hello")
///     .theme(Theme::light())
///     .dark_theme(Theme::dark())
///     .theme_mode(ThemeMode::System);
/// # let _ = app;
/// ```
///
/// How it is dressed — its title, themes, language, zoom — is kept where a component can reach
/// it too, in [`host::app`]: what the builders here set, a component changes with the same
/// names (`host::app().set_theme_mode(..)`), and the shell reads it every frame.
pub struct FrusApp {
    root: Root,
    router: Option<GoRouter>,
    window_size: Option<(f32, f32)>,
    on_start: Option<Box<dyn FnOnce()>>,
    save: Option<Box<dyn Fn() -> Option<Vec<u8>>>>,
    restore: Option<Box<dyn Fn(&[u8])>>,
}

impl FrusApp {
    /// An application whose interface is the [`StatelessWidget`] `root`.
    pub fn new(root: impl StatelessWidget) -> Self {
        Self::with_root(Rc::new(move |cx| root.build(cx)))
    }

    /// An application whose interface is a function of the build context — hooks may be
    /// called in it.
    pub fn from_fn(root: impl Fn(&BuildContext) -> Box<dyn Widget> + 'static) -> Self {
        Self::with_root(Rc::new(root))
    }

    /// An application whose root is a [`StatefulWidget`]. It is configuration, made again
    /// on every rebuild — hence `Clone`.
    pub fn stateful<W: StatefulWidget + Clone>(root: W) -> Self {
        Self::with_root(Rc::new(move |_| {
            Box::new(Component::stateful(root.clone()))
        }))
    }

    /// An application whose pages are the routes of `router`: the router builds the interface,
    /// moves it when told a location, and answers the back gesture.
    ///
    /// ```
    /// use frus_shell::FrusApp;
    /// use frus_widgets::{text, GoRoute, GoRouter};
    ///
    /// let router = GoRouter::new(vec![
    ///     GoRoute::new("/", |_, _| text("home")),
    ///     GoRoute::new("/about", |_, _| text("about")),
    /// ]);
    /// let app = FrusApp::router(router).title("Pages");
    /// # let _ = app;
    /// ```
    pub fn router(router: GoRouter) -> Self {
        let building = router.clone();
        let mut app = Self::with_root(Rc::new(move |cx| building.build(cx)));
        app.router = Some(router);
        app
    }

    fn with_root(root: Root) -> Self {
        // A new application starts from the defaults, whatever the last one left behind.
        let app = host::app();
        app.set_title("frus".to_string());
        app.set_theme(Theme::light());
        app.set_dark_theme(None);
        app.set_theme_mode(ThemeMode::System);
        app.set_density(1.0);
        app.set_locale(None);
        app.set_supported_locales(Vec::new());
        app.set_localizations(None);
        Self {
            root,
            router: None,
            window_size: None,
            on_start: None,
            save: None,
            restore: None,
        }
    }

    /// The window's title.
    pub fn title(self, title: impl Into<String>) -> Self {
        host::app().set_title(title.into());
        self
    }

    /// The application's theme — its light one, where it has two.
    pub fn theme(self, theme: Theme) -> Self {
        host::app().set_theme(theme);
        self
    }

    /// The theme for a dark interface. With one, the application follows the system's
    /// brightness, or [`theme_mode`](Self::theme_mode).
    pub fn dark_theme(self, theme: Theme) -> Self {
        host::app().set_dark_theme(Some(theme));
        self
    }

    /// Which of the two themes is on display: the system's choice by default.
    pub fn theme_mode(self, mode: ThemeMode) -> Self {
        host::app().set_theme_mode(mode);
        self
    }

    /// The languages the application has, best first. The framework resolves the device's
    /// list against them.
    pub fn supported_locales(self, locales: Vec<Locale>) -> Self {
        host::app().set_supported_locales(locales);
        self
    }

    /// The words the framework says on the application's behalf, where they are not English.
    pub fn localizations(self, table: std::rc::Rc<dyn Localizations>) -> Self {
        host::app().set_localizations(Some(table));
        self
    }

    /// Runs `start` once, when the application starts and before anything is drawn: the place
    /// for what has to be true from the first frame — registering data, opening a store.
    pub fn on_start(mut self, start: impl FnOnce() + 'static) -> Self {
        self.on_start = Some(Box::new(start));
        self
    }

    /// Keeps what `save` returns across a live reload, and hands it to `restore` afterwards.
    /// A development convenience: the state of an application survives a recompilation.
    pub fn persist(
        mut self,
        save: impl Fn() -> Option<Vec<u8>> + 'static,
        restore: impl Fn(&[u8]) + 'static,
    ) -> Self {
        self.save = Some(Box::new(save));
        self.restore = Some(Box::new(restore));
        self
    }

    /// The window's initial size, in logical pixels.
    pub fn window_size(mut self, width: f32, height: f32) -> Self {
        self.window_size = Some((width, height));
        self
    }

    /// A zoom on the whole interface, on top of the system's scale.
    pub fn density(self, density: f32) -> Self {
        host::app().set_density(density);
        self
    }
}

impl Application for FrusApp {
    type Message = Callback;

    fn update(&mut self, callback: Callback) -> Command<Callback> {
        callback.call();
        Command::none()
    }

    fn init(&mut self) -> Command<Callback> {
        if let Some(start) = self.on_start.take() {
            start();
        }
        self.effects()
    }

    fn effects(&mut self) -> Command<Callback> {
        Command::batch(host::take_effects().into_iter().map(|effect| match effect {
            Effect::Focus(key) => Command::focus_hashed(key),
            Effect::Scroll(key, to) => Command::scroll_hashed(key, to),
            Effect::Sheet(key, to) => Command::sheet_hashed(key, to),
            Effect::Spawn(task) => Command::run(task),
            Effect::After(delay, callback) => Command::after(delay, callback),
        }))
    }

    fn save_state(&self) -> Option<Vec<u8>> {
        self.save.as_ref().and_then(|save| save())
    }

    fn restore_state(&mut self, bytes: &[u8]) {
        if let Some(restore) = &self.restore {
            restore(bytes);
        }
    }

    fn view(&self, _theme: &Theme) -> Box<dyn Widget<Callback>> {
        let root = self.root.clone();
        Box::new(Component::stateless(move |cx: &BuildContext| root(cx)))
    }

    fn subscription(&self) -> Subscription<Callback> {
        // The timers the components of the latest build asked for. The shell diffs them: a
        // timer whose component no longer asks for it is stopped.
        Subscription::batch(frus_widgets::intervals().into_iter().map(|interval| {
            let callback = interval.callback;
            Subscription::every_keyed(interval.id, interval.period, move |_| callback.clone())
        }))
    }

    fn tick(&mut self, dt: f32) -> bool {
        self.router.as_ref().is_some_and(|router| router.tick(dt))
    }

    fn can_go_back(&self) -> bool {
        // What an open menu or dialog takes first is not the router's to answer.
        !frus_widgets::back_blocked()
            && self
                .router
                .as_ref()
                .is_some_and(|router| router.can_go_back())
    }

    fn back_gesture(&mut self, progress: f32) {
        if let Some(router) = &self.router {
            router.back_gesture(progress);
        }
    }

    fn back_gesture_end(&mut self, velocity: f32) {
        if let Some(router) = &self.router {
            router.back_gesture_end(velocity);
        }
    }

    fn theme(&self) -> Theme {
        host::app().theme()
    }

    fn dark_theme(&self) -> Option<Theme> {
        host::app().dark_theme()
    }

    fn theme_mode(&self) -> ThemeMode {
        host::app().theme_mode()
    }

    fn supported_locales(&self) -> Vec<Locale> {
        host::app().supported_locales()
    }

    fn locale(&self) -> Option<Locale> {
        host::app().locale()
    }

    fn localizations(&self) -> Option<std::rc::Rc<dyn Localizations>> {
        host::app().localizations()
    }

    fn title(&self) -> String {
        host::app().title()
    }

    fn window_size(&self) -> Option<(f32, f32)> {
        self.window_size
    }

    fn density(&self) -> f32 {
        host::app().density()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::Driver;
    use frus_widgets::{Container, Point, State, StateContext};
    use std::cell::RefCell;

    const SIDE: f32 = 200.0;
    type Seen = Rc<RefCell<Vec<i32>>>;

    /// A window-sized target: a tap anywhere lands on it.
    fn target(on_tap: impl Into<Callback>) -> Box<dyn Widget> {
        Box::new(Container::new().width(SIDE).height(SIDE).on_click(on_tap))
    }

    fn tap(driver: &mut Driver<FrusApp>) {
        let at = Point::new(SIDE / 2.0, SIDE / 2.0);
        driver.press(at);
        driver.release(at);
        driver.frame(1.0 / 60.0);
    }

    #[test]
    fn a_hook_survives_a_tap_and_the_view_is_built_again_with_the_new_value() {
        let seen: Seen = Rc::default();
        let probe = seen.clone();
        let mut driver = Driver::new(
            FrusApp::from_fn(move |cx| {
                let count = cx.use_state(|| 0);
                probe.borrow_mut().push(count.get());
                let add = count.clone();
                target(move || add.update(|n| *n += 1))
            }),
            SIDE,
            SIDE,
        );
        driver.frame(1.0 / 60.0);
        assert_eq!(*seen.borrow(), [0], "built once to begin with");
        tap(&mut driver);
        tap(&mut driver);
        tap(&mut driver);
        assert_eq!(
            seen.borrow().last(),
            Some(&3),
            "three taps, three changes, each kept: {:?}",
            seen.borrow()
        );
    }

    #[derive(Clone)]
    struct Counter {
        seen: Seen,
    }

    struct CounterState {
        count: i32,
        seen: Seen,
    }

    impl StatefulWidget for Counter {
        type State = CounterState;
        fn create_state(&self) -> CounterState {
            CounterState {
                count: 100,
                seen: self.seen.clone(),
            }
        }
    }

    impl State for CounterState {
        type Widget = Counter;
        fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
            self.seen.borrow_mut().push(self.count);
            target(cx.callback(|state| state.count += 1))
        }
    }

    #[test]
    fn a_stateful_root_keeps_its_state_between_taps() {
        let seen: Seen = Rc::default();
        let mut driver = Driver::new(
            FrusApp::stateful(Counter { seen: seen.clone() }),
            SIDE,
            SIDE,
        );
        driver.frame(1.0 / 60.0);
        tap(&mut driver);
        tap(&mut driver);
        assert_eq!(seen.borrow().last(), Some(&102), "{:?}", seen.borrow());
    }

    #[test]
    fn a_value_handler_that_returns_nothing_runs_once_per_event() {
        use frus_widgets::Checkbox;
        let toggles: Rc<RefCell<Vec<bool>>> = Rc::default();
        let log = toggles.clone();
        let mut driver = Driver::new(
            FrusApp::from_fn(move |_| {
                let log = log.clone();
                Box::new(Checkbox::new(false).on_toggle(move |on| log.borrow_mut().push(on)))
            }),
            SIDE,
            SIDE,
        );
        driver.frame(1.0 / 60.0);
        // a checkbox is small, and at the window's corner
        let at = Point::new(12.0, 12.0);
        driver.press(at);
        driver.release(at);
        driver.frame(1.0 / 60.0);
        assert_eq!(*toggles.borrow(), [true], "one tap, one call");
    }

    #[test]
    fn a_timer_a_component_asks_for_is_a_subscription_while_it_asks() {
        let on = Rc::new(std::cell::Cell::new(true));
        let wanted = on.clone();
        let mut driver = Driver::new(
            FrusApp::from_fn(move |cx| {
                if wanted.get() {
                    cx.use_interval(std::time::Duration::from_secs(1), || {});
                }
                target(|| {})
            }),
            SIDE,
            SIDE,
        );
        assert!(driver.app().subscription().is_empty(), "nothing built yet");
        driver.frame(1.0 / 60.0);
        assert_eq!(driver.app().subscription().ids().len(), 1);
        on.set(false);
        frus_widgets::request_rebuild();
        driver.frame(1.0 / 60.0);
        assert!(
            driver.app().subscription().is_empty(),
            "and it stops when it is not asked for"
        );
    }

    #[test]
    fn an_effect_runs_after_the_frame_and_a_change_it_makes_is_shown() {
        let seen: Seen = Rc::default();
        let probe = seen.clone();
        let mut driver = Driver::new(
            FrusApp::from_fn(move |cx| {
                let count = cx.use_state(|| 0);
                probe.borrow_mut().push(count.get());
                let set = count.clone();
                cx.use_effect((), move || set.set(7));
                target(|| {})
            }),
            SIDE,
            SIDE,
        );
        driver.frame(1.0 / 60.0);
        driver.frame(1.0 / 60.0);
        assert_eq!(
            *seen.borrow(),
            [0, 7],
            "built with 0, the effect set 7, and the next frame built with it"
        );
    }
}

#[cfg(test)]
mod router_tests {
    use super::*;
    use crate::app::testing::Driver;
    use frus_widgets::{component, Container, GoRoute, Point};
    use std::cell::RefCell;

    const SIDE: f32 = 200.0;

    fn target(on_tap: impl Into<Callback>) -> Box<dyn Widget> {
        Box::new(Container::new().width(SIDE).height(SIDE).on_click(on_tap))
    }

    fn tap(driver: &mut Driver<FrusApp>) {
        let at = Point::new(SIDE / 2.0, SIDE / 2.0);
        driver.press(at);
        driver.release(at);
        driver.frame(1.0 / 60.0);
    }

    /// Two pages: the first counts its taps in a hook and pushes the second; the second pops.
    fn app(seen: Rc<RefCell<Vec<i32>>>) -> (FrusApp, GoRouter) {
        let router = GoRouter::new(vec![
            GoRoute::new("/", move |_, _| {
                let seen = seen.clone();
                component(move |cx| {
                    let visits = cx.use_state(|| 0);
                    seen.borrow_mut().push(visits.get());
                    let router = cx.router();
                    target(move || {
                        visits.update(|n| *n += 1);
                        router.push("/second");
                    })
                })
            }),
            GoRoute::new("/second", |cx, _| {
                let router = cx.router();
                target(move || {
                    router.pop();
                })
            }),
        ]);
        (FrusApp::router(router.clone()), router)
    }

    #[test]
    fn a_tap_pushes_a_page_and_the_covered_page_keeps_what_its_components_hold() {
        let seen = Rc::default();
        let (app, router) = app(Rc::clone(&seen));
        let mut driver = Driver::new(app, SIDE, SIDE);
        driver.frame(1.0 / 60.0);
        assert_eq!((router.location(), router.depth()), ("/".to_string(), 1));

        tap(&mut driver);
        driver.run(1.5);
        assert_eq!(router.location(), "/second", "the tap pushed a page");
        assert_eq!(router.depth(), 2);
        assert_eq!(
            seen.borrow().last(),
            Some(&1),
            "the page underneath is still built, holding what it held: {:?}",
            seen.borrow()
        );

        tap(&mut driver);
        driver.run(1.5);
        assert_eq!(
            router.location(),
            "/",
            "the tap on the second page went back"
        );
        assert_eq!(
            seen.borrow().last(),
            Some(&1),
            "and the first page is shown again with its state, not made anew"
        );
    }

    #[test]
    fn a_back_gesture_carries_the_top_page_away() {
        let (app, router) = app(Rc::default());
        let mut driver = Driver::new(app, SIDE, SIDE);
        driver.frame(1.0 / 60.0);
        router.push("/second");
        driver.run(1.5);
        assert!(FrusApp::router(router.clone()).can_go_back());

        router.back_gesture(0.9);
        driver.frame(1.0 / 60.0);
        router.back_gesture_end(0.0);
        driver.run(1.5);
        assert_eq!((router.location(), router.depth()), ("/".to_string(), 1));
    }

    #[test]
    fn a_popped_page_is_disposed_once_its_transition_is_over() {
        let disposed = Rc::new(std::cell::Cell::new(0));
        let counted = disposed.clone();
        let router = GoRouter::new(vec![
            GoRoute::new("/", |_, _| Container::new().width(SIDE).height(SIDE)),
            GoRoute::new("/gone", move |_, _| {
                let counted = counted.clone();
                component(move |cx| {
                    let counted = counted.clone();
                    cx.use_effect((), move || move || counted.set(counted.get() + 1));
                    Box::new(Container::new().width(SIDE).height(SIDE)) as Box<dyn Widget>
                })
            }),
        ]);
        let mut driver = Driver::new(FrusApp::router(router.clone()), SIDE, SIDE);
        driver.frame(1.0 / 60.0);
        router.push("/gone");
        driver.run(1.5);
        assert_eq!(disposed.get(), 0, "on the stack, it is alive");
        router.pop();
        driver.run(1.5);
        assert_eq!(disposed.get(), 1, "popped and gone, its cleanup has run");
    }
}

#[cfg(test)]
mod host_tests {
    use super::*;
    use crate::app::testing::Driver;
    use frus_widgets::{component, Container, GoRoute, ScrollTo};
    use std::cell::{Cell, RefCell};

    const SIDE: f32 = 200.0;

    fn target(on_tap: impl Into<Callback>) -> Box<dyn Widget> {
        Box::new(Container::new().width(SIDE).height(SIDE).on_click(on_tap))
    }

    fn quiet() -> FrusApp {
        FrusApp::from_fn(|_| target(|| {}))
    }

    #[test]
    fn the_builders_and_a_component_write_to_the_same_settings_the_shell_reads() {
        let app = quiet()
            .title("first")
            .theme_mode(ThemeMode::Light)
            .density(1.1);
        assert_eq!(Application::title(&app), "first");
        assert_eq!(Application::theme_mode(&app), ThemeMode::Light);
        assert_eq!(Application::density(&app), 1.1);
        // What a handler does from inside the tree.
        host::app().set_title("second".to_string());
        host::app().set_theme_mode(ThemeMode::Dark);
        host::app().set_dark_theme(Some(Theme::dark()));
        assert_eq!(Application::title(&app), "second");
        assert_eq!(Application::theme_mode(&app), ThemeMode::Dark);
        assert!(Application::dark_theme(&app).is_some());
    }

    #[test]
    fn a_new_application_starts_from_the_defaults_not_from_the_last_one() {
        host::app().set_theme_mode(ThemeMode::Dark);
        host::app().set_density(1.4);
        host::app().set_locale(Some(Locale::new("fr")));
        let app = quiet();
        assert_eq!(Application::theme_mode(&app), ThemeMode::System);
        assert_eq!(Application::density(&app), 1.0);
        assert_eq!(Application::locale(&app), None);
    }

    #[test]
    fn the_language_a_component_picks_is_the_one_the_application_answers() {
        let app = quiet().supported_locales(vec![Locale::new("en"), Locale::new("fr")]);
        assert_eq!(Application::supported_locales(&app).len(), 2);
        host::app().set_locale(Some(Locale::new("fr")));
        assert_eq!(Application::locale(&app), Some(Locale::new("fr")));
    }

    #[test]
    fn what_a_component_asks_of_the_shell_becomes_a_command_once() {
        let mut app = quiet();
        let _ = host::take_effects();
        assert!(app.effects().is_empty(), "nothing asked, nothing to run");
        host::focus("name");
        host::scroll_to("list", ScrollTo::start());
        host::after(std::time::Duration::from_secs(1), || {});
        host::spawn(|| 1 + 1, |_| {});
        assert!(!app.effects().is_empty());
        assert!(app.effects().is_empty(), "and taken once");
    }

    #[test]
    fn on_start_runs_once_before_the_first_frame() {
        let started = Rc::new(Cell::new(0));
        let count = started.clone();
        let mut app = quiet().on_start(move || count.set(count.get() + 1));
        let _ = app.init();
        let _ = app.init();
        assert_eq!(started.get(), 1);
    }

    #[test]
    fn a_state_can_be_kept_across_a_live_reload() {
        let held = Rc::new(RefCell::new(0u8));
        let (read, write) = (held.clone(), held.clone());
        let mut app = quiet().persist(
            move || Some(vec![*read.borrow()]),
            move |bytes| *write.borrow_mut() = bytes[0],
        );
        *held.borrow_mut() = 7;
        let bytes = app.save_state().expect("something to keep");
        *held.borrow_mut() = 0;
        app.restore_state(&bytes);
        assert_eq!(*held.borrow(), 7);
        assert!(quiet().save_state().is_none(), "nothing kept unless asked");
    }

    #[test]
    fn an_open_menu_takes_the_back_gesture_before_the_router_does() {
        let open = Rc::new(Cell::new(false));
        let asked = open.clone();
        let router = GoRouter::new(vec![GoRoute::new("/", |_, _| target(|| {})).routes(vec![
            GoRoute::new("second", move |_, _| {
                let asked = asked.clone();
                component(move |cx| {
                    cx.block_back(asked.get());
                    target(|| {})
                })
            }),
        ])]);
        let mut driver = Driver::new(FrusApp::router(router.clone()), SIDE, SIDE);
        driver.frame(1.0 / 60.0);
        assert!(
            !driver.app().can_go_back(),
            "nowhere to go back to from the first page"
        );
        router.push("/second");
        driver.run(1.5);
        assert!(driver.app().can_go_back(), "a page to go back to");
        open.set(true);
        frus_widgets::request_rebuild();
        driver.frame(1.0 / 60.0);
        assert!(
            !driver.app().can_go_back(),
            "the menu is open: back closes it, it does not leave the page"
        );
        open.set(false);
        frus_widgets::request_rebuild();
        driver.frame(1.0 / 60.0);
        assert!(driver.app().can_go_back(), "and once it is shut, it does");
    }
}

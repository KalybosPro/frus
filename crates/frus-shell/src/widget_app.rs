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

use frus_widgets::{
    BuildContext, Callback, Component, GoRouter, StatefulWidget, StatelessWidget, Theme, ThemeMode,
    Widget,
};

use crate::application::Application;
use crate::command::Command;

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
pub struct FrusApp {
    root: Root,
    router: Option<GoRouter>,
    title: String,
    theme: Theme,
    dark_theme: Option<Theme>,
    theme_mode: ThemeMode,
    window_size: Option<(f32, f32)>,
    density: f32,
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
    ///     GoRoute::new("/", |_, _| Box::new(text("home"))),
    ///     GoRoute::new("/about", |_, _| Box::new(text("about"))),
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
        Self {
            root,
            router: None,
            title: "frus".to_string(),
            theme: Theme::light(),
            dark_theme: None,
            theme_mode: ThemeMode::System,
            window_size: None,
            density: 1.0,
        }
    }

    /// The window's title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// The application's theme — its light one, where it has two.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// The theme for a dark interface. With one, the application follows the system's
    /// brightness, or [`theme_mode`](Self::theme_mode).
    pub fn dark_theme(mut self, theme: Theme) -> Self {
        self.dark_theme = Some(theme);
        self
    }

    /// Which of the two themes is on display: the system's choice by default.
    pub fn theme_mode(mut self, mode: ThemeMode) -> Self {
        self.theme_mode = mode;
        self
    }

    /// The window's initial size, in logical pixels.
    pub fn window_size(mut self, width: f32, height: f32) -> Self {
        self.window_size = Some((width, height));
        self
    }

    /// A zoom on the whole interface, on top of the system's scale.
    pub fn density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }
}

impl Application for FrusApp {
    type Message = Callback;

    fn update(&mut self, callback: Callback) -> Command<Callback> {
        callback.call();
        Command::none()
    }

    fn view(&self, _theme: &Theme) -> Box<dyn Widget<Callback>> {
        let root = self.root.clone();
        Box::new(Component::stateless(move |cx: &BuildContext| root(cx)))
    }

    fn tick(&mut self, dt: f32) -> bool {
        self.router.as_ref().is_some_and(|router| router.tick(dt))
    }

    fn can_go_back(&self) -> bool {
        self.router
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
        self.theme.clone()
    }

    fn dark_theme(&self) -> Option<Theme> {
        self.dark_theme.clone()
    }

    fn theme_mode(&self) -> ThemeMode {
        self.theme_mode
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn window_size(&self) -> Option<(f32, f32)> {
        self.window_size
    }

    fn density(&self) -> f32 {
        self.density
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

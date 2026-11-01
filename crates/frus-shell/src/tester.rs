//! [`Tester`]: an application's page, driven by a test the way a person drives it
//! (milestone 716).
//!
//! The shell has had a driver with no window since early on: frames run by hand, presses
//! and keys sent down the same paths a window's go down. frus's own tests use it. This is
//! that driver in the words an application's test is written in — tap what says "Send",
//! type into the field labelled "Title", wait, look for "Created" — so that a page is
//! tested as it is used, without a window, a GPU or a device.

use crate::app::testing::Driver;
use crate::application::Application;

/// A page under test: an application on a surface of a stated size, with no window.
///
/// Add frus with its `testing` feature for the tests only:
///
/// ```toml
/// [dev-dependencies]
/// frus = { version = "0.5", features = ["testing"] }
/// ```
///
/// ```
/// use frus_shell::{FrusApp, Tester};
/// use frus_widgets::{component, Button, Column, Text, Widget};
///
/// let page = || {
///     component(|cx| {
///         let count = cx.use_state(|| 0);
///         let add = count.clone();
///         Box::new(
///             Column::new()
///                 .child(Text::new(format!("Count: {}", count.get())))
///                 .child(Button::new("Add").on_press(move || add.update(|n| *n += 1))),
///         ) as Box<dyn Widget>
///     })
/// };
/// let mut tester = Tester::new(FrusApp::from_fn(move |_| Box::new(page())));
/// assert!(tester.shows("Count: 0"));
/// tester.tap("Add");
/// assert!(tester.shows("Count: 1"));
/// ```
pub struct Tester<A: Application> {
    driver: Driver<A>,
}

impl<A: Application> Tester<A> {
    /// The application on a phone-sized surface (400 by 800), drawn until it is still.
    pub fn new(app: A) -> Self {
        Self::sized(app, 400.0, 800.0)
    }

    /// The application on a `width` by `height` surface, drawn until it is still.
    pub fn sized(app: A, width: f32, height: f32) -> Self {
        let mut tester = Self {
            driver: Driver::new(app, width, height),
        };
        tester.settle();
        tester
    }

    /// Taps what reads `label` — a button's text, a list row, a link — and draws the frame
    /// that follows.
    ///
    /// # Panics
    ///
    /// When nothing on screen reads `label`: a test that taps what is not there has found
    /// what it was written to find.
    pub fn tap(&mut self, label: &str) -> &mut Self {
        if !self.driver.tap_text(label) && !self.driver.assistive_click(label) {
            panic!(
                "nothing on screen reads {label:?}; the screen shows {:?}",
                self.texts()
            );
        }
        self.settle()
    }

    /// Writes `text` into the field labelled `label`, as a person typing it would, and
    /// draws the frame that follows.
    ///
    /// # Panics
    ///
    /// When there is no field labelled `label`.
    pub fn enter_text(&mut self, label: &str, text: &str) -> &mut Self {
        if !self.driver.assistive_set_value(label, text) {
            panic!(
                "there is no field labelled {label:?}; the screen shows {:?}",
                self.texts()
            );
        }
        self.settle()
    }

    /// The system's Back — Android's back button, the browser's — and the frames that
    /// follow, until the screen is still. Whether it went back from anything.
    pub fn back(&mut self) -> bool {
        let went = self.driver.system_back();
        self.settle();
        went
    }

    /// Lets `seconds` pass, frame by frame: timers fire, animations run, what a handler
    /// awaited arrives.
    pub fn wait(&mut self, seconds: f32) -> &mut Self {
        self.driver.run(seconds);
        self
    }

    /// Draws frames until nothing moves — a page transition, a fade, a list settling — or
    /// for five seconds at most.
    pub fn settle(&mut self) -> &mut Self {
        for _ in 0..300 {
            self.driver.frame(1.0 / 60.0);
            let moving = self.driver.animating()
                || self
                    .driver
                    .frame_parts()
                    .is_some_and(|(ui, _)| ui.wants_animation());
            if !moving {
                break;
            }
        }
        // And one more, for what the last one changed.
        self.driver.frame(1.0 / 60.0);
        self
    }

    /// Whether some text on screen contains `text`.
    pub fn shows(&self, text: &str) -> bool {
        self.texts().iter().any(|shown| shown.contains(text))
    }

    /// The text on screen, in the order it was drawn.
    pub fn texts(&self) -> Vec<String> {
        self.driver
            .texts()
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    /// The application itself, to look at its state.
    pub fn app(&self) -> &A {
        self.driver.app()
    }
}

/// Milestone 716, end to end: each of the things that got easier, through a whole
/// application and the tester an application's tests use.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrusApp;
    use frus_widgets::{
        component, host, AppBar, Button, Column, GoRoute, GoRouter, Icons, ListView, Scaffold,
        SnackBar, Text, TextField, Widget,
    };
    use frus_widgets::{AnimationController, Curve};

    fn app(page: impl Fn() -> Box<dyn Widget> + 'static) -> FrusApp {
        FrusApp::from_fn(move |_| page())
    }

    #[test]
    fn a_tap_and_typing_change_what_the_page_shows() {
        let mut tester = Tester::new(app(|| {
            Box::new(component(|cx| {
                let name = cx.use_state(String::new);
                let count = cx.use_state(|| 0);
                let (typed, add) = (name.clone(), count.clone());
                Box::new(
                    Column::new()
                        .child(
                            TextField::new(name.get())
                                .label("Name")
                                .on_input(move |t: String| typed.set(t)),
                        )
                        .child(Text::new(format!("Hello {} #{}", name.get(), count.get())))
                        .child(Button::new("Add").on_press(move || add.update(|n| *n += 1))),
                ) as Box<dyn Widget>
            }))
        }));
        assert!(tester.shows("Hello  #0"), "{:?}", tester.texts());
        tester.enter_text("Name", "Ada").tap("Add");
        assert!(tester.shows("Hello Ada #1"), "{:?}", tester.texts());
    }

    #[test]
    #[should_panic(expected = "nothing on screen reads")]
    fn a_tap_on_what_is_not_there_fails_the_test() {
        Tester::new(app(|| Box::new(Text::new("only this")))).tap("Send");
    }

    #[test]
    fn a_snack_bar_is_shown_over_the_page_and_goes_after_its_time() {
        host::clear_snack_bars();
        let mut tester = Tester::new(app(|| {
            Box::new(Button::new("Save").on_press(|| host::show_snack_bar(SnackBar::new("Saved"))))
        }));
        assert!(!tester.shows("Saved"));
        tester.tap("Save");
        assert!(tester.shows("Saved"), "shown at once: {:?}", tester.texts());
        // The page under it is still there.
        assert!(tester.texts().iter().any(|t| t == "Save"));
        tester.wait(4.5);
        assert!(
            !tester.shows("Saved"),
            "gone after four seconds: {:?}",
            tester.texts()
        );
    }

    #[test]
    fn a_snack_bars_action_runs_and_hides_it() {
        host::clear_snack_bars();
        let mut tester = Tester::new(app(|| {
            Box::new(component(|cx| {
                let undone = cx.use_state(|| false);
                let undo = undone.clone();
                Box::new(
                    Column::new()
                        .child(Text::new(if undone.get() { "restored" } else { "deleted" }))
                        .child(Button::new("Delete").on_press(move || {
                            let undo = undo.clone();
                            host::show_snack_bar(
                                SnackBar::new("Message deleted")
                                    .action("Undo", (move || undo.set(true)).into()),
                            )
                        })),
                ) as Box<dyn Widget>
            }))
        }));
        tester.tap("Delete").tap("UNDO");
        assert!(tester.shows("restored"), "{:?}", tester.texts());
        assert!(!tester.shows("Message deleted"));
    }

    fn pages() -> GoRouter {
        GoRouter::new(vec![
            GoRoute::new("/", |cx, _| {
                let router = cx.router();
                // A scaffold and an app bar handed over as they are: no `.build()`.
                Scaffold::new()
                    .app_bar(AppBar::new().title(Text::new("Home")))
                    .body(Button::new("Open").on_press(move || router.push("/detail")))
            }),
            GoRoute::new("/detail", |_, _| {
                Scaffold::new().app_bar(AppBar::new().title(Text::new("Detail")))
            }),
        ])
    }

    /// What a screen reader calls the back arrow.
    const BACK: &str = "Back";

    fn offers_back(tester: &mut Tester<FrusApp>) -> bool {
        tester.driver.assistive_click("__nothing__");
        tester.driver.frame_parts().is_some_and(|(ui, _)| {
            ui.semantics()
                .iter()
                .any(|(_, _, sem)| sem.label.as_deref() == Some(BACK))
        })
    }

    #[test]
    fn a_pushed_page_s_app_bar_offers_the_way_back_and_the_first_does_not() {
        let mut tester = Tester::new(FrusApp::router(pages()));
        assert!(tester.shows("Home"));
        assert!(!offers_back(&mut tester), "nothing beneath the first page");
        tester.tap("Open");
        assert!(tester.shows("Detail"), "{:?}", tester.texts());
        assert!(offers_back(&mut tester));
        tester.tap(BACK);
        assert!(!tester.shows("Detail"), "{:?}", tester.texts());
        assert!(tester.shows("Home"));
    }

    #[test]
    fn a_function_component_picks_a_destination_with_a_plain_closure() {
        let mut tester = Tester::new(app(|| {
            Box::new(component(|cx| {
                let tab = cx.use_state(|| 0usize);
                let pick = tab.clone();
                Box::new(
                    Scaffold::new()
                        .body(Text::new(["On notes", "On alarms"][tab.get()]))
                        .nav(tab.get(), move |i: usize| pick.set(i))
                        .destination(Icons::NOTE, "Notes")
                        .destination(Icons::ALARM, "Alarms"),
                ) as Box<dyn Widget>
            }))
        }));
        assert!(tester.shows("On notes"));
        tester.tap("Alarms");
        assert!(tester.shows("On alarms"), "{:?}", tester.texts());
    }

    #[test]
    fn rows_of_different_heights_are_built_only_in_view_and_stacked_by_their_own_height() {
        let tester = Tester::sized(
            app(|| {
                Box::new(ListView::builder(1000, |i| {
                    let tall = if i % 2 == 0 { 20.0 } else { 60.0 };
                    Column::new()
                        .child(Text::new(format!("row {i}")))
                        .child(frus_widgets::SizedBox::empty().height(tall))
                }))
            }),
            300.0,
            400.0,
        );
        let texts = tester.driver.texts();
        let rows: Vec<_> = texts
            .iter()
            .filter(|(t, _)| t.starts_with("row "))
            .collect();
        assert!(
            !rows.is_empty() && rows.len() < 30,
            "only what is in view: {}",
            rows.len()
        );
        let y = |name: &str| {
            rows.iter()
                .find(|(t, _)| t == name)
                .map(|(_, r)| r.y)
                .expect(name)
        };
        let (r0, r1, r2) = (y("row 0"), y("row 1"), y("row 2"));
        // Row 1 is 40 taller than row 0, so the gap after it is 40 longer.
        assert!(((r2 - r1) - (r1 - r0) - 40.0).abs() < 1.0, "{r0} {r1} {r2}");
    }

    #[test]
    fn an_animation_a_component_starts_is_moved_by_the_frames() {
        let mut tester = Tester::new(app(|| {
            Box::new(component(|cx| {
                let grow = cx.use_animation_controller(AnimationController::unit);
                let start = grow.clone();
                Box::new(
                    Column::new()
                        .child(Text::new(format!("at {:.1}", grow.value())))
                        .child(
                            Button::new("Go").on_press(move || start.forward(0.5, Curve::Linear)),
                        ),
                ) as Box<dyn Widget>
            }))
        }));
        assert!(tester.shows("at 0.0"));
        // `tap` settles: the frames run the animation to its end.
        tester.tap("Go");
        assert!(tester.shows("at 1.0"), "{:?}", tester.texts());
    }

    /// **A row above the window that grows does not move the rows being read**
    /// (milestone 717): a picture that loads in a row half scrolled past, a row seen for the
    /// first time on the way back up.
    #[test]
    fn a_row_half_scrolled_past_that_grows_leaves_the_rows_in_view_where_they_were() {
        const LIST: &str = "feed";
        let mut tester = Tester::sized(
            app(|| {
                Box::new(component(|cx| {
                    let tall = cx.use_state(|| false);
                    let grow = tall.clone();
                    let third = tall.get();
                    Box::new(
                        Column::new()
                            .child(Button::new("Grow").on_press(move || grow.set(true)))
                            .child(frus_widgets::Expanded::new(frus_widgets::keyed(
                                LIST,
                                ListView::builder(100, move |i| {
                                    let height = if i == 3 && third { 150.0 } else { 50.0 };
                                    Column::new().child(Text::new(format!("row {i}"))).child(
                                        frus_widgets::SizedBox::empty().height(height - 20.0),
                                    )
                                }),
                            ))),
                    ) as Box<dyn Widget>
                }))
            }),
            300.0,
            500.0,
        );
        // Down to the middle of row 3 (150..200): it starts above the window's top.
        host::scroll_to(LIST, frus_widgets::ScrollTo::y(175.0).instant());
        tester.settle();
        let y = |tester: &Tester<FrusApp>, name: &str| {
            tester
                .driver
                .texts()
                .into_iter()
                .find(|(t, _)| t == name)
                .map(|(_, r)| r.y)
                .unwrap_or_else(|| panic!("{name} not on screen: {:?}", tester.texts()))
        };
        let before = y(&tester, "row 6");
        tester.tap("Grow");
        let after = y(&tester, "row 6");
        assert!(
            (after - before).abs() < 1.0,
            "row 6 moved from {before} to {after} when row 3, above it, grew"
        );
    }

    /// **A list a component returns as its whole tree is found by its key** (milestone 729):
    /// a component's key is its own, so the lookup that `host::scroll_to` and `host::focus`
    /// use walked past the `keyed(..)` the component built, though the list was laid out under
    /// the component's identity.
    #[test]
    fn a_keyed_list_that_is_a_component_s_whole_tree_can_be_scrolled_by_its_key() {
        let mut tester = Tester::sized(
            app(|| {
                Box::new(frus_widgets::keyed(
                    "list",
                    ListView::new(500, 40.0, |i| Text::new(format!("row {i}"))).height(400.0),
                ))
            }),
            300.0,
            600.0,
        );
        host::scroll_to("list", frus_widgets::ScrollTo::y(400.0).instant());
        tester.settle();
        assert!(tester.shows("row 10"), "{:?}", tester.texts());
        assert!(!tester.shows("row 0"), "{:?}", tester.texts());
    }

    /// Where the row named `name` was drawn, and where the list's window starts.
    fn row_y(tester: &Tester<FrusApp>, name: &str) -> f32 {
        tester
            .driver
            .texts()
            .into_iter()
            .find(|(t, _)| t == name)
            .map(|(_, r)| r.y)
            .unwrap_or_else(|| panic!("{name} not on screen: {:?}", tester.texts()))
    }

    fn list_top(tester: &Tester<FrusApp>) -> f32 {
        tester.driver.offsets()[0].0.y
    }

    fn list_page(variable: bool) -> FrusApp {
        app(move || {
            let rows = move |i: usize| {
                // Every third row is three times as tall, in the list of any height.
                let height = if variable && i.is_multiple_of(3) {
                    120.0
                } else {
                    40.0
                };
                Column::new()
                    .child(Text::new(format!("row {i}")))
                    .child(frus_widgets::SizedBox::empty().height(height - 20.0))
            };
            let list = if variable {
                ListView::builder(500, rows)
            } else {
                ListView::new(500, 40.0, rows)
            };
            Box::new(
                Column::new()
                    .child(Text::new("Inbox"))
                    .child(frus_widgets::keyed("list", list.height(400.0))),
            )
        })
    }

    /// **`ScrollTo::index` brings an item to the window's edge** (milestone 719), in a
    /// list of one height and in one of any height, whose rows before it were never seen.
    #[test]
    fn scrolling_to_an_index_brings_that_row_to_the_top() {
        for variable in [false, true] {
            let mut tester = Tester::sized(list_page(variable), 300.0, 600.0);
            host::scroll_to("list", frus_widgets::ScrollTo::index(120));
            tester.settle();
            let (top, row) = (list_top(&tester), row_y(&tester, "row 120"));
            assert!(
                (row - top).abs() < 1.0,
                "variable {variable}: row 120 at {row}, the window starts at {top}"
            );
        }
    }

    #[test]
    fn an_aligned_index_comes_to_rest_in_the_middle() {
        let mut tester = Tester::sized(list_page(false), 300.0, 600.0);
        host::scroll_to(
            "list",
            frus_widgets::ScrollTo::index(60).aligned(0.5).instant(),
        );
        tester.settle();
        let top = list_top(&tester);
        let row = row_y(&tester, "row 60");
        // A 40-pixel row in the middle of a 400-pixel window starts 180 pixels in.
        assert!(
            (row - top - 180.0).abs() < 1.0,
            "row 60 at {row}, window at {top}"
        );
    }
}

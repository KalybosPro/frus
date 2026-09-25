//! Tests that cut across modules — a whole screen's behaviour, an interaction from a tap to
//! the scene. Tests of a single module's own logic live next to it.
//!
//! They drive the application the way the shell does: built from its router, on a surface of a
//! stated size, one frame at a time. A screen is reached the way a reader reaches it — by
//! telling the router where to go — and a control is pressed by what it says.

use crate::prelude::*;
use crate::screens::*;
use frus_shell::testing::Driver;
use frus_shell::{Application, FrusApp};
use frus_widgets::{
    build_ui, build_ui_inspected, find_widget, Insets, Point, Primitive, Runtime, Ui, WindowInsets,
};

/// The demo on a surface, frame by frame, with the handles a test wants beside it.
///
/// Building a frame is what the shell does around `view`: a surface described, the states of
/// the components marked as reached, the tree laid out, and what it did not reach let go.
/// The runtime is kept from one frame to the next, so what a screen keeps is kept.
struct Bench {
    app: FrusApp,
    router: GoRouter,
    demo: Rc<Demo>,
    runtime: Runtime,
    size: Size,
    insets: WindowInsets,
}

impl Bench {
    fn new(width: f32, height: f32) -> Self {
        let (app, router, demo) = crate::build();
        let bench = Self {
            app,
            router,
            demo,
            runtime: Runtime::default(),
            size: Size::new(width, height),
            insets: WindowInsets::ZERO,
        };
        // The first frame starts the router.
        let _ = bench.frame();
        bench
    }

    /// Adds tasks, as the reader would have.
    fn with_tasks(self, labels: &[&str]) -> Self {
        for label in labels {
            self.demo.add(label);
        }
        self
    }

    /// Goes to a screen and lets the slide finish.
    fn go(&mut self, location: &str) -> &mut Self {
        self.router.push(location);
        self.settle();
        self
    }

    /// Runs the transition in flight, if any, to its end.
    fn settle(&mut self) {
        for _ in 0..400 {
            if !Application::tick(&mut self.app, 0.05) {
                return;
            }
        }
        panic!("the transition never ended");
    }

    /// One frame: the tree, and the interface laid out from it.
    fn frame(&self) -> (Box<dyn Widget>, Ui) {
        let theme = self
            .app
            .resolved_theme(frus_widgets::Brightness::Dark, false);
        MediaQuery::new(self.size)
            .with_insets(self.insets)
            .scope(|| {
                self.runtime.states.begin_build();
                let tree = Application::view(&self.app, &theme);
                frus_widgets::build_deferred(tree.as_ref(), &theme, &self.runtime);
                let ui = build_ui(tree.as_ref(), self.size, &self.runtime, &theme);
                self.runtime.states.end_frame();
                (tree, ui)
            })
    }

    /// Every word the frame paints, with where.
    fn texts(&self) -> Vec<(String, f32, f32)> {
        let (_, ui) = self.frame();
        words(&ui)
    }

    /// Just the words.
    fn words(&self) -> Vec<String> {
        self.texts().into_iter().map(|(text, _, _)| text).collect()
    }

    /// Presses whatever the first word reading `label` is on, the way a tap does: what is
    /// under its middle is asked for its message, and the message is delivered. Whether there
    /// was such a word with something to press.
    fn press(&self, label: &str) -> bool {
        let (_, ui) = self.frame();
        press_in(&ui, label)
    }
}

/// The words a frame paints: the text and where it starts.
fn words(ui: &Ui) -> Vec<(String, f32, f32)> {
    fn walk(primitives: &[Primitive], out: &mut Vec<(String, f32, f32)>) {
        for p in primitives {
            match p {
                Primitive::Text { text, position, .. } => {
                    out.push((text.clone(), position.x, position.y))
                }
                Primitive::Layer { primitives, .. } => walk(primitives, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(ui.scene().primitives(), &mut out);
    out
}

/// Presses the first thing under a word that reads `label`. Whether it found one.
fn press_in(ui: &Ui, label: &str) -> bool {
    let bounds: Vec<frus_widgets::Rect> = {
        fn walk(primitives: &[Primitive], label: &str, out: &mut Vec<frus_widgets::Rect>) {
            for p in primitives {
                match p {
                    Primitive::Text { text, .. } if text == label => out.push(p.bounds()),
                    Primitive::Layer { primitives, .. } => walk(primitives, label, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(ui.scene().primitives(), label, &mut out);
        out
    };
    for rect in bounds {
        let at = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        if let Some(callback) = ui.hit(at).and_then(|id| ui.msg_for(id)) {
            callback.call();
            return true;
        }
    }
    false
}

/// A phone in portrait, in logical pixels.
const PHONE: (f32, f32) = (411.0, 869.0);

/// Every screen, by where it lives.
const SCREENS: [&str; 11] = [
    "/",
    "/settings",
    "/journal",
    "/wizard",
    "/grid",
    "/charts",
    "/data",
    "/board",
    "/tour",
    "/licenses",
    "/sheet",
];

#[test]
fn view_builds_a_non_empty_scene() {
    let bench = Bench::new(800.0, 600.0).with_tasks(&["a task"]);
    let (_, ui) = bench.frame();
    assert!(!ui.scene().primitives().is_empty());
    assert!(bench.words().iter().any(|w| w == "a task"));
}

/// **The screen keeps clear of the bars, and nothing in the application says so.**
///
/// The application is not handed the size and does not subtract the notch from it; the
/// surface description carries both, the `Scaffold` reads it, and a screen built without one
/// wraps itself in a `SafeArea` that reads the same description.
#[test]
fn a_screen_keeps_clear_of_the_bars_without_being_told() {
    // A tall status bar and a navigation bar, as a phone reports them.
    const TOP: f32 = 84.0;
    const BOTTOM: f32 = 45.0;
    for location in ["/", "/settings", "/journal", "/charts", "/wizard", "/board"] {
        let mut bench = Bench::new(400.0, 800.0).with_tasks(&["short"]);
        bench.insets = WindowInsets::bars(Insets::new(TOP, 0.0, BOTTOM, 0.0));
        if location != "/" {
            bench.go(location);
        }
        let (_, ui) = bench.frame();
        // What a scroll region holds beyond its viewport is emitted but clipped away: it is
        // the region's business, and the region itself is checked against the bars below.
        let regions = ui.scroll_regions().to_vec();
        for region in &regions {
            let v = region.viewport;
            // Beside the window (a page leaving), or wholly below it: a scroll region inside
            // another one, scrolled out of view.
            if v.x < 0.0 || v.x >= bench.size.width || v.y >= bench.size.height - BOTTOM {
                continue;
            }
            assert!(
                v.y >= TOP - 0.5,
                "{location}: a scroll region starts at {}, under the status bar",
                v.y
            );
            assert!(
                v.y + v.height <= bench.size.height - BOTTOM + 0.5,
                "{location}: a scroll region ends at {}, under the navigation bar",
                v.y + v.height
            );
        }
        for (text, x, y) in words(&ui) {
            // Only what is **on** the window: a page leaving is drawn beside the viewport.
            if x < 0.0 || x >= bench.size.width {
                continue;
            }
            let clipped = regions.iter().any(|r| {
                let v = r.viewport;
                x >= v.x && x < v.x + v.width && (y < v.y || y >= v.y + v.height)
            });
            if clipped {
                continue;
            }
            assert!(
                y >= TOP,
                "{location}: {text:?} sits at y = {y}, under the status bar"
            );
            assert!(
                y <= bench.size.height - BOTTOM,
                "{location}: {text:?} sits at y = {y}, under the navigation bar"
            );
        }
    }
}

/// **Nothing in the application may draw outside its parent** — checked on every screen, at a
/// phone's width and at a desktop's, because that is where the difference shows.
///
/// The chart dashboard's segmented control was once 584 px of segments in a 363 px row on a
/// phone, running 221 px past the card. Nothing had ever said so.
#[test]
fn no_screen_draws_outside_itself() {
    let mut worst: Vec<String> = Vec::new();
    for location in SCREENS {
        for (label, w, h) in [("phone", PHONE.0, PHONE.1), ("desktop", 1200.0, 800.0)] {
            let mut bench = Bench::new(w, h).with_tasks(&["short"]);
            if location != "/" {
                bench.go(location);
            }
            let (_, ui) = bench.frame();
            for o in ui.overflows() {
                if o.amount > 0.0 {
                    // The box and the edge as well as the amount: "2 px" on nine screens at
                    // once says a shared widget grew, and only the rectangle says which.
                    worst.push(format!(
                        "{location}/{label} overflows {:?} by {:.1} px at {:?}",
                        o.side, o.amount, o.rect
                    ));
                }
            }
        }
    }
    assert!(worst.is_empty(), "{worst:#?}");
}

/// Milestone 289: a long task's title **wraps**, and what follows clears the lines it wrapped
/// onto — the title used to be painted on two lines with the state label sitting on the second.
#[test]
fn a_long_task_title_wraps_without_overlapping_what_follows() {
    let mut bench = Bench::new(424.0, 918.0);
    bench
        .demo
        .add("A rather long task name that certainly wraps");
    let id = bench.demo.todos()[0].id;
    bench.go(&format!("/task/{id}"));
    let (_, ui) = bench.frame();
    let texts: Vec<(String, f32, Option<f32>)> = ui
        .scene()
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Text {
                position,
                text,
                max_width,
                ..
            } => Some((text.clone(), position.y, *max_width)),
            _ => None,
        })
        .collect();
    let title = texts
        .iter()
        .find(|(t, _, _)| t.starts_with("A rather long"))
        .expect("the task's title is on its screen");
    let state = texts
        .iter()
        .find(|(t, _, _)| t == "Still to do")
        .expect("the state label is under it");
    // The title is wrapped: it is painted with a width narrower than one line of it would need.
    assert!(
        title.2.is_some_and(|w| w < 400.0),
        "the title is a paragraph in a narrow box: {:?}",
        title.2
    );
    // Two lines at 24 px is about 58 px, plus the column's 18 px gap: 76. One line would put
    // the state label 46 px below. 60 separates the two cleanly.
    assert!(
        state.1 - title.1 > 60.0,
        "the state label overlaps the wrapped title: title y={}, state y={}",
        title.1,
        state.1
    );
}

/// **The task screen's words are centred on the screen**, under the avatar above them. The
/// column holding the title and the state label centres its children, but it took the width of its
/// widest child and sat at the start of its box, so on a phone both words were centred on a point
/// a quarter of the way across.
#[test]
fn the_task_screens_words_are_centred_under_its_avatar() {
    let mut bench = Bench::new(PHONE.0, PHONE.1);
    bench.demo.add("Write code");
    let id = bench.demo.todos()[0].id;
    bench.go(&format!("/task/{id}"));
    // Frames, so that the arrival animation has come to rest.
    for _ in 0..30 {
        let _ = bench.frame();
    }
    let (_, ui) = bench.frame();
    fn middle_of(primitives: &[Primitive], label: &str) -> Option<f32> {
        primitives.iter().find_map(|p| match p {
            Primitive::Text {
                text,
                position,
                size,
                weight,
                italic,
                ..
            } if text == label => {
                // A text primitive is a point; the word's own width says where its middle is.
                Some(
                    position.x
                        + frus_text::measure_styled(text, *size, *weight, *italic).width / 2.0,
                )
            }
            Primitive::Layer { primitives, .. } => middle_of(primitives, label),
            _ => None,
        })
    }
    let primitives = ui.scene().primitives();
    let title = middle_of(primitives, "Write code").expect("the title");
    let state = middle_of(primitives, "Still to do").expect("the state label");
    let middle = PHONE.0 / 2.0;
    assert!(
        (title - middle).abs() < 2.0,
        "the title is centred at {title}, the screen's middle is {middle}"
    );
    assert!(
        (state - middle).abs() < 2.0,
        "the state label is centred at {state}, the screen's middle is {middle}"
    );
}

/// The task's own screen names the task it was asked for, and says so when it is gone.
#[test]
fn a_task_screen_shows_its_task_and_survives_its_deletion() {
    let mut bench = Bench::new(400.0, 800.0).with_tasks(&["Water the plants"]);
    let id = bench.demo.todos()[0].id;
    bench.go(&format!("/task/{id}"));
    assert!(bench.words().iter().any(|w| w == "Water the plants"));
    assert!(bench.words().iter().any(|w| w == "Still to do"));
    // The ✓ on the docked button ticks it.
    assert!(bench.press("✓"));
    assert!(bench.demo.todo(id).is_some_and(|t| t.done));
    // Deleted while its screen is open: say so rather than show an empty page.
    bench.demo.delete(id);
    assert!(bench
        .words()
        .iter()
        .any(|w| w == "This task no longer exists."));
}

/// **The task screen's bottom bar continues into the system's navigation bar** (#46): the
/// screen asks for the navigation bar in its bottom app bar's colour, and says nothing of the
/// status bar, which stays the theme's.
#[test]
fn the_task_screen_asks_for_the_navigation_bar_in_its_bottom_bars_colour() {
    let mut bench = Bench::new(400.0, 800.0).with_tasks(&["Water the plants"]);
    let id = bench.demo.todos()[0].id;
    bench.go(&format!("/task/{id}"));
    let theme = bench
        .app
        .resolved_theme(frus_widgets::Brightness::Dark, false);
    let (_, ui) = bench.frame();
    let style = ui.system_ui_style(Point::new(200.0, 0.5), Point::new(200.0, 799.5));
    assert_eq!(style.navigation_bar_color, Some(theme.surface));
    assert_eq!(
        style.status_bar_color, None,
        "the status bar is left to the theme"
    );
}

/// The device finding of milestone 327, closed in 334. A task label long enough to overflow the
/// row used to be laid out at its own content width, which pushed the delete button off the
/// card, out of the window, and — the part that mattered — out of the hit registry: the × was
/// not merely invisible, it was unclickable, and that task could not be deleted at all.
///
/// Read from the **hit registry** rather than from the picture, because the registry is what
/// the report was about: each distinct thing a tap can do in the window is done, and the one
/// that deletes the long task is the one whose targets are looked at.
#[test]
fn a_long_task_label_still_leaves_its_delete_button_clickable() {
    let bench = Bench::new(PHONE.0, PHONE.1).with_tasks(&[
        "short",
        "a task label far longer than any phone is wide, which is exactly the case that used to push the delete button out of the window entirely",
    ]);
    let long_id = bench.demo.todos()[1].id;
    let (_, ui) = bench.frame();

    // Sweep the window and ask what a tap there would do, keeping each distinct answer with
    // where it was found.
    let mut found: Vec<(Callback, Vec<(f32, f32)>)> = Vec::new();
    let mut y = 1.0;
    while y < PHONE.1 {
        let mut x = 1.0;
        while x < PHONE.0 {
            if let Some(callback) = ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id)) {
                match found.iter_mut().find(|(c, _)| c.ptr_eq(&callback)) {
                    Some((_, at)) => at.push((x, y)),
                    None => found.push((callback, vec![(x, y)])),
                }
            }
            x += 2.0;
        }
        y += 2.0;
    }
    let targets = found
        .into_iter()
        .find_map(|(callback, at)| {
            callback.call();
            bench.demo.todo(long_id).is_none().then_some(at)
        })
        .expect("no tap anywhere in the window deletes the long task");

    // And it is a real target, not a sliver: an icon button's 40 px, near the right edge.
    let left = targets.iter().map(|t| t.0).fold(f32::MAX, f32::min);
    let right = targets.iter().map(|t| t.0).fold(f32::MIN, f32::max);
    assert!(
        right - left > 20.0,
        "the delete target is a sliver {left}..{right}, not a button"
    );
    assert!(right < PHONE.0, "and it is inside the window");
    // And it is where a trailing button belongs: against the row's right edge, not sitting on
    // top of the label because the label was given no width at all.
    assert!(
        left > PHONE.0 * 0.5,
        "the delete target is at {left}..{right}, not on the right-hand side"
    );
}

/// **A ticked task still reads as ticked**, which stopped being obvious in milestone 498.
///
/// Its label's colour and its line through were stated on the text itself until that milestone
/// and are now *handed down* to it, so that both can move when a task is ticked rather than
/// jumping. The failure mode of getting that wrong is silence: a done task that looks exactly
/// like an active one, on a screen where every row still lays out, still paints and still
/// passes every count. So the row is asked what it actually drew — the active row beside it is
/// the control.
#[test]
fn a_ticked_task_is_still_muted_and_struck_through() {
    fn label_of(done: bool) -> (Color, bool) {
        // Tall, so the row is on screen and not scrolled out of the painted frame.
        let mut bench = Bench::new(600.0, 1600.0).with_tasks(&["Buy milk"]);
        if done {
            bench.demo.toggle(0);
        }
        // Settled, so what is painted is the target rather than a frame of a movement: this is
        // about where the style arrives, not how it gets there.
        let (tree, _) = bench.frame();
        bench.runtime.advance_text_styles(tree.as_ref(), 1.0);
        let (_, ui) = bench.frame();
        fn walk(primitives: &[Primitive], out: &mut Vec<(Color, bool)>) {
            for p in primitives {
                match p {
                    Primitive::Text {
                        text,
                        color,
                        decoration,
                        ..
                    } if text == "Buy milk" => out.push((*color, decoration.strikethrough)),
                    Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut found = Vec::new();
        walk(ui.scene().primitives(), &mut found);
        assert_eq!(found.len(), 1, "one label: {found:?}");
        found[0]
    }

    let theme = Bench::new(600.0, 1600.0)
        .app
        .resolved_theme(frus_widgets::Brightness::Dark, false);
    let (active_color, active_line) = label_of(false);
    let (done_color, done_line) = label_of(true);
    assert!(!active_line, "an active task is not struck through");
    assert!(done_line, "a ticked one is");
    assert_eq!(
        active_color, theme.on_surface,
        "an active task reads as ink"
    );
    assert_eq!(done_color, theme.muted, "and a ticked one as muted");
    assert_ne!(
        active_color, done_color,
        "the two states must not look alike"
    );
}

/// The filters and the confirmation on the home screen do what they say, pressed by what they
/// say through the hit registry: a filter that shows nothing says so, another brings the list
/// back, and the "clear completed" confirmation opens and is put away by Cancel.
#[test]
fn the_controls_on_the_home_screen_do_what_they_say() {
    let bench = Bench::new(600.0, 900.0).with_tasks(&["first"]);
    assert!(bench.words().iter().any(|w| w == "first"));
    // The filters: the only task is active, so *Done* shows nothing.
    assert!(bench.press("Done"));
    let words = bench.words();
    assert!(
        words
            .iter()
            .any(|w| w == "Nothing to show for this filter."),
        "{words:?}"
    );
    assert!(!words.iter().any(|w| w == "first"));
    // *All* brings it back.
    assert!(bench.press("All"));
    assert!(bench.words().iter().any(|w| w == "first"));
    // The confirmation before clearing what is done: opened, then put away.
    bench.demo.toggle(0);
    assert!(bench.press("Clear completed"));
    assert!(
        bench.words().iter().any(|w| w == "Clear completed tasks?"),
        "the confirmation opens"
    );
    assert!(bench.press("Cancel"));
    assert!(
        !bench.words().iter().any(|w| w == "Clear completed tasks?"),
        "and Cancel puts it away"
    );
    assert_eq!(bench.demo.len(), 1, "nothing was cleared");
}

/// The section the navigation names is the one on show: the bottom bar's destinations are the
/// drawer's, and choosing Stats shows the master-detail pane.
#[test]
fn choosing_a_section_changes_what_is_on_show() {
    let bench = Bench::new(700.0, 900.0).with_tasks(&["a", "b"]);
    assert!(
        bench.words().iter().any(|w| w == "My Tasks"),
        "the tasks section"
    );
    assert!(bench.press("Stats"));
    let stats = bench.words();
    assert!(stats.iter().any(|w| w == "Total tasks"), "{stats:?}");
    assert!(stats.iter().any(|w| w == "Completed"));
    assert!(bench.press("About"));
    assert!(bench.words().iter().any(|w| w == "About frus"));
}

/// Presses the control the accessibility tree calls `label` — an icon button has no word to
/// find it by. Whether there was one.
fn press_labelled(bench: &Bench, label: &str) -> bool {
    let (_, ui) = bench.frame();
    let Some(rect) = ui
        .semantics()
        .iter()
        .find(|(_, _, node)| node.label.as_deref() == Some(label))
        .map(|(_, rect, _)| *rect)
    else {
        return false;
    };
    let at = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    match ui.hit(at).and_then(|id| ui.msg_for(id)) {
        Some(callback) => {
            callback.call();
            true
        }
        None => false,
    }
}

/// **The drawer's list scrolls** when the window is too short for it — a phone on its side, a
/// browser tab. It was a column that only laid out its children, so the last entries ran off the
/// bottom of the panel and nothing could bring them back. In a window tall enough for all of it
/// there is nothing to scroll.
#[test]
fn the_drawers_list_scrolls_when_the_window_is_too_short_for_it() {
    let inside_the_drawer = Point::new(100.0, 250.0);

    let short = Bench::new(411.0, 400.0);
    assert!(
        press_labelled(&short, "Menu"),
        "the menu button opens the drawer"
    );
    let (_, ui) = short.frame();
    let area = ui
        .scroll_hit(inside_the_drawer)
        .expect("a scroll area under the drawer's list");
    // The page behind the drawer scrolls too, across the whole window: what is asked for is the
    // drawer's *own* area, which is the narrower one.
    assert!(
        area.viewport.width < 380.0,
        "the drawer's own scroll area, not the page's behind it: {area:?}"
    );
    assert!(
        area.max_y > 0.0,
        "there is more list than window, so it can be scrolled: {area:?}"
    );

    let tall = Bench::new(411.0, 1400.0);
    assert!(press_labelled(&tall, "Menu"));
    let (_, ui) = tall.frame();
    assert!(
        ui.scroll_hit(inside_the_drawer)
            .is_none_or(|area| area.max_y == 0.0),
        "everything fits, so nothing is left to scroll to"
    );
    assert!(
        tall.words().iter().any(|w| w == "Kanban board →"),
        "and the last entry is on show"
    );
}

// ---------------------------------------------------------------------------------------
// Reordering the list
// ---------------------------------------------------------------------------------------

/// What a carried row takes with it: the rows, and only the rows. Everything a row paints moves
/// with it, and nothing painted outside one does — the page's own background behind the whole
/// list merely has its centre on one.
#[test]
fn what_makes_room_for_a_carried_row_is_the_rows() {
    let size = Size::new(424.0, 918.0);
    let bench = Bench::new(size.width, size.height).with_tasks(&["one", "two", "three"]);
    let theme = bench
        .app
        .resolved_theme(frus_widgets::Brightness::Dark, false);
    let (tree, ui, nodes) = MediaQuery::new(size).scope(|| {
        bench.runtime.states.begin_build();
        let tree = Application::view(&bench.app, &theme);
        frus_widgets::build_deferred(tree.as_ref(), &theme, &bench.runtime);
        let (ui, nodes) = build_ui_inspected(tree.as_ref(), size, &bench.runtime, &theme);
        bench.runtime.states.end_frame();
        (tree, ui, nodes)
    });
    let rows: Vec<frus_widgets::Rect> = nodes
        .iter()
        .filter(|n| n.name == "ReorderRow")
        .map(|n| n.rect)
        .collect();
    assert_eq!(rows.len(), 3, "one row per task");

    let movable = frus_widgets::reorderable_owners(&ui, tree.as_ref());
    let centre = |b: frus_widgets::Rect| Point::new(b.x + b.width * 0.5, b.y + b.height * 0.5);
    let within = |row: &frus_widgets::Rect, b: frus_widgets::Rect| {
        b.x >= row.x - 0.5
            && b.y >= row.y - 0.5
            && b.x + b.width <= row.x + row.width + 0.5
            && b.y + b.height <= row.y + row.height + 0.5
    };
    let mut left_behind = Vec::new();
    let mut strays = Vec::new();
    let mut moving = 0;
    for p in ui.scene().primitives() {
        let bounds = p.bounds();
        if movable.contains(&p.owner()) {
            if rows.iter().any(|row| row.contains(centre(bounds))) {
                moving += 1;
            } else {
                strays.push(bounds);
            }
        } else if rows.iter().any(|row| within(row, bounds)) {
            left_behind.push(bounds);
        }
    }
    assert!(moving > 0, "the rows move");
    assert!(
        left_behind.is_empty(),
        "everything a row paints moves with it: {left_behind:?}"
    );
    assert!(
        strays.is_empty(),
        "and nothing painted outside the rows does: {strays:?}"
    );
}

/// **A row carried past the last one lands at the end.**
///
/// On a phone the list is followed by more of the page, so a finger that carries a row to the
/// bottom edge is over that and not over a row — and the release, which asked only what was
/// under the finger, put the row back. This reads the demo's real page: below the last row
/// nothing can be dropped on, the first row's own list offers its three rows, the nearest is
/// the last, and a drop after it moves the first task to the end.
#[test]
fn a_row_carried_past_the_last_one_lands_at_the_end() {
    let size = Size::new(424.0, 918.0);
    let bench = Bench::new(size.width, size.height).with_tasks(&["one", "two", "three"]);
    let (tree, ui) = bench.frame();
    let droppable = |id| find_widget(tree.as_ref(), id).is_some_and(|w| w.reorder_droppable());
    let first = ui
        .reorderables()
        .iter()
        .map(|(id, _)| *id)
        .find(|id| {
            droppable(*id)
                && find_widget(tree.as_ref(), *id).and_then(|w| w.reorder_index()) == Some(0)
        })
        .expect("the first row");

    let slots = frus_widgets::reorder_siblings(&ui, tree.as_ref(), first);
    assert_eq!(
        slots.len(),
        3,
        "the first row's list is the three rows: {slots:?}"
    );
    let last = slots[2].1;
    let below = Point::new(last.x + last.width * 0.5, last.y + last.height + 24.0);
    assert!(
        below.y < size.height,
        "the point is still on the page: {below:?}"
    );
    assert!(
        !ui.reorderables_at(below).any(droppable),
        "below the last row there is no row to drop on"
    );

    let boxes: Vec<frus_widgets::Rect> = slots.iter().map(|(_, rect)| *rect).collect();
    let nearest =
        frus_widgets::nearest_reorder_slot(below, &boxes, frus_widgets::ReorderAxis::Vertical);
    assert_eq!(nearest, Some(2), "the nearest slot is the last row");
    // After it, as the lower half of a row is: raw index 3, from the first row.
    let message = find_widget(tree.as_ref(), first).and_then(|w| w.on_reorder(3));
    let Some(message) = message else {
        panic!("a drop after the last row moves the first");
    };
    message.call();
    let labels: Vec<String> = bench.demo.todos().into_iter().map(|t| t.text).collect();
    assert_eq!(labels, ["two", "three", "one"]);
}

/// **The whole route a reorder takes, driven through the demo's own view.**
///
/// The unit tests read the hooks off a widget in isolation; this one builds the application's
/// real tree, lays it out, and asks the registries the questions the shell asks in the order
/// the shell asks them — which is where a grip that shadows its row, or a row that never
/// registers at all, would show up.
#[test]
fn a_grip_grabs_its_row_and_the_drop_routes_to_a_position() {
    let size = Size::new(424.0, 918.0);
    let bench = Bench::new(size.width, size.height).with_tasks(&["one", "two", "three"]);
    let theme = bench
        .app
        .resolved_theme(frus_widgets::Brightness::Dark, false);
    let (tree, ui, nodes) = MediaQuery::new(size).scope(|| {
        bench.runtime.states.begin_build();
        let tree = Application::view(&bench.app, &theme);
        frus_widgets::build_deferred(tree.as_ref(), &theme, &bench.runtime);
        let (ui, nodes) = build_ui_inspected(tree.as_ref(), size, &bench.runtime, &theme);
        bench.runtime.states.end_frame();
        (tree, ui, nodes)
    });
    let boxes = |name: &str| -> Vec<frus_widgets::Rect> {
        nodes
            .iter()
            .filter(|n| n.name == name)
            .map(|n| n.rect)
            .collect()
    };
    let rows = boxes("ReorderRow");
    let grips = boxes("ReorderHandle");
    assert_eq!(rows.len(), 3, "one wrapper per visible task");
    assert_eq!(grips.len(), 3, "and a grip in each");

    // 1) A press on the first row's grip grabs **the grip** — it is the topmost reorderable
    //    there, which is what makes the rest of the row still scroll.
    let middle = |r: frus_widgets::Rect| Point::new(r.x + r.width * 0.5, r.y + r.height * 0.5);
    let grabbed = ui
        .reorderables_at(middle(grips[0]))
        .next()
        .expect("something to grab on the grip");
    let grip = find_widget(tree.as_ref(), grabbed).expect("the grabbed widget");
    assert!(grip.reorder_draggable(), "the grip is a source");
    assert!(!grip.reorder_droppable(), "and not a target");
    assert_eq!(grip.reorder_index(), Some(0));

    // 2) What the drag actually moves is the row behind it, found the way the shell finds it:
    //    the droppable reorderable of the same index under the grip's own middle.
    let source = ui
        .reorderables_at(middle(grips[0]))
        .find(|id| {
            find_widget(tree.as_ref(), *id)
                .is_some_and(|w| w.reorder_droppable() && w.reorder_index() == Some(0))
        })
        .expect("the grip sits inside its row");
    let source_rect = ui.widget_rect(source).expect("the row has a box");
    assert!(
        source_rect.width > grips[0].width * 4.0,
        "the row is much wider than the grip that moves it: {source_rect:?}"
    );

    // 3) Dropped **over the last row's own grip**, in the lower half of that row: the case
    //    `reorder_droppable` exists for. The grip is on top there and cannot be dropped on, so
    //    what the drop aims at is the row behind it.
    let last = rows[2];
    let last_grip = grips[2];
    let over_grip = Point::new(
        last_grip.x + last_grip.width * 0.5,
        last_grip.y + last_grip.height * 0.9,
    );
    assert!(
        last_grip.contains(over_grip) && over_grip.y > last.y + last.height * 0.5,
        "the point is on the grip and in the row's lower half: {over_grip:?} in {last_grip:?}"
    );
    let target = ui
        .reorderables_at(over_grip)
        .find(|id| find_widget(tree.as_ref(), *id).is_some_and(|w| w.reorder_droppable()))
        .expect("a row to drop on");
    let target_rect = ui.widget_rect(target).expect("the target has a box");
    assert!(
        (target_rect.width - last.width).abs() < 0.5,
        "the drop aims at the row and not at the grip on top of it: {target_rect:?}"
    );
    let base = find_widget(tree.as_ref(), target)
        .and_then(|w| w.reorder_index())
        .expect("the row's index");
    assert_eq!(base, 2);
    let raw = base + usize::from(over_grip.y > target_rect.y + target_rect.height * 0.5);
    assert_eq!(raw, 3, "the lower half means the slot after it");
    // 4) And what the shell would deliver: the first row ends up **last**, index 2 and not the
    //    raw 3, because it is no longer in the list it is being counted in.
    grip.on_reorder(raw)
        .expect("a drop after the last row moves the first")
        .call();
    let labels: Vec<String> = bench.demo.todos().into_iter().map(|t| t.text).collect();
    assert_eq!(labels, ["two", "three", "one"]);
    // Dropped back on its own lower half, the same arithmetic asks for nothing.
    assert!(
        grip.on_reorder(1).is_none(),
        "a row dropped where it already is moves nothing"
    );
}

// ---------------------------------------------------------------------------------------
// The other screens
// ---------------------------------------------------------------------------------------

/// **The panels still swipe with a button floating over them.** Milestone 495 put the tour's
/// way-out chip in a `Stack` above the page view so it could slide off the top, and a page view
/// that stopped registering as a paged region would still draw four panels, still lay out,
/// still pass the overflow test — and would not turn.
#[test]
fn the_tour_panels_still_turn_under_the_button_that_floats_over_them() {
    let mut bench = Bench::new(420.0, 900.0);
    bench.go("/tour");
    let (_, ui) = bench.frame();
    let paged: Vec<_> = ui
        .scroll_regions()
        .iter()
        .filter(|area| area.page.is_some())
        .collect();
    assert_eq!(
        paged.len(),
        1,
        "the stack kept the page view a paged region"
    );
    assert!(
        paged[0].max_x > bench.size.width,
        "and four panels of it still have somewhere to go: {}",
        paged[0].max_x
    );
}

/// The tour's page is one number the finger and the pager both drive: "Skip" goes to the last
/// panel, and the picker says so.
#[test]
fn the_tour_skips_to_its_last_panel() {
    let mut bench = Bench::new(420.0, 900.0);
    bench.go("/tour");
    assert!(bench.words().iter().any(|w| w == "Panel 1 of 4"));
    assert!(bench.press("Skip"));
    assert!(bench.words().iter().any(|w| w == "Panel 4 of 4"));
}

/// **The licence list is generated, and this is what generated has to mean**: it parses, it
/// covers what this application actually links, and every notice has a text.
///
/// The one failure mode that matters for a licence list is a list that does not match what was
/// linked. Nothing here can prove the file was regenerated after the last dependency changed —
/// that is what running `scripts/gen_licenses.py` is for — but a list that has lost `wgpu`, or
/// `winit`, or the framework itself is a list that is wrong in the way somebody would notice in
/// court rather than in a test, so it is worth one.
#[test]
fn the_generated_licences_cover_what_the_demo_links() {
    let notices = frus_widgets::licenses::parse(include_str!("../assets/licenses.txt"));
    assert!(
        notices.len() > 100,
        "four hundred packages do not fit in {} notices",
        notices.len()
    );
    let packages: Vec<String> = notices
        .iter()
        .flat_map(|n| n.packages.iter().map(|p| p.name.clone()))
        .collect();
    for linked in [
        "wgpu",
        "winit",
        "taffy",
        "cosmic-text",
        "frus-shell",
        "frus-widgets",
    ] {
        assert!(
            packages.iter().any(|p| p == linked),
            "{linked} is linked and is not in the list"
        );
    }
    for notice in &notices {
        assert!(!notice.packages.is_empty(), "a notice covering nothing");
        assert!(
            !notice.text.trim().is_empty(),
            "no text for {:?}",
            notice.packages
        );
    }
    // And the page shows them: the list is what the screen puts on screen.
    let mut bench = Bench::new(420.0, 900.0);
    frus_widgets::licenses::add_all(include_str!("../assets/licenses.txt"));
    bench.go("/licenses");
    assert!(
        bench.words().iter().any(|w| w.starts_with("wgpu ")),
        "the page lists wgpu"
    );
}

/// Milestone 493: the log screen **answers its own scroll offset**, and offers the way back
/// only once there is one worth offering — the loop is the thing checked: the list reports, the
/// screen keeps, the next build reads.
#[test]
fn the_log_says_where_it_is_and_offers_the_way_back() {
    let mut bench = Bench::new(420.0, 900.0);
    bench.go("/journal");

    // Nothing has moved, so nothing has been measured: a row number here would be the screen
    // guessing rather than the list reporting, and there is nowhere to go back to.
    let quiet = bench.words();
    assert!(quiet.iter().any(|w| w == "5000 rows"), "{quiet:?}");
    assert!(!quiet.iter().any(|w| w == "Top"), "{quiet:?}");

    // The list is reached by the name the screen gave it, and reports where it is.
    let report = |offset: f32| {
        let (tree, _) = bench.frame();
        let key = frus_widgets::host::key_hash(JOURNAL_LIST);
        let id = frus_widgets::find_by_key(tree.as_ref(), key).expect("the screen named its list");
        find_widget(tree.as_ref(), id)
            .and_then(|list| list.on_scroll(at(offset)))
            .expect("the list answers a scroll")
            .call();
    };
    // A hundred pixels down is still a flick from the top: the number moves, the button stays
    // away.
    report(100.0);
    let near = bench.words();
    assert!(near.iter().any(|w| w == "Row 3 of 5000"), "{near:?}");
    assert!(!near.iter().any(|w| w == "Top"), "{near:?}");

    // Twenty thousand pixels down — row 455 — and the way back is worth a button.
    report(20_000.0);
    let far = bench.words();
    assert!(far.iter().any(|w| w == "Row 455 of 5000"), "{far:?}");
    assert!(far.iter().any(|w| w == "Top"), "{far:?}");

    // And the button is an **effect**, not a change of state: the offset it moves lives in the
    // runtime, and pressing it asks the host to scroll the list by name.
    let _ = frus_widgets::host::take_effects();
    assert!(bench.press("Top"));
    let effects = frus_widgets::host::take_effects();
    assert!(
        matches!(effects.as_slice(), [frus_widgets::host::Effect::Scroll(key, _)]
            if *key == frus_widgets::host::key_hash(JOURNAL_LIST)),
        "a request addressed to the list's name"
    );
}

/// The log list resting `offset` pixels down, as the region itself would report it.
fn at(offset: f32) -> frus_widgets::ScrollPosition {
    frus_widgets::ScrollPosition {
        offset: (0.0, offset),
        max: (0.0, 5000.0 * 44.0 - 700.0),
        viewport: Size::new(372.0, 700.0),
    }
}

/// The wiring, through the real screen: **the name the button commands is the region the frame
/// registers**. The failure this exists to catch is milestone 477's — a hook declared on one
/// side and never reached on the other — and it is invisible from either end alone.
#[test]
fn the_log_list_is_reachable_by_the_name_the_button_commands() {
    let mut bench = Bench::new(420.0, 900.0);
    bench.go("/journal");
    let (tree, ui) = bench.frame();
    let key = frus_widgets::host::key_hash(JOURNAL_LIST);
    let id = frus_widgets::find_by_key(tree.as_ref(), key).expect("the screen named its list");
    let area = ui
        .scroll_region(id)
        .expect("and the name reaches the region the frame registered, not a wrapper");
    assert!(
        area.max_y > 100_000.0,
        "five thousand rows of 44 px have somewhere to go: {}",
        area.max_y
    );
}

/// The demo through the shell, on a surface, at a screen: what a tap can reach that a message
/// cannot — a step marker or a summary bullet answers a click **at a position**.
fn wizard_driver(width: f32, height: f32) -> (Driver<FrusApp>, Rc<Demo>) {
    let (app, router, demo) = crate::build();
    let mut driver = Driver::new(app, width, height);
    driver.run(0.3);
    router.push("/wizard");
    driver.run(1.5);
    (driver, demo)
}

fn said(driver: &Driver<FrusApp>, word: &str) -> bool {
    driver.texts().iter().any(|(text, _)| text == word)
}

/// Taps the marker of the wizard's step `index`. The markers answer a click **at a position**
/// — a hotspot laid over each — so they are found where the hit registry says a tap does
/// something, in the band of the window the steps row lives in, left to right.
fn tap_step(driver: &mut Driver<FrusApp>, index: usize) {
    let (ui, _) = driver.frame_parts().expect("a frame");
    let mut hotspots: Vec<(Callback, Vec<Point>)> = Vec::new();
    let mut y = 70.0;
    while y < 260.0 {
        let mut x = 2.0;
        while x < 400.0 {
            let at = Point::new(x, y);
            if let Some(callback) = ui.hit(at).and_then(|id| ui.msg_for(id)) {
                match hotspots.iter_mut().find(|(c, _)| c.ptr_eq(&callback)) {
                    Some((_, points)) => points.push(at),
                    None => hotspots.push((callback, vec![at])),
                }
            }
            x += 4.0;
        }
        y += 4.0;
    }
    hotspots.sort_by(|a, b| a.1[0].x.total_cmp(&b.1[0].x));
    let (_, points) = &hotspots[index];
    let at = points[points.len() / 2];
    driver.press(at);
    driver.release(at);
    driver.run(0.2);
}

/// The sign-up wizard's flow, tapped through: an empty submission reveals the errors and lands
/// on the review; a bullet of the summary jumps to the step of the field it names; a step marker
/// jumps too.
#[test]
fn wizard_flow_validates_navigates_and_reveals_errors() {
    let (mut driver, demo) = wizard_driver(420.0, 900.0);
    assert!(said(&driver, "Full name"), "the Account step");
    // The marker of the last step jumps to it — the errors show only after a submission.
    tap_step(&mut driver, 2);
    assert!(said(&driver, "Create account"));
    assert!(!said(&driver, "• Name is required"));
    assert!(driver.tap_text("Create account"));
    driver.run(0.2);
    assert!(said(&driver, "• Name is required"), "{:?}", driver.texts());
    assert!(demo.current_toast().is_none(), "nothing was created");
    // A summary bullet jumps to the step of the field it names.
    assert!(driver.tap_text("• Name is required"));
    driver.run(0.2);
    assert!(said(&driver, "Full name"), "back on Account");
    // And the markers move both ways.
    tap_step(&mut driver, 1);
    assert!(said(&driver, "Password"));
}

/// **The sign-up wizard is one form, for autofill** (milestone 512). On each step the fields
/// are grouped, and nothing else is — not the Next button, not the back arrow — and the group is
/// the **same one** whatever the step: the account's name is typed on the first and its password
/// on the second, and a password manager shown them as two forms would save a password with no
/// account to save it under.
#[test]
fn the_wizard_is_one_form_across_its_steps() {
    use frus_widgets::{AutofillHint, WidgetId};
    let (mut driver, _) = wizard_driver(400.0, 800.0);
    let grouped = |driver: &Driver<FrusApp>| -> Vec<(WidgetId, Option<WidgetId>)> {
        let (ui, _) = driver.frame_parts().expect("a frame");
        let stops: Vec<WidgetId> = ui.focusable_ids().collect();
        stops
            .into_iter()
            .map(|id| (id, ui.form_group(id)))
            .filter(|(_, group)| group.is_some())
            .collect()
    };
    let account = grouped(&driver);
    assert_eq!(account.len(), 2, "the name and the email, and nothing else");
    assert_eq!(account[0].1, account[1].1, "in one group");
    tap_step(&mut driver, 1);
    let security = grouped(&driver);
    assert_eq!(security.len(), 2, "the password and its confirmation");
    assert_eq!(
        security[0].1, account[0].1,
        "the same form on the second step as on the first"
    );
    // And each field says what it is for: the email is the account's name as well.
    assert_eq!(wizard_hints(0), &[AutofillHint::Name]);
    assert_eq!(
        wizard_hints(1),
        &[AutofillHint::Username, AutofillHint::Email]
    );
    assert_eq!(wizard_hints(2), &[AutofillHint::NewPassword]);
    assert_eq!(wizard_hints(3), &[AutofillHint::NewPassword]);
}

/// **Each wizard field opens the keyboard it is for** (milestone 514), read off the tree the
/// view builds — what the shell hands the platform — and not off the helper alone. And a
/// password revealed by the eye keeps a secret's keyboard, so revealing it never lets the
/// keyboard learn it.
#[test]
fn each_wizard_field_opens_the_keyboard_it_is_for() {
    use frus_widgets::KeyboardType;
    let (mut driver, _) = wizard_driver(400.0, 800.0);
    let keyboards = |driver: &Driver<FrusApp>| -> Vec<KeyboardType> {
        let (ui, tree) = driver.frame_parts().expect("a frame");
        let stops: Vec<_> = ui.focusable_ids().collect();
        stops
            .into_iter()
            .filter(|id| ui.form_group(*id).is_some())
            .map(|id| {
                find_widget(tree, id)
                    .expect("a focus stop is a widget in the tree")
                    .ime()
                    .keyboard
            })
            .collect()
    };
    assert_eq!(
        keyboards(&driver),
        [KeyboardType::Name, KeyboardType::Email],
        "the account step"
    );
    tap_step(&mut driver, 1);
    assert_eq!(
        keyboards(&driver),
        [KeyboardType::Password, KeyboardType::Password],
        "masked passwords"
    );
    // The eye inside the field toggles the masking. Revealed, it is still never learned.
    assert_eq!(wizard_keyboard(2, false), KeyboardType::VisiblePassword);
    assert_eq!(wizard_keyboard(3, true), KeyboardType::Password);
}

/// **The demo's sheet, driven through the registries in the order the shell reads them**
/// (milestone 515): the list is a scroll area walked inside the sheet, a finger moving up on it
/// grows the sheet, a flick settles it on the next stop, and a flick down from under the lowest
/// one puts it away with the screen's own message.
#[test]
fn the_sheet_shares_the_finger_with_its_list_and_puts_itself_away() {
    use frus_widgets::{split_sheet_drag, WidgetId};
    let mut bench = Bench::new(400.0, 800.0);
    bench.go("/sheet");
    let mut runtime = Runtime::default();
    let size = bench.size;
    let theme = bench
        .app
        .resolved_theme(frus_widgets::Brightness::Dark, false);

    // A frame built on the runtime being driven, as the shell's is.
    let build = |runtime: &Runtime| {
        MediaQuery::new(size).scope(|| {
            runtime.states.begin_build();
            let tree = Application::view(&bench.app, &theme);
            frus_widgets::build_deferred(tree.as_ref(), &theme, runtime);
            let ui = build_ui(tree.as_ref(), size, runtime, &theme);
            runtime.states.end_frame();
            (tree, ui)
        })
    };
    let (_, ui) = build(&runtime);
    let sheet = ui
        .sheets()
        .first()
        .cloned()
        .expect("the screen has a sheet");
    assert_eq!(sheet.spec.stops, vec![0.0, 0.25, 0.5, 1.0]);
    let list: WidgetId = *sheet.areas.first().expect("its list was walked inside it");
    let region = ui.scroll_region(list).expect("the list scrolls");
    assert!(region.max_y > 0.0, "long enough to scroll at half height");
    assert!(
        region.viewport.y >= sheet.panel.y
            && region.viewport.y + region.viewport.height
                <= sheet.panel.y + sheet.panel.height + 0.5,
        "the list lies inside the panel: {:?} in {:?}",
        region.viewport,
        sheet.panel
    );
    assert_eq!(ui.sheet_holding(list).map(|s| s.id), Some(sheet.id));
    // The page behind is no part of the sheet.
    assert!(ui
        .sheet_at(Point::new(200.0, sheet.panel.y - 10.0))
        .is_none());
    assert!(
        ui.sheet_at(Point::new(200.0, sheet.panel.y + 10.0))
            .is_some(),
        "a finger on the panel is on the sheet"
    );

    // A finger on the list moves up 100 px, the list at its top: all of it to the sheet.
    let px = sheet.available;
    let half = sheet.panel.height;
    assert!(
        (half - px * 0.5).abs() < 0.5,
        "at half height: {half} of {px}"
    );
    let (grown, listed) = split_sheet_drag(100.0, 0.0, half, 0.0, px);
    assert_eq!((grown, listed), (100.0, 0.0));
    runtime.sheet_drag(sheet.id, &sheet.spec, grown / px, px);
    let raised = build(&runtime)
        .1
        .sheet(sheet.id)
        .expect("still there")
        .panel;
    assert!((raised.height - (half + 100.0)).abs() < 0.5, "{raised:?}");

    // Flicked up: the next stop its way, which is the whole box.
    let settle = |runtime: &mut Runtime| {
        let mut closed = Vec::new();
        for _ in 0..120 {
            let areas = build(runtime).1.sheets().to_vec();
            closed.extend(runtime.advance_sheets(&areas, 1.0 / 60.0).1);
        }
        closed
    };
    runtime.sheet_release(sheet.id, &sheet.spec, px, 900.0, None);
    assert!(settle(&mut runtime).is_empty());
    assert_eq!(runtime.sheet_size(sheet.id, &sheet.spec), 1.0);

    // Lowered to just under a quarter and flicked down: put away, with the screen's message.
    runtime.sheet_drag(sheet.id, &sheet.spec, -0.76, px);
    runtime.sheet_release(sheet.id, &sheet.spec, px, -1200.0, None);
    let closed = settle(&mut runtime);
    assert_eq!(closed, vec![sheet.id], "dismissed once");
    let (tree, _) = build(&runtime);
    find_widget(tree.as_ref(), sheet.id)
        .and_then(|panel| panel.on_sheet_dismissed())
        .expect("the sheet says what to do when it is put away")
        .call();
    assert!(build(&runtime).1.sheets().is_empty(), "put away");
    // And asked back, it is there again.
    let (_, ui) = build(&runtime);
    assert!(press_in(&ui, "Show the sheet"));
    assert_eq!(build(&runtime).1.sheets().len(), 1);
}

/// **The page raises its sheet by name** (milestone 523). "Raise it" asks the host for the sheet
/// under the page's key; that key names the very sheet the frame reports, and what the request
/// asks for carries it to full height along its curve — not at once, and past the half-way stop
/// without settling on it.
#[test]
fn the_page_raises_its_sheet_by_name() {
    use frus_widgets::{find_sheet_by_key, SheetTo};
    let mut bench = Bench::new(400.0, 800.0);
    bench.go("/sheet");
    let mut runtime = Runtime::default();
    let size = bench.size;
    let theme = bench
        .app
        .resolved_theme(frus_widgets::Brightness::Dark, false);
    let build = |runtime: &Runtime| {
        MediaQuery::new(size).scope(|| {
            runtime.states.begin_build();
            let tree = Application::view(&bench.app, &theme);
            frus_widgets::build_deferred(tree.as_ref(), &theme, runtime);
            let ui = build_ui(tree.as_ref(), size, runtime, &theme);
            runtime.states.end_frame();
            (tree, ui)
        })
    };
    let (tree, ui) = build(&runtime);
    let sheet = ui
        .sheets()
        .first()
        .cloned()
        .expect("the screen has a sheet");
    assert_eq!(
        find_sheet_by_key(tree.as_ref(), frus_widgets::host::key_hash(PLACES_SHEET)),
        Some(sheet.id),
        "the page's key names the sheet the frame reports"
    );

    // Pressing "Raise it" asks the host for the sheet, by name.
    let _ = frus_widgets::host::take_effects();
    assert!(press_in(&ui, "Raise it"));
    let asked = frus_widgets::host::take_effects();
    let Some(frus_widgets::host::Effect::Sheet(key, to)) = asked.into_iter().next() else {
        panic!("a request to move the sheet");
    };
    assert_eq!(key, frus_widgets::host::key_hash(PLACES_SHEET));
    // Placed as the shell places it.
    let _: &SheetTo = &to;
    assert!(runtime.sheet_to(sheet.id, &sheet.spec, &to));
    let step = |runtime: &mut Runtime, frames: usize| {
        for _ in 0..frames {
            let areas = build(runtime).1.sheets().to_vec();
            runtime.advance_sheets(&areas, 1.0 / 60.0);
        }
        runtime.sheet_size(sheet.id, &sheet.spec)
    };
    let under_way = step(&mut runtime, 9);
    assert!(
        under_way > 0.5 && under_way < 1.0,
        "on its way, not there at once: {under_way}"
    );
    assert_eq!(
        step(&mut runtime, 30),
        1.0,
        "at full height once it is done"
    );
    assert_eq!(step(&mut runtime, 60), 1.0, "and it stays there");
}

/// **The end of the sheet's list clears the system's bottom bar** (milestone 515). The sheet
/// reaches the bottom of the window and draws under the navigation bar, as the reference's does;
/// seen on a phone, the list's last place sat under the buttons, out of reach. Scrolled to its
/// end at full height, under a surface with a bar, the last place is above it.
#[test]
fn the_end_of_the_sheets_list_clears_the_bottom_bar() {
    let mut bench = Bench::new(400.0, 800.0);
    bench.go("/sheet");
    let bar = 48.0;
    bench.insets = WindowInsets::bars(Insets::new(0.0, 0.0, bar, 0.0));
    let (_, ui) = bench.frame();
    let sheet = ui.sheets().first().cloned().expect("a sheet");
    let list = *sheet.areas.first().expect("its list");
    let mut runtime = Runtime::default();
    let size = bench.size;
    let theme = bench
        .app
        .resolved_theme(frus_widgets::Brightness::Dark, false);
    let build = |runtime: &Runtime| {
        MediaQuery::new(size).with_insets(bench.insets).scope(|| {
            runtime.states.begin_build();
            let tree = Application::view(&bench.app, &theme);
            frus_widgets::build_deferred(tree.as_ref(), &theme, runtime);
            let ui = build_ui(tree.as_ref(), size, runtime, &theme);
            runtime.states.end_frame();
            ui
        })
    };
    runtime.sheet_drag(sheet.id, &sheet.spec, 1.0, sheet.available);
    let max = build(&runtime)
        .scroll_region(list)
        .expect("the list scrolls")
        .max_y;
    runtime.scroll.insert(list, (0.0, max));

    fn last_place(primitives: &[Primitive]) -> Option<(f32, f32)> {
        primitives.iter().find_map(|p| match p {
            Primitive::Text {
                position,
                text,
                size,
                ..
            } if text == "20. Cable car" => Some((position.y, *size)),
            Primitive::Layer { primitives, .. } => last_place(primitives),
            _ => None,
        })
    }
    let ui = build(&runtime);
    let (top, glyphs) = last_place(ui.scene().primitives()).expect("the last place is painted");
    assert!(
        top + glyphs <= size.height - bar,
        "the last place ends at {} and the bar starts at {}",
        top + glyphs,
        size.height - bar
    );
}

// ---------------------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------------------

/// **Scrolling one page does not scroll another** (milestone 528). Reported from the phone:
/// "when I scroll another page, the one I just left scrolls too." The data table's page and the
/// editable grid's are built the same way, and a navigator's settled page is always its first
/// child, so their scroll regions had one identity between them. Each page is keyed by its place
/// in the stack and its route now.
#[test]
fn scrolling_one_page_does_not_scroll_another() {
    let mut bench = Bench::new(420.0, 360.0);
    bench.go("/data");
    let table = bench.frame().1.scroll_regions()[0].id;
    bench.runtime.scroll.insert(table, (120.0, 0.0));

    bench.router.pop();
    bench.settle();
    bench.go("/grid");
    let grid = bench.frame().1.scroll_regions()[0].id;
    assert_eq!(
        bench.runtime.scroll.get(&grid),
        None,
        "the grid was never scrolled, and read the table's offset (table {table:?}, grid {grid:?})"
    );
}

/// **A page returned to keeps its scroll on the way back** (milestone 528). The page arriving on
/// a pop was the navigator's second child for as long as the pop lasted and its first once it
/// was over, so settings scrolled down came back at the top through the whole slide — and
/// through a back gesture's preview — and jumped to where they had been left when it ended.
#[test]
fn a_page_returned_to_keeps_its_scroll_through_the_pop() {
    let mut bench = Bench::new(420.0, 360.0);
    bench.go("/settings");
    let settings = bench.frame().1.scroll_regions()[0].id;
    bench.runtime.scroll.insert(settings, (0.0, 300.0));

    // The page below: the one drawn behind, parallaxed to the left — as the push slides it out,
    // as the back gesture previews it, and as the pop slides it back in.
    let below = |bench: &Bench| {
        bench
            .frame()
            .1
            .scroll_regions()
            .iter()
            .find(|area| area.viewport.x < -1.0)
            .expect("the page below is in the frame")
            .id
    };

    bench.router.push("/journal");
    Application::tick(&mut bench.app, 0.05);
    Application::tick(&mut bench.app, 0.05);
    let leaving = below(&bench);
    assert_eq!(
        bench.runtime.scroll.get(&leaving),
        Some(&(0.0, 300.0)),
        "the push slides settings out where they were left"
    );
    bench.settle();

    Application::back_gesture(&mut bench.app, 0.5);
    let previewed = below(&bench);
    assert_eq!(
        bench.runtime.scroll.get(&previewed),
        Some(&(0.0, 300.0)),
        "the back gesture previews settings where they were left"
    );
    Application::back_gesture_end(&mut bench.app, -5.0);
    bench.settle();

    bench.router.pop();
    Application::tick(&mut bench.app, 0.05);
    Application::tick(&mut bench.app, 0.05);
    let sliding = below(&bench);
    assert_eq!(
        bench.runtime.scroll.get(&sliding),
        Some(&(0.0, 300.0)),
        "the pop slides settings in where they were left"
    );
}

#[test]
fn back_gesture_flick_commits_pop() {
    let mut bench = Bench::new(400.0, 800.0);
    bench.go("/settings");
    assert_eq!(bench.router.depth(), 2);
    // A small drag but a fast flick → it must commit the back.
    Application::back_gesture(&mut bench.app, 0.2);
    Application::back_gesture_end(&mut bench.app, 5.0);
    bench.settle();
    assert_eq!(bench.router.depth(), 1, "the flick popped the screen");
    assert_eq!(bench.router.location(), "/");
}

/// Going to a task's own screen puts its id in the address, and the router's page state carries
/// it: the location a reader could be sent back to.
#[test]
fn a_task_is_an_address() {
    let mut bench = Bench::new(400.0, 800.0).with_tasks(&["a", "b"]);
    let id = bench.demo.todos()[1].id;
    bench.go(&format!("/task/{id}"));
    assert_eq!(bench.router.location(), format!("/task/{id}"));
    assert_eq!(bench.router.depth(), 2, "home is underneath");
    assert!(bench.words().iter().any(|w| w == "b"));
}

/// An open menu takes the back gesture before the router does: the settings screen's dropdown
/// blocks it while open, and gives it back when it is shut.
#[test]
fn an_open_dropdown_takes_the_back_gesture() {
    let mut bench = Bench::new(400.0, 800.0);
    bench.go("/settings");
    let _ = bench.frame();
    assert!(Application::can_go_back(&bench.app), "a page to go back to");
    assert!(bench.press("Option A"));
    let _ = bench.frame();
    assert!(
        !Application::can_go_back(&bench.app),
        "the menu is open: back closes it, it does not leave the page"
    );
}

/// Turning the phone must not move the navigation (milestone 305). Reported from a device:
/// rotating to landscape made the navigation leave the bottom of the screen and reappear as a
/// rail down the left edge. The cause was in `Scaffold`, which measured its own width; this is
/// the demo's own screen, at the reporter's own logical size — a widget test can pass while the
/// application still looks wrong.
#[test]
fn rotating_the_phone_leaves_the_navigation_at_the_bottom() {
    // The reporter's device: 1080 × 2340 at a density that gives 424 × 918 logical.
    let portrait = (424.0, 918.0);
    let landscape = (918.0, 424.0);
    fn nav_bottom(width: f32, height: f32) -> f32 {
        let bench = Bench::new(width, height);
        let lowest = bench
            .texts()
            .into_iter()
            .filter(|(text, _, _)| text == "Tasks" || text == "Stats" || text == "About")
            .map(|(_, _, y)| y)
            .fold(f32::MIN, f32::max);
        assert!(lowest > f32::MIN, "the destinations were never painted");
        lowest
    }
    let low_portrait = nav_bottom(portrait.0, portrait.1);
    assert!(
        low_portrait > portrait.1 * 0.7,
        "portrait: destinations at y = {low_portrait} of {}",
        portrait.1
    );
    let low_landscape = nav_bottom(landscape.0, landscape.1);
    assert!(
        low_landscape > landscape.1 * 0.7,
        "landscape: destinations at y = {low_landscape} of {} — the navigation moved",
        landscape.1
    );
}

/// **The frame a push ends on still holds the page it left** (milestone 531).
///
/// Seen on a phone: a back gesture from the left edge of the data table, at the height of a home
/// task row, lifted that row instead of sliding the page. The shell builds the view again only
/// while the application says it is moving, and the tick that ends a push says it is not — so
/// the frame it hit-tests against is the push's last, with the home page still in it,
/// parallaxed under the edge. The shell now builds once more on the frame an animation settles
/// in; this is the premise, on the demo's own page, at the phone's coordinates.
#[test]
fn the_last_frame_of_a_push_still_holds_the_page_it_left() {
    // The Huawei STK-L21 in logical pixels, and the finger: 8 px in, 1400 px down.
    let size = Size::new(392.7, 850.9);
    let edge = Point::new(3.0, 509.0);
    let mut bench = Bench::new(size.width, size.height).with_tasks(&["Write code"]);
    bench.router.push("/data");
    // The loop as it was: the view built only while the application says it is moving.
    let mut last_built = None;
    while Application::tick(&mut bench.app, 1.0 / 60.0) {
        last_built = Some(bench.frame().1);
    }
    let last_built = last_built.expect("the push takes frames");
    assert!(
        last_built.drag_source_at(edge).is_some(),
        "the push's last frame has a home row under the back gesture's finger"
    );
    assert!(
        bench.frame().1.drag_source_at(edge).is_none(),
        "a frame built once the push has settled has nothing there"
    );
}

/// **The board's strip, carried through the shell, moves a label and no card** (milestone 527,
/// seen on a phone: a label carried along the strip moved a card of the board instead).
///
/// The demo itself, on the phone's surface, through the shell's own input path and frame: the
/// Kanban screen pushed and settled, then Feature carried along x to Design's right half and let
/// go. On a desktop the strip's labels are picked up by the grip under each one.
#[test]
fn the_board_strip_carried_through_the_shell_moves_a_label_and_no_card() {
    let (app, router, _) = crate::build();
    let mut driver = Driver::new(app, 392.7, 850.9);
    driver.run(0.3);
    router.push("/board");
    driver.run(1.5);
    let column = |driver: &Driver<FrusApp>, label: &str| {
        driver
            .texts()
            .into_iter()
            .find(|(text, _)| text == label)
            .map(|(_, rect)| rect.x)
            .unwrap_or_else(|| panic!("{label:?} is on the board"))
    };
    assert!(column(&driver, "Feature") < column(&driver, "Design"));
    let cards = |driver: &Driver<FrusApp>| {
        driver
            .texts()
            .into_iter()
            .filter(|(text, _)| {
                [
                    "Design API",
                    "Write spec",
                    "Triage bugs",
                    "Build widget",
                    "Kickoff",
                    "Research",
                ]
                .contains(&text.as_str())
            })
            .map(|(text, _)| text)
            .collect::<Vec<_>>()
    };
    let before = cards(&driver);
    // Feature's grip: under its label, 128 to 224 across.
    driver.press(Point::new(176.0, 129.0));
    for x in [190.0, 220.0, 250.0, 280.0, 306.0] {
        driver.move_to(Point::new(x, 129.0));
        driver.run(0.1);
    }
    assert!(driver.carried().is_some(), "Feature is carried");
    driver.release(Point::new(306.0, 129.0));
    driver.run(0.2);
    assert!(
        column(&driver, "Design") < column(&driver, "Feature"),
        "Feature went past Design"
    );
    assert_eq!(cards(&driver), before, "no card moved");
}

/// The application, through the shell: built, started, driven by real taps. The counter of the
/// task list moves as a task is added and ticked, and the notification queue plays out.
#[test]
fn the_application_runs_through_the_shell() {
    let (app, router, demo) = crate::build();
    let mut driver = Driver::new(app, 500.0, 800.0);
    driver.run(0.3);
    demo.add("write the milestone");
    driver.run(0.1);
    assert!(driver
        .texts()
        .iter()
        .any(|(t, _)| t == "write the milestone"));
    // Save queues the notification; the host is asked to bring it down after a while.
    let _ = frus_widgets::host::take_effects();
    demo.save();
    driver.run(0.1);
    assert!(
        driver.texts().iter().any(|(t, _)| t == "Saved"),
        "the toast is up"
    );
    let _ = router;
}

/// **The demonstration's own light switch is light on a phone in night mode.**
///
/// It pins the mode, and the framework then asks `theme()` for the light theme and
/// `dark_theme()` for the dark one. `theme()` read the *platform's* brightness instead, so under
/// a night-mode phone the pinned light theme came back dark and the switch did nothing — found
/// on a device, trying to photograph the light status bar for #46.
#[test]
fn the_light_switch_is_light_under_a_dark_platform() {
    let (app, _, demo) = crate::build();
    demo.toggle_theme();
    assert!(demo.prefs().light, "the fixture: the switch is on light");
    let mut night = MediaQuery::new(Size::new(400.0, 800.0));
    night.platform_brightness = frus_widgets::Brightness::Dark;
    let theme =
        night.scope(|| Application::resolved_theme(&app, frus_widgets::Brightness::Dark, false));
    assert_eq!(theme.brightness(), frus_widgets::Brightness::Light);
}

/// **The language menu switches the framework's words too**, not only the application's.
///
/// Two different mechanisms answer one gesture: this application's own strings come from its
/// Fluent resources through `locale`, and the framework's — a calendar's months, the label on a
/// back arrow — from a table through `localizations`. A reader who picks Français and gets a
/// French interface around an English calendar has been told the switch did not work.
#[test]
fn choosing_french_hands_the_framework_a_french_table() {
    let (app, _, demo) = crate::build();
    demo.cycle_lang();
    demo.cycle_lang();
    assert_eq!(
        Application::locale(&app)
            .map(|l| l.language_code().to_string())
            .as_deref(),
        Some("fr")
    );
    let table = Application::localizations(&app).expect("a French table");
    assert_eq!(table.months()[0], "janvier");
    assert_eq!(
        table.first_day_of_week_index(),
        1,
        "the week starts on Monday"
    );
    // **Arabic mirrors but is not translated**, deliberately: there is no Arabic table in the
    // framework yet, and a machine-translated one would be worse than none.
    demo.cycle_lang();
    assert!(Application::localizations(&app).is_none());
    // The title of the home screen follows the language.
    let bench = Bench::new(800.0, 600.0);
    bench.demo.cycle_lang();
    bench.demo.cycle_lang();
    assert!(
        bench
            .words()
            .iter()
            .any(|w| w == "Tâches" || w.contains("tâche")),
        "{:?}",
        bench.words()
    );
}

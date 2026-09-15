//! [`Navigator`]: shows a full-window **screen**, with a slide transition
//! between the outgoing and the incoming screen on a push or a pop.
//!
//! The `Navigator` is **controlled**: the application holds the route stack and
//! the transition's progress, and (re)builds the screens on every frame.

use std::hash::Hash;

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::keyed::Keyed;
use crate::media::MediaQuery;
use crate::theme::Theme;
use crate::widget::Widget;

/// A screen container with a slide transition.
///
/// # Every page has a key
///
/// What a page keeps from one frame to the next — how far it is scrolled, where its
/// caret is, the sheet it has raised — is kept under the identity of the widget it
/// belongs to, and an identity is a position in the tree unless a key says otherwise. A
/// navigator's pages have no position worth the name. The page on show is its first
/// child whichever page it is, so two pages built the same way had one scroll offset
/// between them; and during a transition the page arriving is its second child, then its
/// first once it has arrived, so it had one identity while it slid in and another when it
/// stopped. [`Navigator::new`] and [`Navigator::from`] therefore each take a **key**, and
/// everything inside a page takes its identity from that key rather than from where the
/// page sits. The reference gives every route its own subtree and its own storage for the
/// same reason.
///
/// **A key names an entry of the stack, not a route.** The same route can be on the stack
/// twice — a thread opened from a thread — and those are two pages, each scrolled on its
/// own; a key made of the route alone would give them one offset again. `(depth, route)`
/// is the shape to reach for: the entry's index in the stack and what it shows. The two
/// pages of a transition must never share a key, and a debug build says so.
///
/// **A page that is not in the tree keeps its scroll offsets.** A page below the top is
/// not built, and when it is shown again under the same key it comes back where it was
/// left. What a page was animating is not kept: a widget seen again adopts its target,
/// as one seen for the first time does. And since nothing forgets the offsets of an entry
/// that was popped, an entry pushed again under the same key opens where the last one
/// was left.
///
/// A key a page's own root declares is replaced by the page's key. To name something
/// inside a page for a request by key — a focus, a scroll, a sheet — key it inside the
/// page.
///
/// ```
/// use frus_widgets::{Navigator, Text};
///
/// #[derive(Clone, Copy, Hash)]
/// enum Route {
///     Inbox,
///     Thread(u64),
/// }
///
/// // The stack the application holds: the inbox at depth 0, a thread pushed on it.
/// let settled: Navigator<()> = Navigator::new((1, Route::Thread(7)), Text::new("Thread 7"));
///
/// // Part-way through that push: the thread arriving, the inbox it came from leaving.
/// let pushing: Navigator<()> = Navigator::new((1, Route::Thread(7)), Text::new("Thread 7"))
///     .from((0, Route::Inbox), Text::new("Inbox"), 0.4, true);
/// # let _ = (settled, pushing);
/// ```
pub struct Navigator<Msg> {
    width: f32,
    height: f32,
    /// Transition progress (`1.0` = no transition in flight).
    progress: f32,
    /// Whether the pages are cut off at the navigator's own edge.
    clips: bool,
    /// `true` = push (entering from the right), `false` = pop (entering from the left).
    forward: bool,
    /// `[screen]` or `[outgoing, incoming]`, each wrapped in the [`Keyed`] its key makes.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: 'static> Navigator<Msg> {
    /// Shows a full-window screen (no transition), on **the surface it is being built
    /// for**, under the key `key` — unique to its entry of the stack (see
    /// [`Navigator`]).
    ///
    /// The size comes from [`MediaQuery::of`] — a window is a window, and the
    /// application has no business measuring one to say how far a screen slides.
    /// [`Navigator::size`] overrides it for a stack that is not the whole window, and
    /// for a test that would rather state a size than install a description.
    pub fn new(key: impl Hash, screen: impl Widget<Msg> + 'static) -> Self {
        let surface = MediaQuery::of();
        Self {
            width: surface.size.width,
            height: surface.size.height,
            progress: 1.0,
            forward: true,
            clips: true,
            children: vec![Box::new(Keyed::new(key, screen))],
        }
    }

    /// Whether the pages are **cut off at the navigator's own edge**. `true` by default,
    /// as the reference's `Clip.hardEdge` is.
    ///
    /// A screen sliding in comes from outside the box and one sliding out goes outside
    /// it, so without this the only thing stopping them is the window: a navigator that
    /// is not the whole window paints its pages over whatever is beside it. `false` is
    /// for the rare transition meant to spill — a card that grows past its own frame —
    /// and it is a decision, not a default.
    pub fn clip_behavior(mut self, clips: bool) -> Self {
        self.clips = clips;
        self
    }

    /// The window's size, in logical pixels — an **override** of what
    /// [`Navigator::new`] read from the ambient description.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Adds the **outgoing** screen, under its own key, and the progress of a transition
    /// in flight.
    ///
    /// `key` is the key the outgoing screen had while it was on show, so that it keeps
    /// its state as it leaves; it must differ from the incoming screen's.
    pub fn from(
        mut self,
        key: impl Hash,
        previous: impl Widget<Msg> + 'static,
        progress: f32,
        forward: bool,
    ) -> Self {
        let previous = Keyed::new(key, previous);
        debug_assert!(
            self.children
                .last()
                .is_none_or(|incoming| incoming.key() != Widget::<Msg>::key(&previous)),
            "a navigator's two pages share a key, and would share their state: key each \
             page by its entry of the stack, `(depth, route)`"
        );
        self.children.insert(0, Box::new(previous));
        self.progress = progress.clamp(0.0, 1.0);
        self.forward = forward;
        self
    }
}

impl<Msg> Widget<Msg> for Navigator<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn navigator(&self) -> Option<(f32, bool)> {
        Some((self.progress, self.forward))
    }

    fn navigator_clips(&self) -> bool {
        self.clips
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    fn screen(color: Color) -> Container<()> {
        Container::<()>::new()
            .width(400.0)
            .height(300.0)
            .color(color)
    }

    /// **A navigator's pages stop at its own edge.**
    ///
    /// A screen sliding in comes from outside the box and one sliding out goes outside
    /// it. Until milestone 398 the only thing stopping them was the window, so a
    /// navigator that was **not** the whole window painted its pages straight over
    /// whatever sat beside it — and a full-window one spent every transition frame
    /// drawing a screen nobody could see. The reference clips by default
    /// (`Clip.hardEdge`).
    ///
    /// The check is a 200×200 navigator in a 400×400 viewport, mid-transition: every
    /// primitive it paints has to be confined to its own box.
    #[test]
    fn a_navigators_pages_stop_at_its_own_edge() {
        let nav = |clips: bool| {
            let red = Color::rgb(1.0, 0.0, 0.0);
            let blue = Color::rgb(0.0, 0.0, 1.0);
            let mut navigator = Navigator::new("blue", screen(blue)).size(200.0, 200.0);
            navigator = navigator.clip_behavior(clips);
            let navigator = navigator.from("red", screen(red), 0.5, true);
            let ui = build_ui(
                &navigator,
                Size::new(400.0, 400.0),
                &Runtime::default(),
                &crate::Theme::default(),
            );
            // The furthest right anything is allowed to be painted.
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { clip, .. } => Some(clip.x + clip.width),
                    _ => None,
                })
                .fold(0.0_f32, f32::max)
        };
        assert!(
            nav(true) <= 200.5,
            "clipped, nothing may be painted past x = 200: got {}",
            nav(true)
        );
        assert!(
            nav(false) > 200.5,
            "and a navigator told not to clip really does not"
        );
    }

    #[test]
    fn transition_renders_both_screens() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let nav = Navigator::new("blue", screen(blue))
            .size(400.0, 300.0)
            .from("red", screen(red), 0.5, true);
        let ui = build_ui(
            &nav,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        let has = |c: Color| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == c))
        };
        assert!(has(red), "the outgoing screen is rendered");
        assert!(has(blue), "the incoming screen is rendered");
    }

    /// **An overlay belongs to its screen (milestone 326).** Found on a device: an app
    /// bar's overflow menu, left open while choosing an item that navigates, stayed drawn
    /// over the screen that replaced it.
    ///
    /// `process_overlays` runs after both screens and paints above the whole window, so a
    /// deferred overlay outranks everything — including the screen that covers its owner.
    /// The parallax is why nothing corrected it on its own: the outgoing screen travels
    /// only 30 % of the width, so the anchor the menu hangs from never leaves the window.
    ///
    /// `from` inserts the screen being left at index 0, so `children[1]` is always the
    /// destination — on a push, on a pop, and under a back gesture alike.
    #[test]
    fn a_departing_screens_overlay_is_not_drawn_over_the_incoming_one() {
        let mark = Color::rgb(0.0, 1.0, 0.0);
        let menu = || {
            crate::OverlayPortal::new(screen(Color::rgb(1.0, 0.0, 0.0))).overlay(
                Container::<()>::new().width(80.0).height(30.0).color(mark),
                crate::Placement::Below,
            )
        };
        let drawn = |nav: &Navigator<()>| {
            let ui = build_ui(
                nav,
                Size::new(400.0, 300.0),
                &Runtime::default(),
                &crate::Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == mark))
        };

        // The screen being left holds the menu: it goes with the screen.
        let blue = Color::rgb(0.0, 0.0, 1.0);
        assert!(
            !drawn(
                &Navigator::new("blue", screen(blue))
                    .size(400.0, 300.0)
                    .from("menu", menu(), 0.5, true)
            ),
            "a push: the menu belongs to the screen being left"
        );
        assert!(
            !drawn(
                &Navigator::new("blue", screen(blue))
                    .size(400.0, 300.0)
                    .from("menu", menu(), 0.5, false)
            ),
            "a pop: the same, and the screen being left is the *front* one here"
        );

        // The destination's own overlay is untouched — this must not suppress overlays
        // wholesale, only the ones belonging to a screen on its way out.
        assert!(
            drawn(&Navigator::new("menu", menu()).size(400.0, 300.0).from(
                "blue",
                screen(blue),
                0.5,
                true
            )),
            "the destination's own menu is still drawn"
        );
        // And with no transition in flight, nothing changes at all.
        assert!(
            drawn(&Navigator::new("menu", menu()).size(400.0, 300.0)),
            "no transition: the menu is simply drawn"
        );
    }

    /// A page whose content scrolls — the shape of nearly every page an application has,
    /// which is what made two different ones indistinguishable by position. Its content
    /// is painted in `mark`, so where the content is drawn says how far it is scrolled.
    fn scrolling(mark: Color) -> Container<()> {
        Container::<()>::new().width(400.0).height(300.0).child(
            crate::SingleChildScrollView::new()
                .width(400.0)
                .height(300.0)
                .child(Container::new().width(400.0).height(1200.0).color(mark)),
        )
    }

    /// A navigator showing one page, settled.
    fn settled(key: impl Hash, page: Container<()>) -> Navigator<()> {
        Navigator::new(key, page).size(400.0, 300.0)
    }

    /// A navigator half-way through a transition from `(from, previous)` to `(key, page)`.
    fn moving(
        key: &str,
        page: Container<()>,
        from: &str,
        previous: Container<()>,
        forward: bool,
    ) -> Navigator<()> {
        Navigator::new(key, page)
            .size(400.0, 300.0)
            .from(from, previous, 0.5, forward)
    }

    /// How far the content painted in `mark` is scrolled, as drawn: its top edge sits at
    /// minus the offset. `None` when nothing in that colour is painted.
    fn drawn_offset(nav: &Navigator<()>, runtime: &Runtime, mark: Color) -> Option<f32> {
        let ui = build_ui(
            nav,
            Size::new(400.0, 300.0),
            runtime,
            &crate::Theme::default(),
        );
        ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Rect { color, rect, .. } if *color == mark => Some(-rect.y),
            _ => None,
        })
    }

    /// The one scroll region a settled navigator registers.
    fn region(nav: &Navigator<()>) -> crate::WidgetId {
        let ui = build_ui(
            nav,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        let regions = ui.scroll_regions();
        assert_eq!(regions.len(), 1, "one page, one region");
        regions[0].id
    }

    const INBOX: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    const ARCHIVE: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    /// **Scrolling one page does not scroll another (milestone 528).** Found on a phone:
    /// "when I scroll another page, the one I just left scrolls too."
    ///
    /// Retained state is kept by identity, and identity was the position in the tree. A
    /// navigator's settled page is always its first child, so two pages built the same
    /// way — a scroll view in a box — had the same identity for their scroll regions and
    /// read one offset between them.
    #[test]
    fn two_pages_of_the_same_shape_keep_two_offsets() {
        let mut runtime = Runtime::default();
        let inbox = settled("inbox", scrolling(INBOX));
        runtime.scroll.insert(region(&inbox), (0.0, 400.0));
        assert_eq!(drawn_offset(&inbox, &runtime, INBOX), Some(400.0));

        let archive = settled("archive", scrolling(ARCHIVE));
        assert_eq!(
            drawn_offset(&archive, &runtime, ARCHIVE),
            Some(0.0),
            "the archive was never scrolled"
        );
    }

    /// **A page keeps its state through a transition (milestone 528).** Two children in
    /// flight and one at rest put the arriving page at index 1 during the slide and at
    /// index 0 once it had arrived — a different identity on each side of the settle, so
    /// an inbox scrolled down came back at the top all the way through the pop, and
    /// jumped to where it had been only when the pop was over.
    #[test]
    fn a_transition_does_not_move_state_between_its_pages() {
        let mut runtime = Runtime::default();
        runtime
            .scroll
            .insert(region(&settled("inbox", scrolling(INBOX))), (0.0, 400.0));
        runtime.scroll.insert(
            region(&settled("archive", scrolling(ARCHIVE))),
            (0.0, 700.0),
        );

        for forward in [true, false] {
            // Inbox to archive, and archive to inbox, each way round.
            let there = moving(
                "archive",
                scrolling(ARCHIVE),
                "inbox",
                scrolling(INBOX),
                forward,
            );
            let back = moving(
                "inbox",
                scrolling(INBOX),
                "archive",
                scrolling(ARCHIVE),
                forward,
            );
            for nav in [&there, &back] {
                assert_eq!(
                    drawn_offset(nav, &runtime, INBOX),
                    Some(400.0),
                    "the inbox, forward = {forward}"
                );
                assert_eq!(
                    drawn_offset(nav, &runtime, ARCHIVE),
                    Some(700.0),
                    "the archive, forward = {forward}"
                );
            }
        }
    }

    /// **The same route twice is two pages (milestone 528).** A stack can hold one route
    /// more than once — a thread opened from a thread — and each entry scrolls on its own.
    /// That is why the key names the entry and not the route: `(depth, route)` tells the
    /// two apart where the route alone would not.
    #[test]
    fn the_same_route_pushed_twice_keeps_two_offsets() {
        let mut runtime = Runtime::default();
        let lower = settled((1, "thread"), scrolling(INBOX));
        runtime.scroll.insert(region(&lower), (0.0, 400.0));

        let upper = settled((2, "thread"), scrolling(INBOX));
        assert_eq!(
            drawn_offset(&upper, &runtime, INBOX),
            Some(0.0),
            "the thread opened on top starts at its own top"
        );
        assert_eq!(
            drawn_offset(&lower, &runtime, INBOX),
            Some(400.0),
            "and the one below it is still where it was left"
        );
    }

    /// Two pages under one key would be one identity drawn twice in a frame, and a debug
    /// build refuses it rather than let them share their state.
    #[test]
    #[should_panic(expected = "share a key")]
    fn a_transitions_two_pages_may_not_share_a_key() {
        let _ = Navigator::<()>::new((1, "thread"), screen(Color::rgb(0.0, 0.0, 1.0))).from(
            (1, "thread"),
            screen(Color::rgb(1.0, 0.0, 0.0)),
            0.5,
            true,
        );
    }

    #[test]
    fn pop_parallaxes_and_orders_back_screen() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        // A pop half-way through: `red` = outgoing screen (front), `blue` = revealed back.
        let nav = Navigator::new("blue", screen(blue))
            .size(400.0, 300.0)
            .from("red", screen(red), 0.5, false);
        let ui = build_ui(
            &nav,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        let x_of = |c: Color| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, rect, .. } if *color == c => Some(rect.x),
                    _ => None,
                })
                .expect("screen present")
        };
        let front = x_of(red); // +0.5·400 = 200
        let back = x_of(blue); // parallaxe : -0.5·400·0.3 = -60
        assert!(
            front > back,
            "the front ({front}) is to the right of the back ({back})"
        );
        // Without parallax the back would sit at -200; it is compressed toward 0.
        assert!(back > -200.0 && back < 0.0, "back parallaxed: {back}");
    }
}

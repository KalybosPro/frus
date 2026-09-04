//! [`NavScaffold`]: the **adaptive** navigation scaffold. It reads the [`SizeClass`] and
//! gives each of its three bands its own presentation of the primary navigation, the body
//! filling the rest. This is where the size class drives the screen's *structure*.
//!
//! | class | navigation |
//! |---|---|
//! | `Compact` | a bottom bar |
//! | `Medium` | a rail, glyphs alone |
//! | `Expanded` | an [extended](NavigationRail::extended) rail, labels beside the glyphs |
//! | `Expanded`, [asked](NavScaffold::nav_drawer) | a [`NavigationDrawer`] docked on the leading edge |
//!
//! The fourth row is opt-in because the reference gives **two** answers for the widest
//! band and they are both right. Material's adaptive guidance puts a navigation drawer
//! there; the framework's own adaptive study puts an extended rail
//! (`reply/adaptive_nav.dart:97`). The rail is the safer default — it costs 256 pixels
//! against the drawer's 304 and it needs no header to look finished — so it stays the
//! default, and an application with more destinations than a rail can hold says so.

use frus_core::{Color, Rect, Scene, SizeClass};
use frus_layout::{FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::navdrawer::NavigationDrawer;
use crate::navrail::{BottomBar, Destination, NavigationRail, RailLabels};
use crate::theme::Theme;
use crate::widget::Widget;

/// What a caller wants done to the rail this shell built for it. See
/// [`NavScaffold::rail`].
type RailConfig<Msg> = Box<dyn FnOnce(NavigationRail<Msg>) -> NavigationRail<Msg>>;

/// The same, for the drawer. See [`NavScaffold::nav_drawer`].
type DrawerConfig<Msg> = Box<dyn FnOnce(NavigationDrawer<Msg>) -> NavigationDrawer<Msg>>;

/// Adaptive navigation shell: a bottom bar, a rail, or an extended rail, by size.
pub struct NavScaffold<Msg> {
    class: SizeClass,
    selected: usize,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    destinations: Vec<Destination>,
    labels: Option<RailLabels>,
    rail: Option<RailConfig<Msg>>,
    /// Whether the widest band gets a drawer rather than an extended rail. See
    /// [`NavScaffold::nav_drawer`].
    drawer: Option<DrawerConfig<Msg>>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> NavScaffold<Msg> {
    /// Whether the navigation is a bottom bar rather than a rail. Free of the bounds the
    /// builders carry, since the layout asks it too.
    fn compact(&self) -> bool {
        self.class == SizeClass::Compact
    }
}

impl<Msg: Clone + 'static> NavScaffold<Msg> {
    /// Creates a scaffold: `class` decides the presentation, `selected` the active
    /// destination, and `on_select(i)` is emitted when a destination is chosen.
    pub fn new(
        class: SizeClass,
        selected: usize,
        on_select: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        Self {
            class,
            selected,
            on_select: Some(Box::new(on_select)),
            destinations: Vec::new(),
            labels: None,
            rail: None,
            drawer: None,
            children: Vec::new(),
        }
    }

    /// Adds a destination, a glyph plus a label. Call this **before** [`body`].
    ///
    /// [`body`]: NavScaffold::body
    pub fn destination(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.describing("destination");
        self.destinations.push(Destination::new(icon, label));
        self
    }

    /// Adds a notification count to the **last** destination.
    pub fn badge(self, count: u32) -> Self {
        self.describing("badge");
        self.decorate(|last| last.badge = Some(count))
    }

    /// The glyph the **last** destination shows while it is selected, where that differs
    /// from its resting one. See [`NavigationRail::selected_icon`].
    pub fn selected_icon(self, icon: impl Into<String>) -> Self {
        self.describing("selected_icon");
        let icon = icon.into();
        self.decorate(move |last| last.selected_icon = Some(icon))
    }

    /// Marks the **last** destination inaccessible. See [`NavigationRail::disabled`].
    pub fn disabled(self) -> Self {
        self.describing("disabled");
        self.decorate(|last| last.disabled = true)
    }

    /// The **last** destination's own indicator colour, over the theme's. See
    /// [`NavigationRail::indicator_color`].
    pub fn indicator_color(self, color: Color) -> Self {
        self.describing("indicator_color");
        self.decorate(move |last| last.indicator_color = Some(color))
    }

    /// Applies `f` to the destination just added. Silent when there is none.
    fn decorate(mut self, f: impl FnOnce(&mut Destination)) -> Self {
        if let Some(last) = self.destinations.last_mut() {
            f(last);
        }
        self
    }

    /// **When the destinations say what they are**, whichever widget the class chose.
    /// Unsaid, each keeps its own default — a bar labels everything, a plain rail
    /// nothing, an extended rail everything (milestones 432 and 433).
    pub fn nav_labels(mut self, labels: RailLabels) -> Self {
        self.describing("nav_labels");
        self.labels = Some(labels);
        self
    }

    /// **What to do to the rail** once the shell has built it — the door
    /// [`Scaffold::rail`](crate::Scaffold::rail) opens on the fixed shell, and the way to
    /// decline this one's extended form: `.rail(|rail| rail.extended(false))`.
    ///
    /// It runs last, after the destinations and after [`Self::nav_labels`], and is silent
    /// when the class chose a bottom bar.
    pub fn rail(
        mut self,
        configure: impl FnOnce(NavigationRail<Msg>) -> NavigationRail<Msg> + 'static,
    ) -> Self {
        self.describing("rail");
        self.rail = Some(Box::new(configure));
        self
    }

    /// **Give the widest band a drawer** instead of an extended rail, and say what to do
    /// to it once the shell has built it — the door [`Self::rail`] opens on the other two
    /// forms. `.nav_drawer(|drawer| drawer.header(title))` is the ordinary use: a drawer
    /// with nothing above its destinations is a rail that spent 48 more pixels.
    ///
    /// It is silent below `Expanded`. A drawer takes 304 pixels of a window and the two
    /// narrower bands do not have them to give — which is the whole reason there are three
    /// forms — so a shell that swapped one in at `Medium` would be answering a question
    /// about taste with the body's width.
    pub fn nav_drawer(
        mut self,
        configure: impl FnOnce(NavigationDrawer<Msg>) -> NavigationDrawer<Msg> + 'static,
    ) -> Self {
        self.describing("nav_drawer");
        self.drawer = Some(Box::new(configure));
        self
    }

    /// Everything above **describes** the navigation, and [`Self::body`] is what builds
    /// it. Saying so afterwards is a mistake with no effect, so it says which one.
    fn describing(&self, what: &str) {
        assert!(
            self.on_select.is_some(),
            "{what}() describes the navigation and has to come before body(), which \
             builds it"
        );
    }

    /// Sets the body and **finalises** the scaffold; call it last. The body fills the
    /// space left beside, or above, the navigation.
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        let on_select = self.on_select.take().expect("body called exactly once");
        let destinations = std::mem::take(&mut self.destinations);
        let body_pane: Box<dyn Widget<Msg>> = Box::new(Flex::column().flex(1.0).child(body));

        // Only one arm runs, so `on_select` is moved exactly once.
        let nav: Box<dyn Widget<Msg>> =
            if self.class == SizeClass::Expanded && self.drawer.is_some() {
                let configure = self.drawer.take().expect("just checked");
                let mut drawer =
                    NavigationDrawer::new(self.selected, on_select).width(crate::DRAWER_WIDTH);
                for destination in destinations {
                    drawer = drawer.destination(destination);
                }
                // `nav_labels` says nothing here on purpose: a drawer's destinations are named
                // or they are not destinations. The three modes exist because a rail and a bar
                // have to trade the words for room, and a drawer never makes that trade.
                Box::new(configure(drawer))
            } else if self.compact() {
                let mut bar = BottomBar::new(self.selected, on_select);
                for destination in destinations {
                    bar = bar.destination(destination);
                }
                if let Some(labels) = self.labels {
                    bar = bar.labels(labels);
                }
                Box::new(bar)
            } else {
                let mut rail = NavigationRail::new(self.selected, on_select);
                for destination in destinations {
                    rail = rail.destination(destination);
                }
                // **The third band gets the third presentation.** A window past the expanded
                // threshold has room for the words, and the reference's own adaptive study
                // spends it: `extended: !isTablet` on the desktop navigation
                // (`reply/adaptive_nav.dart:97`) — a bar below `medium`, a plain rail at
                // `medium`, an extended rail above it. This shell had two answers for three
                // bands, and gave the widest window the tablet's rail.
                rail = rail.extended(self.class == SizeClass::Expanded);
                if let Some(labels) = self.labels {
                    rail = rail.labels(labels);
                }
                if let Some(configure) = self.rail.take() {
                    rail = configure(rail);
                }
                Box::new(rail)
            };

        // Compact puts the body above and the bar below; otherwise the rail is on the
        // left and the body on the right.
        self.children = if self.compact() {
            vec![body_pane, nav]
        } else {
            vec![nav, body_pane]
        };
        self
    }
}

impl<Msg: Clone> Widget<Msg> for NavScaffold<Msg> {
    /// It asks to fill **both** axes rather than declaring `100%` on either — see
    /// [`FillAxes`](crate::widget::FillAxes). A percentage resolves against the parent's
    /// *resolved* size, which a parent that shrink-wraps has not got yet, so a shell
    /// nested in one vanished entirely (milestone 404).
    fn style(&self) -> Style {
        Style {
            flex_direction: if self.compact() {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            },
            ..Default::default()
        }
    }

    /// A shell takes everything it is offered, on both axes.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::BOTH
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
    }

    fn scaffold(class: SizeClass) -> NavScaffold<Msg> {
        NavScaffold::new(class, 0, Msg::Go)
            .destination("H", "Home")
            .destination("S", "Settings")
            .body(Container::new())
    }

    #[test]
    fn compact_puts_body_first_then_bottom_bar_in_a_column() {
        let s = scaffold(SizeClass::Compact);
        assert_eq!(
            Widget::<Msg>::style(&s).flex_direction,
            FlexDirection::Column
        );
        // [body, bar]: navigation is the last child, at the bottom.
        assert_eq!(Widget::<Msg>::children(&s).len(), 2);
    }

    /// The width the navigation declared, whichever of the three it is.
    fn nav_width(s: &NavScaffold<Msg>) -> frus_layout::Dimension {
        Widget::<Msg>::style(&*Widget::<Msg>::children(s)[0]).width
    }

    /// **The widest band can be asked for the third form.** It is not the default — see
    /// [`NavScaffold::nav_drawer`] — so this is the only test that will notice if the
    /// answer silently becomes one.
    #[test]
    fn the_widest_band_can_be_asked_for_a_drawer() {
        let s = NavScaffold::new(SizeClass::Expanded, 0, Msg::Go)
            .destination("H", "Home")
            .destination("S", "Settings")
            .nav_drawer(|drawer| drawer)
            .body(Container::new());
        assert_eq!(Widget::<Msg>::style(&s).flex_direction, FlexDirection::Row);
        assert_eq!(
            nav_width(&s),
            frus_layout::Dimension::Length(crate::DRAWER_WIDTH),
            "a drawer, not the extended rail this band gives by default"
        );
        assert_eq!(
            nav_width(&scaffold(SizeClass::Expanded)),
            frus_layout::Dimension::Length(crate::navrail::EXTENDED_RAIL_WIDTH),
            "and unasked, the band is unchanged"
        );
    }

    /// **And the two narrower ones decline.** A drawer takes 304 pixels of a window that
    /// has 600, and a shell that handed them over would be answering a question about
    /// taste with the body's width.
    #[test]
    fn a_narrower_band_keeps_the_form_that_fits() {
        let asked = |class| {
            NavScaffold::new(class, 0, Msg::Go)
                .destination("H", "Home")
                .nav_drawer(|drawer| drawer)
                .body(Container::new())
        };
        assert_eq!(
            nav_width(&asked(SizeClass::Medium)),
            frus_layout::Dimension::Length(crate::navrail::RAIL_WIDTH),
            "medium keeps its rail"
        );
        let compact = asked(SizeClass::Compact);
        assert_eq!(
            Widget::<Msg>::style(&compact).flex_direction,
            FlexDirection::Column,
            "and compact keeps its bar, at the bottom"
        );
    }

    /// The closure runs **after** the shell has built the drawer, so it has the last word
    /// — the same contract [`NavScaffold::rail`] has.
    #[test]
    fn what_the_caller_says_about_the_drawer_comes_last() {
        let s = NavScaffold::new(SizeClass::Expanded, 0, Msg::Go)
            .destination("H", "Home")
            .nav_drawer(|drawer| drawer.width(200.0))
            .body(Container::new());
        assert_eq!(nav_width(&s), frus_layout::Dimension::Length(200.0));
    }

    #[test]
    fn expanded_puts_rail_first_then_body_in_a_row() {
        let s = scaffold(SizeClass::Expanded);
        assert_eq!(Widget::<Msg>::style(&s).flex_direction, FlexDirection::Row);
        // The rail (1st child) has a fixed width; the body takes the rest.
        assert_eq!(
            nav_width(&s),
            frus_layout::Dimension::Length(crate::navrail::EXTENDED_RAIL_WIDTH)
        );
    }

    /// **Three bands, three presentations** (milestone 435).
    ///
    /// The shell had two answers for three classes, so the widest window got the
    /// tablet's rail. The reference's own adaptive study spends the room instead —
    /// `extended: !isTablet` on its desktop navigation (`reply/adaptive_nav.dart:97`):
    /// a bar below `medium`, a plain rail at `medium`, an extended rail above it.
    #[test]
    fn each_of_the_three_classes_gets_its_own_presentation() {
        assert_eq!(
            Widget::<Msg>::style(&scaffold(SizeClass::Compact)).flex_direction,
            FlexDirection::Column,
            "compact: a bar under the body"
        );
        assert_eq!(
            nav_width(&scaffold(SizeClass::Medium)),
            frus_layout::Dimension::Length(crate::navrail::RAIL_WIDTH),
            "medium: a rail, glyphs alone"
        );
        assert_eq!(
            nav_width(&scaffold(SizeClass::Expanded)),
            frus_layout::Dimension::Length(crate::navrail::EXTENDED_RAIL_WIDTH),
            "expanded: an extended rail, labels beside the glyphs"
        );
    }

    /// And the caller has the last word on it, through the same door the fixed shell
    /// opens: a window may be wide and still want its 176 pixels back.
    #[test]
    fn the_extended_rail_can_be_declined() {
        let s = NavScaffold::new(SizeClass::Expanded, 0, Msg::Go)
            .destination("H", "Home")
            .rail(|rail| rail.extended(false))
            .body(Container::new());
        assert_eq!(
            nav_width(&s),
            frus_layout::Dimension::Length(crate::navrail::RAIL_WIDTH)
        );
    }

    /// The label mode reaches whichever widget the class chose.
    #[test]
    fn the_label_mode_reaches_the_widget_the_class_chose() {
        let says_home = |class, labels: Option<RailLabels>| {
            let mut s = NavScaffold::new(class, 0, Msg::Go).destination("H", "Home");
            if let Some(labels) = labels {
                s = s.nav_labels(labels);
            }
            let s = s.body(Container::new());
            let ui = crate::build_ui(
                &s as &dyn Widget<Msg>,
                frus_core::Size::new(900.0, 600.0),
                &crate::Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "Home"))
        };
        assert!(
            !says_home(SizeClass::Medium, None),
            "a plain rail says nothing until asked"
        );
        assert!(says_home(SizeClass::Medium, Some(RailLabels::All)));
        assert!(!says_home(SizeClass::Compact, Some(RailLabels::None)));
    }

    /// Everything that **describes** the navigation has to come before the `body` that
    /// **builds** it. It used to be silently ignored, which is a bug that looks like a
    /// property that does not work.
    #[test]
    #[should_panic(expected = "has to come before body()")]
    fn describing_the_navigation_after_the_body_is_refused() {
        let _ = NavScaffold::new(SizeClass::Expanded, 0, Msg::Go)
            .destination("H", "Home")
            .body(Container::<Msg>::new())
            .destination("S", "Settings");
    }
}

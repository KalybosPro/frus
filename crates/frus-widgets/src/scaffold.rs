//! [`Scaffold`]: the **screen shell** of frus — the central coordinator of a
//! Material screen structure.
//!
//! The developer declares **slots** (app bar, body, navigation, drawer, FAB, modal
//! sheet); the Scaffold assembles them correctly — the **app bar pinned** at the
//! top, the **body** filling the middle, **navigation** in a bottom bar (or a
//! rail, if one is asked for), all of it **respecting the safe area**
//! (system insets). One piece of code, with no branching on mobile vs desktop.
//!
//! ```ignore
//! Scaffold::new()                            // the surface's size and intrusions,
//!                                            // read from the ambient description
//!     .background(theme.background)
//!     .app_bar(appbar)                       // pinned at the top
//!     .body(content)                         // fills what the bars leave
//!     .nav(app.section, Msg::SetSection)     // destinations, in a bottom bar
//!     .destination("✔", "Tasks").badge(3)
//!     .destination("▦", "Stats")
//!     .rail(|rail| rail.extended(true))      // when the navigation is a rail

//!     .drawer(menu, app.menu_open, Msg::ToggleMenu)          // leading edge
//!     .end_drawer(filters, app.drawer_open, Msg::ToggleDrawer)
//!     .persistent_footer(row![cancel, save])  // never scrolls away
//!     .fab_location(FabLocation::EndFloat)   // or docked, at either end
//!     .fab(fab_button("+", Msg::AddTodo))    // floating action button
//!     .bottom_sheet(sheet, app.sheet_open, Msg::ToggleSheet)
//!     .build()
//! ```
//!
//! **Nobody hands it the screen.** The size and the intrusions come from
//! [`MediaQuery::of`]: an application does not measure the window, subtract the notch
//! and the bars, and carry the remainder down to every widget that might want it. It
//! says `Scaffold::new()`. That is the reference's arrangement — its `Scaffold` takes
//! no size either — and it is not a convenience: a number that travels by hand gets
//! dropped, and the failure is a screen laid out to the wrong width.
//!
//! **What the body is given.** The body gets what the bars leave it: it starts below
//! the app bar, stops above the bottom bar and the persistent footer, and is shortened
//! by the soft keyboard so that a field at the end of a form is not covered by it. Each
//! of those is a decision a screen may reverse —
//! [`Scaffold::extend_body_behind_app_bar`], [`Scaffold::extend_body`] and
//! [`Scaffold::resize_to_avoid_bottom_inset`].
//!
//! **And what it is told.** The system's own intrusions — the notch, the gesture bar —
//! are *described* to the body, not spent on its behalf. With nothing below it the body
//! reaches the screen's edge, and the description it is handed says the gesture bar is
//! there; a body that must be held clear of it says [`SafeArea`](crate::SafeArea) and is
//! answered. That is the reference's arrangement, and it is what a background, a hero
//! image, or a list that should scroll *under* the gesture bar needs — a shell that spent
//! the intrusion for them made all three impossible, and made every screen pay for the
//! notch whether it wanted the room or not.
//!
//! It is only ever the **body**. A bar, a rail or a footer put in a slot consumes what it
//! is told about, so the chrome keeps clear of the intrusions without a screen saying
//! anything at all.
//!
//! **The body does not scroll.** It is placed in the room the bars leave and that is
//! all; a screen that needs to scroll puts a scrolling widget in the body, and picks
//! which one. That is not a limitation the shell happens to have — it is the whole
//! decision. A shell that scrolls everything decides for every screen at once: a
//! centred empty state cannot be centred any more, a screen with its own list ends up
//! with a scroller inside a scroller, and a screen with nothing to scroll still
//! reports itself scrollable to the gesture arena. Scrolling is a property of the
//! content, and the content is the screen's.

use frus_core::{Color, Insets, WindowInsets};
use frus_layout::Justify;

use crate::bottomappbar::BottomAppBar;
use crate::button::Variant;
use crate::container::Container;
use crate::flex::Flex;
use crate::media::MediaQuery;
use crate::navrail::{BottomBar, Destination, NavigationRail, RailLabels, BAR_HEIGHT};
use crate::stack::Stack;
use crate::widget::Widget;

/// The FAB's margin from the edge, and from the bottom bar.
const FAB_MARGIN: f32 = 16.0;
/// The padding around the persistent footer's row.
const FOOTER_PAD: f32 = 12.0;
/// The height a floating action button is assumed to have, absent
/// [`Scaffold::fab_size`]. The conventional Material diameter.
const FAB_SIZE: f32 = 56.0;

/// What a caller wants done to the [`NavigationRail`] the shell built for it, once it is
/// built and before it is measured. See [`Scaffold::rail`].
type RailConfig<Msg> = Box<dyn FnOnce(NavigationRail<Msg>) -> NavigationRail<Msg>>;

/// Where a [`Scaffold`]'s navigation destinations are drawn.
///
/// A **fixed** choice: whichever is asked for is what gets drawn, at every width. A
/// `Scaffold` never changes its own layout, which is the point of this type existing
/// (milestone 305).
///
/// Until then the scaffold measured its own width and swapped a bottom bar for a side
/// rail above a threshold, with no way to ask it not to. Rotating a phone to landscape
/// crosses that threshold — so the navigation moved from the bottom of the screen to
/// the left edge because the user turned their hand, and nothing in the application
/// had asked for that or could prevent it.
///
/// The reference does not do this either: its screen shell has one navigation slot, at
/// the bottom, and a rail is a separate widget placed by whoever wants one. Adapting
/// is a design decision, and it belongs to the application.
///
/// For navigation that **does** follow the size class, reach for
/// [`NavScaffold`](crate::NavScaffold) — a separate shell whose whole purpose is that,
/// and which says so in its name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavPlacement {
    /// A bottom bar, at every width. The default.
    #[default]
    Bottom,
    /// A vertical rail on the leading edge, at every width.
    Rail,
}

/// Where the floating action button sits.
///
/// Two questions, and they are independent: which end of the row, and whether the
/// button **floats** clear of the bottom bar or **docks** astride its top edge. The
/// docked placements are the ones a notched bottom bar is cut for.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum FabLocation {
    /// Leading end, clear of the bar.
    StartFloat,
    /// Centred, clear of the bar.
    CenterFloat,
    /// Trailing end, clear of the bar — the default, and where a thumb reaches.
    #[default]
    EndFloat,
    /// Leading end, centred on the bar's top edge.
    StartDocked,
    /// Centred on the bar's top edge, horizontally centred.
    CenterDocked,
    /// Trailing end, centred on the bar's top edge.
    EndDocked,
}

impl FabLocation {
    /// Whether the button straddles the bar's top edge rather than floating above it.
    pub fn docked(self) -> bool {
        matches!(
            self,
            FabLocation::StartDocked | FabLocation::CenterDocked | FabLocation::EndDocked
        )
    }

    /// Which end of the row it goes to.
    pub fn justify(self) -> Justify {
        match self {
            FabLocation::StartFloat | FabLocation::StartDocked => Justify::Start,
            FabLocation::CenterFloat | FabLocation::CenterDocked => Justify::Center,
            FabLocation::EndFloat | FabLocation::EndDocked => Justify::End,
        }
    }
}

/// The screen shell. A fluent builder finished by [`Scaffold::build`].
pub struct Scaffold<Msg> {
    width: f32,
    height: f32,
    insets: Insets,
    view_insets: Insets,
    /// The intrusions ignoring the keyboard; see [`Scaffold::window_insets`].
    view_padding: Insets,
    resize_to_avoid_bottom_inset: bool,
    extend_body: bool,
    extend_body_behind_app_bar: bool,
    background: Option<Color>,
    app_bar: Option<Box<dyn Widget<Msg>>>,
    body: Option<Box<dyn Widget<Msg>>>,
    selected: usize,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    destinations: Vec<Destination>,
    nav_placement: NavPlacement,
    nav_labels: Option<RailLabels>,
    /// What the caller wants done to the rail once the shell has built it. `None` leaves
    /// the rail the shell would have built anyway.
    rail: Option<RailConfig<Msg>>,
    drawer: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    end_drawer: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    bottom_app_bar: Option<BottomAppBar<Msg>>,
    fab: Option<Box<dyn Widget<Msg>>>,
    fab_location: FabLocation,
    fab_size: f32,
    persistent_footer: Option<Box<dyn Widget<Msg>>>,
    persistent_footer_alignment: Justify,
    persistent_footer_divider: bool,
    persistent_footer_color: Option<Color>,
    bottom_sheet: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    primary: bool,
    drawer_scrim_color: Option<Color>,
    drawer_barrier_dismissible: bool,
}

impl<Msg: Clone + 'static> Default for Scaffold<Msg> {
    /// The same as [`Scaffold::new`] — a shell for the surface being built for, with
    /// every slot empty.
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone + 'static> Scaffold<Msg> {
    /// Creates a shell for **the surface it is being built for**.
    ///
    /// The size and the intrusions both come from [`MediaQuery::of`]: the application
    /// does not measure the screen, subtract the notch and the bars from it, and hand
    /// the remainder down: it says `Scaffold::new()` and the shell keeps its own slots
    /// clear of whatever the platform reported. That is the reference's arrangement,
    /// where `Scaffold` reads `MediaQuery.of(context)` and takes no size at all.
    ///
    /// Outside any surface description — a unit test building a shell on its own —
    /// the size is zero, and [`Scaffold::size`] is how a test says how big the screen
    /// is. Wrapping the build in `MediaQuery::new(size).scope(..)` says it once for
    /// everything inside, which is usually what a test wants.
    ///
    /// The navigation is a **bottom bar** whatever the width; ask for
    /// [`Scaffold::nav_placement`] to have it anywhere else.
    pub fn new() -> Self {
        let surface = MediaQuery::of();
        Self {
            width: surface.size.width,
            height: surface.size.height,
            insets: surface.padding,
            view_insets: surface.view_insets,
            view_padding: surface.view_padding,
            resize_to_avoid_bottom_inset: true,
            extend_body: false,
            extend_body_behind_app_bar: false,
            background: None,
            app_bar: None,
            body: None,
            selected: 0,
            on_select: None,
            destinations: Vec::new(),
            nav_placement: NavPlacement::default(),
            // **Not a default of its own.** Left unsaid, each of the two navigation
            // widgets keeps the one the reference gives it — a rail labels nothing, a
            // bar labels everything (milestone 432) — and collapsing them to one answer
            // here would quietly undo that.
            nav_labels: None,
            rail: None,
            drawer: None,
            end_drawer: None,
            bottom_app_bar: None,
            fab: None,
            fab_location: FabLocation::default(),
            fab_size: FAB_SIZE,
            persistent_footer: None,
            persistent_footer_alignment: Justify::End,
            // The reference's footer carries a one-pixel line along its top by default
            // — `Divider.createBorderSide` — and ours had none.
            persistent_footer_divider: true,
            persistent_footer_color: None,
            bottom_sheet: None,
            primary: true,
            drawer_scrim_color: None,
            drawer_barrier_dismissible: true,
        }
    }

    /// The surface's size, in logical pixels — an **override**.
    ///
    /// [`Scaffold::new`] already takes it from [`MediaQuery::of`]. This is for the two
    /// cases that are not the whole screen: a shell laid into a sub-region of one, and
    /// a test that would rather state a size than install a description for it.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// The **permanent** safe area (system bars, notch): the Scaffold keeps the slots
    /// clear of it.
    ///
    /// This is the padding half of [`WindowInsets`]. To let the Scaffold lift its
    /// content off the soft keyboard as well, give it both halves with
    /// [`Scaffold::window_insets`].
    pub fn insets(mut self, insets: Insets) -> Self {
        self.insets = insets;
        self
    }

    /// The window insets, **both** kinds: the permanent padding and the transient
    /// area the keyboard covers.
    ///
    /// The two are kept apart because only one of them may be ignored:
    /// [`Scaffold::resize_to_avoid_bottom_inset`] decides whether the keyboard pushes
    /// the content up, and no setting lets content sit under the navigation bar.
    pub fn window_insets(mut self, insets: WindowInsets) -> Self {
        self.insets = insets.padding;
        self.view_insets = insets.view_insets;
        self.view_padding = insets.view_padding;
        self
    }

    /// Whether the body is lifted clear of the soft keyboard (`true` by default).
    ///
    /// Set it to `false` when the screen would rather be covered than shortened — a
    /// full-bleed image or a map, where losing half the height is worse than losing
    /// the bottom of it. The permanent insets are unaffected: content never goes
    /// under the navigation bar.
    pub fn resize_to_avoid_bottom_inset(mut self, resize: bool) -> Self {
        self.resize_to_avoid_bottom_inset = resize;
        self
    }

    /// Lets the body run **under** the bottom bar and the persistent footer rather
    /// than stopping above them.
    ///
    /// For a bar that is translucent or notched, so that what scrolls past is seen
    /// through it. The body's own content then needs bottom padding of its own, or
    /// its last row hides behind the bar.
    pub fn extend_body(mut self, extend: bool) -> Self {
        self.extend_body = extend;
        self
    }

    /// Lets the body run **under** the app bar, its top aligned with the top of the
    /// screen rather than with the bottom of the bar.
    ///
    /// For a translucent bar over an image or a hero header. Same warning as
    /// [`Scaffold::extend_body`]: the content behind the bar is the body's business.
    pub fn extend_body_behind_app_bar(mut self, extend: bool) -> Self {
        self.extend_body_behind_app_bar = extend;
        self
    }

    /// Background color, spread edge to edge, including under the system bars.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// The application bar, pinned at the top.
    pub fn app_bar(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.app_bar = Some(Box::new(widget));
        self
    }

    /// The screen's body: it **fills** the space between the bars.
    ///
    /// It does **not** scroll. A body that may be taller than the room it is given
    /// goes inside a scrolling widget the screen chooses — [`SingleChildScrollView`](crate::SingleChildScrollView)
    /// for a page that occasionally overflows, [`ListView`](crate::ListView) for a long one:
    ///
    /// ```ignore
    /// .body(SingleChildScrollView::new().flex(1.0).child(form))
    /// ```
    ///
    /// Left plain, the body is positioned at the top of that room, so a body that
    /// wants all of it says so — `.flex(1.0)`, or a `Flex` that centres its content.
    ///
    /// **The room includes the system's intrusions when nothing else holds them off.**
    /// With a bottom bar or a footer below it, they keep the body clear of the gesture
    /// bar; with neither, the body reaches the screen's edge and is *told* the intrusion
    /// is there. A body whose content must not sit under it wraps in
    /// [`SafeArea`](crate::SafeArea):
    ///
    /// ```ignore
    /// .body(SafeArea::new(form))
    /// ```
    pub fn body(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.body = Some(Box::new(widget));
        self
    }

    /// Enables navigation: `selected` = the active destination, `on_select(i)`
    /// emitted on choice. Then add [`Scaffold::destination`]s.
    ///
    /// The destinations go in a **bottom bar** unless
    /// [`Scaffold::nav_placement`] says otherwise.
    pub fn nav(mut self, selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.selected = selected;
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Where the destinations are drawn: a bottom bar (the default) or a rail.
    ///
    /// Fixed either way — the scaffold will not move the navigation because the
    /// window changed size. For that, use [`NavScaffold`](crate::NavScaffold).
    pub fn nav_placement(mut self, placement: NavPlacement) -> Self {
        self.nav_placement = placement;
        self
    }

    /// **When the destinations say what they are**, whichever of the two widgets ends
    /// up carrying them. See [`RailLabels`].
    ///
    /// Unsaid, each keeps the default the reference gives it: a rail shows no labels, a
    /// bar shows them all (milestone 432).
    pub fn nav_labels(mut self, labels: RailLabels) -> Self {
        self.nav_labels = Some(labels);
        self
    }

    /// **What to do to the rail** once the shell has built it from the destinations —
    /// `.rail(|rail| rail.extended(true).group_alignment(0.0))`.
    ///
    /// The shell builds the navigation itself, which is what makes it a shell: an
    /// application says `.destination("✔", "Tasks")` and never sees the widget. That
    /// left everything a rail can do and a bar cannot — its extended form, where its
    /// destinations sit, the slots above and below them — reachable only by building the
    /// rail by hand and giving up the shell. This is the door: the shell builds the rail,
    /// hands it over, and takes back whatever comes out.
    ///
    /// It runs **last**, after the destinations and after [`Self::nav_labels`], so it has
    /// the final word on both. Silent when the navigation is a bottom bar, which has none
    /// of these properties to set.
    ///
    /// The shell then measures the rail it was handed rather than assuming the width it
    /// started with — an extended rail is 256 wide, and everything the shell puts beside
    /// it has to know.
    pub fn rail(
        mut self,
        configure: impl FnOnce(NavigationRail<Msg>) -> NavigationRail<Msg> + 'static,
    ) -> Self {
        self.rail = Some(Box::new(configure));
        self
    }

    /// Adds a navigation destination (glyph + label).
    pub fn destination(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.destinations.push(Destination::new(icon, label));
        self
    }

    /// A notification count on the **last** destination.
    pub fn badge(self, count: u32) -> Self {
        self.decorate(|last| last.badge = Some(count))
    }

    /// The glyph the **last** destination shows while it is selected, where that differs
    /// from its resting one. See [`NavigationRail::selected_icon`].
    pub fn selected_icon(self, icon: impl Into<String>) -> Self {
        let icon = icon.into();
        self.decorate(move |last| last.selected_icon = Some(icon))
    }

    /// Marks the **last** destination inaccessible. See [`NavigationRail::disabled`].
    pub fn disabled(self) -> Self {
        self.decorate(|last| last.disabled = true)
    }

    /// The **last** destination's own indicator colour, over the theme's. See
    /// [`NavigationRail::indicator_color`].
    pub fn indicator_color(self, color: Color) -> Self {
        self.decorate(move |last| last.indicator_color = Some(color))
    }

    /// Applies `f` to the destination just added. Silent when there is none.
    fn decorate(mut self, f: impl FnOnce(&mut Destination)) -> Self {
        if let Some(last) = self.destinations.last_mut() {
            f(last);
        }
        self
    }

    /// A modal side drawer on the **leading** edge (left): `panel` = the content,
    /// `open` = expanded, `toggle` = the toggle message (the button, and a click on
    /// the scrim).
    ///
    /// The usual home of the navigation on a phone. A screen may have this one and
    /// [`Scaffold::end_drawer`] at once — they are different drawers with different
    /// jobs, not a choice of side.
    pub fn drawer(mut self, panel: impl Widget<Msg> + 'static, open: bool, toggle: Msg) -> Self {
        self.drawer = Some((Box::new(panel), open, toggle));
        self
    }

    /// A modal side drawer (right edge): `panel` = the content, `open` = expanded,
    /// `toggle` = the toggle message (the button, and a click on the scrim).
    pub fn end_drawer(
        mut self,
        panel: impl Widget<Msg> + 'static,
        open: bool,
        toggle: Msg,
    ) -> Self {
        self.end_drawer = Some((Box::new(panel), open, toggle));
        self
    }

    /// A floating action button, anchored bottom-right, above the bottom bar.
    ///
    /// It rides in a full-screen `Stack` layer over the shell. That layer does not
    /// swallow what is under it: only a widget that asks for clicks is registered as
    /// a target, so the transparent remainder of the layer is not one. (This carried
    /// an "experimental — intercepts the bottom half of the screen" warning for a
    /// long time; a test now says otherwise.)
    pub fn fab(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.fab = Some(Box::new(widget));
        self
    }

    /// A **bottom app bar** in place of the navigation one: the actions belonging to
    /// the screen you are on, rather than a choice of screen.
    ///
    /// Taken by its own type, not as an opaque widget, so that the scaffold can cut
    /// the bar's notch once it knows where it is putting the FAB. A docked button and
    /// a bar that does not know about it would sit on top of each other.
    ///
    /// A scaffold given both this and [`Scaffold::destination`]s keeps the navigation
    /// — being able to leave the screen outranks the actions on it.
    pub fn bottom_app_bar(mut self, bar: BottomAppBar<Msg>) -> Self {
        self.bottom_app_bar = Some(bar);
        self
    }

    /// Where the FAB sits: which end of the row, and whether it floats clear of the
    /// bottom bar or docks astride its top edge. [`FabLocation::EndFloat`] by default.
    pub fn fab_location(mut self, location: FabLocation) -> Self {
        self.fab_location = location;
        self
    }

    /// The height the FAB is taken to have when it is **docked**, since that
    /// placement puts its centre on the bar's edge and so has to know how tall it is.
    /// 56 px — the conventional diameter — unless said otherwise.
    ///
    /// A declared number rather than a measured one, and that is a divergence worth
    /// naming: the scaffold is handed the button as an opaque widget it cannot
    /// measure. A button of another size docks correctly once it says so.
    pub fn fab_size(mut self, size: f32) -> Self {
        self.fab_size = size;
        self
    }

    /// A strip pinned **between the body and the bottom bar**, always visible: the
    /// screen's committing actions (Save, Cancel), which must not be scrolled away.
    ///
    /// Unlike the FAB it is not an overlay — it takes its own height out of the body,
    /// which is the point: it is part of the screen, not floating over it.
    pub fn persistent_footer(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.persistent_footer = Some(Box::new(widget));
        self
    }

    /// How the persistent footer is aligned in its row ([`Justify::End`] by default,
    /// the conventional place for a confirming action).
    pub fn persistent_footer_alignment(mut self, alignment: Justify) -> Self {
        self.persistent_footer_alignment = alignment;
        self
    }

    /// A modal sheet sliding up from the bottom.
    pub fn bottom_sheet(
        mut self,
        panel: impl Widget<Msg> + 'static,
        open: bool,
        toggle: Msg,
    ) -> Self {
        self.bottom_sheet = Some((Box::new(panel), open, toggle));
        self
    }

    /// Whether the shell sits at the **top of the screen**, so that its app bar is the
    /// thing that absorbs the status bar. `true` by default.
    ///
    /// `false` says something else is above it — a shell nested in a page, a second one
    /// beside the first — and the app bar then takes only its own height, with the top
    /// intrusion left to whatever is actually up there.
    ///
    /// **This is both halves of the reference's switch.** There, `Scaffold.primary` makes
    /// the slot tall enough and `AppBar.primary` is what actually pads, the bar wrapping
    /// itself in a `SafeArea` — a split that works because its slots are built lazily
    /// under a description the shell controls. Ours are handed a widget that is already
    /// built, so the shell insets the slot from outside and there is one switch, not two.
    /// Milestone 394 records what a builder-based slot would change about that.
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    /// The line along the **top of the persistent footer**. Drawn by default, as the
    /// reference draws it — its footer is a container decorated with a one-pixel top
    /// border unless the caller replaces the decoration.
    ///
    /// The line is a [`Divider`](crate::Divider), so its colour and its thickness follow
    /// the theme like every other one, and a caller who wants neither says `false`.
    pub fn persistent_footer_divider(mut self, divider: bool) -> Self {
        self.persistent_footer_divider = divider;
        self
    }

    /// The background behind the persistent footer. Unset, it is the shell's own — the
    /// footer is part of the screen, not a bar laid on it.
    pub fn persistent_footer_color(mut self, color: Color) -> Self {
        self.persistent_footer_color = Some(color);
        self
    }

    /// The scrim behind an open drawer, **alpha included**: the transparency of a scrim
    /// *is* its colour, so [`Color::TRANSPARENT`] darkens nothing and an opaque value
    /// hides the screen behind it.
    ///
    /// Both drawers take it, as the reference's single `drawerScrimColor` does.
    pub fn drawer_scrim_color(mut self, color: Color) -> Self {
        self.drawer_scrim_color = Some(color);
        self
    }

    /// Whether a click on the scrim closes the drawer. `true` by default, and the
    /// reference's `drawerBarrierDismissible` is the same switch.
    ///
    /// `false` is for a drawer holding something that must be answered — the way out is
    /// then a control inside the panel, and the screen behind stays unreachable.
    pub fn drawer_barrier_dismissible(mut self, dismissible: bool) -> Self {
        self.drawer_barrier_dismissible = dismissible;
        self
    }

    /// Assembles the shell into a widget ready to display.
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        let Scaffold {
            width,
            height,
            insets,
            background,
            app_bar,
            body,
            selected,
            on_select,
            destinations,
            nav_placement,
            nav_labels,
            rail: configure_rail,
            drawer,
            end_drawer,
            bottom_app_bar,
            fab,
            fab_location,
            fab_size,
            persistent_footer,
            persistent_footer_alignment,
            persistent_footer_divider,
            persistent_footer_color,
            bottom_sheet,
            primary,
            drawer_scrim_color,
            drawer_barrier_dismissible,
            view_insets,
            view_padding,
            resize_to_avoid_bottom_inset,
            extend_body,
            extend_body_behind_app_bar,
        } = self;

        // Where the navigation goes is what the caller asked for, and nothing else.
        // This used to be `SizeClass::from_width(width)`, which is how a phone
        // turned to landscape moved its own navigation (milestone 305).
        let rail_nav = nav_placement == NavPlacement::Rail;
        let bg = background.unwrap_or(Color::TRANSPARENT);
        let has_nav = !destinations.is_empty();
        let body_widget = body.unwrap_or_else(|| Box::new(Container::new()));

        // Navigation: a bottom bar, or a side rail if one was asked for.
        //
        // **How wide the rail came out**, which is not a constant since milestone 433: a
        // caller can extend it, and everything the shell puts beside it — the persistent
        // footer's row, for one — is laid out against this number.
        let mut rail_width = 0.0;
        let nav: Option<Box<dyn Widget<Msg>>> = if has_nav {
            let on_select =
                on_select.expect("nav(selected, on_select) is required with destinations");
            if !rail_nav {
                let mut bar = BottomBar::new(selected, on_select);
                for destination in &destinations {
                    bar = bar.destination(destination.clone());
                }
                if let Some(labels) = nav_labels {
                    bar = bar.labels(labels);
                }
                Some(Box::new(bar))
            } else {
                let mut rail = NavigationRail::new(selected, on_select);
                for destination in &destinations {
                    rail = rail.destination(destination.clone());
                }
                if let Some(labels) = nav_labels {
                    rail = rail.labels(labels);
                }
                // The caller's word is the last one, and the measurement is taken after
                // it rather than before.
                if let Some(configure) = configure_rail {
                    rail = configure(rail);
                }
                rail_width = rail.declared_width();
                Some(Box::new(rail))
            }
        } else {
            None
        };

        // Where the button's centre falls, in the window's own coordinates. Needed
        // before the bottom bar is built, because a notched bar is cut around it.
        let fab_centre_x = match fab_location.justify() {
            Justify::Start => insets.left + FAB_MARGIN + fab_size / 2.0,
            Justify::Center => width / 2.0,
            _ => width - insets.right - FAB_MARGIN - fab_size / 2.0,
        };

        // A bottom app bar takes the bottom slot when there is no navigation in it —
        // being able to leave the screen outranks the actions on it. Its notch is cut
        // here, since the scaffold is the only party that knows both positions.
        let mut bottom_bar_height = 0.0;
        let nav: Option<Box<dyn Widget<Msg>>> = match (nav, bottom_app_bar) {
            (Some(nav), _) => Some(nav),
            (None, Some(bar)) => {
                bottom_bar_height = bar.declared_height();
                let bar = if fab.is_some() && fab_location.docked() {
                    // In the bar's own coordinates, which are the window's: since
                    // milestone 418 the bottom slot spans the full width and consumes
                    // the side intrusions itself, so its origin is no longer held off
                    // by `insets.left` and the notch is no longer moved back by it.
                    bar.notched_at(fab_centre_x, fab_size / 2.0)
                } else {
                    bar
                };
                Some(Box::new(bar))
            }
            (None, None) => None,
        };
        let has_nav = has_nav || bottom_bar_height > 0.0;

        // How far the bottom-most slot is held off the edge. The keyboard is the only
        // inset a screen may decline: `view_insets.bottom` measures the occlusion from
        // the window edge, bar included, so the two combine with `max` and never add up.
        // Declining, the screen has said the keyboard is an **overlay** — so the
        // geometry it wants is the keyboard-free one. `insets.bottom` is zero while the
        // keyboard covers the navigation bar (the bar being covered, nothing has to
        // avoid it), and reading it here would drop the bottom bar and the floating
        // button onto the window's edge the moment a field was tapped, then lift them
        // back when the keyboard closed. `view_padding` is the intrusion that does not
        // move, which is exactly what "ignore the keyboard" means.
        //
        // It is worked out here, above the footer, because the footer is one of the
        // slots it falls to (milestone 419).
        let bottom_clear = if resize_to_avoid_bottom_inset {
            insets.bottom.max(view_insets.bottom)
        } else {
            view_padding.bottom
        };
        // Whether something below the footer already holds the edge off. A rail is
        // beside the body, not beneath it, so it holds nothing off.
        let bar_below_body = has_nav && !rail_nav;
        // How far the bottom bar reaches up, which the body needs when it is told to run
        // under it and the floating button needs to sit on its edge.
        let nav_h = if bottom_bar_height > 0.0 {
            bottom_bar_height
        } else if !rail_nav && has_nav {
            BAR_HEIGHT
        } else {
            0.0
        };

        // The persistent footer: its own row, aligned as asked, kept clear of the side
        // insets. It sits between the body and the bottom bar and never scrolls.
        let footer: Option<Box<dyn Widget<Msg>>> = persistent_footer.map(|widget| {
            // The row is **given** the width it has to fill. A row that hugged its
            // content would leave the alignment nothing to distribute, and every
            // footer would sit at the leading edge whatever it was asked for.
            let row_width =
                (width - insets.left - insets.right - rail_width - FOOTER_PAD * 2.0).max(0.0);
            let row = Flex::row()
                .width(row_width)
                .justify(persistent_footer_alignment)
                .child(widget);
            // **The decoration outside, the safe area inside.** The reference's footer
            // is a decorated container — background, and a one-pixel top border unless
            // the caller replaces it — wrapping a `SafeArea(top: false)`
            // (`scaffold.dart:3133`). So the line and the background run the full width
            // of the screen and the *content* is what keeps clear of the intrusions; a
            // border inset by the notch would be a rule that stops short of the edge it
            // is ruling off.
            //
            // It is a **real safe area** since milestone 419, not a padding worked out
            // here, and that is what fixed the bottom: this used to pass zero, so a
            // footer with nothing under it left its buttons sitting on the gesture bar.
            // The shell holds the clearance for whatever is bottom-most, and with a
            // footer there, that is the footer.
            let mut stack = Flex::column();
            if persistent_footer_divider {
                stack = stack.child(crate::Divider::new());
            }
            stack = stack.child(
                crate::SafeArea::new(Container::new().padding(FOOTER_PAD).child(row))
                    .edges(crate::Edges::ALL.without_top()),
            );
            let mut decorated = Container::new().child(stack);
            if let Some(color) = persistent_footer_color {
                decorated = decorated.color(color);
            }
            // The description the slot is handed: the top removed, the sides as the
            // shell resolved them, and the bottom only when nothing below it holds the
            // edge off — which is what the reference's `removeBottomPadding:
            // bottomNavigationBar != null` says (`scaffold.dart:3158`).
            // Beside a rail the leading intrusion is the rail's — it is inside the
            // rail's own box, and the footer sits in the column to its right (milestone
            // 420). `row_width` above already reads it that way: subtracting the rail's
            // bare width *and* `insets.left` is the same number as subtracting the rail's
            // box, which is the two added together since the rail took the intrusion.
            let left = if rail_nav { 0.0 } else { insets.left };
            let right = insets.right;
            let bottom = if bar_below_body { 0.0 } else { bottom_clear };
            Box::new(crate::MediaScope::tweak(
                move |mq: &mut crate::MediaQuery| {
                    mq.padding.top = 0.0;
                    mq.padding.left = left;
                    mq.padding.right = right;
                    mq.padding.bottom = bottom;
                },
                decorated,
            )) as Box<dyn Widget<Msg>>
        });

        // **The body is told, not padded** (milestone 421), the last of the four slots.
        // The shell used to apply the side intrusions to its content and tell it nothing,
        // so a `SafeArea` inside a body read the **ambient** description and padded for
        // intrusions that had already been dealt with — the status bar twice over under an
        // app bar, and the sides twice over anywhere.
        //
        // The reference lays its body out full width and hands the slot a description
        // (`scaffold.dart:3019`): the sides kept, the top removed when there is an app bar,
        // the bottom removed when something below it holds the edge off, and the keyboard
        // removed when the layout has already shortened the body for it.
        //
        // Which is also to say: **a body that wants the notch avoided says so.** The slot
        // is now told the truth about its edges, and `SafeArea` is what reads it.
        //
        // It is still **not** wrapped in a scroller: a pane rather than a viewport —
        // whether this screen scrolls is the screen's to say, and it says it by what it
        // puts here.
        let bar_over_body = extend_body_behind_app_bar && app_bar.is_some();
        // The top is the app bar's when there is one in front of the body. Behind it
        // (`extend_body_behind_app_bar`) the body faces the status bar itself again, which
        // is the `max(padding.top, appBarHeight)` half of the reference's `_BodyBuilder`
        // (`scaffold.dart:973`) — the bar's own height is not in it yet.
        let body_top = if app_bar.is_some() && !bar_over_body {
            0.0
        } else {
            insets.top
        };
        // And the bottom, which changed hands at milestone 423. The shell shortens the
        // body for the **keyboard** and for the widgets below it, **never for the gesture
        // bar**: the reference's `minInsets.bottom` is `resize ? viewInsets.bottom : 0`
        // (`scaffold.dart:3220`), so a body with nothing under it reaches the screen's edge
        // and is **told** what is there. With a bar or a footer under it they hold the edge
        // off and the body is told nothing, which is the reference's `removeBottomPadding:
        // bottomNavigationBar != null || persistentFooterButtons != null`.
        //
        // Told to run **under** them, it faces the further of two things: the intrusion, or
        // how far the slots it runs under reach. That is `_BodyBuilder`'s
        // `max(padding.bottom, bottomWidgetsHeight)` (`scaffold.dart:969`) — with the
        // footer's own height still missing from the second term, since nothing measures it.
        let below_body = bar_below_body || footer.is_some();
        let body_bottom = if extend_body {
            insets.bottom.max(nav_h)
        } else if below_body {
            0.0
        } else {
            insets.bottom
        };
        let (body_left, body_right) = (insets.left, insets.right);
        let body_keyboard = resize_to_avoid_bottom_inset;
        let body_pane = Flex::column().flex(1.0).child(crate::MediaScope::tweak(
            move |mq: &mut crate::MediaQuery| {
                mq.padding.top = body_top;
                mq.padding.left = body_left;
                mq.padding.right = body_right;
                mq.padding.bottom = body_bottom;
                // `removeBottomInset: _resizeToAvoidBottomInset` — the column has already
                // shortened the body for the keyboard, so there is nothing left to avoid.
                if body_keyboard {
                    mq.view_insets.bottom = 0.0;
                }
            },
            body_widget,
        ));

        // Whether the bottom clearance falls to the body — and **which** clearance.
        //
        // The keyboard, and nothing else (milestone 423). It is taken as a **sibling**
        // rather than as padding inside the body: the room the body is given shrinks, so a
        // scrolling body scrolls within what is left instead of running under the keyboard.
        // The gesture bar is *not* here any more; it reaches the body as a description, and
        // a body that wants to be held clear of it says `SafeArea` — which is what the
        // reference does, `minInsets.bottom` being the keyboard alone.
        //
        // With a bar or a footer below it they hold the edge off, and a body told to run
        // under them has asked for the room.
        let body_owns_bottom = !extend_body && footer.is_none() && !bar_below_body;
        let keyboard_clear = if resize_to_avoid_bottom_inset {
            view_insets.bottom
        } else {
            0.0
        };
        let body_spacer = body_owns_bottom && keyboard_clear > 0.0;

        // Which slots the body must make room for, and which it runs under. A slot that
        // is extended behind moves out of the body's column and into an overlay layer
        // drawn on top of it — the same widget either way, in one place or the other.
        //
        // The bottom slots only move on a **compact** layout, where the bottom bar
        // actually is one. Wide, the navigation is a rail beside the body and the
        // overlay would span the rail as well as the content; a footer there stays in
        // the column, which is what it should do anyway when nothing is under it.
        let bottom_over_body = extend_body && !rail_nav && (footer.is_some() || nav.is_some());
        // **Only a primary shell absorbs the status bar.** One nested in a page, or
        // sitting beside another, takes the bar's own height and leaves the top
        // intrusion to whatever is actually above it. The reference computes the same
        // thing in the same place: `primary ? MediaQuery.paddingOf(context).top : 0.0`.
        // The slot is handed a **description**, not a padding (milestone 417). An
        // `AppBar` consumes the intrusion itself when it is `primary`, exactly as the
        // reference's does — the shell's job is to say what there is to consume.
        //
        // It says it even when the answer is "the same as outside", because the shell's
        // idea of its intrusions is not always the ambient one: `Scaffold::insets` is an
        // explicit override, and a slot told nothing would read past it to the surface and
        // pad by something the shell had already decided against. The left is per call —
        // beside a rail, the rail has taken it.
        let bar_top = if primary { insets.top } else { 0.0 };
        let bar_right = insets.right;
        let app_bar_pad = move |bar: Box<dyn Widget<Msg>>, left: f32| -> Box<dyn Widget<Msg>> {
            Box::new(crate::MediaScope::tweak(
                move |mq: &mut crate::MediaQuery| {
                    mq.padding.top = bar_top;
                    mq.padding.left = left;
                    mq.padding.right = bar_right;
                },
                bar,
            ))
        };
        // **The bottom slot is told, not padded** (milestone 418) — the same split
        // milestone 417 made at the top of the screen. The reference hands this slot a
        // description with the **top** intrusion removed and the bottom one left in
        // (`scaffold.dart:3167`), and the bar consumes what it is told about, inside its
        // own surface: `NavigationBar` wraps its row in a safe area and leaves the
        // `Material` outside it (`navigation_bar.dart:285`), and so does `BottomAppBar`
        // (`bottom_app_bar.dart:230`). That is what makes a bar's background run behind
        // the gesture bar instead of stopping short above it, with a strip of the
        // scaffold showing through — which is what a padding from outside produced.
        //
        // `bottom_clear` is already the number the reference arrives at through
        // `maintainBottomViewPadding: !resizeToAvoidBottomInset`, worked out above.
        let nav_left = insets.left;
        let nav_right = insets.right;
        let nav_pad = move |n: Box<dyn Widget<Msg>>| -> Box<dyn Widget<Msg>> {
            Box::new(crate::MediaScope::tweak(
                move |mq: &mut crate::MediaQuery| {
                    mq.padding.top = 0.0;
                    mq.padding.left = nav_left;
                    mq.padding.right = nav_right;
                    mq.padding.bottom = bottom_clear;
                },
                n,
            ))
        };

        // The pinned shell: app bar · body · footer · (bottom bar | rail).
        let mut app_bar = app_bar;
        let mut footer = footer;
        let mut nav = nav;
        let main: Box<dyn Widget<Msg>> = if !rail_nav {
            let mut col = Flex::column().width(width).height(height);
            if !bar_over_body {
                if let Some(bar) = app_bar.take() {
                    col = col.child(app_bar_pad(bar, insets.left));
                }
            }
            col = col.child(body_pane);
            if body_spacer {
                col = col.child(Container::new().height(keyboard_clear));
            }
            if !bottom_over_body {
                if let Some(f) = footer.take() {
                    col = col.child(f);
                }
                if let Some(n) = nav.take() {
                    col = col.child(nav_pad(n));
                }
            }
            Box::new(col)
        } else {
            let mut row = Flex::row().width(width).height(height);
            if let Some(n) = nav.take() {
                // The rail is a sibling, never an overlay: `extend_body` speaks of the
                // bottom bar, and a body sliding under a side rail is nobody's design.
                //
                // Told, not padded (milestone 420): the trailing side is the body's, so
                // it is removed; the leading side, the top and the bottom are the rail's
                // to consume, which is the set the reference's own safe area takes
                // (`navigation_rail.dart:556`). The rule down the rail's edge then runs
                // the full height of the screen instead of stopping at the notch.
                let (rail_top, rail_left) = (insets.top, insets.left);
                row = row.child(crate::MediaScope::tweak(
                    move |mq: &mut crate::MediaQuery| {
                        mq.padding.top = rail_top;
                        mq.padding.right = 0.0;
                        mq.padding.left = rail_left;
                        mq.padding.bottom = bottom_clear;
                    },
                    n,
                ));
            }
            let mut content = Flex::column().flex(1.0);
            if !bar_over_body {
                if let Some(bar) = app_bar.take() {
                    content = content.child(app_bar_pad(bar, 0.0));
                }
            }
            content = content.child(body_pane);
            if body_spacer {
                content = content.child(Container::new().height(keyboard_clear));
            }
            if !bottom_over_body {
                if let Some(f) = footer.take() {
                    content = content.child(f);
                }
            }
            row = row.child(content);
            Box::new(row)
        };

        // Whatever the body was told to run under is drawn over it, in a layer of its
        // own: the bar at the top, the footer and the bar at the bottom, a spring
        // between. Nothing here needs a measurement — the spring takes what is left.
        let main: Box<dyn Widget<Msg>> = if app_bar.is_some() || footer.is_some() || nav.is_some() {
            let mut over = Flex::column().width(width).height(height);
            if let Some(bar) = app_bar {
                over = over.child(app_bar_pad(bar, insets.left));
            }
            over = over.child(Container::new().flex(1.0));
            if let Some(f) = footer {
                over = over.child(f);
            }
            if let Some(n) = nav {
                over = over.child(nav_pad(n));
            }
            Box::new(
                Stack::new()
                    .width(width)
                    .height(height)
                    .layer(main)
                    .layer(over),
            )
        } else {
            main
        };

        // The FAB, in a layer of its own over the shell, at the corner it was given.
        let mut content: Box<dyn Widget<Msg>> = main;
        if let Some(fab) = fab {
            // Where the body stops: the top of the bottom bar, which is what both
            // vertical placements are measured from.
            let content_bottom = bottom_clear + nav_h;
            let fab_bottom = if fab_location.docked() {
                // Docked: the button's **centre** on that edge, straddling the bar.
                (content_bottom - fab_size / 2.0).max(0.0)
            } else {
                // Floating: clear of the edge by the usual margin.
                content_bottom + FAB_MARGIN
            };
            let (left_pad, right_pad) = match fab_location.justify() {
                Justify::Start => (insets.left + FAB_MARGIN, 0.0),
                Justify::End => (0.0, insets.right + FAB_MARGIN),
                _ => (0.0, 0.0),
            };
            let fab_layer = Flex::column()
                .width(width)
                .height(height)
                .justify(Justify::End)
                .child(
                    Flex::row().justify(fab_location.justify()).child(
                        Container::new()
                            .padding_each(0.0, right_pad, fab_bottom, left_pad)
                            .child(fab),
                    ),
                );
            content = Box::new(
                Stack::new()
                    .width(width)
                    .height(height)
                    .layer(content)
                    .layer(fab_layer),
            );
        }

        // What this shell knows, for the slots that cannot see it (milestone 422). Taken
        // before the drawers are consumed below, since the message is what a bar needs.
        let mut info = crate::ScaffoldInfo::default();
        if let Some((_, _, toggle)) = &drawer {
            info = info.with_drawer(toggle.clone());
        }
        if let Some((_, _, toggle)) = &end_drawer {
            info = info.with_end_drawer(toggle.clone());
        }

        // The modal drawers, then the modal sheet, wrap the shell as overlays. The
        // leading drawer goes on first so that the trailing one, wrapping it, is the
        // outer layer — with both open the end drawer is the one on top, which is the
        // one the user opened last.
        let dress = |mut side: crate::Drawer<Msg>, toggle: Msg| {
            if drawer_barrier_dismissible {
                side = side.on_dismiss(toggle);
            }
            if let Some(color) = drawer_scrim_color {
                side = side.scrim_color(color);
            }
            side
        };
        if let Some((panel, open, toggle)) = drawer {
            content = Box::new(
                dress(crate::Drawer::new(open), toggle)
                    .panel(panel)
                    .body(content),
            );
        }
        if let Some((panel, open, toggle)) = end_drawer {
            content = Box::new(
                dress(crate::Drawer::new(open).right(), toggle)
                    .panel(panel)
                    .body(content),
            );
        }
        if let Some((panel, open, toggle)) = bottom_sheet {
            content = Box::new(
                crate::BottomSheet::new(open)
                    .on_dismiss(toggle)
                    .sheet(panel)
                    .body(content),
            );
        }

        // A full-window background (edge to edge) giving the slots a definite size.
        let shell: Box<dyn Widget<Msg>> = Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(bg)
                .child(content),
        );
        // **And the shell says what it is** (milestone 422). Its slots were handed to it
        // already built and cannot see the screen they stand on; this is how a bar with no
        // leading of its own learns that there is a drawer, and what opens it. It wraps the
        // whole shell rather than the app-bar slot alone, because the reference's
        // `Scaffold.of` is readable from anywhere below the scaffold and not only from the
        // bar (`scaffold.dart:3232`).
        if info.has_drawer() || info.has_end_drawer() {
            Box::new(crate::ScaffoldScope::new(info, shell))
        } else {
            shell
        }
    }
}

/// A conventional floating action button (round, accent), to be passed to
/// [`Scaffold::fab`]. Sugar for `button(label, msg)` styled as primary.
pub fn fab_button<Msg: Clone + 'static>(
    label: impl Into<String>,
    message: Msg,
) -> crate::Button<Msg> {
    crate::Button::new(label)
        .variant(Variant::Filled)
        .size(24.0)
        // **Round**, and not only because the convention is round: a docked button
        // sits in a circular notch, and a square one would leave the bar curving
        // around a shape that is not there.
        .radius(FAB_SIZE / 2.0)
        .on_press(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    // Named by the tests alone since milestone 434: the shell asks the rail how wide it
    // came out rather than reading the constant, because a rail can be extended.
    use crate::navrail::{EXTENDED_RAIL_WIDTH, RAIL_WIDTH};
    use crate::{build_ui, dsl::button, dsl::text, Runtime, Size, Theme};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
        Drawer,
        Add,
    }

    /// **The shell takes its size and its intrusions from the surface, unasked.**
    ///
    /// Milestone 393: nothing passes a `Scaffold` the screen. It reads the description
    /// the framework installed, which is where the size, the notch and the bars all
    /// already are. The same shell built under two different surfaces comes out at two
    /// different sizes, and neither call said a number.
    #[test]
    fn the_shell_takes_its_size_and_its_intrusions_from_the_surface() {
        let bars = WindowInsets::bars(Insets::new(40.0, 0.0, 30.0, 0.0));
        let build = |size: Size| {
            let surface = MediaQuery::new(size).with_insets(bars);
            let tree = surface.scope(|| {
                Scaffold::<Msg>::new()
                    // A real bar, because since milestone 417 the shell hands this slot a
                    // *description* rather than a padding: an `AppBar` consumes the status
                    // bar itself, as the reference's does, and a bare box in this slot is a
                    // bare box that does not.
                    .app_bar(crate::AppBar::<Msg>::new("bar").height(56.0).build())
                    .body(Container::new().flex(1.0).child(text("body")))
                    .build()
            });
            build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default())
        };
        // The **app bar**, which is the slot the intrusion is the shell's to handle.
        // A `Scaffold` does not pad its body for the status bar and neither does the
        // reference's — `contentTop` there is the app bar's height, zero without one,
        // and a body that wants the notch avoided says `SafeArea` itself.
        let top_of_body = |ui: &crate::Ui<Msg>| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    crate::Primitive::Text { position, .. } => Some(position.y),
                    _ => None,
                })
                .expect("the body is drawn")
        };
        let phone = build(Size::new(400.0, 800.0));
        let tablet = build(Size::new(1000.0, 700.0));
        // Kept clear of the status bar on both, without being told there was one.
        assert!(top_of_body(&phone) >= 40.0, "the phone runs under the bar");
        assert!(
            top_of_body(&tablet) >= 40.0,
            "the tablet runs under the bar"
        );
        // And the shell really did read two different surfaces.
        let widest = |ui: &crate::Ui<Msg>| {
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    crate::Primitive::Rect { rect, .. } => Some(rect.width),
                    _ => None,
                })
                .fold(0.0_f32, f32::max)
        };
        assert!(
            widest(&tablet) > widest(&phone) + 500.0,
            "the same code, two surfaces, two widths"
        );
    }

    fn scaffold(width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        Scaffold::new()
            .size(width, height)
            .insets(Insets::new(40.0, 0.0, 30.0, 0.0))
            .background(Color::rgb(0.1, 0.1, 0.1))
            .app_bar(text("Title").size(20.0))
            .body(text("Body").size(16.0))
            .nav(0, Msg::Go)
            .destination("H", "Home")
            .destination("S", "Stats")
            .end_drawer(text("Menu"), false, Msg::Drawer)
            .fab(button("＋", Msg::Add))
            .build()
    }

    #[test]
    fn assembles_at_compact_and_expanded_without_panic() {
        for w in [400.0_f32, 1000.0_f32] {
            let s = scaffold(w, 800.0);
            let ui = build_ui(
                s.as_ref(),
                Size::new(w, 800.0),
                &Runtime::default(),
                &Theme::default(),
            );
            assert!(
                !ui.scene().primitives().is_empty(),
                "empty scene for width={w}"
            );
        }
    }

    /// A colour no theme uses, so the marked slot can be picked out of the scene.
    const MARK: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    const W: f32 = 400.0;
    const H: f32 = 800.0;

    /// A block painted in [`MARK`], to stand for a slot's content.
    fn marked<M: Clone + 'static>(height: f32) -> Container<M> {
        Container::new().height(height).color(MARK)
    }

    /// A body that **fills** the room it is given, which is how that room is measured.
    ///
    /// Before milestone 321 these tests passed an over-tall block and read the *clip*
    /// instead: the body was wrapped in a scroller, so what showed of an oversized block
    /// was the viewport. Now that the scaffold places the body rather than scrolling it,
    /// nothing clips, and an over-tall block would simply paint past the bottom of the
    /// screen and measure nothing. Asking to fill is both the honest measurement and the
    /// documented way for a body to take all of its room.
    fn filling<M: Clone + 'static>() -> Container<M> {
        Container::new().flex(1.0).color(MARK)
    }

    /// Every marked rectangle in the assembled scaffold, top to bottom, **as seen**:
    /// clipped to the box it was drawn in.
    fn marks(scaffold: Box<dyn Widget<Msg>>) -> Vec<frus_core::Rect> {
        let ui = build_ui(
            scaffold.as_ref(),
            Size::new(W, H),
            &Runtime::default(),
            &Theme::default(),
        );
        let mut found: Vec<frus_core::Rect> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect {
                    rect, color, clip, ..
                } if *color == MARK => Some(rect.intersect(*clip)),
                _ => None,
            })
            .collect();
        found.sort_by(|a, b| a.y.total_cmp(&b.y));
        found
    }

    /// The window insets of a phone with a keyboard up: bars top and bottom, and the
    /// keyboard occluding 300 px from the window's edge.
    fn keyboard_up() -> WindowInsets {
        WindowInsets::from_baseline(
            Insets::new(40.0, 0.0, 30.0, 0.0),
            Insets::new(40.0, 0.0, 300.0, 0.0),
        )
    }

    /// The body is lifted clear of the keyboard — and only if it is asked to be.
    #[test]
    fn the_keyboard_shortens_the_body_unless_the_screen_declines() {
        let lifted = marks(
            Scaffold::new()
                .size(W, H)
                .window_insets(keyboard_up())
                .body(filling())
                .build(),
        );
        assert_eq!(lifted.len(), 1);
        assert!(
            (lifted[0].y + lifted[0].height - (H - 300.0)).abs() < 1.0,
            "the body must stop at the keyboard: {:?}",
            lifted[0]
        );

        let covered = marks(
            Scaffold::new()
                .size(W, H)
                .window_insets(keyboard_up())
                .resize_to_avoid_bottom_inset(false)
                .body(filling())
                .build(),
        );
        // Declined: the screen has said the keyboard is an **overlay**, so nothing
        // shortens the body at all and it keeps the whole window — `minInsets.bottom` is
        // zero when `resizeToAvoidBottomInset` is false, and the permanent bottom bar was
        // never in that number (milestone 423). It reaches the body as a description, and
        // a body that wants to be clear of it says `SafeArea`.
        assert!(
            (covered[0].y + covered[0].height - H).abs() < 1.0,
            "the body must run under the keyboard: {:?}",
            covered[0]
        );
    }

    /// Whether the finger finds anything to scroll, asked of the registry the gesture
    /// arena actually consults.
    fn scrollable_under_the_middle(scaffold: Box<dyn Widget<Msg>>) -> bool {
        build_ui(
            scaffold.as_ref(),
            Size::new(W, H),
            &Runtime::default(),
            &Theme::default(),
        )
        .scroll_hit(frus_core::Point::new(W / 2.0, H / 2.0))
        .is_some()
    }

    /// **The body is not a scroller.** A screen with nothing to scroll must offer the
    /// gesture arena nothing to scroll, and a screen that wants to scroll says so.
    ///
    /// This is the test the device asked for. A shell that wrapped every body registered
    /// a scrollable area on every screen, so a page whose content fitted still answered
    /// the finger — and, before milestone 316 taught the arena to decline, still lit an
    /// end-of-list glow on a page that had no end to reach. Fixing the arena stopped the
    /// glow; this stops the phantom scroller that fed it.
    #[test]
    fn a_body_that_does_not_ask_to_scroll_is_not_scrollable() {
        assert!(
            !scrollable_under_the_middle(Scaffold::new().size(W, H).body(filling()).build()),
            "a plain body must not register a scrollable area"
        );
        assert!(
            !scrollable_under_the_middle(Scaffold::new().size(W, H).body(marked(2000.0)).build()),
            "not even one taller than the screen — overflowing is not the same as scrolling"
        );
        assert!(
            scrollable_under_the_middle(
                Scaffold::new()
                    .size(W, H)
                    .body(
                        crate::SingleChildScrollView::new()
                            .flex(1.0)
                            .child(marked::<Msg>(2000.0))
                    )
                    .build()
            ),
            "and a body that asks for a scroller gets one"
        );
    }

    /// **A body alone reaches the screen's edge, and is told what is there** (milestone
    /// 423).
    ///
    /// The reference shortens its body for the keyboard and for the widgets below it,
    /// **never for the gesture bar**: `minInsets.bottom` is `resize ? viewInsets.bottom : 0`
    /// (`scaffold.dart:3220`). So a plain body runs to the edge — which is what a
    /// background, a hero image, or a list that should scroll *under* the gesture bar
    /// wants — and a body that must be held clear of it says `SafeArea`.
    #[test]
    fn a_body_alone_reaches_the_edge_and_is_told_what_is_there() {
        const BOTTOM: f32 = 30.0;
        let size = Size::new(W, H);
        let surface = MediaQuery::new(size)
            .with_insets(WindowInsets::bars(Insets::new(40.0, 0.0, BOTTOM, 0.0)));

        // Nothing below it and nothing asked for: it is the whole screen.
        let bare = marked_under(surface, size, || {
            Scaffold::new().size(W, H).body(filling::<Msg>()).build()
        });
        assert!(
            (bare.y + bare.height - H).abs() < 1.0,
            "the body stopped short of the edge: {bare:?}"
        );

        // And a body that must be clear of the gesture bar says so, and is answered —
        // which it could not be before, the description having said the bottom was
        // already dealt with.
        let asked = marked_under(surface, size, || {
            Scaffold::new()
                .size(W, H)
                .body(crate::SafeArea::new(filling::<Msg>()))
                .build()
        });
        assert!(
            (asked.y + asked.height - (H - BOTTOM)).abs() < 1.0,
            "a body that asked to be held clear of the gesture bar was not: {asked:?}"
        );
    }

    /// `extend_body` is the difference between stopping above the bar and running
    /// under it; the bar is drawn on top either way.
    #[test]
    fn an_extended_body_runs_under_the_bottom_bar() {
        let stops = marks(
            Scaffold::new()
                .size(W, H)
                .body(filling())
                .nav(0, Msg::Go)
                .destination("H", "Home")
                .build(),
        );
        let runs_under = marks(
            Scaffold::new()
                .size(W, H)
                .extend_body(true)
                .body(filling())
                .nav(0, Msg::Go)
                .destination("H", "Home")
                .build(),
        );
        assert!(
            (runs_under[0].height - H).abs() < 1.0,
            "an extended body takes the whole height: {:?}",
            runs_under[0]
        );
        assert!(
            runs_under[0].height > stops[0].height + BAR_HEIGHT - 1.0,
            "it must gain the bar's height: {:?} vs {:?}",
            runs_under[0],
            stops[0]
        );
    }

    /// `extend_body_behind_app_bar` aligns the body with the top of the screen rather
    /// than with the bottom of the bar.
    #[test]
    fn an_extended_body_starts_above_the_app_bar() {
        let below = marks(
            Scaffold::new()
                .size(W, H)
                .insets(Insets::new(40.0, 0.0, 0.0, 0.0))
                .app_bar(crate::AppBar::<Msg>::new("").height(56.0).build())
                .body(marked(200.0))
                .build(),
        );
        assert!(below[0].y >= 96.0, "below the bar: {:?}", below[0]);

        let behind = marks(
            Scaffold::new()
                .size(W, H)
                .insets(Insets::new(40.0, 0.0, 0.0, 0.0))
                .extend_body_behind_app_bar(true)
                .app_bar(crate::AppBar::<Msg>::new("").height(56.0).build())
                .body(marked(200.0))
                .build(),
        );
        assert!(
            behind[0].y < 1.0,
            "behind the bar, from the very top: {:?}",
            behind[0]
        );
    }

    /// The footer is between the body and the bar, and it does not scroll away: it
    /// takes its height out of the body rather than floating over it.
    #[test]
    fn the_persistent_footer_sits_between_the_body_and_the_bar() {
        let rects = marks(
            Scaffold::new()
                .size(W, H)
                .body(filling())
                .persistent_footer(marked::<Msg>(40.0).width(100.0))
                .nav(0, Msg::Go)
                .destination("H", "Home")
                .build(),
        );
        assert_eq!(rects.len(), 2, "body and footer: {rects:?}");
        let (body, footer) = (rects[0], rects[1]);
        assert!(
            footer.y >= body.y + body.height - 1.0,
            "the footer is below the body, not over it: {rects:?}"
        );
        assert!(
            footer.y + footer.height <= H - BAR_HEIGHT + 1.0,
            "and above the bottom bar: {rects:?}"
        );
    }

    /// The footer's alignment is honoured, which means the row it sits in must be
    /// **given** its width — a row that hugged its content would put every footer at
    /// the leading edge whatever it was asked for.
    #[test]
    fn the_persistent_footer_is_aligned_where_it_was_asked_to_be() {
        let at = |alignment| {
            marks(
                Scaffold::new()
                    .size(W, H)
                    .body(Container::<Msg>::new())
                    .persistent_footer_alignment(alignment)
                    .persistent_footer(marked::<Msg>(40.0).width(100.0))
                    .build(),
            )[0]
            .x
        };
        assert!(at(Justify::Start) < 20.0, "start: {}", at(Justify::Start));
        assert!(at(Justify::End) > W - 120.0, "end: {}", at(Justify::End));
        let centre = at(Justify::Center);
        assert!((centre - (W - 100.0) / 2.0).abs() < 2.0, "centre: {centre}");
    }

    /// The FAB goes to the end it was given, and a docked one straddles the bar's
    /// top edge instead of floating clear of it.
    #[test]
    fn the_fab_goes_where_it_was_placed() {
        let at = |location| {
            let scaffold = Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .nav(0, Msg::Go)
                .destination("H", "Home")
                .fab_location(location)
                .fab_size(56.0)
                .fab(marked::<Msg>(56.0).width(56.0))
                .build();
            marks(scaffold)[0]
        };

        // Horizontally: the three ends, in order and distinct.
        let start = at(FabLocation::StartFloat);
        let centre = at(FabLocation::CenterFloat);
        let end = at(FabLocation::EndFloat);
        assert!(
            start.x < centre.x && centre.x < end.x,
            "{start:?} {centre:?} {end:?}"
        );
        assert!(
            (start.x - FAB_MARGIN).abs() < 1.0,
            "leading margin: {start:?}"
        );
        assert!(
            (end.x + end.width - (W - FAB_MARGIN)).abs() < 1.0,
            "trailing margin: {end:?}"
        );
        assert!(
            (centre.x + centre.width / 2.0 - W / 2.0).abs() < 1.0,
            "centred: {centre:?}"
        );

        // Vertically: floating clears the bar; docked has its centre on the bar's edge.
        let bar_top = H - BAR_HEIGHT;
        assert!(
            (end.y + end.height - (bar_top - FAB_MARGIN)).abs() < 1.0,
            "floating clear of the bar: {end:?}"
        );
        let docked = at(FabLocation::EndDocked);
        assert!(
            (docked.y + docked.height / 2.0 - bar_top).abs() < 1.0,
            "centred on the bar's top edge: {docked:?}"
        );
    }

    /// A bottom app bar takes the bottom slot, the docked button lands on its top
    /// edge, and the bar is cut to receive it — the three facts that have to agree.
    #[test]
    fn a_docked_button_is_received_by_the_bar_it_sits_on() {
        use crate::BottomAppBar;
        let bar_height = 64.0;
        let scaffold = Scaffold::new()
            .size(W, H)
            .body(Container::<Msg>::new())
            .bottom_app_bar(BottomAppBar::new().height(bar_height).color(MARK))
            .fab_location(FabLocation::EndDocked)
            .fab_size(56.0)
            .fab(Container::<Msg>::new().width(56.0).height(56.0))
            .build();
        let ui = build_ui(
            scaffold.as_ref(),
            Size::new(W, H),
            &Runtime::default(),
            &Theme::default(),
        );
        // The bar paints itself as a **path**, not a rectangle: that is the notch.
        let outline = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Path { path, fill, .. } if *fill == Some(MARK) => {
                    Some(path.clone())
                }
                _ => None,
            })
            .expect("a notched bar paints an outline");

        let bar_top = H - bar_height;
        let fab_centre_x = W - FAB_MARGIN - 28.0;
        // No node of the outline intrudes into the button's circle.
        for verb in outline.verbs() {
            let p = match verb {
                frus_core::PathVerb::MoveTo(p) | frus_core::PathVerb::LineTo(p) => *p,
                frus_core::PathVerb::QuadTo { to, .. }
                | frus_core::PathVerb::CubicTo { to, .. } => *to,
                frus_core::PathVerb::Close => continue,
            };
            let d = ((p.x - fab_centre_x).powi(2) + (p.y - bar_top).powi(2)).sqrt();
            assert!(d >= 27.9, "the bar cuts into the button: {p:?} ({d})");
        }
    }

    /// The first marked rectangle of a scaffold built under `surface`.
    fn marked_under(
        surface: MediaQuery,
        size: Size,
        build: impl FnOnce() -> Box<dyn Widget<Msg>>,
    ) -> frus_core::Rect {
        let ui = surface.scope(|| {
            let tree = build();
            build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default())
        });
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. } if *color == MARK => Some(*rect),
                _ => None,
            })
            .expect("the body is drawn")
    }

    /// Every message a click anywhere in `tree` would emit, the deferred subtrees built
    /// first — an app bar is composed there, which is the whole point here.
    fn click_messages(tree: &dyn Widget<Msg>) -> Vec<Msg> {
        crate::build_deferred(tree, &Theme::default());
        fn walk(widget: &dyn Widget<Msg>, out: &mut Vec<Msg>) {
            if let Some(message) = widget.on_click() {
                out.push(message);
            }
            for child in widget.children() {
                walk(child.as_ref(), out);
            }
        }
        let mut found = Vec::new();
        walk(tree, &mut found);
        found
    }

    /// **What the shell knows and the bar does not** (milestone 422).
    ///
    /// An `AppBar` is handed to its `Scaffold` already built, so it cannot see the screen
    /// it is about to stand on. The reference's reads it from the context: a bar with no
    /// `leading` of its own, on a screen that has a drawer, grows the button that opens it
    /// (`app_bar.dart:1010`), and one with nothing at its trailing end grows the button for
    /// an end drawer (`app_bar.dart:1113`).
    ///
    /// Counted rather than searched for, because a closed drawer has a dismiss target of
    /// its own carrying the same message: what this asserts is **one more** click that
    /// opens the drawer, which is the button.
    #[test]
    fn a_bar_grows_the_buttons_for_drawers_it_could_not_see() {
        let bar = |leading: bool, actions: bool| {
            let tree = Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .drawer(text("Menu"), false, Msg::Drawer)
                .end_drawer(text("Filters"), false, Msg::Add)
                .app_bar(
                    crate::AppBar::<Msg>::new("Title")
                        .automatically_imply_leading(leading)
                        .automatically_imply_actions(actions)
                        .build(),
                )
                .build();
            click_messages(tree.as_ref())
        };
        let count = |found: &[Msg], wanted: &Msg| found.iter().filter(|m| *m == wanted).count();

        let implied = bar(true, true);
        let asked_not_to = bar(false, false);
        assert_eq!(
            count(&implied, &Msg::Drawer),
            count(&asked_not_to, &Msg::Drawer) + 1,
            "the bar did not grow the button for the drawer it stands over"
        );
        assert_eq!(
            count(&implied, &Msg::Add),
            count(&asked_not_to, &Msg::Add) + 1,
            "the bar did not grow the button for the end drawer"
        );
    }

    /// And it implies nothing where there is nothing to imply, or where the caller has
    /// already filled the slot. The implied button **fills an empty end**; it is never
    /// added beside what the bar was given.
    #[test]
    fn a_bar_implies_a_button_only_into_an_empty_slot() {
        let without_a_drawer = click_messages(
            Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .app_bar(crate::AppBar::<Msg>::new("Title").build())
                .build()
                .as_ref(),
        );
        assert!(
            !without_a_drawer.contains(&Msg::Drawer),
            "a bar on a screen with no drawer grew a button for one"
        );

        // A leading of its own, and an action of its own: neither slot is empty, so
        // neither is filled. The drawer's own dismiss target is the only `Drawer` left,
        // and the caller's own action is the only `Go(1)`.
        let filled = click_messages(
            Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .drawer(text("Menu"), false, Msg::Drawer)
                .end_drawer(text("Filters"), false, Msg::Add)
                .app_bar(
                    crate::AppBar::<Msg>::new("Title")
                        .leading(button("Back", Msg::Go(1)))
                        .action("Save", Msg::Go(2))
                        .build(),
                )
                .build()
                .as_ref(),
        );
        let bare = click_messages(
            Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .drawer(text("Menu"), false, Msg::Drawer)
                .end_drawer(text("Filters"), false, Msg::Add)
                .body(Container::<Msg>::new())
                .build()
                .as_ref(),
        );
        let count = |found: &[Msg], wanted: &Msg| found.iter().filter(|m| *m == wanted).count();
        assert_eq!(
            count(&filled, &Msg::Drawer),
            count(&bare, &Msg::Drawer),
            "a bar with its own leading grew a menu button beside it"
        );
        assert_eq!(
            count(&filled, &Msg::Add),
            count(&bare, &Msg::Add),
            "a bar with its own action grew an end-drawer button beside it"
        );
    }

    /// **A `SafeArea` in the body padded for a status bar the app bar had already taken**
    /// (milestone 421).
    ///
    /// The shell told the body nothing, so a safe area inside it read the **ambient**
    /// description — the shell's whole notch — and held its content off by it a second
    /// time, below a bar that was already clear of it. The reference removes the top from
    /// that slot's description whenever there is an app bar (`scaffold.dart:3030`).
    #[test]
    fn a_safe_area_in_the_body_is_told_what_the_bar_already_took() {
        const TOP: f32 = 40.0;
        const BAR: f32 = 56.0;
        let size = Size::new(W, H);
        let surface =
            MediaQuery::new(size).with_insets(WindowInsets::bars(Insets::new(TOP, 0.0, 0.0, 0.0)));
        let top_of_content = |with_bar: bool| {
            marked_under(surface, size, || {
                let mut scaffold = Scaffold::new().size(W, H);
                if with_bar {
                    scaffold =
                        scaffold.app_bar(crate::AppBar::<Msg>::new("bar").height(BAR).build());
                }
                scaffold
                    .body(crate::SafeArea::new(marked::<Msg>(20.0).width(20.0)))
                    .build()
            })
            .y
        };
        // No bar: the body faces the status bar, and its safe area is what holds it off.
        assert!(
            (top_of_content(false) - TOP).abs() < 0.5,
            "a body alone must clear the status bar itself: {}",
            top_of_content(false)
        );
        // A bar: it stands `BAR + TOP` tall, having taken the notch (milestone 417), and
        // the body's safe area is told there is nothing left to take. Padding twice put
        // the content a whole notch lower.
        assert!(
            (top_of_content(true) - (BAR + TOP)).abs() < 0.5,
            "the body padded for a notch the bar had already taken: {}",
            top_of_content(true)
        );
    }

    /// **The body is told about the cutout beside it, not padded for it** (milestone 421).
    ///
    /// The reference lays its body out full width and keeps the side intrusions in the
    /// description it hands the slot (`scaffold.dart:3029`). So a body that says nothing
    /// reaches the screen's edge — which is what a background, a hero image or a list's
    /// own scrollbar wants — and a body that says `SafeArea` is held clear of it.
    #[test]
    fn a_body_is_told_about_the_cutout_beside_it_rather_than_padded_for_it() {
        const CUTOUT: f32 = 48.0;
        let size = Size::new(W, H);
        let surface = MediaQuery::new(size)
            .with_insets(WindowInsets::bars(Insets::new(0.0, 0.0, 0.0, CUTOUT)));
        let bare = marked_under(surface, size, || {
            Scaffold::new()
                .size(W, H)
                .body(marked::<Msg>(20.0).width(20.0))
                .build()
        });
        assert!(
            bare.x.abs() < 0.5,
            "the body was padded for the cutout instead of told about it: {bare:?}"
        );
        let asked = marked_under(surface, size, || {
            Scaffold::new()
                .size(W, H)
                .body(crate::SafeArea::new(marked::<Msg>(20.0).width(20.0)))
                .build()
        });
        assert!(
            (asked.x - CUTOUT).abs() < 0.5,
            "the body was not told about the cutout: {asked:?}"
        );
    }

    /// **A footer alone held nothing off the bottom edge** (milestone 419).
    ///
    /// The shell leaves the bottom clearance to whatever is bottom-most, and with a
    /// footer there it is the footer's. But the footer only ever consumed the *sides* —
    /// the bottom it passed itself was a literal zero — so its buttons sat on the gesture
    /// bar. The reference removes the bottom intrusion from that slot **only when a
    /// navigation bar is below it** (`scaffold.dart:3158`); with nothing below, the
    /// footer's own `SafeArea(top: false)` takes it.
    ///
    /// And the decoration stays outside that safe area, so the rule and the background
    /// still run to the screen's edge. A footer that padded its own box would be a rule
    /// stopping short of the edge it is ruling off.
    #[test]
    fn a_footer_holds_the_bottom_edge_off_unless_a_bar_below_it_does() {
        const GESTURE: f32 = 24.0;
        /// The footer's content, in a colour the decoration does not use.
        const CONTENT: Color = Color {
            r: 0.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        let size = Size::new(W, H);
        let footer = |with_bar: bool| {
            let surface = MediaQuery::new(size)
                .with_insets(WindowInsets::bars(Insets::new(0.0, 0.0, GESTURE, 0.0)));
            let ui = surface.scope(|| {
                let mut scaffold = Scaffold::new()
                    .size(W, H)
                    .body(Container::<Msg>::new().flex(1.0))
                    .persistent_footer_color(MARK)
                    .persistent_footer(
                        Container::<Msg>::new()
                            .width(100.0)
                            .height(40.0)
                            .color(CONTENT),
                    );
                if with_bar {
                    scaffold = scaffold.nav(0, Msg::Go).destination("H", "Home");
                }
                let tree = scaffold.build();
                build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default())
            });
            let find = |wanted: Color| {
                ui.scene()
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        frus_core::Primitive::Rect { rect, color, .. } if *color == wanted => {
                            Some(*rect)
                        }
                        _ => None,
                    })
                    .expect("the footer is drawn")
            };
            (find(MARK), find(CONTENT))
        };

        // Alone: the background reaches the window's edge, the buttons stop above the
        // gesture bar.
        let (decoration, buttons) = footer(false);
        assert!(
            (decoration.y + decoration.height - H).abs() < 0.5,
            "the footer's background stops short of the edge: {decoration:?}"
        );
        assert!(
            buttons.y + buttons.height <= H - GESTURE + 0.5,
            "the footer's buttons sit on the gesture bar: {buttons:?}"
        );

        // With a bar below it, the bar holds the edge off and the footer must not pad
        // for it a second time: its buttons stop a plain footer padding from its own
        // bottom edge, and no further.
        let (decoration, buttons) = footer(true);
        let gap = (decoration.y + decoration.height) - (buttons.y + buttons.height);
        assert!(
            gap <= FOOTER_PAD + 0.5,
            "the footer padded for an intrusion the bar below it already holds off: {gap}"
        );
    }

    /// **The bottom slot is told, not padded** (milestone 418), and the difference is
    /// one you can see. The reference puts the safe area *inside* the shape that carries
    /// the colour (`bottom_app_bar.dart:230`), so a bar's surface runs behind the gesture
    /// bar and only its actions are held clear of it. Padded from outside, the surface
    /// stopped short of the screen's edge and a strip of the scaffold showed through
    /// underneath it.
    #[test]
    fn a_bottom_bar_s_surface_runs_behind_the_gesture_bar() {
        use crate::BottomAppBar;
        const GESTURE: f32 = 24.0;
        const BAR: f32 = 64.0;
        let size = Size::new(W, H);
        let surface = MediaQuery::new(size)
            .with_insets(WindowInsets::bars(Insets::new(0.0, 0.0, GESTURE, 0.0)));
        let ui = surface.scope(|| {
            let tree = Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .bottom_app_bar(
                    BottomAppBar::new()
                        .height(BAR)
                        .padding(0.0)
                        .color(MARK)
                        .child(marked::<Msg>(20.0)),
                )
                .build();
            build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default())
        });
        let mut rects: Vec<frus_core::Rect> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. } if *color == MARK => Some(*rect),
                _ => None,
            })
            .collect();
        rects.sort_by(|a, b| a.height.total_cmp(&b.height));
        let (content, bar) = (rects[0], rects[rects.len() - 1]);
        // The surface: it took the intrusion into itself and reaches the window's edge.
        assert!(
            (bar.height - (BAR + GESTURE)).abs() < 0.5,
            "the bar did not consume what it was told about: {bar:?}"
        );
        assert!(
            (bar.y + bar.height - H).abs() < 0.5,
            "the bar's surface stops short of the edge: {bar:?}"
        );
        // Its content: inside that surface, clear of the gesture bar.
        assert!(
            content.y + content.height <= H - GESTURE + 0.5,
            "the actions run under the gesture bar: {content:?}"
        );
    }

    /// The notch is cut in the **bar's own** coordinates, and since milestone 418 those
    /// are the window's: the bottom slot spans the full width and consumes the side
    /// intrusions itself, so its box is no longer held off by the left one and the notch
    /// must no longer be moved back by it. With no side intrusion the two readings agree
    /// and the mistake hides, which is why this test puts a cutout down one edge.
    #[test]
    fn a_notch_stays_under_its_button_beside_a_cutout() {
        use crate::BottomAppBar;
        const CUTOUT: f32 = 48.0;
        const BAR: f32 = 64.0;
        const FAB: f32 = 56.0;
        let size = Size::new(W, H);
        let surface = MediaQuery::new(size)
            .with_insets(WindowInsets::bars(Insets::new(0.0, 0.0, 0.0, CUTOUT)));
        let ui = surface.scope(|| {
            let tree = Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .bottom_app_bar(BottomAppBar::new().height(BAR).color(MARK))
                .fab_location(FabLocation::StartDocked)
                .fab_size(FAB)
                .fab(Container::<Msg>::new().width(FAB).height(FAB))
                .build();
            build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default())
        });
        let outline = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Path { path, fill, .. } if *fill == Some(MARK) => {
                    Some(path.clone())
                }
                _ => None,
            })
            .expect("a notched bar paints an outline");
        // Where the button is: past the cutout, then the usual margin.
        let fab_centre_x = CUTOUT + FAB_MARGIN + FAB / 2.0;
        let bar_top = H - BAR;
        // Every node the notch put below the top edge belongs to the notch, so every one
        // of them is within the notch's own reach of the button's centre. Cut at the old
        // place these sit a whole cutout away.
        let mut dipped = 0;
        for verb in outline.verbs() {
            let p = match verb {
                frus_core::PathVerb::MoveTo(p) | frus_core::PathVerb::LineTo(p) => *p,
                frus_core::PathVerb::QuadTo { to, .. }
                | frus_core::PathVerb::CubicTo { to, .. } => *to,
                frus_core::PathVerb::Close => continue,
            };
            if p.y > bar_top + 1.0 && p.y < bar_top + BAR - 1.0 {
                dipped += 1;
                assert!(
                    (p.x - fab_centre_x).abs() <= FAB,
                    "the notch was cut away from its button: {p:?} (centre {fab_centre_x})"
                );
            }
        }
        assert!(dipped > 0, "no notch was cut at all");
    }

    /// Two drawers, two edges, and a screen may have both.
    /// **Only a primary shell absorbs the status bar.**
    ///
    /// The reference's `primary` decides whether the app bar's height is the bar's own
    /// or the bar's plus the top intrusion (`scaffold.dart:3049`). A shell nested in a
    /// page has something else above it, and adding the notch there would inset for it
    /// twice. Same shell, same surface, one switch, and the bar's title moves by exactly
    /// the intrusion.
    ///
    /// Since milestone 417 the switch works the way the reference's does — through the
    /// **description** the slot is handed, not through a padding applied from outside. A
    /// non-primary shell tells its bar's subtree that there is no status bar; the bar,
    /// which is what actually pads, then has nothing to pad by.
    #[test]
    fn only_a_primary_shell_absorbs_the_status_bar() {
        const TOP: f32 = 40.0;
        let top_of_bar = |primary: bool| {
            let size = Size::new(400.0, 800.0);
            let surface = MediaQuery::new(size)
                .with_insets(WindowInsets::bars(Insets::new(TOP, 0.0, 0.0, 0.0)));
            let tree = surface.scope(|| {
                Scaffold::<Msg>::new()
                    .primary(primary)
                    .app_bar(crate::AppBar::<Msg>::new("bar").height(56.0).build())
                    .body(Container::new().flex(1.0))
                    .build()
            });
            let ui = build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default());
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    crate::Primitive::Text { position, .. } => Some(position.y),
                    _ => None,
                })
                .expect("the bar is drawn")
        };
        let inset = top_of_bar(true) - top_of_bar(false);
        assert!(
            (inset - TOP).abs() < 0.5,
            "primary should hold the bar off by exactly the intrusion, moved by {inset}"
        );
    }

    /// **The footer carries a line along its top, and it can be taken off.**
    ///
    /// The reference's footer is a container decorated with a one-pixel top border
    /// (`Divider.createBorderSide`, `scaffold.dart:3136`) unless the caller replaces the
    /// decoration. Ours drew none until milestone 394.
    #[test]
    fn the_persistent_footer_is_ruled_off_from_the_body() {
        let lines = |divider: bool| {
            let size = Size::new(400.0, 800.0);
            let tree = MediaQuery::new(size).scope(|| {
                Scaffold::<Msg>::new()
                    .persistent_footer_divider(divider)
                    .persistent_footer(button("Save", Msg::Add))
                    .body(Container::new().flex(1.0))
                    .build()
            });
            let ui = build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default());
            // A divider is the only thing here a pixel tall and the width of the shell.
            ui.scene()
                .primitives()
                .iter()
                .filter(|p| {
                    matches!(p, crate::Primitive::Rect { rect, .. }
                        if rect.height <= 1.5 && rect.width > 300.0)
                })
                .count()
        };
        assert_eq!(lines(true), 1, "ruled off by default, as the reference is");
        assert_eq!(lines(false), 0, "and the caller may say no");
    }

    /// **A drawer whose scrim does not dismiss it.**
    ///
    /// The reference's `drawerBarrierDismissible`. `false` is for a panel holding
    /// something that has to be answered: the way out is a control inside it, and the
    /// screen behind stays unreachable.
    #[test]
    fn a_drawer_may_refuse_to_close_on_its_scrim() {
        let dismisses = |dismissible: bool| {
            let size = Size::new(400.0, 800.0);
            let tree = MediaQuery::new(size).scope(|| {
                Scaffold::<Msg>::new()
                    .drawer_barrier_dismissible(dismissible)
                    .drawer(text("Menu"), true, Msg::Drawer)
                    .body(Container::new().flex(1.0))
                    .build()
            });
            let ui = build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default());
            // A click far from the panel, on the scrim.
            ui.hit(crate::Point::new(380.0, 400.0))
                .and_then(|id| ui.msg_for(id))
        };
        assert_eq!(dismisses(true), Some(Msg::Drawer), "the scrim closes it");
        assert_eq!(dismisses(false), None, "and here it does not");
    }

    #[test]
    fn the_leading_drawer_opens_on_the_left() {
        let rects = marks(
            Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .drawer(marked::<Msg>(100.0), true, Msg::Drawer)
                .end_drawer(marked::<Msg>(100.0), false, Msg::Drawer)
                .build(),
        );
        assert_eq!(rects.len(), 1, "only the open one is drawn: {rects:?}");
        assert!(rects[0].x < 1.0, "docked to the left edge: {:?}", rects[0]);
    }

    /// An overlay layer — the FAB's, and now the extended body's chrome — must not
    /// swallow the clicks of what is under it where it draws nothing.
    #[test]
    fn an_overlay_layer_does_not_swallow_the_body_s_clicks() {
        let scaffold = Scaffold::new()
            .size(W, H)
            .body(button("Tap me", Msg::Add).size(16.0))
            .extend_body(true)
            .nav(0, Msg::Go)
            .destination("H", "Home")
            .fab(button("+", Msg::Add))
            .build();
        let ui = build_ui(
            scaffold.as_ref(),
            Size::new(W, H),
            &Runtime::default(),
            &Theme::default(),
        );
        let target = ui
            .hit(frus_core::Point::new(40.0, 20.0))
            .expect("the body's button must be reachable under two overlay layers");
        assert_eq!(ui.msg_for(target), Some(Msg::Add));
    }

    /// **The shell forwards what a destination says about itself** (milestone 436). A
    /// property the navigation widgets have and the shell drops is a property most
    /// applications do not have, which is milestone 434's lesson applied one level down.
    #[test]
    fn the_shell_forwards_a_disabled_destination() {
        let shell = Scaffold::new()
            .size(W, H)
            .body(Container::<Msg>::new())
            .nav(0, Msg::Go)
            .destination("H", "Home")
            .destination("S", "Stats")
            .disabled()
            .build();
        let ui = build_ui(
            shell.as_ref(),
            Size::new(W, H),
            &Runtime::default(),
            &Theme::default(),
        );
        let msg_at = |x: f32| {
            ui.hit(frus_core::Point::new(x, H - 30.0))
                .and_then(|target| ui.msg_for(target))
        };
        assert_eq!(
            msg_at(W * 0.25),
            Some(Msg::Go(0)),
            "the live destination is where this test thinks it is"
        );
        assert_eq!(msg_at(W * 0.75), None, "and the disabled one emits nothing");
    }

    /// **The shell can say what kind of rail it wants** (milestone 434).
    ///
    /// It builds the navigation itself, which is what makes it a shell — and until now
    /// that meant everything a rail can do and a bar cannot was reachable only by giving
    /// the shell up and building the rail by hand.
    #[test]
    fn the_shell_can_ask_for_an_extended_rail() {
        let body_x = |placement, extended: bool| {
            marks(
                Scaffold::new()
                    .size(W, H)
                    .body(filling::<Msg>())
                    .nav(0, Msg::Go)
                    .nav_placement(placement)
                    .destination("H", "Home")
                    .rail(move |rail| rail.extended(extended))
                    .build(),
            )[0]
            .x
        };
        assert!(
            (body_x(NavPlacement::Rail, false) - RAIL_WIDTH).abs() < 0.01,
            "a rail is 80 wide: {}",
            body_x(NavPlacement::Rail, false)
        );
        assert!(
            (body_x(NavPlacement::Rail, true) - EXTENDED_RAIL_WIDTH).abs() < 0.01,
            "an extended one is 256, and the body starts after it: {}",
            body_x(NavPlacement::Rail, true)
        );
        assert_eq!(
            body_x(NavPlacement::Bottom, true),
            0.0,
            "and the door is shut when the navigation is a bar, which has none of it"
        );
    }

    /// **What the shell puts beside a rail has to know how wide the rail came out.**
    ///
    /// The persistent footer's row is *given* its width, so that the alignment has
    /// something to distribute. That width was `window - RAIL_WIDTH - padding`, read off
    /// the constant — which is right until a caller extends the rail, and then it is 176
    /// pixels too wide and an end-aligned footer is pushed clean off the screen. The
    /// shell asks the rail it was handed instead.
    #[test]
    fn a_footer_beside_an_extended_rail_stays_on_the_screen() {
        let footer_x = |extended: bool| {
            marks(
                Scaffold::new()
                    .size(W, H)
                    .body(Container::<Msg>::new())
                    .nav(0, Msg::Go)
                    .nav_placement(NavPlacement::Rail)
                    .destination("H", "Home")
                    .persistent_footer_alignment(Justify::End)
                    .persistent_footer(marked::<Msg>(40.0).width(100.0))
                    .rail(move |rail| rail.extended(extended))
                    .build(),
            )[0]
            .x
        };
        assert!(
            footer_x(true) + 100.0 <= W,
            "the footer was pushed off the screen: {}",
            footer_x(true)
        );
        // And it lands in the *same* place either way: the footer's row ends where the
        // window does, whatever the rail took off the front of it.
        assert!(
            (footer_x(true) - footer_x(false)).abs() < 0.01,
            "{} against {}",
            footer_x(true),
            footer_x(false)
        );
    }

    /// The label mode reaches **whichever** of the two widgets the placement chose, and
    /// saying nothing leaves each of them on the default the reference gives it
    /// (milestone 432) rather than collapsing the two onto one answer.
    #[test]
    fn the_shell_hands_the_label_mode_on() {
        let says_home = |placement, labels: Option<RailLabels>| {
            let mut shell = Scaffold::new()
                .size(W, H)
                .body(Container::<Msg>::new())
                .nav(0, Msg::Go)
                .nav_placement(placement)
                .destination("H", "Home");
            if let Some(labels) = labels {
                shell = shell.nav_labels(labels);
            }
            let shell = shell.build();
            let ui = build_ui(
                shell.as_ref(),
                Size::new(W, H),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "Home"))
        };
        assert!(
            !says_home(NavPlacement::Rail, None),
            "a rail says nothing until it is asked"
        );
        assert!(
            says_home(NavPlacement::Bottom, None),
            "a bar says everything"
        );
        assert!(says_home(NavPlacement::Rail, Some(RailLabels::All)));
        assert!(!says_home(NavPlacement::Bottom, Some(RailLabels::None)));
    }

    /// Where the navigation is drawn: `(min x, max y)` of the destinations' **glyphs**.
    ///
    /// The glyph rather than the label, since milestone 432: a rail shows no labels
    /// unless it is asked to, so a helper that looked for them stopped finding the
    /// navigation it was measuring. The glyph is the one thing every destination paints
    /// in every mode.
    fn nav_extent(root: &dyn Widget<Msg>, width: f32, height: f32) -> (f32, f32) {
        let ui = build_ui(
            root,
            Size::new(width, height),
            &Runtime::default(),
            &Theme::default(),
        );
        let mut leftmost = f32::MAX;
        let mut lowest: f32 = 0.0;
        for primitive in ui.scene().primitives() {
            // The glyphs are what identify the navigation: "H" and "S" are in the
            // destinations and nowhere else in the test's scaffold.
            if let frus_core::Primitive::Text { text, position, .. } = primitive {
                if text == "H" || text == "S" {
                    leftmost = leftmost.min(position.x);
                    lowest = lowest.max(position.y);
                }
            }
        }
        assert!(leftmost < f32::MAX, "the destinations were never painted");
        (leftmost, lowest)
    }

    /// The default, and the whole point of milestone 305: a wide window does **not**
    /// move the navigation. A phone turned to landscape crosses the size-class
    /// threshold, and its bottom bar has to stay at the bottom.
    #[test]
    fn a_wide_window_keeps_the_navigation_at_the_bottom() {
        // 900 × 420: landscape, comfortably past the Compact threshold.
        let (_, lowest) = nav_extent(scaffold(900.0, 420.0).as_ref(), 900.0, 420.0);
        assert!(
            lowest > 420.0 * 0.6,
            "the destinations were painted at y = {lowest}, which is not a bottom bar"
        );
    }

    /// And the same tree at the same width, having asked for a rail, puts them against
    /// the leading edge instead. The behaviour is not gone — it is opted into.
    #[test]
    fn a_rail_is_drawn_when_it_is_asked_for() {
        let railed = Scaffold::new()
            .size(900.0, 420.0)
            .app_bar(text("Title").size(20.0))
            .body(text("Body").size(16.0))
            .nav(0, Msg::Go)
            .nav_placement(NavPlacement::Rail)
            .destination("H", "Home")
            .destination("S", "Stats")
            .build();
        let (leftmost, lowest) = nav_extent(railed.as_ref(), 900.0, 420.0);
        assert!(
            leftmost < RAIL_WIDTH,
            "a rail sits against the leading edge, not at x = {leftmost}"
        );
        assert!(
            lowest < 420.0 * 0.6,
            "a rail stacks from the top, not at y = {lowest}"
        );
    }

    /// **The rail is told, not padded** (milestone 420).
    ///
    /// The reference keeps its safe area inside the `Material`
    /// (`navigation_rail.dart:553`) and takes the **leading** side, the top and the
    /// bottom — never the trailing one, which is where the body is. So the rail's box
    /// swallows the cutout, the rule down its edge runs the full height of the screen,
    /// and the destinations are what stays clear of the notch and the gesture bar.
    ///
    /// Padded from outside, the rail was a shorter box floated inside the intrusions and
    /// the rule stopped at the notch — a rule that does not reach the edge it is ruling.
    #[test]
    fn a_rail_rules_the_full_height_and_holds_its_destinations_clear() {
        const TOP: f32 = 40.0;
        const BOTTOM: f32 = 30.0;
        const CUTOUT: f32 = 24.0;
        const WIDE: f32 = 900.0;
        const TALL: f32 = 420.0;
        let size = Size::new(WIDE, TALL);
        let surface = MediaQuery::new(size)
            .with_insets(WindowInsets::bars(Insets::new(TOP, 0.0, BOTTOM, CUTOUT)));
        let ui = surface.scope(|| {
            let tree = Scaffold::new()
                .size(WIDE, TALL)
                .body(Container::<Msg>::new().flex(1.0))
                .nav(0, Msg::Go)
                .nav_placement(NavPlacement::Rail)
                .destination("H", "Home")
                .destination("S", "Stats")
                .build();
            build_ui(tree.as_ref(), size, &Runtime::default(), &Theme::default())
        });
        // The rule: the one-pixel column down the rail's trailing edge.
        let rule = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, .. }
                    if rect.width <= 1.5 && rect.height > TALL / 2.0 =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the rail rules itself off from the body");
        assert!(
            rule.y.abs() < 0.5 && (rule.y + rule.height - TALL).abs() < 0.5,
            "the rule stops at the intrusions: {rule:?}"
        );
        assert!(
            (rule.x - (CUTOUT + RAIL_WIDTH - 1.0)).abs() < 0.5,
            "the rail did not take the cutout into its own box: {rule:?}"
        );
        // The destinations: clear of the cutout beside them and the bars above and below.
        let (mut left, mut top, mut bottom) = (f32::MAX, f32::MAX, f32::MIN);
        for p in ui.scene().primitives() {
            if let frus_core::Primitive::Text {
                position, size: em, ..
            } = p
            {
                left = left.min(position.x);
                top = top.min(position.y);
                bottom = bottom.max(position.y + em);
            }
        }
        assert!(
            left >= CUTOUT - 0.5,
            "a destination sits in the cutout: {left}"
        );
        assert!(
            top >= TOP - 0.5,
            "a destination sits under the status bar: {top}"
        );
        assert!(
            bottom <= TALL - BOTTOM + 0.5,
            "a destination sits on the gesture bar: {bottom}"
        );
    }

    /// A narrow window with a rail asked for still gets a rail: the placement is a
    /// decision, not a hint the scaffold may overrule.
    #[test]
    fn a_rail_survives_a_narrow_window() {
        let railed = Scaffold::new()
            .size(360.0, 800.0)
            .body(text("Body").size(16.0))
            .nav(0, Msg::Go)
            .nav_placement(NavPlacement::Rail)
            .destination("H", "Home")
            .destination("S", "Stats")
            .build();
        let (leftmost, _) = nav_extent(railed.as_ref(), 360.0, 800.0);
        assert!(leftmost < RAIL_WIDTH, "leftmost = {leftmost}");
    }

    #[test]
    fn compact_pins_bottom_bar_near_the_bottom() {
        // The bottom bar is pinned at the bottom: primitives are painted in the low
        // band (y ≥ 700), above the bottom inset (30), not in the middle.
        let s = scaffold(400.0, 800.0);
        let ui = build_ui(
            s.as_ref(),
            Size::new(400.0, 800.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let pinned_low = ui.scene().primitives().iter().any(|p| match p {
            frus_core::Primitive::Rect { rect, .. } => rect.y >= 700.0 && rect.height < 100.0,
            frus_core::Primitive::Text { position, .. }
            | frus_core::Primitive::RichText { position, .. } => position.y >= 700.0,
            frus_core::Primitive::Path { .. }
            | frus_core::Primitive::Image { .. }
            | frus_core::Primitive::Layer { .. } => false,
        });
        assert!(pinned_low, "the bottom bar must be pinned in the low band");
    }
}

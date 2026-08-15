//! [`Scaffold`]: the **screen shell** of frus — the central coordinator of a
//! Material screen structure.
//!
//! The developer declares **slots** (app bar, body, navigation, drawer, FAB, modal
//! sheet); the Scaffold assembles them correctly — the **app bar pinned** at the
//! top, a **scrolling body** in the middle, **navigation** in a bottom bar (or a
//! rail, if one is asked for), all of it **respecting the safe area**
//! (system insets). One piece of code, with no branching on mobile vs desktop.
//!
//! ```ignore
//! Scaffold::new(width, height)
//!     .window_insets(app.insets)             // system bars **and** the keyboard
//!     .background(theme.background)
//!     .app_bar(appbar)                       // pinned at the top
//!     .body(content)                         // scrolls
//!     .nav(app.section, Msg::SetSection)     // destinations, in a bottom bar
//!     .destination("✔", "Tasks").badge(3)
//!     .destination("▦", "Stats")
//!     .drawer(menu, app.menu_open, Msg::ToggleMenu)          // leading edge
//!     .end_drawer(filters, app.drawer_open, Msg::ToggleDrawer)
//!     .persistent_footer(row![cancel, save])  // never scrolls away
//!     .fab_location(FabLocation::EndFloat)   // or docked, at either end
//!     .fab(fab_button("+", Msg::AddTodo))    // floating action button
//!     .bottom_sheet(sheet, app.sheet_open, Msg::ToggleSheet)
//!     .build()
//! ```
//!
//! **What the body is given, and what it is given under.** By default the body gets
//! what the bars leave it: it starts below the app bar, stops above the bottom bar,
//! and is shortened by the soft keyboard so that a field at the end of a form can
//! still be scrolled to. Each of those three is a decision a screen may reverse —
//! [`Scaffold::extend_body_behind_app_bar`], [`Scaffold::extend_body`] and
//! [`Scaffold::resize_to_avoid_bottom_inset`] — and none of them lets content sit
//! under the system's own bars, which are not the application's to spend.

use frus_core::{Color, Insets, WindowInsets};
use frus_layout::Justify;

use crate::bottomappbar::BottomAppBar;
use crate::button::Variant;
use crate::container::Container;
use crate::flex::Flex;
use crate::navrail::{BottomBar, NavRail, BAR_HEIGHT, RAIL_WIDTH};
use crate::scroll::Scroll;
use crate::stack::Stack;
use crate::widget::Widget;

/// The FAB's margin from the edge, and from the bottom bar.
const FAB_MARGIN: f32 = 16.0;
/// The padding around the persistent footer's row.
const FOOTER_PAD: f32 = 12.0;
/// The height a floating action button is assumed to have, absent
/// [`Scaffold::fab_size`]. The conventional Material diameter.
const FAB_SIZE: f32 = 56.0;

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
    resize_to_avoid_bottom_inset: bool,
    extend_body: bool,
    extend_body_behind_app_bar: bool,
    background: Option<Color>,
    app_bar: Option<Box<dyn Widget<Msg>>>,
    body: Option<Box<dyn Widget<Msg>>>,
    selected: usize,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    destinations: Vec<(String, String, Option<u32>)>,
    nav_placement: NavPlacement,
    drawer: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    end_drawer: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    bottom_app_bar: Option<BottomAppBar<Msg>>,
    fab: Option<Box<dyn Widget<Msg>>>,
    fab_location: FabLocation,
    fab_size: f32,
    persistent_footer: Option<Box<dyn Widget<Msg>>>,
    persistent_footer_alignment: Justify,
    bottom_sheet: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
}

impl<Msg: Clone + 'static> Scaffold<Msg> {
    /// Creates a shell for a `width × height` surface, in logical pixels.
    ///
    /// The navigation is a **bottom bar** whatever the width; ask for
    /// [`Scaffold::nav_placement`] to have it anywhere else.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            insets: Insets::ZERO,
            view_insets: Insets::ZERO,
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
            drawer: None,
            end_drawer: None,
            bottom_app_bar: None,
            fab: None,
            fab_location: FabLocation::default(),
            fab_size: FAB_SIZE,
            persistent_footer: None,
            persistent_footer_alignment: Justify::End,
            bottom_sheet: None,
        }
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

    /// The screen's body: it **scrolls** in the space between the bars.
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

    /// Adds a navigation destination (glyph + label).
    pub fn destination(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.destinations.push((icon.into(), label.into(), None));
        self
    }

    /// A notification count on the **last** destination.
    pub fn badge(mut self, count: u32) -> Self {
        if let Some(last) = self.destinations.last_mut() {
            last.2 = Some(count);
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
            drawer,
            end_drawer,
            bottom_app_bar,
            fab,
            fab_location,
            fab_size,
            persistent_footer,
            persistent_footer_alignment,
            bottom_sheet,
            view_insets,
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
        let nav: Option<Box<dyn Widget<Msg>>> = if has_nav {
            let on_select =
                on_select.expect("nav(selected, on_select) is required with destinations");
            if !rail_nav {
                let mut bar = BottomBar::new(selected, on_select);
                for (icon, label, badge) in &destinations {
                    bar = bar.item(icon.clone(), label.clone());
                    if let Some(count) = *badge {
                        bar = bar.badge(count);
                    }
                }
                Some(Box::new(bar))
            } else {
                let mut rail = NavRail::new(selected, on_select);
                for (icon, label, badge) in &destinations {
                    rail = rail.item(icon.clone(), label.clone());
                    if let Some(count) = *badge {
                        rail = rail.badge(count);
                    }
                }
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
                    bar.notched_at(fab_centre_x - insets.left, fab_size / 2.0)
                } else {
                    bar
                };
                Some(Box::new(bar))
            }
            (None, None) => None,
        };
        let has_nav = has_nav || bottom_bar_height > 0.0;

        // The persistent footer: its own row, aligned as asked, kept clear of the side
        // insets. It sits between the body and the bottom bar and never scrolls.
        let footer: Option<Box<dyn Widget<Msg>>> = persistent_footer.map(|widget| {
            // The row is **given** the width it has to fill. A row that hugged its
            // content would leave the alignment nothing to distribute, and every
            // footer would sit at the leading edge whatever it was asked for.
            let rail = if rail_nav { RAIL_WIDTH } else { 0.0 };
            let row_width = (width - insets.left - insets.right - rail - FOOTER_PAD * 2.0).max(0.0);
            let row = Flex::row()
                .width(row_width)
                .justify(persistent_footer_alignment)
                .child(widget);
            inset_pad(
                Box::new(Container::new().padding(FOOTER_PAD).child(row)),
                0.0,
                insets.right,
                0.0,
                insets.left,
            )
        });

        // How far the bottom-most slot is held off the edge. The keyboard is the only
        // inset a screen may decline: `view_insets.bottom` measures the occlusion from
        // the window edge, bar included, so the two combine with `max` and never add up.
        let bottom_clear = if resize_to_avoid_bottom_inset {
            insets.bottom.max(view_insets.bottom)
        } else {
            insets.bottom
        };

        // A scrolling body, with the side insets applied to its content.
        let scroll_body = Scroll::new().flex(1.0).child(inset_pad(
            body_widget,
            0.0,
            insets.right,
            0.0,
            insets.left,
        ));

        // Whether the bottom clearance falls to the body. With a bar or a footer below
        // it, they hold the edge off; alone — or with the body told to run under them —
        // it is on the body, and it is the **viewport** that must shrink, not the
        // content that must be padded: a field at the bottom of a form has to be
        // scrolled to, not merely followed by empty space under the keyboard.
        // A rail is beside the body, not beneath it, so it holds nothing off the edge.
        let bar_below_body = has_nav && !rail_nav;
        let body_owns_bottom = !extend_body && footer.is_none() && !bar_below_body;
        let body_spacer = body_owns_bottom && bottom_clear > 0.0;

        // Which slots the body must make room for, and which it runs under. A slot that
        // is extended behind moves out of the body's column and into an overlay layer
        // drawn on top of it — the same widget either way, in one place or the other.
        //
        // The bottom slots only move on a **compact** layout, where the bottom bar
        // actually is one. Wide, the navigation is a rail beside the body and the
        // overlay would span the rail as well as the content; a footer there stays in
        // the column, which is what it should do anyway when nothing is under it.
        let bar_over_body = extend_body_behind_app_bar && app_bar.is_some();
        let bottom_over_body = extend_body && !rail_nav && (footer.is_some() || nav.is_some());
        let app_bar_pad = |bar: Box<dyn Widget<Msg>>, left: f32| {
            inset_pad(bar, insets.top, insets.right, 0.0, left)
        };
        let nav_pad =
            |n: Box<dyn Widget<Msg>>| inset_pad(n, 0.0, insets.right, bottom_clear, insets.left);

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
            col = col.child(scroll_body);
            if body_spacer {
                col = col.child(Container::new().height(bottom_clear));
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
                row = row.child(inset_pad(n, insets.top, 0.0, bottom_clear, insets.left));
            }
            let mut content = Flex::column().flex(1.0);
            if !bar_over_body {
                if let Some(bar) = app_bar.take() {
                    content = content.child(app_bar_pad(bar, 0.0));
                }
            }
            content = content.child(scroll_body);
            if body_spacer {
                content = content.child(Container::new().height(bottom_clear));
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
            let nav_h = if bottom_bar_height > 0.0 {
                bottom_bar_height
            } else if !rail_nav && has_nav {
                BAR_HEIGHT
            } else {
                0.0
            };
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

        // The modal drawers, then the modal sheet, wrap the shell as overlays. The
        // leading drawer goes on first so that the trailing one, wrapping it, is the
        // outer layer — with both open the end drawer is the one on top, which is the
        // one the user opened last.
        if let Some((panel, open, toggle)) = drawer {
            content = Box::new(
                crate::Drawer::new(open)
                    .on_dismiss(toggle)
                    .panel(panel)
                    .body(content),
            );
        }
        if let Some((panel, open, toggle)) = end_drawer {
            content = Box::new(
                crate::Drawer::new(open)
                    .on_dismiss(toggle)
                    .right()
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
        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(bg)
                .child(content),
        )
    }
}

/// Keeps a slot clear of the system bars, **without a superfluous wrapper**: if
/// all the insets are zero, returns the widget as is (preserving the parent's
/// stretch); otherwise wraps it in a padding `Container`.
fn inset_pad<Msg: Clone + 'static>(
    widget: Box<dyn Widget<Msg>>,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) -> Box<dyn Widget<Msg>> {
    if top == 0.0 && right == 0.0 && bottom == 0.0 && left == 0.0 {
        widget
    } else {
        Box::new(
            Container::new()
                .padding_each(top, right, bottom, left)
                .child(widget),
        )
    }
}

/// A conventional floating action button (round, accent), to be passed to
/// [`Scaffold::fab`]. Sugar for `button(label, msg)` styled as primary.
pub fn fab_button<Msg: Clone + 'static>(
    label: impl Into<String>,
    message: Msg,
) -> crate::Button<Msg> {
    crate::Button::new(label)
        .variant(Variant::Primary)
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
    use crate::{build_ui, dsl::button, dsl::text, Runtime, Size, Theme};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
        Drawer,
        Add,
    }

    fn scaffold(width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        Scaffold::new(width, height)
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

    /// Every marked rectangle in the assembled scaffold, top to bottom, **as seen**:
    /// clipped to the box it was drawn in. A body taller than its viewport is exactly
    /// how its viewport gets measured.
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
        WindowInsets {
            padding: Insets::new(40.0, 0.0, 30.0, 0.0),
            view_insets: Insets::new(0.0, 0.0, 300.0, 0.0),
        }
    }

    /// The body is lifted clear of the keyboard — and only if it is asked to be.
    #[test]
    fn the_keyboard_shortens_the_body_unless_the_screen_declines() {
        let lifted = marks(
            Scaffold::new(W, H)
                .window_insets(keyboard_up())
                .body(marked(2000.0))
                .build(),
        );
        assert_eq!(lifted.len(), 1);
        assert!(
            (lifted[0].y + lifted[0].height - (H - 300.0)).abs() < 1.0,
            "the body must stop at the keyboard: {:?}",
            lifted[0]
        );

        let covered = marks(
            Scaffold::new(W, H)
                .window_insets(keyboard_up())
                .resize_to_avoid_bottom_inset(false)
                .body(marked(2000.0))
                .build(),
        );
        // Declined: only the permanent bottom bar is kept clear, the keyboard covers.
        assert!(
            (covered[0].y + covered[0].height - (H - 30.0)).abs() < 1.0,
            "the body must run under the keyboard: {:?}",
            covered[0]
        );
    }

    /// Without a bottom bar or a footer, the body itself must clear the system bar —
    /// there is nobody else below it to do it.
    #[test]
    fn a_body_alone_still_clears_the_navigation_bar() {
        let body = marks(
            Scaffold::new(W, H)
                .insets(Insets::new(40.0, 0.0, 30.0, 0.0))
                .body(marked(2000.0))
                .build(),
        );
        assert!(
            (body[0].y + body[0].height - (H - 30.0)).abs() < 1.0,
            "the body ran under the navigation bar: {:?}",
            body[0]
        );
    }

    /// `extend_body` is the difference between stopping above the bar and running
    /// under it; the bar is drawn on top either way.
    #[test]
    fn an_extended_body_runs_under_the_bottom_bar() {
        let stops = marks(
            Scaffold::new(W, H)
                .body(marked(2000.0))
                .nav(0, Msg::Go)
                .destination("H", "Home")
                .build(),
        );
        let runs_under = marks(
            Scaffold::new(W, H)
                .extend_body(true)
                .body(marked(2000.0))
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
            Scaffold::new(W, H)
                .insets(Insets::new(40.0, 0.0, 0.0, 0.0))
                .app_bar(Container::<Msg>::new().height(56.0))
                .body(marked(200.0))
                .build(),
        );
        assert!(below[0].y >= 96.0, "below the bar: {:?}", below[0]);

        let behind = marks(
            Scaffold::new(W, H)
                .insets(Insets::new(40.0, 0.0, 0.0, 0.0))
                .extend_body_behind_app_bar(true)
                .app_bar(Container::<Msg>::new().height(56.0))
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
            Scaffold::new(W, H)
                .body(marked(2000.0))
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
                Scaffold::new(W, H)
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
            let scaffold = Scaffold::new(W, H)
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
        let scaffold = Scaffold::new(W, H)
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

    /// Two drawers, two edges, and a screen may have both.
    #[test]
    fn the_leading_drawer_opens_on_the_left() {
        let rects = marks(
            Scaffold::new(W, H)
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
        let scaffold = Scaffold::new(W, H)
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

    /// Where the navigation is drawn: `(min x, max y)` of the destinations' labels.
    /// A bottom bar sits low and spans the width; a rail is a narrow column against
    /// the leading edge.
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
            // The labels are what identify the navigation: "Home" and "Stats" are in
            // the destinations and nowhere else in the test's scaffold.
            if let frus_core::Primitive::Text { text, position, .. } = primitive {
                if text == "Home" || text == "Stats" {
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
        let railed = Scaffold::new(900.0, 420.0)
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

    /// A narrow window with a rail asked for still gets a rail: the placement is a
    /// decision, not a hint the scaffold may overrule.
    #[test]
    fn a_rail_survives_a_narrow_window() {
        let railed = Scaffold::new(360.0, 800.0)
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

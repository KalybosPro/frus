//! [`Scaffold`]: the **screen shell** of frus — the central coordinator of a
//! Material screen structure.
//!
//! The developer declares **slots** (app bar, body, navigation, drawer, FAB, modal
//! sheet); the Scaffold assembles them correctly — the **app bar pinned** at the
//! top, a **scrolling body** in the middle, **adaptive navigation** (a bottom bar
//! when narrow, a side rail when wide), all of it **respecting the safe area**
//! (system insets). One piece of code, with no branching on mobile vs desktop.
//!
//! ```ignore
//! Scaffold::new(width, height)
//!     .window_insets(app.insets)             // system bars **and** the keyboard
//!     .background(theme.background)
//!     .app_bar(appbar)                       // pinned at the top
//!     .body(content)                         // scrolls
//!     .nav(app.section, Msg::SetSection)     // adaptive navigation
//!     .destination("✔", "Tasks").badge(3)
//!     .destination("▦", "Stats")
//!     .drawer(menu, app.menu_open, Msg::ToggleMenu)          // leading edge
//!     .end_drawer(filters, app.drawer_open, Msg::ToggleDrawer)
//!     .persistent_footer(row![cancel, save])  // never scrolls away
//!     .fab(button("＋", Msg::AddTodo))        // floating action button
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

use frus_core::{Color, Insets, SizeClass, WindowInsets};
use frus_layout::Justify;

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

/// An adaptive screen shell. A fluent builder finished by [`Scaffold::build`].
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
    drawer: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    end_drawer: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    fab: Option<Box<dyn Widget<Msg>>>,
    persistent_footer: Option<Box<dyn Widget<Msg>>>,
    persistent_footer_alignment: Justify,
    bottom_sheet: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
}

impl<Msg: Clone + 'static> Scaffold<Msg> {
    /// Creates a shell for a `width × height` surface, in logical pixels. The size
    /// class (rail vs bottom bar) is derived from the width.
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
            drawer: None,
            end_drawer: None,
            fab: None,
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

    /// Enables adaptive navigation: `selected` = the active destination,
    /// `on_select(i)` emitted on choice. Then add [`Scaffold::destination`]s.
    pub fn nav(mut self, selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.selected = selected;
        self.on_select = Some(Box::new(on_select));
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
            drawer,
            end_drawer,
            fab,
            persistent_footer,
            persistent_footer_alignment,
            bottom_sheet,
            view_insets,
            resize_to_avoid_bottom_inset,
            extend_body,
            extend_body_behind_app_bar,
        } = self;

        let compact = SizeClass::from_width(width) == SizeClass::Compact;
        let bg = background.unwrap_or(Color::TRANSPARENT);
        let has_nav = !destinations.is_empty();
        let body_widget = body.unwrap_or_else(|| Box::new(Container::new()));

        // Navigation: a bottom bar (narrow) or a side rail (wide).
        let nav: Option<Box<dyn Widget<Msg>>> = if has_nav {
            let on_select =
                on_select.expect("nav(selected, on_select) is required with destinations");
            if compact {
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

        // The persistent footer: its own row, aligned as asked, kept clear of the side
        // insets. It sits between the body and the bottom bar and never scrolls.
        let footer: Option<Box<dyn Widget<Msg>>> = persistent_footer.map(|widget| {
            // The row is **given** the width it has to fill. A row that hugged its
            // content would leave the alignment nothing to distribute, and every
            // footer would sit at the leading edge whatever it was asked for.
            let rail = if compact { 0.0 } else { RAIL_WIDTH };
            let row_width =
                (width - insets.left - insets.right - rail - FOOTER_PAD * 2.0).max(0.0);
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
        let body_owns_bottom = !extend_body && footer.is_none() && !(compact && has_nav);
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
        let bottom_over_body = extend_body && compact && (footer.is_some() || nav.is_some());
        let app_bar_pad = |bar: Box<dyn Widget<Msg>>, left: f32| {
            inset_pad(bar, insets.top, insets.right, 0.0, left)
        };
        let nav_pad = |n: Box<dyn Widget<Msg>>| {
            inset_pad(n, 0.0, insets.right, bottom_clear, insets.left)
        };

        // The pinned shell: app bar · body · footer · (bottom bar | rail).
        let mut app_bar = app_bar;
        let mut footer = footer;
        let mut nav = nav;
        let main: Box<dyn Widget<Msg>> = if compact {
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

        // The FAB anchored bottom-right, above the bottom bar and the inset.
        let mut content: Box<dyn Widget<Msg>> = main;
        if let Some(fab) = fab {
            let nav_h = if compact && has_nav { BAR_HEIGHT } else { 0.0 };
            let fab_bottom = bottom_clear + nav_h + FAB_MARGIN;
            let fab_layer = Flex::column()
                .width(width)
                .height(height)
                .justify(Justify::End)
                .child(
                    Flex::row().justify(Justify::End).child(
                        Container::new()
                            .padding_each(0.0, insets.right + FAB_MARGIN, fab_bottom, 0.0)
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
                frus_core::Primitive::Rect { rect, color, clip, .. } if *color == MARK => {
                    Some(rect.intersect(*clip))
                }
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
        assert!(
            (centre - (W - 100.0) / 2.0).abs() < 2.0,
            "centre: {centre}"
        );
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

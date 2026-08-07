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
//!     .insets(app.insets)
//!     .background(theme.background)
//!     .app_bar(appbar)                       // pinned at the top
//!     .body(content)                         // scrolls
//!     .nav(app.section, Msg::SetSection)     // navigation adaptative
//!     .destination("✔", "Tasks").badge(3)
//!     .destination("▦", "Stats")
//!     .end_drawer(menu, app.drawer_open, Msg::ToggleDrawer)
//!     .fab(button("＋", Msg::AddTodo))        // floating action button
//!     .bottom_sheet(sheet, app.sheet_open, Msg::ToggleSheet)
//!     .build()
//! ```

use frus_core::{Color, Insets, SizeClass};
use frus_layout::Justify;

use crate::button::Variant;
use crate::container::Container;
use crate::flex::Flex;
use crate::navrail::{BottomBar, NavRail, BAR_HEIGHT};
use crate::scroll::Scroll;
use crate::stack::Stack;
use crate::widget::Widget;

/// The FAB's margin from the edge, and from the bottom bar.
const FAB_MARGIN: f32 = 16.0;

/// An adaptive screen shell. A fluent builder finished by [`Scaffold::build`].
pub struct Scaffold<Msg> {
    width: f32,
    height: f32,
    insets: Insets,
    background: Option<Color>,
    app_bar: Option<Box<dyn Widget<Msg>>>,
    body: Option<Box<dyn Widget<Msg>>>,
    selected: usize,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    destinations: Vec<(String, String, Option<u32>)>,
    end_drawer: Option<(Box<dyn Widget<Msg>>, bool, Msg)>,
    fab: Option<Box<dyn Widget<Msg>>>,
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
            background: None,
            app_bar: None,
            body: None,
            selected: 0,
            on_select: None,
            destinations: Vec::new(),
            end_drawer: None,
            fab: None,
            bottom_sheet: None,
        }
    }

    /// The safe area (system bars): the Scaffold keeps the slots clear of it.
    pub fn insets(mut self, insets: Insets) -> Self {
        self.insets = insets;
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
    /// ⚠ **Experimental**: the FAB is overlaid through a full-screen `Stack` layer,
    /// and such a top layer **intercepts the clicks** of the bottom half of the
    /// screen (a limitation of `Stack` hit-testing). To be fixed (a non-blocking
    /// overlay) before real use — see milestone 52c.
    pub fn fab(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.fab = Some(Box::new(widget));
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
            end_drawer,
            fab,
            bottom_sheet,
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

        // A scrolling body, with the side insets applied to its content.
        let scroll_body = Scroll::new().flex(1.0).child(inset_pad(
            body_widget,
            0.0,
            insets.right,
            0.0,
            insets.left,
        ));

        // The pinned shell: app bar · body · (bottom bar | rail).
        let main: Box<dyn Widget<Msg>> = if compact {
            let mut col = Flex::column().width(width).height(height);
            if let Some(bar) = app_bar {
                col = col.child(inset_pad(bar, insets.top, insets.right, 0.0, insets.left));
            }
            col = col.child(scroll_body);
            if let Some(n) = nav {
                col = col.child(inset_pad(n, 0.0, insets.right, insets.bottom, insets.left));
            }
            Box::new(col)
        } else {
            let mut row = Flex::row().width(width).height(height);
            if let Some(n) = nav {
                row = row.child(inset_pad(n, insets.top, 0.0, insets.bottom, insets.left));
            }
            let mut content = Flex::column().flex(1.0);
            if let Some(bar) = app_bar {
                content = content.child(inset_pad(bar, insets.top, insets.right, 0.0, 0.0));
            }
            content = content.child(scroll_body);
            row = row.child(content);
            Box::new(row)
        };

        // The FAB anchored bottom-right, above the bottom bar and the inset.
        let mut content: Box<dyn Widget<Msg>> = main;
        if let Some(fab) = fab {
            let nav_h = if compact && has_nav { BAR_HEIGHT } else { 0.0 };
            let fab_bottom = insets.bottom + nav_h + FAB_MARGIN;
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

        // The modal drawer, then the modal sheet, wrap the shell as overlays.
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

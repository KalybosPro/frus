//! [`Drawer`]: a retractable **side drawer** — the 3rd tier of Material
//! navigation, alongside `NavRail` (the rail) and `BottomBar` (the bar).
//!
//! Two modes:
//! - **modal** (the default): the body stays visible behind; when the drawer is
//!   open, a full-height panel slides in from an edge (left or right) over it,
//!   with a scrim that closes it on click. Opening is animated automatically
//!   (see milestone 46).
//! - **permanent** ([`Drawer::permanent`]): the panel is **docked** in the flow,
//!   always visible beside the body (no scrim). Typically enabled at the
//!   `Expanded` breakpoint.
//!
//! ```ignore
//! Drawer::new(app.menu_open)
//!     .on_dismiss(Msg::CloseMenu)
//!     .permanent(class == SizeClass::Expanded)
//!     .panel(nav_list)   // the drawer's content
//!     .body(main_screen) // the background content (always visible)
//! ```

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

/// The width of a side drawer, in logical pixels, when the caller has not set one —
/// the reference's own figure, and wide enough for a navigation label not to wrap.
///
/// It is a default, not a constraint: see [`Drawer::width`].
pub const DRAWER_WIDTH: f32 = 304.0;

/// The drawer's inner panel: full height, a themed background, a hairline on the
/// **inner** edge (right for a left drawer, left for a right drawer).
struct DrawerPanel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    /// Draws the hairline on the left edge (a drawer docked to the right).
    border_left: bool,
    /// The panel's width; `None` = the theme's, then [`DRAWER_WIDTH`].
    width: Option<f32>,
}

impl<Msg> DrawerPanel<Msg> {
    /// The width actually used: what the caller said, then the theme, then ours.
    fn resolved_width(&self, theme: Option<&Theme>) -> f32 {
        self.width
            .or_else(|| theme.and_then(|t| t.widgets.drawer.width))
            .unwrap_or(DRAWER_WIDTH)
    }
}

impl<Msg: Clone> Widget<Msg> for DrawerPanel<Msg> {
    fn style(&self) -> Style {
        Style {
            // A panel wider than the window **overflows** rather than shrinking, unlike
            // the reference's, whose width is enforced against the parent's constraints.
            // `max_width: Percent(1.0)` does not fix it: the overlay layer the panel is
            // laid out in has no definite width to take a percentage of. Recorded in
            // milestone 307 rather than patched with something that does nothing.
            width: Dimension::Length(self.resolved_width(None)),
            // The height expands to the whole window (side placement) or to the
            // row's height (permanent mode).
            height: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            width: Dimension::Length(self.resolved_width(Some(theme))),
            ..Widget::<Msg>::style(self)
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // The drawer's opaque surface + a thin hairline on the inner edge.
        scene.fill_rect(bounds, theme.surface.fade(o));
        let x = if self.border_left {
            bounds.x
        } else {
            bounds.x + bounds.width - 1.0
        };
        scene.fill_rect(
            Rect::new(x, bounds.y, 1.0, bounds.height),
            theme.border.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A retractable side drawer: a background body + a panel, modal or docked.
pub struct Drawer<Msg> {
    open: bool,
    right: bool,
    permanent: bool,
    on_dismiss: Option<Msg>,
    /// The drawer's content, supplied by the caller (before `DrawerPanel` wrapping).
    panel_content: Option<Box<dyn Widget<Msg>>>,
    /// The modal panel (non-permanent mode), wrapped, ready for the overlay.
    modal_panel: Option<Box<dyn Widget<Msg>>>,
    /// The panel's width; `None` = [`DRAWER_WIDTH`].
    width: Option<f32>,
    /// Children in the flow: `[body]` (modal) or `[panel, body]` (permanent).
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Drawer<Msg> {
    /// Creates a drawer; `open` says whether it is expanded (ignored when permanent).
    pub fn new(open: bool) -> Self {
        Self {
            open,
            right: false,
            permanent: false,
            on_dismiss: None,
            panel_content: None,
            modal_panel: None,
            width: None,
            children: Vec::new(),
        }
    }

    /// Overrides the panel's width, in logical pixels. Defaults to [`DRAWER_WIDTH`].
    ///
    /// A narrow drawer beside a wide one is a layout decision, not a constant: an
    /// icon-only rail, a settings panel that wants half the window. The default is what
    /// a drawer takes when nobody says otherwise, and nothing more than that.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Message emitted on a click on the scrim (outside the panel) — to close it.
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Docks the drawer to the **right** edge (left by default).
    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }

    /// Makes the drawer **permanent** (docked in the flow, no scrim) when
    /// `permanent` is true — typically at the `Expanded` breakpoint.
    pub fn permanent(mut self, permanent: bool) -> Self {
        self.permanent = permanent;
        self
    }

    /// Sets the **drawer's content**, usually the navigation.
    pub fn panel(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.panel_content = Some(Box::new(content));
        self
    }

    /// Sets the **background body** (always visible) and finalises the drawer.
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        let panel = self.panel_content.take().map(|content| {
            Box::new(DrawerPanel {
                children: vec![content],
                border_left: self.right,
                width: self.width,
            }) as Box<dyn Widget<Msg>>
        });

        if self.permanent {
            // Docked in the flow: a `[panel, body]` row (or the reverse on the right).
            let body_pane: Box<dyn Widget<Msg>> = Box::new(Flex::column().flex(1.0).child(body));
            self.children = match panel {
                Some(panel) if self.right => vec![body_pane, panel],
                Some(panel) => vec![panel, body_pane],
                None => vec![body_pane],
            };
        } else {
            // Modal: only the body is in the flow; the panel goes to the overlay.
            self.modal_panel = panel;
            self.children = vec![Box::new(body)];
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Drawer<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            // Permanent: a row (panel + body side by side). Modal: the body fills
            // on its own (the panel floats as an overlay).
            flex_direction: if self.permanent {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
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

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        // Modal mode only: it is the animated **progress** (`anim_target`)
        // that decides both the display and the slide.
        let placement = if self.right {
            Placement::Right
        } else {
            Placement::Left
        };
        self.modal_panel.as_ref().map(|p| (p.as_ref(), placement))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn anim_target(&self) -> Option<f32> {
        // No animation when permanent (it is always shown). Otherwise the `0↔1`
        // open target, interpolated by the runtime (slide + fade).
        if self.permanent {
            None
        } else {
            Some(if self.open { 1.0 } else { 0.0 })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size, Text};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
    }

    #[test]
    fn anim_target_reflects_open_state() {
        let closed = Drawer::new(false)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(Widget::<Msg>::anim_target(&closed), Some(0.0));
        assert!(Widget::<Msg>::overlay(&closed).is_some());

        let open = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(Widget::<Msg>::anim_target(&open), Some(1.0));
        assert_eq!(Widget::<Msg>::overlay_dismiss(&open), Some(Msg::Close));
    }

    #[test]
    fn right_drawer_uses_right_placement() {
        let d = Drawer::new(true)
            .right()
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert!(matches!(
            Widget::<Msg>::overlay(&d),
            Some((_, Placement::Right))
        ));
    }

    #[test]
    fn closed_drawer_draws_no_scrim() {
        let drawer = Drawer::new(false)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &drawer,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let scrim =
            ui.scene().primitives().iter().any(
                |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0),
            );
        assert!(!scrim, "a closed drawer paints no scrim");
    }

    #[test]
    fn the_panel_is_as_wide_as_it_was_told_to_be() {
        // The default is the reference's 304, and it is a default: an application that
        // wants a narrow panel says so, and gets exactly that.
        let panel_width = |drawer: &dyn Widget<Msg>| {
            let ui = build_ui(
                drawer,
                Size::new(900.0, 400.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { rect, .. } if rect.height >= 399.0 => {
                        Some(rect.width)
                    }
                    _ => None,
                })
                .expect("the panel is a full-height rectangle")
        };
        let default = Drawer::new(true)
            .permanent(true)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(panel_width(&default), DRAWER_WIDTH);
        assert_eq!(DRAWER_WIDTH, 304.0, "the reference's figure");

        let narrow = Drawer::new(true)
            .permanent(true)
            .width(96.0)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(panel_width(&narrow), 96.0);
    }

    #[test]
    fn open_drawer_draws_scrim_and_full_height_panel() {
        let drawer = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &drawer,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let scrim =
            ui.scene().primitives().iter().any(
                |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0),
            );
        assert!(scrim, "the scrim must cover the window");
        let panel = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - DRAWER_WIDTH).abs() < 1.0 && rect.height >= 399.0)
        });
        assert!(panel, "the panel must expand to the full height");
    }

    #[test]
    fn mid_animation_slides_panel_and_fades_scrim() {
        let drawer = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let mut rt = Runtime::default();
        rt.set_value(crate::WidgetId::ROOT, 0.5);
        let ui = build_ui(&drawer, Size::new(500.0, 400.0), &rt, &Theme::default());
        let panel_edge = ui.scene().primitives().iter().find_map(|p| match p {
            frus_core::Primitive::Rect { rect, .. } if (rect.width - DRAWER_WIDTH).abs() < 1.0 => {
                Some(rect.x + rect.width)
            }
            _ => None,
        });
        let edge = panel_edge.expect("the drawer's panel must be present");
        // The slide follows the spring curve: at linear t=0.5, the right edge sits
        // at `spring_ease(0.5)·width`, already well advanced.
        let expected = crate::spring_ease(0.5) * DRAWER_WIDTH;
        assert!(
            (edge - expected).abs() < 2.0,
            "right edge expected ≈ {expected}, got {edge}"
        );
    }

    #[test]
    fn permanent_drawer_docks_panel_beside_body_without_scrim() {
        let drawer = Drawer::new(false) // `open` is ignored when permanent
            .permanent(true)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        // No animation and no overlay: the panel is in the flow.
        assert_eq!(Widget::<Msg>::anim_target(&drawer), None);
        assert!(Widget::<Msg>::overlay(&drawer).is_none());
        // A [panel, body] row.
        assert_eq!(
            Widget::<Msg>::style(&drawer).flex_direction,
            FlexDirection::Row
        );
        assert_eq!(Widget::<Msg>::children(&drawer).len(), 2);

        let ui = build_ui(
            &drawer,
            Size::new(900.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // No full-screen scrim (the panel only covers its own width).
        let scrim =
            ui.scene().primitives().iter().any(
                |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 900.0),
            );
        assert!(!scrim, "a permanent drawer paints no scrim");
        // The docked panel is present, full-height, on the left (x ≈ 0).
        let docked = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - DRAWER_WIDTH).abs() < 1.0 && rect.x < 1.0 && rect.height >= 399.0)
        });
        assert!(docked, "the panel must be docked left, full-height");
    }

    #[test]
    fn permanent_right_docks_panel_on_the_right() {
        let drawer = Drawer::new(false)
            .permanent(true)
            .right()
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        // A [body, panel] order for docking on the right.
        assert_eq!(Widget::<Msg>::children(&drawer).len(), 2);
        let ui = build_ui(
            &drawer,
            Size::new(900.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // A full-height panel whose right edge touches the window (x+w ≈ 900).
        let on_right = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - DRAWER_WIDTH).abs() < 1.0 && (rect.x + rect.width - 900.0).abs() < 1.0)
        });
        assert!(on_right, "the panel must be docked on the right");
    }
}

//! [`Drawer`]: a retractable **side drawer** — the 3rd tier of Material
//! navigation, alongside `NavigationRail` (the rail) and `BottomBar` (the bar).
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
//!
//! Everything it paints is a **default**, not a rule: the fill, the hairline on the
//! inner edge and its thickness, the rounding of that edge, how far off the surface the
//! panel sits, and the scrim behind a modal one — each resolved instance, then
//! [`DrawerTheme`](crate::widgettheme::DrawerTheme), then the scheme's role.
//!
//! ```ignore
//! Drawer::new(open)
//!     .background_color(theme.scheme.surface_container)
//!     .border_width(0.0)          // told apart by colour, not by a rule
//!     .elevation(2.0)             // …or by a shadow along its inner edge
//!     .radius(0.0)                // …or squared off entirely
//!     .scrim_color(Color::TRANSPARENT) // an overlay that darkens nothing
//! ```
//!
//! Which edge is the *inner* one is decided when the panel paints, not when it is
//! built: a leading drawer sits on the left of the screen in English and on the right in
//! Arabic, and the rounding and the hairline follow it across.

use std::cell::{Cell, OnceCell};

use frus_core::{BorderRadius, Color, Rect, Scene, TextDirection};
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

/// The rounding of the panel's **inner** edge — the reference's figure, and the shape
/// its own source calls *shown in the spec*. The outer edge stays square: a panel docked
/// against the window rounds the corners that face the content, not the ones that face
/// nothing.
///
/// A default, not a constraint: see [`Drawer::radius`].
pub const DRAWER_RADIUS: f32 = 16.0;

/// The drawer's inner panel: full height, a themed background, and its **inner** edge
/// rounded and ruled — the edge that faces the content.
///
/// Which edge that is, the panel does not know until it paints. `end` says which side it
/// was docked to in **logical** terms (leading or trailing), and the direction in force
/// turns that into a side of the screen: a leading drawer is on the left in LTR and on
/// the right in RTL, and its inner edge follows.
struct DrawerPanel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    /// Docked to the **trailing** edge ([`Drawer::right`]) rather than the leading one.
    end: bool,
    /// The panel's width; `None` = the theme's, then [`DRAWER_WIDTH`].
    width: Option<f32>,
    style: PanelStyle,
}

/// Everything the panel paints that a caller can name, gathered so [`Drawer`]'s builders
/// have one place to put it. Every field is `None` for *ask the theme, then the default*.
#[derive(Clone, Copy, Default)]
struct PanelStyle {
    background_color: Option<Color>,
    border_color: Option<Color>,
    border_width: Option<f32>,
    radius: Option<f32>,
    elevation: Option<f32>,
}

impl<Msg> DrawerPanel<Msg> {
    /// The width actually used: what the caller said, then the theme, then ours.
    fn resolved_width(&self, theme: Option<&Theme>) -> f32 {
        self.width
            .or_else(|| theme.and_then(|t| t.widgets.drawer.width))
            .unwrap_or(DRAWER_WIDTH)
    }

    /// Whether the panel has landed on the **right** of the screen.
    ///
    /// The trailing edge is the right one in LTR and the left one in RTL, and a panel
    /// that guessed would rule its hairline down the window's own edge in Arabic.
    fn docked_right(&self, theme: &Theme) -> bool {
        self.end != (theme.direction == TextDirection::Rtl)
    }

    /// The rounding, on the two corners of the inner edge only.
    fn radius(&self, theme: &Theme) -> BorderRadius {
        let r = self
            .style
            .radius
            .or(theme.widgets.drawer.radius)
            .unwrap_or(DRAWER_RADIUS);
        match self.docked_right(theme) {
            true => BorderRadius::left(r),
            false => BorderRadius::right(r),
        }
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
        let radius = self.radius(theme);
        let right = self.docked_right(theme);
        let depth = self
            .style
            .elevation
            .or(theme.widgets.drawer.elevation)
            .unwrap_or(0.0);

        // The shadow, when the caller has lifted the panel off the surface at all. The
        // drop is **sideways** rather than down: a panel is lifted along its inner edge,
        // and a shadow cast below a full-height panel falls outside the window entirely.
        if depth > 0.0 {
            let blur = depth * 4.0 + 8.0;
            let sideways = if right { -depth } else { depth } * 2.0;
            scene.shadow(
                Rect::new(
                    bounds.x + sideways - blur,
                    bounds.y - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                theme.scheme.shadow.with_alpha(0.30).fade(o),
                radius.inflate(blur),
                blur,
            );
        }

        let fill = self
            .style
            .background_color
            .or(theme.widgets.drawer.background_color)
            .unwrap_or(theme.surface);
        scene.draw_rect(bounds, fill.fade(o), radius, 0.0, Color::TRANSPARENT);

        // The hairline on the inner edge, drawn as its own sliver rather than as a border
        // round the whole shape: the other three edges are flush against the window, and a
        // rule down them would be a line against nothing. It stops short of the corners
        // for the same reason — a straight sliver crossing a rounded corner sticks out
        // past the shape it is meant to edge.
        let thickness = self
            .style
            .border_width
            .or(theme.widgets.drawer.border_width)
            .unwrap_or(1.0);
        if thickness > 0.0 {
            let color = self
                .style
                .border_color
                .or(theme.widgets.drawer.border_color)
                .unwrap_or(theme.scheme.outline_variant);
            let x = match right {
                true => bounds.x,
                false => bounds.x + bounds.width - thickness,
            };
            let inset = radius
                .top_left
                .max(radius.top_right)
                .min(bounds.height / 2.0);
            scene.fill_rect(
                Rect::new(x, bounds.y + inset, thickness, bounds.height - 2.0 * inset),
                color.fade(o),
            );
        }
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
    panel_content: Cell<Option<Box<dyn Widget<Msg>>>>,
    /// The background content, likewise.
    body_content: Cell<Option<Box<dyn Widget<Msg>>>>,
    /// The panel's width; `None` = [`DRAWER_WIDTH`].
    width: Option<f32>,
    /// Everything else the panel paints; each `None` = the theme's, then the default.
    style: PanelStyle,
    /// The scrim's colour behind a modal panel, alpha included; `None` = the scheme's.
    scrim_color: Option<Color>,
    /// The assembled tree, built once, the first time the walk asks for it.
    assembled: OnceCell<Assembled<Msg>>,
}

/// What the two pieces the caller handed over become, once every builder has had its
/// say: the row (or the lone body) in the flow, and the panel that floats.
struct Assembled<Msg> {
    /// Children in the flow: `[body]` (modal) or `[panel, body]` (permanent).
    children: Vec<Box<dyn Widget<Msg>>>,
    /// The modal panel (non-permanent mode), wrapped, ready for the overlay.
    modal_panel: Option<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Drawer<Msg> {
    /// Creates a drawer; `open` says whether it is expanded (ignored when permanent).
    pub fn new(open: bool) -> Self {
        Self {
            open,
            right: false,
            permanent: false,
            on_dismiss: None,
            panel_content: Cell::new(None),
            body_content: Cell::new(None),
            width: None,
            style: PanelStyle::default(),
            scrim_color: None,
            assembled: OnceCell::new(),
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

    /// The panel's fill. Defaults to the theme's `drawer.background_color`, then to the
    /// theme's surface.
    pub fn background_color(mut self, color: Color) -> Self {
        self.style.background_color = Some(color);
        self
    }

    /// The hairline on the panel's **inner** edge — the one facing the content.
    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self
    }

    /// That hairline's thickness. `0.0` removes it, which is what a drawer told apart
    /// from its body by colour or by a shadow wants.
    pub fn border_width(mut self, width: f32) -> Self {
        self.style.border_width = Some(width);
        self
    }

    /// The rounding of the inner edge's two corners; the outer edge stays square against
    /// the window. Defaults to [`DRAWER_RADIUS`], and `0.0` squares it off.
    ///
    /// Which edge is the inner one is decided **at paint time** from the direction in
    /// force, not here: a leading drawer sits on the left of the screen in English and on
    /// the right in Arabic, and the rounding follows it across.
    pub fn radius(mut self, radius: f32) -> Self {
        self.style.radius = Some(radius);
        self
    }

    /// How far off the surface the panel sits. `0.0` — the default — casts no shadow.
    ///
    /// The drop is sideways, along the inner edge: a shadow cast below a panel as tall as
    /// the window falls outside it and is never seen.
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.style.elevation = Some(elevation);
        self
    }

    /// The scrim behind a **modal** panel, **alpha included** — a scrim's transparency
    /// is the colour, so an opaque value here hides the body entirely and
    /// [`Color::TRANSPARENT`] darkens nothing at all.
    ///
    /// Ignored by a permanent drawer, which has no scrim to colour.
    pub fn scrim_color(mut self, color: Color) -> Self {
        self.scrim_color = Some(color);
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
    pub fn panel(self, content: impl Widget<Msg> + 'static) -> Self {
        self.panel_content.set(Some(Box::new(content)));
        self
    }

    /// Sets the **background body**: the content that stays visible behind a modal
    /// panel, or beside a docked one.
    ///
    /// It used to *finalise* the drawer, wrapping the panel there and then — which
    /// quietly made every builder called after it a no-op, since the panel it would have
    /// changed already existed. The assembly now happens the first time the walk asks
    /// for the children, so the builders can come in any order.
    pub fn body(self, body: impl Widget<Msg> + 'static) -> Self {
        self.body_content.set(Some(Box::new(body)));
        self
    }

    /// The tree, assembled on first use from the two pieces the caller handed over.
    fn assembled(&self) -> &Assembled<Msg> {
        self.assembled.get_or_init(|| {
            let panel = self.panel_content.take().map(|content| {
                Box::new(DrawerPanel {
                    children: vec![content],
                    end: self.right,
                    width: self.width,
                    style: self.style,
                }) as Box<dyn Widget<Msg>>
            });
            let Some(body) = self.body_content.take() else {
                // A drawer with no body: the panel alone, docked or floating. Nothing to
                // put in the flow, and nothing to fill the window with either.
                return Assembled {
                    children: Vec::new(),
                    modal_panel: panel.filter(|_| !self.permanent),
                };
            };

            if self.permanent {
                // Docked in the flow: a `[panel, body]` row (or the reverse on the right).
                let body_pane: Box<dyn Widget<Msg>> =
                    Box::new(Flex::column().flex(1.0).child_boxed(body));
                Assembled {
                    children: match panel {
                        Some(panel) if self.right => vec![body_pane, panel],
                        Some(panel) => vec![panel, body_pane],
                        None => vec![body_pane],
                    },
                    modal_panel: None,
                }
            } else {
                // Modal: only the body is in the flow; the panel goes to the overlay.
                Assembled {
                    children: vec![body],
                    modal_panel: panel,
                }
            }
        })
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Drawer<Msg> {
    /// It asks to **fill the width it is offered** rather than declaring one — see
    /// [`Widget::fill_axes`]. A `width: 100%` resolves against the parent's *resolved*
    /// width, which a parent that shrink-wraps does not have yet.
    fn style(&self) -> Style {
        Style {
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

    /// The width it was **offered**, not the width its parent came out at.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::WIDTH
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.assembled().children
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
        self.assembled()
            .modal_panel
            .as_ref()
            .map(|p| (p.as_ref(), placement))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn overlay_scrim(&self, theme: &Theme) -> Option<Color> {
        self.scrim_color.or(theme.widgets.drawer.scrim_color)
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

    /// The panel's rectangle: the full-height one as wide as the drawer.
    fn panel_rect(drawer: &dyn Widget<Msg>, theme: &Theme) -> frus_core::Primitive {
        let ui = build_ui(drawer, Size::new(900.0, 400.0), &Runtime::default(), theme);
        ui.scene()
            .primitives()
            .iter()
            .find(|p| {
                matches!(p, frus_core::Primitive::Rect { rect, radius, blur, .. }
                    if (rect.height - 400.0).abs() < 1.0
                        && *blur == 0.0
                        && *radius != frus_core::BorderRadius::ZERO)
            })
            .cloned()
            .expect("the panel is a full-height rounded rectangle")
    }

    /// The inner edge is rounded and the outer one is square. A panel docked against the
    /// window rounds the corners that face the content; rounding the pair against the
    /// window edge would cut two notches out of the screen.
    #[test]
    fn the_inner_edge_is_rounded_and_the_outer_one_is_not() {
        let theme = Theme::default();
        let leading = Drawer::new(false)
            .permanent(true)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let frus_core::Primitive::Rect { radius, .. } = panel_rect(&leading, &theme) else {
            panic!("a rectangle");
        };
        assert_eq!(radius, frus_core::BorderRadius::right(DRAWER_RADIUS));
        assert_eq!(DRAWER_RADIUS, 16.0, "the reference's figure");

        let trailing = Drawer::new(false)
            .permanent(true)
            .right()
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let frus_core::Primitive::Rect { radius, .. } = panel_rect(&trailing, &theme) else {
            panic!("a rectangle");
        };
        assert_eq!(radius, frus_core::BorderRadius::left(DRAWER_RADIUS));
    }

    /// **The bug this milestone found.** A leading drawer is on the left in English and
    /// on the **right** in Arabic — the walk mirrors the whole frame — so its inner
    /// edge changes sides with it. The panel used to rule its hairline and round its
    /// corners from `right`, the side it was *asked* for, and in RTL that put both down
    /// the window's own edge.
    #[test]
    fn the_inner_edge_follows_the_direction_rather_than_the_side_it_was_asked_for() {
        let rtl = Theme::default().rtl();
        let leading = Drawer::new(false)
            .permanent(true)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let frus_core::Primitive::Rect { rect, radius, .. } = panel_rect(&leading, &rtl) else {
            panic!("a rectangle");
        };
        // Mirrored: a leading panel sits against the *right* of the window…
        assert!(
            (rect.x + rect.width - 900.0).abs() < 1.0,
            "a leading panel is on the right in RTL, got x={}",
            rect.x
        );
        // …so the edge facing the content is its left one.
        assert_eq!(radius, frus_core::BorderRadius::left(DRAWER_RADIUS));

        // And the hairline is on that same edge, not on the window's.
        let ui = build_ui(&leading, Size::new(900.0, 400.0), &Runtime::default(), &rtl);
        let hairline = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, .. } if rect.width <= 1.5 => Some(rect.x),
                _ => None,
            })
            .expect("a hairline");
        assert!(
            (hairline - rect.x).abs() < 1.5,
            "the hairline belongs on the inner edge (x≈{}), got {hairline}",
            rect.x
        );
    }

    /// The colours resolve **instance, then theme, then the scheme's role**.
    #[test]
    fn the_panels_colours_are_the_instances_then_the_themes() {
        let mut theme = Theme::default();
        let fill = |drawer: &dyn Widget<Msg>, theme: &Theme| match panel_rect(drawer, theme) {
            frus_core::Primitive::Rect { color, .. } => color,
            _ => unreachable!(),
        };
        let plain = || {
            Drawer::new(false)
                .permanent(true)
                .panel(Text::new("menu"))
                .body(Container::<Msg>::new())
        };
        assert_eq!(fill(&plain(), &theme), theme.surface, "the theme's surface");

        theme.widgets.drawer.background_color = Some(Color::rgb(0.0, 1.0, 0.0));
        assert_eq!(
            fill(&plain(), &theme),
            Color::rgb(0.0, 1.0, 0.0),
            "the theme"
        );

        // Deliberately *after* `body()`: see `a_builder_after_body_is_not_a_no_op`.
        let own = plain().background_color(Color::rgb(0.0, 0.0, 1.0));
        assert_eq!(
            fill(&own, &theme),
            Color::rgb(0.0, 0.0, 1.0),
            "the instance wins"
        );
    }

    /// A hairline of nothing is no hairline: a drawer told apart from its body by colour
    /// or by a shadow does not want a rule as well.
    #[test]
    fn a_zero_border_removes_the_hairline() {
        let bare = Drawer::new(false)
            .permanent(true)
            .border_width(0.0)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &bare,
            Size::new(900.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let slivers = ui
            .scene()
            .primitives()
            .iter()
            .filter(|p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width <= 1.5))
            .count();
        assert_eq!(slivers, 0, "no rule at all");
    }

    /// The scrim is the drawer's to colour — alpha included, since a scrim's
    /// transparency *is* the colour.
    #[test]
    fn the_scrim_is_the_drawers_to_colour() {
        let window = |theme: &Theme, drawer: &dyn Widget<Msg>| {
            let ui = build_ui(drawer, Size::new(500.0, 400.0), &Runtime::default(), theme);
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { rect, color, .. } if rect.width >= 500.0 => {
                        Some(*color)
                    }
                    _ => None,
                })
                .expect("the scrim covers the window")
        };
        let theme = Theme::default();
        let plain = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(window(&theme, &plain), theme.scheme.scrim.with_alpha(0.5));

        let own = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .scrim_color(Color::rgba(1.0, 0.0, 0.0, 0.25))
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(window(&theme, &own), Color::rgba(1.0, 0.0, 0.0, 0.25));

        // Transparent is an answer: an overlay that darkens nothing.
        let clear = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .scrim_color(Color::TRANSPARENT)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(window(&theme, &clear).a, 0.0);
    }

    /// The scrim fades **with the slide**: at half the animation it is half painted,
    /// whatever colour it was given.
    #[test]
    fn the_scrim_fades_with_the_slide() {
        let drawer = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .scrim_color(Color::rgba(0.0, 0.0, 0.0, 0.8))
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let mut rt = Runtime::default();
        rt.set_value(crate::WidgetId::ROOT, 0.5);
        let ui = build_ui(&drawer, Size::new(500.0, 400.0), &rt, &Theme::default());
        let alpha = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. } if rect.width >= 500.0 => {
                    Some(color.a)
                }
                _ => None,
            })
            .expect("a scrim");
        let expected = 0.8 * crate::spring_ease(0.5);
        assert!(
            (alpha - expected).abs() < 0.02,
            "expected ≈ {expected}, got {alpha}"
        );
    }

    /// Elevation casts a shadow **sideways**, along the inner edge: one dropped below a
    /// panel as tall as the window falls outside it and is never seen.
    #[test]
    fn elevation_casts_the_shadow_along_the_inner_edge() {
        let shadows = |drawer: &dyn Widget<Msg>| {
            let ui = build_ui(
                drawer,
                Size::new(900.0, 400.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Rect { rect, blur, .. } if *blur > 0.0 => Some(rect.x),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let flat = Drawer::new(false)
            .permanent(true)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert!(shadows(&flat).is_empty(), "no shadow by default");

        let lifted = Drawer::new(false)
            .permanent(true)
            .elevation(3.0)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let cast = shadows(&lifted);
        assert_eq!(cast.len(), 1, "one shadow");
        // A leading panel is lifted towards the content: the shadow's envelope starts to
        // the right of the panel's own left edge, offset by the depth.
        let blur = 3.0 * 4.0 + 8.0;
        assert!(
            (cast[0] - (6.0 - blur)).abs() < 0.5,
            "expected ≈ {}, got {}",
            6.0 - blur,
            cast[0]
        );
    }

    /// **The second bug this milestone found.** `body()` used to finalise the drawer,
    /// wrapping the panel there and then — so a builder that came after it changed a
    /// field nobody would read again, and did nothing at all without saying so. Written
    /// in the order a caller reaches for, the panel is 96 px wide and blue either way.
    #[test]
    fn a_builder_after_body_is_not_a_no_op() {
        let theme = Theme::default();
        let measured = |drawer: &dyn Widget<Msg>| match panel_rect(drawer, &theme) {
            frus_core::Primitive::Rect { rect, color, .. } => (rect.width, color),
            _ => unreachable!(),
        };
        let before = Drawer::new(false)
            .permanent(true)
            .width(96.0)
            .background_color(Color::rgb(0.0, 0.0, 1.0))
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let after = Drawer::new(false)
            .permanent(true)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new())
            .width(96.0)
            .background_color(Color::rgb(0.0, 0.0, 1.0));
        assert_eq!(measured(&before), measured(&after));
        assert_eq!(measured(&after), (96.0, Color::rgb(0.0, 0.0, 1.0)));
    }
}

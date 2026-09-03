//! [`BottomSheet`]: a **modal sheet** that slides up from the bottom of the
//! window — for a set of contextual actions or a short form, without leaving
//! the current screen.
//!
//! The body stays visible behind; when the sheet is open, a full-width panel
//! rises from the bottom edge over it, with a scrim that closes it on click.
//! The slide is animated automatically (a spring curve, like the drawer —
//! milestones 46/48), with no wiring on the application side.
//!
//! ```ignore
//! BottomSheet::new(app.sheet_open)
//!     .on_dismiss(Msg::CloseSheet)
//!     .sheet(actions_column)  // the sheet's content
//!     .body(main_screen)      // the background content (always visible)
//! ```

use frus_core::{Color, Insets, Rect, Scene, ShapeBorder};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

/// The grabber at the top of the sheet: width and height in logical pixels.
const GRABBER_WIDTH: f32 = 36.0;
const GRABBER_HEIGHT: f32 = 4.0;

/// The sheet's inner panel: full width, natural height, a themed background,
/// a hairline and a grabber at the top.
struct SheetPanel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    /// The caller's surface colour, if one was named.
    background: Option<Color>,
    /// The caller's shape, if one was named.
    shape: Option<ShapeBorder>,
}

impl<Msg> SheetPanel<Msg> {
    /// **What shape the sheet is**: the caller's word, then the theme's shape, then the
    /// theme's plain radius on the **top** two corners, then the framework's own.
    ///
    /// The bottom edge is flush against the window: rounding it would cut two notches out
    /// of the screen, so the framework's default rounds the top pair and nothing else.
    fn shape_of(&self, theme: &Theme) -> ShapeBorder {
        crate::resolve_shape(
            self.shape,
            theme.widgets.bottom_sheet.shape,
            theme
                .widgets
                .bottom_sheet
                .radius
                .map(frus_core::BorderRadius::top),
            ShapeBorder::rounded(frus_core::BorderRadius::top(theme.radius + 6.0)),
        )
    }
}

impl<Msg: Clone> Widget<Msg> for SheetPanel<Msg> {
    /// It asks to **fill the width it is offered** rather than declaring one — see
    /// [`Widget::fill_axes`]. A `width: 100%` resolves against the parent's *resolved*
    /// width, which a parent that shrink-wraps does not have yet.
    fn style(&self) -> Style {
        Style {
            // Natural height: the content sets the height and the sheet adjusts to it.
            height: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            // Top padding to let the grabber breathe above the content.
            padding: Insets::new(20.0, 0.0, 0.0, 0.0),
            ..Default::default()
        }
    }

    /// The width it was **offered**, not the width its parent came out at.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::WIDTH
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // An opaque surface with rounded **top** corners (the bottom edge is flush
        // with the window) + a thin top hairline, inset from the rounding.
        let shape = self.shape_of(theme);
        // The corners that shape resolves to — what the fill takes, and what the hairline
        // below is inset by so it does not cross a curve.
        let radius = shape
            .as_rounded(bounds)
            .map(|(_, r)| r)
            .unwrap_or(frus_core::BorderRadius::ZERO);
        let inset = radius.top_left.max(radius.top_right);
        // `bottom_sheet.dart:1496` — a sheet rises off the page, on the low rung.
        let fill = self
            .background
            .or(theme.widgets.bottom_sheet.background_color)
            .unwrap_or(theme.scheme.surface_container_low);
        scene.draw_shape(bounds, shape, fill.fade(o));
        scene.fill_rect(
            Rect::new(
                bounds.x + inset,
                bounds.y,
                (bounds.width - 2.0 * inset).max(0.0),
                1.0,
            ),
            theme.scheme.outline_variant.fade(o),
        );
        // A rounded grabber, centred near the top.
        let gx = bounds.x + (bounds.width - GRABBER_WIDTH) * 0.5;
        let gy = bounds.y + 8.0;
        scene.draw_rect(
            Rect::new(gx, gy, GRABBER_WIDTH, GRABBER_HEIGHT),
            theme.muted.fade(0.5 * o),
            GRABBER_HEIGHT * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A modal sheet sliding up from the bottom: a background body + a retractable panel.
pub struct BottomSheet<Msg> {
    open: bool,
    on_dismiss: Option<Msg>,
    /// The sheet's content, supplied by the caller (before `SheetPanel` wrapping).
    sheet_content: Option<Box<dyn Widget<Msg>>>,
    /// The wrapped modal panel, ready for the overlay.
    modal_panel: Option<Box<dyn Widget<Msg>>>,
    /// The sheet's surface, over the theme's and the framework's.
    background: Option<Color>,
    /// The sheet's shape, over the theme's and the framework's.
    shape: Option<ShapeBorder>,
    /// Children in the flow: `[body]` (the panel floats as an overlay).
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> BottomSheet<Msg> {
    /// Creates a sheet; `open` says whether it is expanded.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            on_dismiss: None,
            sheet_content: None,
            modal_panel: None,
            background: None,
            shape: None,
            children: Vec::new(),
        }
    }

    /// The sheet's surface. Unset, the theme's, then the scheme's
    /// `surface_container_low`.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// **What shape the sheet is** — the reference's `shape`
    /// (`BottomSheetThemeData.shape`), over the theme's.
    ///
    /// A sheet's bottom edge is flush against the window, so the framework rounds the top
    /// pair only. A caller naming a shape has taken that decision on: a
    /// `ShapeBorder::rounded(28.0)` rounds all four, and two of them are off-screen.
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Message emitted on a click on the scrim (outside the sheet) — to close it.
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Sets the **sheet's content** (actions, a short form…).
    pub fn sheet(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.sheet_content = Some(Box::new(content));
        self
    }

    /// Sets the **background body** (always visible) and finalises the sheet.
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        let background = self.background;
        let shape = self.shape;
        self.modal_panel = self.sheet_content.take().map(|content| {
            Box::new(SheetPanel {
                children: vec![content],
                background,
                shape,
            }) as Box<dyn Widget<Msg>>
        });
        self.children = vec![Box::new(body)];
        self
    }
}

impl<Msg: Clone> Widget<Msg> for BottomSheet<Msg> {
    /// It asks to **fill the width it is offered** rather than declaring one — see
    /// [`Widget::fill_axes`]. A `width: 100%` resolves against the parent's *resolved*
    /// width, which a parent that shrink-wraps does not have yet.
    fn style(&self) -> Style {
        Style {
            height: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    /// The width it was **offered**, not the width its parent came out at.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::WIDTH
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        // It is the animated **progress** (`anim_target`) that decides both the
        // display and the upward slide.
        self.modal_panel
            .as_ref()
            .map(|p| (p.as_ref(), Placement::Bottom))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn anim_target(&self) -> Option<f32> {
        // The `0↔1` open target, interpolated by the runtime (slide + fade).
        Some(if self.open { 1.0 } else { 0.0 })
    }
}

#[cfg(test)]
mod tests {
    /// **A sheet takes a shape**, which it could not: its corner was `theme.radius + 6.0`,
    /// an expression nobody chose, with no way for a caller or a theme to say otherwise.
    ///
    /// The framework's own default still rounds the **top** pair only — the bottom edge is
    /// flush against the window, and rounding it would cut two notches out of the screen —
    /// so the pixels of a sheet that says nothing are what they were.
    #[test]
    fn a_sheet_takes_a_shape() {
        let corners = |sheet: BottomSheet<Msg>, theme: &Theme| {
            let ui = build_ui(&sheet, Size::new(400.0, 600.0), &Runtime::default(), theme);
            fn find(primitives: &[frus_core::Primitive]) -> Option<frus_core::BorderRadius> {
                for p in primitives {
                    match p {
                        frus_core::Primitive::Rect { rect, radius, .. }
                            if rect.width >= 399.0 && *radius != frus_core::BorderRadius::ZERO =>
                        {
                            return Some(*radius)
                        }
                        frus_core::Primitive::Layer { primitives, .. } => {
                            if let Some(found) = find(primitives) {
                                return Some(found);
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            find(ui.scene().primitives())
        };

        let theme = Theme::default();
        // `body` is what wraps the panel, so anything the panel reads has to be said
        // **before** it — the same ordering `background` has always had.
        let open = |shape: Option<frus_core::ShapeBorder>| {
            let mut sheet =
                BottomSheet::<Msg>::new(true).sheet(Container::<Msg>::new().height(120.0));
            if let Some(shape) = shape {
                sheet = sheet.shape(shape);
            }
            sheet.body(Container::<Msg>::new())
        };
        assert_eq!(
            corners(open(None), &theme),
            Some(frus_core::BorderRadius::top(theme.radius + 6.0)),
            "the framework's own, unchanged"
        );
        assert_eq!(
            corners(
                open(Some(frus_core::ShapeBorder::rounded(
                    frus_core::BorderRadius::top(28.0),
                ))),
                &theme
            ),
            Some(frus_core::BorderRadius::top(28.0)),
            "the caller's"
        );

        let mut themed = Theme::default();
        themed.widgets.bottom_sheet.radius = Some(20.0);
        assert_eq!(
            corners(open(None), &themed),
            Some(frus_core::BorderRadius::top(20.0)),
            "a theme's plain radius, on the top pair"
        );
        themed.widgets.bottom_sheet.shape = Some(frus_core::ShapeBorder::rounded(
            frus_core::BorderRadius::top(4.0),
        ));
        assert_eq!(
            corners(open(None), &themed),
            Some(frus_core::BorderRadius::top(4.0)),
            "and a theme's shape outranks its radius"
        );
    }
    use super::*;
    use crate::{build_ui, Container, Runtime, Size, Text};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
    }

    #[test]
    fn anim_target_reflects_open_state() {
        let closed = BottomSheet::new(false)
            .on_dismiss(Msg::Close)
            .sheet(Text::new("actions"))
            .body(Container::<Msg>::new());
        assert_eq!(Widget::<Msg>::anim_target(&closed), Some(0.0));
        assert!(Widget::<Msg>::overlay(&closed).is_some());

        let open = BottomSheet::new(true)
            .on_dismiss(Msg::Close)
            .sheet(Text::new("actions"))
            .body(Container::<Msg>::new());
        assert_eq!(Widget::<Msg>::anim_target(&open), Some(1.0));
        assert_eq!(Widget::<Msg>::overlay_dismiss(&open), Some(Msg::Close));
    }

    #[test]
    fn sheet_uses_bottom_placement() {
        let s = BottomSheet::new(true)
            .sheet(Text::new("actions"))
            .body(Container::<Msg>::new());
        assert!(matches!(
            Widget::<Msg>::overlay(&s),
            Some((_, Placement::Bottom))
        ));
    }

    #[test]
    fn closed_sheet_draws_no_scrim() {
        let sheet = BottomSheet::new(false)
            .on_dismiss(Msg::Close)
            .sheet(Text::new("actions"))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &sheet,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let scrim = ui.scene().primitives().iter().any(
            |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0 && rect.height >= 400.0),
        );
        assert!(!scrim, "a closed sheet paints no scrim");
    }

    #[test]
    fn open_sheet_draws_scrim_and_full_width_panel() {
        // Fixed-height content, so the sheet has a measurable height.
        let sheet = BottomSheet::new(true)
            .on_dismiss(Msg::Close)
            .sheet(Container::<Msg>::new().height(120.0))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &sheet,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let scrim = ui.scene().primitives().iter().any(
            |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0 && rect.height >= 399.0),
        );
        assert!(scrim, "the scrim must cover the window");
        // The full-width panel (h ≈ 140, not the scrim), flush with the bottom (y+h ≈ 400).
        let docked = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - 500.0).abs() < 1.0 && rect.height < 300.0
                    && (rect.y + rect.height - 400.0).abs() < 1.0)
        });
        assert!(
            docked,
            "the panel must be full-width and docked at the bottom"
        );
    }

    #[test]
    fn mid_animation_slides_sheet_up() {
        let sheet = BottomSheet::new(true)
            .on_dismiss(Msg::Close)
            .sheet(Container::<Msg>::new().height(120.0))
            .body(Container::<Msg>::new());
        let mut rt = Runtime::default();
        rt.set_value(crate::WidgetId::ROOT, 0.5);
        let ui = build_ui(&sheet, Size::new(500.0, 400.0), &rt, &Theme::default());
        // The top edge of the full-width panel (height ≈ 140, not the 500×400 scrim).
        let top = ui.scene().primitives().iter().find_map(|p| match p {
            frus_core::Primitive::Rect { rect, .. }
                if (rect.width - 500.0).abs() < 1.0 && rect.height < 300.0 =>
            {
                Some(rect.y)
            }
            _ => None,
        });
        let top = top.expect("the sheet's panel must be present");
        // At linear t=0.5, the sheet has risen by `spring_ease(0.5)·height` from the
        // bottom: top edge ≈ 400 − spring_ease(0.5)·140 (120 + 20 of padding).
        let progress = crate::spring_ease(0.5);
        let sheet_h = 140.0;
        let expected = 400.0 - progress * sheet_h;
        assert!(
            (top - expected).abs() < 2.0,
            "top edge expected ≈ {expected}, got {top}"
        );
    }
}

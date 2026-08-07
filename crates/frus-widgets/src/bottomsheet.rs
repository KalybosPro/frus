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

use frus_core::{Color, Insets, Rect, Scene};
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
}

impl<Msg: Clone> Widget<Msg> for SheetPanel<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            // Natural height: the content sets the height and the sheet adjusts to it.
            height: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            // Top padding to let the grabber breathe above the content.
            padding: Insets::new(20.0, 0.0, 0.0, 0.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // An opaque surface with rounded **top** corners (the bottom edge is flush
        // with the window) + a thin top hairline, inset from the rounding.
        let radius = theme.radius + 6.0;
        scene.draw_rect(
            bounds,
            theme.surface.fade(o),
            frus_core::BorderRadius::top(radius),
            0.0,
            Color::TRANSPARENT,
        );
        scene.fill_rect(
            Rect::new(
                bounds.x + radius,
                bounds.y,
                (bounds.width - 2.0 * radius).max(0.0),
                1.0,
            ),
            theme.border.fade(o),
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
            children: Vec::new(),
        }
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
        self.modal_panel = self.sheet_content.take().map(|content| {
            Box::new(SheetPanel {
                children: vec![content],
            }) as Box<dyn Widget<Msg>>
        });
        self.children = vec![Box::new(body)];
        self
    }
}

impl<Msg: Clone> Widget<Msg> for BottomSheet<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Column,
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
            "bord haut attendu ≈ {expected}, obtenu {top}"
        );
    }
}

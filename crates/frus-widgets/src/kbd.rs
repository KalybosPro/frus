//! [`Kbd`]: a **keyboard key** cap, used as a shortcut hint.

use frus_core::{Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const SIZE: f32 = 13.0;

/// The style this widget's text is drawn in, **resolved once** so that the number the box
/// is measured with is the number the glyphs are drawn at. Resolving is the single place
/// the reader's font setting is applied (milestone 403); a size that never passes through
/// it is a size the reader cannot change.
fn label_style() -> ResolvedTextStyle {
    TextStyle::new(SIZE).resolved()
}

const PAD_X: f32 = 7.0;
const PAD_Y: f32 = 3.0;

/// A key cap showing a label, such as "Ctrl" or "Enter".
pub struct Kbd {
    label: String,
}

impl Kbd {
    /// Creates a key cap with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl<Msg> Widget<Msg> for Kbd {
    fn style(&self) -> Style {
        let measured = frus_text::measure_resolved(&self.label, &label_style());
        Style {
            width: Dimension::Length((measured.width + PAD_X * 2.0).ceil()),
            height: Dimension::Length((measured.height + PAD_Y * 2.0).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        scene.draw_rect(
            bounds,
            theme.surface.fade(o),
            5.0,
            1.0,
            theme.border.fade(o),
        );
        scene.text(
            Point::new(bounds.x + PAD_X, bounds.y + PAD_Y),
            self.label.clone(),
            &label_style(),
            theme.muted.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[test]
    fn paints_cap_and_label() {
        let kbd = Kbd::new("Ctrl");
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &kbd,
            Rect::new(0.0, 0.0, 40.0, 20.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { border_width, .. } if *border_width > 0.0)));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Ctrl")));
    }
}

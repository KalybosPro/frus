//! [`Badge`] : une petite pastille (compteur ou étiquette), aux couleurs d'accent.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 2.0;
const SIZE: f32 = 13.0;

/// Une pastille compacte affichant un court texte.
pub struct Badge {
    text: String,
}

impl Badge {
    /// Crée une pastille avec le texte donné.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl<Msg> Widget<Msg> for Badge {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.text, SIZE);
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
        // Pastille (pilule) d'accent.
        scene.draw_rect(bounds, theme.primary.fade(o), bounds.height * 0.5, 0.0, Color::TRANSPARENT);
        scene.text(
            Point::new(bounds.x + PAD_X, bounds.y + PAD_Y),
            self.text.clone(),
            SIZE,
            theme.on_primary.fade(o),
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
    fn paints_pill_and_text() {
        let badge = Badge::new("3");
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &badge,
            Rect::new(0.0, 0.0, 24.0, 18.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        // Une pastille (rect) + le texte.
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { .. })));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "3")));
    }
}

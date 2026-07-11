//! [`Toast`] : une notification transitoire (carte stylée). Le *système*
//! (empilement, minuterie d'auto-fermeture) est du ressort de l'application
//! (typiquement via un `Command` minuté).

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 16.0;
const PAD_Y: f32 = 12.0;
const SIZE: f32 = 16.0;
const ACCENT: f32 = 4.0;

/// Nature d'une notification (couleur d'accent).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

/// Une notification transitoire.
pub struct Toast {
    text: String,
    kind: ToastKind,
}

impl Toast {
    /// Crée une notification d'information.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: ToastKind::Info,
        }
    }

    /// Variante succès.
    pub fn success(mut self) -> Self {
        self.kind = ToastKind::Success;
        self
    }

    /// Variante erreur.
    pub fn error(mut self) -> Self {
        self.kind = ToastKind::Error;
        self
    }

    fn accent(&self, theme: &Theme) -> Color {
        match self.kind {
            ToastKind::Info => theme.primary,
            ToastKind::Success => Color::rgb8(70, 190, 120),
            ToastKind::Error => Color::rgb8(210, 96, 96),
        }
    }
}

impl<Msg> Widget<Msg> for Toast {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.text, SIZE);
        Style {
            width: Dimension::Length((measured.width + PAD_X * 2.0 + ACCENT).ceil()),
            height: Dimension::Length((measured.height + PAD_Y * 2.0).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Ombre + carte.
        scene.shadow(
            Rect::new(bounds.x - 8.0, bounds.y - 4.0, bounds.width + 16.0, bounds.height + 16.0),
            Color::rgba(0.0, 0.0, 0.0, 0.3).fade(o),
            theme.radius + 8.0,
            8.0,
        );
        scene.draw_rect(bounds, theme.surface.fade(o), theme.radius, 1.0, theme.border.fade(o));
        // Barre d'accent à gauche.
        scene.draw_rect(
            Rect::new(bounds.x, bounds.y, ACCENT, bounds.height),
            self.accent(theme).fade(o),
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
        scene.text(
            Point::new(bounds.x + ACCENT + PAD_X, bounds.y + PAD_Y),
            self.text.clone(),
            SIZE,
            theme.on_surface.fade(o),
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
    fn paints_card_accent_and_text() {
        let toast = Toast::new("Enregistré").success();
        let mut scene = Scene::new();
        Widget::<()>::paint(&toast, Rect::new(0.0, 0.0, 160.0, 44.0), Status::default(), &Theme::default(), &mut scene);
        // Accent succès présent + texte.
        let green = Color::rgb8(70, 190, 120);
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == green)));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Enregistré")));
    }
}

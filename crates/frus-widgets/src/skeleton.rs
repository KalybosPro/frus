//! [`Skeleton`] : un placeholder de chargement dont l'intensité **pulse** dans le
//! temps (shimmer). Réutilise l'horloge continue (`Status::time` + `continuous`).

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Vitesse de pulsation (radians/seconde).
const SPEED: f32 = 2.5;

/// Un bloc de chargement animé.
pub struct Skeleton {
    width: Dimension,
    height: f32,
    radius: f32,
}

impl Skeleton {
    /// Crée un placeholder (largeur pleine, hauteur 16 par défaut).
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: 16.0,
            radius: 6.0,
        }
    }

    /// Fixe la largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la hauteur, en pixels logiques.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Fixe le rayon des coins.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Skeleton {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn continuous(&self) -> bool {
        true
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Pulsation `0..1` pilotée par le temps.
        let pulse = 0.5 + 0.5 * (status.time * SPEED).sin();
        let alpha = (0.18 + 0.22 * pulse) * o;
        scene.draw_rect(
            bounds,
            theme.muted.fade(alpha),
            self.radius,
            0.0,
            Color::TRANSPARENT,
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

    fn alpha_at(time: f32) -> f32 {
        let sk = Skeleton::new().width(120.0);
        let mut status = Status::default();
        status.time = time;
        let mut scene = Scene::new();
        Widget::<()>::paint(&sk, Rect::new(0.0, 0.0, 120.0, 16.0), status, &Theme::default(), &mut scene);
        match scene.primitives()[0] {
            Primitive::Rect { color, .. } => color.a,
            _ => panic!("attendu un rectangle"),
        }
    }

    #[test]
    fn is_continuous() {
        assert!(Widget::<()>::continuous(&Skeleton::new()));
    }

    #[test]
    fn intensity_varies_with_time() {
        // Deux instants distincts de la pulsation → opacités différentes.
        assert_ne!(alpha_at(0.0), alpha_at(0.6));
    }
}

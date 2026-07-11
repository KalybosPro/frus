//! [`Spinner`] : un indicateur d'activité **animé en continu** (piloté par le
//! temps). Déclare `continuous()` → le framework continue de redessiner.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Nombre de points de la couronne.
const DOTS: usize = 8;
/// Tours par seconde.
const SPEED: f32 = 1.1;

/// Un indicateur de chargement circulaire (couronne de points qui tourne).
pub struct Spinner {
    size: f32,
}

impl Spinner {
    /// Crée un spinner de côté `24` px par défaut.
    pub fn new() -> Self {
        Self { size: 24.0 }
    }

    /// Fixe la taille (côté du carré), en pixels logiques.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Spinner {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.size),
            height: Dimension::Length(self.size),
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
        let side = bounds.width.min(bounds.height);
        let cx = bounds.x + bounds.width * 0.5;
        let cy = bounds.y + bounds.height * 0.5;
        let ring = side * 0.36;
        let dot = (side * 0.12).max(1.0);

        // Tête lumineuse qui progresse dans le temps ; les points en arrière fondent.
        let head = (status.time * SPEED).fract() * DOTS as f32;
        for i in 0..DOTS {
            let angle = (i as f32 / DOTS as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let px = cx + ring * angle.cos();
            let py = cy + ring * angle.sin();
            // Distance angulaire derrière la tête (0 = tête, ~1 = queue).
            let behind = (i as f32 - head).rem_euclid(DOTS as f32) / DOTS as f32;
            let alpha = (0.15 + 0.85 * (1.0 - behind)) * o;
            scene.draw_rect(
                Rect::new(px - dot, py - dot, dot * 2.0, dot * 2.0),
                theme.primary.fade(alpha),
                dot,
                0.0,
                Color::TRANSPARENT,
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn dot_alphas(time: f32) -> Vec<f32> {
        let spinner = Spinner::new().size(40.0);
        let mut status = Status::default();
        status.time = time;
        let mut scene = Scene::new();
        Widget::<()>::paint(&spinner, Rect::new(0.0, 0.0, 40.0, 40.0), status, &Theme::default(), &mut scene);
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { color, .. } => Some(color.a),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn draws_ring_of_dots() {
        assert_eq!(dot_alphas(0.0).len(), DOTS);
    }

    #[test]
    fn animation_depends_on_time() {
        // La distribution des opacités change avec le temps (ça tourne).
        assert_ne!(dot_alphas(0.0), dot_alphas(0.3));
    }

    #[test]
    fn declares_continuous() {
        assert!(Widget::<()>::continuous(&Spinner::new()));
    }
}

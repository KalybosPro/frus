//! [`Spinner`]: an activity indicator **animated continuously**, driven by time. It
//! declares `continuous()`, so the framework keeps redrawing.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Number of dots in the ring.
const DOTS: usize = 8;
/// Tours par seconde.
const SPEED: f32 = 1.1;

/// A circular loading indicator: a ring of dots that spins.
pub struct Spinner {
    size: f32,
}

impl Spinner {
    /// Creates a spinner, `24` px on a side by default.
    pub fn new() -> Self {
        Self { size: 24.0 }
    }

    /// Sets the size, that is, the square's side, in logical pixels.
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

        // A bright head advancing over time; the dots behind it fade out.
        let head = (status.time * SPEED).fract() * DOTS as f32;
        for i in 0..DOTS {
            let angle =
                (i as f32 / DOTS as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let px = cx + ring * angle.cos();
            let py = cy + ring * angle.sin();
            // The angular distance behind the head: 0 is the head, about 1 the tail.
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
        Widget::<()>::paint(
            &spinner,
            Rect::new(0.0, 0.0, 40.0, 40.0),
            status,
            &Theme::default(),
            &mut scene,
        );
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
        // The distribution of opacities changes over time, which is the spinning.
        assert_ne!(dot_alphas(0.0), dot_alphas(0.3));
    }

    #[test]
    fn declares_continuous() {
        assert!(Widget::<()>::continuous(&Spinner::new()));
    }
}

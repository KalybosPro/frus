//! [`Spinner`]: an activity indicator **animated continuously**, driven by time. It
//! declares `continuous()`, so the framework keeps redrawing.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Number of dots in the ring.
const DOTS: usize = 8;
/// Turns per second.
const SPEED: f32 = 1.1;

/// How the ring reads: **turning**, which means work is happening, or **filling**,
/// which means a gesture is part of the way to asking for some.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RingMode {
    /// A bright head advancing, the dots behind it fading out. `head` is in turns.
    Spinning { head: f32 },
    /// The first `progress` of the ring, clockwise from the top. `0..=1`.
    Filling { progress: f32 },
}

/// Draws the framework's activity ring: `DOTS` dots on a circle of `radius` about
/// `(cx, cy)`, each `dot` px in radius.
///
/// Shared by [`Spinner`] and by the pull-to-refresh indicator, so the two cannot drift
/// into looking like different frameworks' idea of "busy".
pub(crate) fn paint_activity_ring(
    scene: &mut Scene,
    cx: f32,
    cy: f32,
    radius: f32,
    dot: f32,
    color: Color,
    mode: RingMode,
) {
    for i in 0..DOTS {
        let fraction = i as f32 / DOTS as f32;
        let angle = fraction * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let alpha = match mode {
            RingMode::Spinning { head } => {
                let head = head.fract() * DOTS as f32;
                // The angular distance behind the head: 0 is the head, about 1 the tail.
                let behind = (i as f32 - head).rem_euclid(DOTS as f32) / DOTS as f32;
                0.15 + 0.85 * (1.0 - behind)
            }
            // A partial dot at the leading edge, so the fill grows smoothly instead of
            // clicking round one eighth at a time.
            RingMode::Filling { progress } => ((progress - fraction) * DOTS as f32).clamp(0.0, 1.0),
        };
        if alpha <= 0.0 {
            continue;
        }
        let px = cx + radius * angle.cos();
        let py = cy + radius * angle.sin();
        scene.draw_rect(
            Rect::new(px - dot, py - dot, dot * 2.0, dot * 2.0),
            color.fade(alpha),
            dot,
            0.0,
            Color::TRANSPARENT,
        );
    }
}

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
        let side = bounds.width.min(bounds.height);
        paint_activity_ring(
            scene,
            bounds.x + bounds.width * 0.5,
            bounds.y + bounds.height * 0.5,
            side * 0.36,
            (side * 0.12).max(1.0),
            theme.primary.fade(status.opacity),
            RingMode::Spinning {
                head: status.time * SPEED,
            },
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

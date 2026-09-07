//! [`CircularProgressIndicator`]: an activity indicator **animated continuously**, driven by time. It
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
/// Shared by [`CircularProgressIndicator`] and by the pull-to-refresh indicator, so the two cannot drift
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
pub struct CircularProgressIndicator {
    size: f32,
    color: Option<Color>,
    track_color: Option<Color>,
    stroke_width: Option<f32>,
}

impl CircularProgressIndicator {
    /// Creates a spinner, `24` px on a side by default.
    pub fn new() -> Self {
        Self {
            size: 24.0,
            color: None,
            track_color: None,
            stroke_width: None,
        }
    }

    /// Sets the size, that is, the square's side, in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// **The colour of the dots**, over the theme's and `primary`.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// **A track behind the ring** — the whole circle drawn once, quietly, so the
    /// part that is not lit still reads as part of a ring
    /// (`progress_indicator.dart:1590`). Unset, none is drawn and the ring is only its
    /// lit dots, which is what this always was.
    #[must_use]
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// **How thick the dots are.** Unset, the framework's own proportional rule
    /// — the reference gives a flat four whatever the size, which draws a hairline on
    /// a large indicator; this keeps the dots in proportion to the ring and lets a
    /// caller name a number when the proportion is not what they want.
    #[must_use]
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = Some(width);
        self
    }

    /// The dot's radius, from the side of the square: the caller's word, then the
    /// theme's, then the proportional rule.
    fn dot(&self, side: f32, theme: &Theme) -> f32 {
        match self.stroke_width.or(theme.widgets.progress.stroke_width) {
            Some(width) => (width * 0.5).max(0.5),
            None => (side * 0.12).max(1.0),
        }
    }
}

impl Default for CircularProgressIndicator {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for CircularProgressIndicator {
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
        let cx = bounds.x + bounds.width * 0.5;
        let cy = bounds.y + bounds.height * 0.5;
        let radius = side * 0.36;
        let dot = self.dot(side, theme);
        // The track first, when anything named one: every dot at full strength in a
        // quiet colour, with the lit ones drawn over it.
        if let Some(track) = self
            .track_color
            .or(theme.widgets.progress.circular_track_color)
        {
            paint_activity_ring(
                scene,
                cx,
                cy,
                radius,
                dot,
                track.fade(status.opacity),
                RingMode::Filling { progress: 1.0 },
            );
        }
        paint_activity_ring(
            scene,
            cx,
            cy,
            radius,
            dot,
            self.color
                .or(theme.widgets.progress.color)
                .unwrap_or(theme.primary)
                .fade(status.opacity),
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
        let spinner = CircularProgressIndicator::new().size(40.0);
        let status = Status {
            time,
            ..Default::default()
        };
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

    /// **The ring answers to a theme and to its caller** — its colour, how thick its
    /// dots are, and whether there is a track behind them. It had one builder, a size.
    ///
    /// The track is **off** unless something names a colour, so a spinner that says
    /// nothing draws exactly what it always drew.
    #[test]
    fn a_ring_answers_to_its_theme_and_to_its_caller() {
        let plain = CircularProgressIndicator::new().size(40.0);
        assert_eq!(
            dot_alphas(0.0).len(),
            DOTS,
            "no track unless one is asked for"
        );

        let mut theme = Theme::default();
        theme.widgets.progress.circular_track_color = Some(frus_core::Color::rgb(0.2, 0.2, 0.2));
        theme.widgets.progress.color = Some(frus_core::Color::rgb(0.9, 0.1, 0.1));
        theme.widgets.progress.stroke_width = Some(6.0);

        let mut scene = Scene::new();
        Widget::<()>::paint(
            &plain,
            Rect::new(0.0, 0.0, 40.0, 40.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        let dots: Vec<(f32, frus_core::Color)> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } => Some((rect.width, *color)),
                _ => None,
            })
            .collect();
        assert_eq!(dots.len(), 2 * DOTS, "a track ring under the lit one");
        assert_eq!(dots[0].0, 6.0, "the theme's stroke, as a diameter");
        assert_eq!(dots[0].1, frus_core::Color::rgb(0.2, 0.2, 0.2));
        assert_eq!(
            dots[DOTS].1.r, 0.9,
            "and the lit dots take the theme's colour"
        );

        // The caller outranks it.
        let told = CircularProgressIndicator::new()
            .size(40.0)
            .stroke_width(2.0)
            .color(frus_core::Color::rgb(0.0, 0.0, 1.0));
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &told,
            Rect::new(0.0, 0.0, 40.0, 40.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        let dots: Vec<(f32, frus_core::Color)> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } => Some((rect.width, *color)),
                _ => None,
            })
            .collect();
        assert_eq!(dots[0].0, 2.0);
        assert_eq!(dots[DOTS].1.b, 1.0);
    }

    #[test]
    fn declares_continuous() {
        assert!(Widget::<()>::continuous(&CircularProgressIndicator::new()));
    }
}

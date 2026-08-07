//! [`ProgressBar`]: a **determinate** progress bar (`0..1`).

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Bar height, in logical pixels.
const HEIGHT: f32 = 8.0;

/// A progress bar: a track plus a fill proportional to `value`.
pub struct ProgressBar {
    value: f32,
    width: Dimension,
}

impl ProgressBar {
    /// Creates a bar filled to `value`, clamped to `0.0..=1.0`.
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            width: Dimension::Length(200.0),
        }
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }
}

impl<Msg> Widget<Msg> for ProgressBar {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: Dimension::Length(HEIGHT),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let radius = bounds.height * 0.5;
        // Piste.
        scene.draw_rect(
            bounds,
            theme.muted.fade(0.3 * o),
            radius,
            0.0,
            Color::TRANSPARENT,
        );
        // Fill (at least a dot when > 0, never wider than the track).
        let fill_w = (bounds.width * self.value).clamp(0.0, bounds.width);
        if fill_w > 0.0 {
            let w = fill_w.max(bounds.height).min(bounds.width);
            scene.draw_rect(
                Rect::new(bounds.x, bounds.y, w, bounds.height),
                theme.primary.fade(o),
                radius,
                0.0,
                Color::TRANSPARENT,
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        let pct = (self.value * 100.0).round();
        Some(
            frus_core::Semantics::new(frus_core::Role::ProgressBar)
                .value(format!("{pct}%"))
                .range(0.0, self.value, 1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn fill_and_track(value: f32) -> (f32, f32) {
        let bar = ProgressBar::new(value).width(100.0);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &bar,
            Rect::new(0.0, 0.0, 100.0, HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let widths: Vec<f32> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } => Some(rect.width),
                _ => None,
            })
            .collect();
        // [piste, remplissage]
        (widths[0], *widths.get(1).unwrap_or(&0.0))
    }

    #[test]
    fn fill_is_proportional_to_value() {
        let (track, fill) = fill_and_track(0.5);
        assert_eq!(track, 100.0);
        assert_eq!(fill, 50.0);
    }

    #[test]
    fn value_is_clamped() {
        // value > 1 → a full fill (= the track width), no more.
        let (_track, fill) = fill_and_track(2.0);
        assert_eq!(fill, 100.0);
        // value 0 → pas de primitive de remplissage.
        let bar = ProgressBar::new(0.0).width(100.0);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &bar,
            Rect::new(0.0, 0.0, 100.0, HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert_eq!(scene.primitives().len(), 1, "only the track is drawn");
    }
}

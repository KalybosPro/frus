//! [`Slider`] : un curseur de valeur `0.0..=1.0`, **contrôlé** et glissable.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const H: f32 = 24.0;
const TRACK_H: f32 = 6.0;
const THUMB: f32 = 18.0;

/// Un curseur linéaire (valeur normalisée `0..=1`).
pub struct Slider<Msg> {
    value: f32,
    width: f32,
    on_change: Option<Box<dyn Fn(f32) -> Msg>>,
}

impl<Msg> Slider<Msg> {
    /// Crée un curseur à la valeur donnée (bornée à `0..=1`).
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            width: 220.0,
            on_change: None,
        }
    }

    /// Largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Closure produisant un message depuis la nouvelle valeur (`0..=1`).
    pub fn on_change(mut self, on_change: impl Fn(f32) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(on_change));
        self
    }
}

impl<Msg> Widget<Msg> for Slider<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let track_y = bounds.y + (H - TRACK_H) * 0.5;
        // Piste.
        scene.draw_rect(
            Rect::new(bounds.x, track_y, bounds.width, TRACK_H),
            theme.border.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // Remplissage.
        let filled = bounds.width * self.value;
        scene.draw_rect(
            Rect::new(bounds.x, track_y, filled, TRACK_H),
            theme.primary.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // Poignée.
        let cx = bounds.x + filled;
        scene.draw_rect(
            Rect::new(cx - THUMB * 0.5, bounds.y + (H - THUMB) * 0.5, THUMB, THUMB),
            Color::WHITE.fade(o),
            THUMB * 0.5,
            2.0,
            theme.primary.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn draggable(&self) -> bool {
        true
    }

    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        self.on_change
            .as_ref()
            .map(|make| make(fraction.clamp(0.0, 1.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Value(f32),
    }

    #[test]
    fn drag_maps_to_value() {
        let slider = Slider::new(0.0).on_change(Msg::Value);
        assert_eq!(Widget::on_drag(&slider, 0.5), Some(Msg::Value(0.5)));
        // Bornée.
        assert_eq!(Widget::on_drag(&slider, 1.5), Some(Msg::Value(1.0)));
    }
}

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

    fn semantics(&self) -> Option<frus_core::Semantics> {
        let pct = (self.value * 100.0).round();
        Some(
            frus_core::Semantics::new(frus_core::Role::Slider)
                .value(format!("{pct}%"))
                .range(0.0, self.value, 1.0),
        )
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

/// Un curseur de **plage** : deux poignées (bas / haut) délimitant un intervalle
/// `0.0..=1.0`, **contrôlé**. Glisser rapproche la poignée la plus proche du
/// curseur ; aux extrêmes, le geste passe la main à l'autre poignée (les poignées
/// ne se croisent pas). L'application reçoit le nouvel intervalle `(low, high)`.
pub struct RangeSlider<Msg> {
    low: f32,
    high: f32,
    width: f32,
    on_change: Option<Box<dyn Fn(f32, f32) -> Msg>>,
}

impl<Msg> RangeSlider<Msg> {
    /// Crée un curseur de plage (valeurs bornées à `0..=1` et ordonnées `low ≤ high`).
    pub fn new(low: f32, high: f32) -> Self {
        let low = low.clamp(0.0, 1.0);
        let high = high.clamp(0.0, 1.0);
        Self {
            low: low.min(high),
            high: low.max(high),
            width: 220.0,
            on_change: None,
        }
    }

    /// Largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Closure produisant un message depuis le nouvel intervalle `(low, high)`.
    pub fn on_change(mut self, on_change: impl Fn(f32, f32) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(on_change));
        self
    }
}

impl<Msg> Widget<Msg> for RangeSlider<Msg> {
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
        // Segment actif entre les deux poignées.
        let lo = bounds.x + bounds.width * self.low;
        let hi = bounds.x + bounds.width * self.high;
        scene.draw_rect(
            Rect::new(lo, track_y, (hi - lo).max(0.0), TRACK_H),
            theme.primary.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // Poignées.
        for cx in [lo, hi] {
            scene.draw_rect(
                Rect::new(cx - THUMB * 0.5, bounds.y + (H - THUMB) * 0.5, THUMB, THUMB),
                Color::WHITE.fade(o),
                THUMB * 0.5,
                2.0,
                theme.primary.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        let pct = |v: f32| (v * 100.0).round();
        Some(
            frus_core::Semantics::new(frus_core::Role::Slider)
                .value(format!("{}%–{}%", pct(self.low), pct(self.high))),
        )
    }

    fn draggable(&self) -> bool {
        true
    }

    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        let f = fraction.clamp(0.0, 1.0);
        // Aux extrêmes, la poignée de ce côté suit (passage de main) ; entre les deux,
        // la plus proche bouge. Les poignées ne se croisent pas.
        let (low, high) = if f <= self.low {
            (f, self.high)
        } else if f >= self.high {
            (self.low, f)
        } else if f - self.low <= self.high - f {
            (f, self.high)
        } else {
            (self.low, f)
        };
        self.on_change.as_ref().map(|make| make(low, high))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Value(f32),
        Range(f32, f32),
    }

    #[test]
    fn drag_maps_to_value() {
        let slider = Slider::new(0.0).on_change(Msg::Value);
        assert_eq!(Widget::on_drag(&slider, 0.5), Some(Msg::Value(0.5)));
        // Bornée.
        assert_eq!(Widget::on_drag(&slider, 1.5), Some(Msg::Value(1.0)));
    }

    #[test]
    fn range_drag_moves_nearest_thumb() {
        let rs = RangeSlider::new(0.2, 0.8).on_change(Msg::Range);
        // Près du bas → bouge le bas ; près du haut → bouge le haut.
        assert_eq!(Widget::on_drag(&rs, 0.25), Some(Msg::Range(0.25, 0.8)));
        assert_eq!(Widget::on_drag(&rs, 0.75), Some(Msg::Range(0.2, 0.75)));
        // Au-delà du haut → le haut suit, borné à 1 ; en-deçà du bas → le bas, borné à 0.
        assert_eq!(Widget::on_drag(&rs, 1.5), Some(Msg::Range(0.2, 1.0)));
        assert_eq!(Widget::on_drag(&rs, -0.5), Some(Msg::Range(0.0, 0.8)));
    }

    #[test]
    fn range_new_orders_and_clamps() {
        // Bornes inversées → réordonnées ; hors [0,1] → bornées.
        let rs = RangeSlider::new(0.9, 0.1).on_change(Msg::Range);
        // Un glissement au centre bouge la poignée la plus proche depuis (0.1, 0.9).
        assert_eq!(Widget::on_drag(&rs, 0.15), Some(Msg::Range(0.15, 0.9)));
    }
}

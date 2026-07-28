//! [`Slider`] : un curseur de valeur `0.0..=1.0`, **contrôlé** et glissable.

use std::rc::Rc;

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::flex::Flex;
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

/// Une cale transparente et inerte (positionne les poignées le long de la piste).
struct Spacer {
    width: f32,
}

impl<Msg: Clone> Widget<Msg> for Spacer {
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

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Côté d'une poignée de [`RangeSlider`].
#[derive(Copy, Clone)]
enum Side {
    Low,
    High,
}

/// Une poignée **glissable** du curseur de plage. Chaque poignée déplace **son**
/// côté (collant : la poignée saisie reste la poignée déplacée), borné par l'autre.
struct RangeThumb<Msg> {
    side: Side,
    low: f32,
    high: f32,
    /// Largeur de la piste, pour convertir un delta px en fraction.
    track: f32,
    divisions: Option<usize>,
    on_change: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
}

impl<Msg: Clone> Widget<Msg> for RangeThumb<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(THUMB),
            height: Dimension::Length(H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        scene.draw_rect(
            Rect::new(bounds.x, bounds.y + (H - THUMB) * 0.5, THUMB, THUMB),
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

    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        if dx == 0.0 || self.track <= 0.0 {
            return None;
        }
        let df = dx / self.track;
        let snap = |v: f32| match self.divisions {
            Some(n) if n > 0 => (v * n as f32).round() / n as f32,
            _ => v,
        };
        // Chaque poignée reste bornée par l'autre : pas de croisement.
        let (low, high) = match self.side {
            Side::Low => (snap((self.low + df).clamp(0.0, self.high)), self.high),
            Side::High => (self.low, snap((self.high + df).clamp(self.low, 1.0))),
        };
        self.on_change.as_ref().map(|make| make(low, high))
    }
}

/// Un curseur de **plage** : deux poignées (bas / haut) délimitant un intervalle
/// `0.0..=1.0`, **contrôlé** et **collant** (chaque poignée déplace son côté, sans
/// croisement). Pas discret optionnel ([`divisions`](RangeSlider::divisions)).
/// L'application reçoit le nouvel intervalle `(low, high)`.
pub struct RangeSlider<Msg> {
    low: f32,
    high: f32,
    width: f32,
    divisions: Option<usize>,
    on_change: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> RangeSlider<Msg> {
    /// Crée un curseur de plage (valeurs bornées à `0..=1` et ordonnées `low ≤ high`).
    pub fn new(low: f32, high: f32) -> Self {
        let low = low.clamp(0.0, 1.0);
        let high = high.clamp(0.0, 1.0);
        let mut slider = Self {
            low: low.min(high),
            high: low.max(high),
            width: 220.0,
            divisions: None,
            on_change: None,
            children: Vec::new(),
        };
        slider.rebuild();
        slider
    }

    /// Largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.rebuild();
        self
    }

    /// Découpe la course en `n` **paliers** : les valeurs glissées s'accrochent à
    /// `k/n`. Sans appel, la course est continue.
    pub fn divisions(mut self, n: usize) -> Self {
        self.divisions = Some(n.max(1));
        self.rebuild();
        self
    }

    /// Closure produisant un message depuis le nouvel intervalle `(low, high)`.
    pub fn on_change(mut self, on_change: impl Fn(f32, f32) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self.rebuild();
        self
    }

    /// (Re)construit la rangée de poignées calées sur les positions `low`/`high`.
    fn rebuild(&mut self) {
        let thumb = |side: Side| RangeThumb {
            side,
            low: self.low,
            high: self.high,
            track: self.width,
            divisions: self.divisions,
            on_change: self.on_change.clone(),
        };
        let lo_gap = (self.low * self.width - THUMB * 0.5).max(0.0);
        let mid_gap = ((self.high - self.low) * self.width - THUMB).max(0.0);
        let row = Flex::row()
            .child(Spacer { width: lo_gap })
            .child(thumb(Side::Low))
            .child(Spacer { width: mid_gap })
            .child(thumb(Side::High));
        self.children = vec![Box::new(row)];
    }
}

impl<Msg: Clone> Widget<Msg> for RangeSlider<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let track_y = bounds.y + (H - TRACK_H) * 0.5;
        // Piste, puis segment actif entre les deux poignées (les poignées, enfants,
        // se peignent par-dessus).
        scene.draw_rect(
            Rect::new(bounds.x, track_y, bounds.width, TRACK_H),
            theme.border.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        let lo = bounds.x + bounds.width * self.low;
        let hi = bounds.x + bounds.width * self.high;
        scene.draw_rect(
            Rect::new(lo, track_y, (hi - lo).max(0.0), TRACK_H),
            theme.primary.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
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

    /// Les deux poignées glissables de la rangée (bas, haut).
    fn thumbs(rs: &RangeSlider<Msg>) -> Vec<&Box<dyn Widget<Msg>>> {
        let row = &Widget::<Msg>::children(rs)[0];
        row.children().iter().filter(|c| c.draggable()).collect()
    }

    fn range_of(msg: Option<Msg>) -> (f32, f32) {
        match msg {
            Some(Msg::Range(lo, hi)) => (lo, hi),
            other => panic!("attendu Range, obtenu {other:?}"),
        }
    }

    #[test]
    fn each_thumb_moves_its_own_side_and_sticks() {
        let rs = RangeSlider::new(0.2, 0.8).on_change(Msg::Range); // piste 220 px
        let t = thumbs(&rs);
        assert_eq!(t.len(), 2, "deux poignées glissables");
        // Poignée basse : +22 px = +0.1 → bas 0.3 ; haute inchangée.
        let (lo, hi) = range_of(t[0].on_drag_delta(22.0));
        assert!((lo - 0.3).abs() < 1e-4 && (hi - 0.8).abs() < 1e-4, "bas bouge ({lo}, {hi})");
        // Poignée haute : −22 px = −0.1 → haut 0.7 ; bas inchangé.
        let (lo, hi) = range_of(t[1].on_drag_delta(-22.0));
        assert!((lo - 0.2).abs() < 1e-4 && (hi - 0.7).abs() < 1e-4, "haut bouge ({lo}, {hi})");
        // Collant : la poignée basse poussée à fond s'arrête au haut (0.8), sans le pousser.
        let (lo, hi) = range_of(t[0].on_drag_delta(10_000.0));
        assert!((lo - 0.8).abs() < 1e-4 && (hi - 0.8).abs() < 1e-4, "collant au haut ({lo}, {hi})");
        // Delta nul : aucun message.
        assert_eq!(t[0].on_drag_delta(0.0), None);
    }

    #[test]
    fn divisions_snap_to_steps() {
        // 10 paliers, piste 200 px : +25 px = +0.125 depuis 0.0 → accroché à 0.1.
        let rs = RangeSlider::new(0.0, 1.0).width(200.0).divisions(10).on_change(Msg::Range);
        let (lo, _) = range_of(thumbs(&rs)[0].on_drag_delta(25.0));
        assert!((lo - 0.1).abs() < 1e-4, "accroché au palier 0.1, obtenu {lo}");
    }

    #[test]
    fn range_new_orders_and_clamps() {
        // Bornes inversées → réordonnées ; hors [0,1] → bornées.
        let rs = RangeSlider::new(0.9, 0.1).on_change(Msg::Range);
        // Depuis (0.1, 0.9), la poignée basse +0.05 → 0.15.
        let (lo, hi) = range_of(thumbs(&rs)[0].on_drag_delta(0.05 * 220.0));
        assert!((lo - 0.15).abs() < 1e-4 && (hi - 0.9).abs() < 1e-4, "réordonné ({lo}, {hi})");
    }
}

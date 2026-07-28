//! [`Slider`] : un curseur de valeur `0.0..=1.0`, **contrôlé** et glissable.

use std::rc::Rc;

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::flex::Flex;
use crate::interaction::{Key, KeyResponse, Status};
use crate::theme::Theme;
use crate::widget::Widget;

/// Hauteur de l'infobulle de valeur (au-dessus des poignées) et son écart à la piste.
const TIP_H: f32 = 20.0;
const TIP_GAP: f32 = 6.0;
const TIP_SIZE: f32 = 12.0;
/// Pas clavier par défaut (sans `divisions`) : une flèche déplace de 5 %.
const KEY_STEP: f32 = 0.05;

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
    height: f32,
}

impl<Msg: Clone> Widget<Msg> for Spacer {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
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
    /// Hauteur totale (piste + éventuelle zone d'infobulle) : la poignée est dessinée
    /// dans la bande **basse** de `H`.
    height: f32,
    divisions: Option<usize>,
    /// Formateur d'infobulle : la bulle n'apparaît qu'au **survol / focus** de la poignée.
    label: Option<Rc<dyn Fn(f32) -> String>>,
    on_change: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
}

impl<Msg> RangeThumb<Msg> {
    /// Accroche `v` au palier le plus proche si `divisions` est défini.
    fn snap(&self, v: f32) -> f32 {
        match self.divisions {
            Some(n) if n > 0 => (v * n as f32).round() / n as f32,
            _ => v,
        }
    }

    /// Valeur portée par cette poignée (selon son côté).
    fn value(&self) -> f32 {
        match self.side {
            Side::Low => self.low,
            Side::High => self.high,
        }
    }

    /// Nouvel intervalle après un déplacement de `delta` du côté de cette poignée,
    /// borné par l'autre (pas de croisement) et accroché.
    fn moved(&self, delta: f32) -> (f32, f32) {
        match self.side {
            Side::Low => (self.snap((self.low + delta).clamp(0.0, self.high)), self.high),
            Side::High => (self.low, self.snap((self.high + delta).clamp(self.low, 1.0))),
        }
    }

    /// Pas d'une flèche : un palier si `divisions`, sinon [`KEY_STEP`].
    fn key_step(&self) -> f32 {
        match self.divisions {
            Some(n) if n > 0 => 1.0 / n as f32,
            _ => KEY_STEP,
        }
    }
}

impl<Msg: Clone> Widget<Msg> for RangeThumb<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(THUMB),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Poignée dans la bande basse `H` ; anneau accentué au focus clavier.
        let y = bounds.y + bounds.height - H + (H - THUMB) * 0.5;
        let border = if status.focused { 3.0 } else { 2.0 };
        scene.draw_rect(
            Rect::new(bounds.x, y, THUMB, THUMB),
            Color::WHITE.fade(o),
            THUMB * 0.5,
            border,
            theme.primary.fade(o),
        );
        // Infobulle révélée au survol ou au focus (zone haute réservée par le slider).
        if let Some(label) = &self.label {
            let active = status.focused || status.hover_progress > 0.01;
            if active {
                let reveal = if status.focused { o } else { status.hover_progress * o };
                paint_tip(bounds.x + THUMB * 0.5, bounds.y, label(self.value()), theme, reveal, scene);
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn focusable(&self) -> bool {
        self.on_change.is_some()
    }

    fn draggable(&self) -> bool {
        true
    }

    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        if dx == 0.0 || self.track <= 0.0 {
            return None;
        }
        let (low, high) = self.moved(dx / self.track);
        self.on_change.as_ref().map(|make| make(low, high))
    }

    fn on_key(&self, key: &Key) -> KeyResponse<Msg> {
        // Flèches : un pas ; Début/Fin : borne min/max de ce côté (le shell propose ces
        // touches au widget focalisé avant l'action par défaut).
        let delta = match key {
            Key::Left { .. } => -self.key_step(),
            Key::Right { .. } => self.key_step(),
            Key::Home { .. } => -2.0, // borné en 0 / voisin bas
            Key::End { .. } => 2.0,   // borné en voisin haut / 1
            _ => return KeyResponse::Ignored,
        };
        let (low, high) = self.moved(delta);
        KeyResponse::Handled(self.on_change.as_ref().map(|make| make(low, high)))
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
    /// Formateur d'**infobulle de valeur** : si défini, une bulle au-dessus de chaque
    /// poignée affiche `label(valeur)` (et la hauteur réserve la place).
    label: Option<Rc<dyn Fn(f32) -> String>>,
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
            label: None,
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

    /// Affiche une **infobulle de valeur** au-dessus de chaque poignée, formatée par
    /// `label(valeur)` (ex. pourcentage, prix). Réserve la place au-dessus de la piste.
    pub fn value_label(mut self, label: impl Fn(f32) -> String + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self.rebuild();
        self
    }

    /// (Re)construit la rangée de poignées calées sur les positions `low`/`high`.
    fn rebuild(&mut self) {
        let height = self.content_h();
        let thumb = |side: Side| RangeThumb {
            side,
            low: self.low,
            high: self.high,
            track: self.width,
            height,
            divisions: self.divisions,
            label: self.label.clone(),
            on_change: self.on_change.clone(),
        };
        let lo_gap = (self.low * self.width - THUMB * 0.5).max(0.0);
        let mid_gap = ((self.high - self.low) * self.width - THUMB).max(0.0);
        let row = Flex::row()
            .child(Spacer { width: lo_gap, height })
            .child(thumb(Side::Low))
            .child(Spacer { width: mid_gap, height })
            .child(thumb(Side::High));
        self.children = vec![Box::new(row)];
    }

}

/// Peint une infobulle de valeur centrée en `cx` (bord haut `top`) affichant `text`.
fn paint_tip(cx: f32, top: f32, text: String, theme: &Theme, o: f32, scene: &mut Scene) {
    let tw = frus_text::measure(&text, TIP_SIZE).width;
    let bw = tw + 12.0;
    let bx = cx - bw * 0.5;
    scene.draw_rect(
        Rect::new(bx, top, bw, TIP_H),
        theme.primary.fade(o),
        TIP_H * 0.5,
        0.0,
        Color::TRANSPARENT,
    );
    let ty = top + (TIP_H - frus_text::line_height(TIP_SIZE)) * 0.5;
    scene.text(Point::new(bx + 6.0, ty), text, TIP_SIZE, theme.on_primary.fade(o));
}

impl<Msg> RangeSlider<Msg> {
    /// Hauteur totale : piste seule, ou piste + zone d'infobulle si un `label` est posé.
    fn content_h(&self) -> f32 {
        if self.label.is_some() {
            TIP_H + TIP_GAP + H
        } else {
            H
        }
    }

    /// Accroche `v` au palier le plus proche si `divisions` est défini.
    fn snap(&self, v: f32) -> f32 {
        match self.divisions {
            Some(n) if n > 0 => (v * n as f32).round() / n as f32,
            _ => v,
        }
    }
}

impl<Msg: Clone> Widget<Msg> for RangeSlider<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.content_h()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Piste + segment dans la bande **basse** `H` (la zone haute accueille les bulles).
        let base_y = bounds.y + bounds.height - H;
        let track_y = base_y + (H - TRACK_H) * 0.5;
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
        // Les infobulles sont peintes par les poignées (révélées au survol / focus).
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
        // La **piste** (hors poignées, qui sont au-dessus) répond au clic/glissement :
        // la poignée la plus proche rejoint la position.
        self.on_change.is_some()
    }

    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        let f = self.snap(fraction.clamp(0.0, 1.0));
        // Poignée la plus proche (bornée par l'autre, sans croisement).
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

    fn range_of_key(resp: KeyResponse<Msg>) -> (f32, f32) {
        match resp {
            KeyResponse::Handled(msg) => range_of(msg),
            other => panic!("attendu Handled, obtenu {other:?}"),
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
    fn arrow_keys_move_focused_thumb_by_a_step() {
        let rs = RangeSlider::new(0.4, 0.6).divisions(10).on_change(Msg::Range); // pas 0.1
        let t = thumbs(&rs);
        // Poignée basse focalisée : flèche droite +0.1 → 0.5 ; flèche gauche −0.1 → 0.3.
        let (lo, hi) = range_of_key(t[0].on_key(&Key::Right { shift: false, word: false }));
        assert!((lo - 0.5).abs() < 1e-4 && (hi - 0.6).abs() < 1e-4, "→ ({lo}, {hi})");
        let (lo, hi) = range_of_key(t[0].on_key(&Key::Left { shift: false, word: false }));
        assert!((lo - 0.3).abs() < 1e-4 && (hi - 0.6).abs() < 1e-4, "← ({lo}, {hi})");
        // Les poignées sont focusables (atteignables au clavier).
        assert!(t[0].focusable() && t[1].focusable());
    }

    #[test]
    fn track_click_moves_nearest_thumb() {
        let rs = RangeSlider::new(0.2, 0.8).on_change(Msg::Range);
        // La piste (le slider) est glissable et vise la poignée la plus proche.
        assert!(Widget::<Msg>::draggable(&rs), "la piste répond au clic");
        let (lo, hi) = range_of(Widget::on_drag(&rs, 0.25));
        assert!((lo - 0.25).abs() < 1e-4 && (hi - 0.8).abs() < 1e-4, "près du bas ({lo}, {hi})");
        let (lo, hi) = range_of(Widget::on_drag(&rs, 0.9));
        assert!((lo - 0.2).abs() < 1e-4 && (hi - 0.9).abs() < 1e-4, "près du haut ({lo}, {hi})");
    }

    #[test]
    fn home_end_snap_thumb_to_bounds() {
        let rs = RangeSlider::new(0.3, 0.7).on_change(Msg::Range);
        let t = thumbs(&rs);
        let home = Key::Home { shift: false, doc: false };
        let end = Key::End { shift: false, doc: false };
        // Poignée basse : Début → 0 ; Fin → butée haute (0.7).
        let (lo, hi) = range_of_key(t[0].on_key(&home));
        assert!((lo - 0.0).abs() < 1e-4 && (hi - 0.7).abs() < 1e-4, "bas Début ({lo}, {hi})");
        let (lo, hi) = range_of_key(t[0].on_key(&end));
        assert!((lo - 0.7).abs() < 1e-4 && (hi - 0.7).abs() < 1e-4, "bas Fin ({lo}, {hi})");
        // Poignée haute : Fin → 1.
        let (lo, hi) = range_of_key(t[1].on_key(&end));
        assert!((lo - 0.3).abs() < 1e-4 && (hi - 1.0).abs() < 1e-4, "haut Fin ({lo}, {hi})");
    }

    #[test]
    fn value_label_reserves_height() {
        let plain = RangeSlider::new(0.2, 0.8).on_change(Msg::Range);
        let tipped = RangeSlider::new(0.2, 0.8).on_change(Msg::Range).value_label(|v| format!("{}", v));
        let h = |rs: &RangeSlider<Msg>| match Widget::<Msg>::style(rs).height {
            Dimension::Length(v) => v,
            _ => 0.0,
        };
        assert!(h(&tipped) > h(&plain), "l'infobulle réserve de la hauteur");
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

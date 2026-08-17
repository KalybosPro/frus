//! [`Slider`]: a `0.0..=1.0` value slider, **controlled** and draggable.

use std::rc::Rc;

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::flex::Flex;
use crate::interaction::{Key, KeyResponse, Status};
use crate::theme::Theme;
use crate::widget::Widget;

/// The height of the value tooltip (above the thumbs) and its gap from the track.
const TIP_H: f32 = 20.0;
const TIP_GAP: f32 = 6.0;
const TIP_SIZE: f32 = 12.0;
/// The default keyboard step (without `divisions`): an arrow moves by 5%.
const KEY_STEP: f32 = 0.05;

const H: f32 = 24.0;
const TRACK_H: f32 = 6.0;
const THUMB: f32 = 18.0;

/// A linear slider (a normalised `0..=1` value).
pub struct Slider<Msg> {
    value: f32,
    width: f32,
    enabled: bool,
    on_change: Option<Box<dyn Fn(f32) -> Msg>>,
}

impl<Msg> Slider<Msg> {
    /// Creates a slider at the given value (clamped to `0..=1`).
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            width: 220.0,
            enabled: true,
            on_change: None,
        }
    }

    /// Width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// A closure producing a message from the new value (`0..=1`).
    pub fn on_change(mut self, on_change: impl Fn(f32) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(on_change));
        self
    }

    /// Whether the slider can be moved. Disabled it is **inert** — it takes no drag and
    /// answers no key — and it still shows where it is set.
    ///
    /// A slider is the control that makes the point of [`crate::disabled`]'s contract:
    /// greying it out while it still answered a drag would leave it inert only to the
    /// gesture nobody was using on it.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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
        // A slider splits cleanly along the framework's one disabled rule: the part of the
        // track still to travel is a **container** (12 %), the part already travelled and
        // the thumb are **content** on it (38 %). That is the reference's own split too.
        let (rail, filled_color, thumb, ring) = if self.enabled {
            // The rail is a filled track, not an edge: it takes a container tone
            // rather than `outline`, which the reference reserves for borders.
            (
                theme.scheme.surface_container_high,
                theme.primary,
                Color::WHITE,
                theme.primary,
            )
        } else {
            let dead = disabled_content(theme);
            (disabled_container(theme), dead, dead, dead)
        };
        // The rail, its whole length.
        scene.draw_rect(
            Rect::new(bounds.x, track_y, bounds.width, TRACK_H),
            rail.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // The travelled part.
        let filled = bounds.width * self.value;
        scene.draw_rect(
            Rect::new(bounds.x, track_y, filled, TRACK_H),
            filled_color.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // The thumb.
        let cx = bounds.x + filled;
        scene.draw_rect(
            Rect::new(cx - THUMB * 0.5, bounds.y + (H - THUMB) * 0.5, THUMB, THUMB),
            thumb.fade(o),
            THUMB * 0.5,
            2.0,
            ring.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // The value survives: a reader who cannot move the slider is still owed where it
        // sits, which is the whole of what a slider says.
        let pct = (self.value * 100.0).round();
        let semantics = frus_core::Semantics::new(frus_core::Role::Slider)
            .value(format!("{pct}%"))
            .range(0.0, self.value, 1.0);
        Some(if self.enabled {
            semantics
        } else {
            semantics.disabled(true)
        })
    }

    fn draggable(&self) -> bool {
        self.enabled
    }

    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        // `draggable` already says no, but a drag in flight when the caller disables the
        // slider must not land either.
        if !self.enabled {
            return None;
        }
        self.on_change
            .as_ref()
            .map(|make| make(fraction.clamp(0.0, 1.0)))
    }
}

/// A transparent, inert shim that positions the thumbs along the track.
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

/// The side of a [`RangeSlider`] thumb.
#[derive(Copy, Clone)]
enum Side {
    Low,
    High,
}

/// A **draggable** thumb of the range slider. Each thumb moves **its own** side
/// (sticky: the thumb grabbed stays the thumb moved), bounded by the other.
struct RangeThumb<Msg> {
    side: Side,
    low: f32,
    high: f32,
    /// The track's width, to convert a pixel delta into a fraction.
    track: f32,
    /// Total height (the track + any tooltip zone): the thumb is drawn in the
    /// **lower** `H` band.
    height: f32,
    divisions: Option<usize>,
    /// The tooltip formatter: the bubble only appears on **hover or focus** of the thumb.
    label: Option<Rc<dyn Fn(f32) -> String>>,
    /// The slider's availability, handed down to each thumb.
    enabled: bool,
    on_change: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
}

impl<Msg> RangeThumb<Msg> {
    /// Snaps `v` to the nearest step if `divisions` is set.
    fn snap(&self, v: f32) -> f32 {
        match self.divisions {
            Some(n) if n > 0 => (v * n as f32).round() / n as f32,
            _ => v,
        }
    }

    /// The value this thumb carries, according to its side.
    fn value(&self) -> f32 {
        match self.side {
            Side::Low => self.low,
            Side::High => self.high,
        }
    }

    /// The new interval after moving this thumb's side by `delta`, bounded by the
    /// other (no crossing) and snapped.
    fn moved(&self, delta: f32) -> (f32, f32) {
        match self.side {
            Side::Low => (
                self.snap((self.low + delta).clamp(0.0, self.high)),
                self.high,
            ),
            Side::High => (
                self.low,
                self.snap((self.high + delta).clamp(self.low, 1.0)),
            ),
        }
    }

    /// One arrow's step: a division if `divisions` is set, otherwise [`KEY_STEP`].
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
        // The thumb in the lower `H` band; an accented ring on keyboard focus.
        let y = bounds.y + bounds.height - H + (H - THUMB) * 0.5;
        // A disabled thumb never shows the focus ring, because it never takes focus.
        let border = if status.focused && self.enabled {
            3.0
        } else {
            2.0
        };
        let (fill, ring) = if self.enabled {
            (Color::WHITE, theme.primary)
        } else {
            let dead = disabled_content(theme);
            (dead, dead)
        };
        scene.draw_rect(
            Rect::new(bounds.x, y, THUMB, THUMB),
            fill.fade(o),
            THUMB * 0.5,
            border,
            ring.fade(o),
        );
        // The tooltip revealed on hover or focus (the upper zone the slider reserves).
        // A disabled thumb shows none: it is a hint about a value being changed.
        if let Some(label) = self.label.as_ref().filter(|_| self.enabled) {
            let active = status.focused || status.hover_progress > 0.01;
            if active {
                let reveal = if status.focused {
                    o
                } else {
                    status.hover_progress * o
                };
                paint_tip(
                    bounds.x + THUMB * 0.5,
                    bounds.y,
                    label(self.value()),
                    theme,
                    reveal,
                    scene,
                );
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn focusable(&self) -> bool {
        self.enabled && self.on_change.is_some()
    }

    fn draggable(&self) -> bool {
        self.enabled
    }

    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        if !self.enabled || dx == 0.0 || self.track <= 0.0 {
            return None;
        }
        let (low, high) = self.moved(dx / self.track);
        self.on_change.as_ref().map(|make| make(low, high))
    }

    fn on_key(&self, key: &Key) -> KeyResponse<Msg> {
        // A disabled thumb cannot be focused, so this should be unreachable - but a key
        // arriving from a stale focus must not move a value the caller has frozen.
        if !self.enabled {
            return KeyResponse::Ignored;
        }
        // Arrows: one step; Home/End: this side's min/max bound (the shell offers
        // these keys to the focused widget before the default action).
        let delta = match key {
            Key::Left { .. } => -self.key_step(),
            Key::Right { .. } => self.key_step(),
            Key::Home { .. } => -2.0, // clamped at 0 or the low neighbour
            Key::End { .. } => 2.0,   // clamped at the high neighbour or 1
            _ => return KeyResponse::Ignored,
        };
        let (low, high) = self.moved(delta);
        KeyResponse::Handled(self.on_change.as_ref().map(|make| make(low, high)))
    }
}

/// A **range** slider: two thumbs (low and high) bounding a `0.0..=1.0` interval,
/// **controlled** and **sticky** (each thumb moves its own side, with no crossing).
/// An optional discrete step ([`divisions`](RangeSlider::divisions)). The
/// application receives the new `(low, high)` interval.
pub struct RangeSlider<Msg> {
    low: f32,
    high: f32,
    width: f32,
    divisions: Option<usize>,
    /// The **value tooltip** formatter: when set, a bubble above each thumb shows
    /// `label(value)` (and the height reserves the room for it).
    label: Option<Rc<dyn Fn(f32) -> String>>,
    enabled: bool,
    on_change: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> RangeSlider<Msg> {
    /// Creates a range slider (values clamped to `0..=1` and ordered `low ≤ high`).
    pub fn new(low: f32, high: f32) -> Self {
        let low = low.clamp(0.0, 1.0);
        let high = high.clamp(0.0, 1.0);
        let mut slider = Self {
            low: low.min(high),
            high: low.max(high),
            width: 220.0,
            divisions: None,
            label: None,
            enabled: true,
            on_change: None,
            children: Vec::new(),
        };
        slider.rebuild();
        slider
    }

    /// Width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.rebuild();
        self
    }

    /// Splits the travel into `n` **steps**: dragged values snap to `k/n`. Without
    /// this call, the travel is continuous.
    pub fn divisions(mut self, n: usize) -> Self {
        self.divisions = Some(n.max(1));
        self.rebuild();
        self
    }

    /// A closure producing a message from the new `(low, high)` interval.
    pub fn on_change(mut self, on_change: impl Fn(f32, f32) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self.rebuild();
        self
    }

    /// Whether the interval can be changed. Disabled the whole control is **inert** -
    /// neither thumb takes a drag, a key or the focus - and it still shows its interval.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Shows a **value tooltip** above each thumb, formatted by `label(value)` (a
    /// percentage, a price…). Reserves the room above the track.
    pub fn value_label(mut self, label: impl Fn(f32) -> String + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self.rebuild();
        self
    }

    /// (Re)builds the row of thumbs set at the `low`/`high` positions.
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
            enabled: self.enabled,
            on_change: self.on_change.clone(),
        };
        let lo_gap = (self.low * self.width - THUMB * 0.5).max(0.0);
        let mid_gap = ((self.high - self.low) * self.width - THUMB).max(0.0);
        let row = Flex::row()
            .child(Spacer {
                width: lo_gap,
                height,
            })
            .child(thumb(Side::Low))
            .child(Spacer {
                width: mid_gap,
                height,
            })
            .child(thumb(Side::High));
        self.children = vec![Box::new(row)];
    }
}

/// Paints a value tooltip centred on `cx` (top edge `top`) showing `text`.
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
    scene.text(
        Point::new(bx + 6.0, ty),
        text,
        TIP_SIZE,
        theme.on_primary.fade(o),
    );
}

impl<Msg> RangeSlider<Msg> {
    /// Total height: the track alone, or the track + a tooltip zone if a `label` is set.
    fn content_h(&self) -> f32 {
        if self.label.is_some() {
            TIP_H + TIP_GAP + H
        } else {
            H
        }
    }

    /// Snaps `v` to the nearest step if `divisions` is set.
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
        // Track + segment in the **lower** `H` band (the upper zone holds the bubbles).
        let base_y = bounds.y + bounds.height - H;
        let track_y = base_y + (H - TRACK_H) * 0.5;
        // The same split as the single slider: rail a container, chosen span content.
        let (rail, span) = if self.enabled {
            (theme.scheme.surface_container_high, theme.primary)
        } else {
            (disabled_container(theme), disabled_content(theme))
        };
        scene.draw_rect(
            Rect::new(bounds.x, track_y, bounds.width, TRACK_H),
            rail.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        let lo = bounds.x + bounds.width * self.low;
        let hi = bounds.x + bounds.width * self.high;
        scene.draw_rect(
            Rect::new(lo, track_y, (hi - lo).max(0.0), TRACK_H),
            span.fade(o),
            TRACK_H * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // The tooltips are painted by the thumbs, revealed on hover or focus.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // The interval survives, as the single slider's value does.
        let pct = |v: f32| (v * 100.0).round();
        let semantics = frus_core::Semantics::new(frus_core::Role::Slider).value(format!(
            "{}%–{}%",
            pct(self.low),
            pct(self.high)
        ));
        Some(if self.enabled {
            semantics
        } else {
            semantics.disabled(true)
        })
    }

    fn draggable(&self) -> bool {
        // The **track** (outside the thumbs, which sit above it) answers clicks and
        // drags: the nearest thumb moves to the position.
        self.enabled && self.on_change.is_some()
    }

    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        let f = self.snap(fraction.clamp(0.0, 1.0));
        // The nearest thumb, bounded by the other, with no crossing.
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
        // Clamped.
        assert_eq!(Widget::on_drag(&slider, 1.5), Some(Msg::Value(1.0)));
    }

    /// The row's two draggable thumbs (low, high).
    fn thumbs(rs: &RangeSlider<Msg>) -> Vec<&dyn Widget<Msg>> {
        let row = &Widget::<Msg>::children(rs)[0];
        row.children()
            .iter()
            .map(|c| c.as_ref())
            .filter(|c| c.draggable())
            .collect()
    }

    fn range_of(msg: Option<Msg>) -> (f32, f32) {
        match msg {
            Some(Msg::Range(lo, hi)) => (lo, hi),
            other => panic!("expected Range, got {other:?}"),
        }
    }

    fn range_of_key(resp: KeyResponse<Msg>) -> (f32, f32) {
        match resp {
            KeyResponse::Handled(msg) => range_of(msg),
            other => panic!("expected Handled, got {other:?}"),
        }
    }

    #[test]
    fn each_thumb_moves_its_own_side_and_sticks() {
        let rs = RangeSlider::new(0.2, 0.8).on_change(Msg::Range); // a 220 px track
        let t = thumbs(&rs);
        assert_eq!(t.len(), 2, "two draggable thumbs");
        // The low thumb: +22 px = +0.1 → low 0.3; the high one unchanged.
        let (lo, hi) = range_of(t[0].on_drag_delta(22.0));
        assert!(
            (lo - 0.3).abs() < 1e-4 && (hi - 0.8).abs() < 1e-4,
            "low moves ({lo}, {hi})"
        );
        // The high thumb: −22 px = −0.1 → high 0.7; the low one unchanged.
        let (lo, hi) = range_of(t[1].on_drag_delta(-22.0));
        assert!(
            (lo - 0.2).abs() < 1e-4 && (hi - 0.7).abs() < 1e-4,
            "high moves ({lo}, {hi})"
        );
        // Sticky: the low thumb pushed all the way stops at the high one (0.8), without moving it.
        let (lo, hi) = range_of(t[0].on_drag_delta(10_000.0));
        assert!(
            (lo - 0.8).abs() < 1e-4 && (hi - 0.8).abs() < 1e-4,
            "stuck to the top ({lo}, {hi})"
        );
        // A zero delta: no message.
        assert_eq!(t[0].on_drag_delta(0.0), None);
    }

    #[test]
    fn arrow_keys_move_focused_thumb_by_a_step() {
        let rs = RangeSlider::new(0.4, 0.6)
            .divisions(10)
            .on_change(Msg::Range); // a step of 0.1
        let t = thumbs(&rs);
        // The low thumb focused: right arrow +0.1 → 0.5; left arrow −0.1 → 0.3.
        let (lo, hi) = range_of_key(t[0].on_key(&Key::Right {
            shift: false,
            word: false,
        }));
        assert!(
            (lo - 0.5).abs() < 1e-4 && (hi - 0.6).abs() < 1e-4,
            "→ ({lo}, {hi})"
        );
        let (lo, hi) = range_of_key(t[0].on_key(&Key::Left {
            shift: false,
            word: false,
        }));
        assert!(
            (lo - 0.3).abs() < 1e-4 && (hi - 0.6).abs() < 1e-4,
            "← ({lo}, {hi})"
        );
        // The thumbs are focusable, so reachable from the keyboard.
        assert!(t[0].focusable() && t[1].focusable());
    }

    #[test]
    fn track_click_moves_nearest_thumb() {
        let rs = RangeSlider::new(0.2, 0.8).on_change(Msg::Range);
        // The track (the slider) is draggable and targets the nearest thumb.
        assert!(Widget::<Msg>::draggable(&rs), "the track answers clicks");
        let (lo, hi) = range_of(Widget::on_drag(&rs, 0.25));
        assert!(
            (lo - 0.25).abs() < 1e-4 && (hi - 0.8).abs() < 1e-4,
            "near the low end ({lo}, {hi})"
        );
        let (lo, hi) = range_of(Widget::on_drag(&rs, 0.9));
        assert!(
            (lo - 0.2).abs() < 1e-4 && (hi - 0.9).abs() < 1e-4,
            "near the high end ({lo}, {hi})"
        );
    }

    #[test]
    fn home_end_snap_thumb_to_bounds() {
        let rs = RangeSlider::new(0.3, 0.7).on_change(Msg::Range);
        let t = thumbs(&rs);
        let home = Key::Home {
            shift: false,
            doc: false,
        };
        let end = Key::End {
            shift: false,
            doc: false,
        };
        // The low thumb: Home → 0; End → the high stop (0.7).
        let (lo, hi) = range_of_key(t[0].on_key(&home));
        assert!(
            (lo - 0.0).abs() < 1e-4 && (hi - 0.7).abs() < 1e-4,
            "low Home ({lo}, {hi})"
        );
        let (lo, hi) = range_of_key(t[0].on_key(&end));
        assert!(
            (lo - 0.7).abs() < 1e-4 && (hi - 0.7).abs() < 1e-4,
            "low End ({lo}, {hi})"
        );
        // The high thumb: End → 1.
        let (lo, hi) = range_of_key(t[1].on_key(&end));
        assert!(
            (lo - 0.3).abs() < 1e-4 && (hi - 1.0).abs() < 1e-4,
            "upper End ({lo}, {hi})"
        );
    }

    #[test]
    fn value_label_reserves_height() {
        let plain = RangeSlider::new(0.2, 0.8).on_change(Msg::Range);
        let tipped = RangeSlider::new(0.2, 0.8)
            .on_change(Msg::Range)
            .value_label(|v| format!("{}", v));
        let h = |rs: &RangeSlider<Msg>| match Widget::<Msg>::style(rs).height {
            Dimension::Length(v) => v,
            _ => 0.0,
        };
        assert!(h(&tipped) > h(&plain), "the tooltip reserves height");
    }

    #[test]
    fn divisions_snap_to_steps() {
        // 10 steps, a 200 px track: +25 px = +0.125 from 0.0 → snapped to 0.1.
        let rs = RangeSlider::new(0.0, 1.0)
            .width(200.0)
            .divisions(10)
            .on_change(Msg::Range);
        let (lo, _) = range_of(thumbs(&rs)[0].on_drag_delta(25.0));
        assert!((lo - 0.1).abs() < 1e-4, "snapped to step 0.1, got {lo}");
    }

    #[test]
    fn range_new_orders_and_clamps() {
        // Inverted bounds → reordered; outside [0,1] → clamped.
        let rs = RangeSlider::new(0.9, 0.1).on_change(Msg::Range);
        // From (0.1, 0.9), the low thumb +0.05 → 0.15.
        let (lo, hi) = range_of(thumbs(&rs)[0].on_drag_delta(0.05 * 220.0));
        assert!(
            (lo - 0.15).abs() < 1e-4 && (hi - 0.9).abs() < 1e-4,
            "reordered ({lo}, {hi})"
        );
    }

    /// The thumbs **by position** (the row is spacer, thumb, spacer, thumb), which is the
    /// only way to reach them once they are disabled: the `thumbs` helper above finds them
    /// by `draggable`, and a disabled thumb is exactly the thing that is not.
    fn thumbs_by_position(rs: &RangeSlider<Msg>) -> Vec<&dyn Widget<Msg>> {
        let row = &Widget::<Msg>::children(rs)[0];
        let kids = row.children();
        vec![kids[1].as_ref(), kids[3].as_ref()]
    }

    #[test]
    fn a_disabled_slider_is_inert_but_still_says_where_it_sits() {
        let dead = Slider::new(0.4).on_change(Msg::Value).enabled(false);
        assert!(!Widget::<Msg>::draggable(&dead), "it takes no drag");
        assert_eq!(
            Widget::on_drag(&dead, 0.9),
            None,
            "and a drag in flight does not land"
        );
        let semantics = Widget::<Msg>::semantics(&dead).expect("still announced");
        assert!(semantics.disabled, "announced as unavailable");
        assert_eq!(semantics.value.as_deref(), Some("40%"), "value survives");
    }

    /// A slider is dragged and keyed rather than tapped, which is why milestone 322's
    /// guard covers those hooks: greying one out while it still answered a drag leaves it
    /// inert only to the gesture nobody was using on it.
    #[test]
    fn a_disabled_range_slider_takes_no_drag_no_key_and_no_focus() {
        let dead = RangeSlider::new(0.2, 0.8)
            .on_change(Msg::Range)
            .enabled(false);
        assert!(!Widget::<Msg>::draggable(&dead), "the track takes no drag");
        assert_eq!(Widget::on_drag(&dead, 0.5), None);
        for (i, thumb) in thumbs_by_position(&dead).into_iter().enumerate() {
            assert!(!thumb.draggable(), "thumb {i} still drags");
            assert!(!thumb.focusable(), "thumb {i} still takes focus");
            assert_eq!(thumb.on_drag_delta(20.0), None, "thumb {i} still moves");
            assert!(
                matches!(
                    thumb.on_key(&Key::Right {
                        shift: false,
                        word: false
                    }),
                    KeyResponse::Ignored
                ),
                "thumb {i} still answers an arrow"
            );
        }
        let semantics = Widget::<Msg>::semantics(&dead).expect("still announced");
        assert!(semantics.disabled);
        assert_eq!(
            semantics.value.as_deref(),
            Some("20%–80%"),
            "interval survives"
        );
    }

    /// The track splits along the framework's one rule: the part still to travel is a
    /// container, the travelled part and the thumb are content on it.
    #[test]
    fn a_disabled_slider_takes_both_halves_of_the_rule() {
        for theme in [Theme::dark(), Theme::light()] {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &Slider::<Msg>::new(0.5).enabled(false),
                Rect::new(0.0, 0.0, 200.0, H),
                Status {
                    opacity: 1.0,
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            let fills: Vec<frus_core::Color> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Rect { color, .. } => Some(*color),
                    _ => None,
                })
                .collect();
            assert_eq!(fills[0], disabled_container(&theme), "the rail");
            assert_eq!(fills[1], disabled_content(&theme), "the travelled part");
            assert_eq!(fills[2], disabled_content(&theme), "the thumb");
            assert!(
                fills[0].a < fills[1].a,
                "the rail is the quieter of the two"
            );
        }
    }
}

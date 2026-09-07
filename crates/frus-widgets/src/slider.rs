//! [`Slider`]: a value slider over a range, **controlled** and draggable.

use std::rc::Rc;

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::flex::Flex;
use crate::interaction::{Key, KeyResponse, Status};
use crate::theme::Theme;
use crate::widget::Widget;

/// The height of the value tooltip (above the thumbs) and its gap from the track.
const TIP_H: f32 = 20.0;
const TIP_GAP: f32 = 6.0;
/// The value bubble's style: what the theme says, else the reference's — it calls this the
/// *value indicator* and sets it in `labelLarge`.
///
/// **Resolved once**, so that the number the bubble is measured with is the number the
/// digits are drawn at.
fn tip_style(theme: Option<&Theme>) -> ResolvedTextStyle {
    theme
        .and_then(|t| t.widgets.slider.value_indicator_text_style)
        .unwrap_or_else(|| crate::theme::type_scale(theme).label_large)
        .resolved()
}
/// The default keyboard step (without `divisions`): an arrow moves by 5%.
const KEY_STEP: f32 = 0.05;

const H: f32 = 24.0;
const TRACK_H: f32 = 6.0;
const THUMB: f32 = 18.0;

/// A linear slider over `min..=max`, **controlled** and draggable.
pub struct Slider<Msg> {
    value: f32,
    min: f32,
    max: f32,
    width: f32,
    divisions: Option<usize>,
    /// The **value tooltip** formatter: a bubble above the thumb, on hover or focus.
    label: Option<Rc<dyn Fn(f32) -> String>>,
    enabled: bool,
    colors: SliderColors,
    on_change: Option<Box<dyn Fn(f32) -> Msg>>,
    /// Sent once when a drag begins, before the first `on_change`.
    on_change_start: Option<Box<dyn Fn(f32) -> Msg>>,
    /// Sent once when it ends, after the last one.
    on_change_end: Option<Box<dyn Fn(f32) -> Msg>>,
}

/// What a slider was told about its own colours; unset entries fall through to the theme
/// and then the scheme, resolved where they are painted rather than where they are built.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SliderColors {
    pub active_track: Option<Color>,
    pub inactive_track: Option<Color>,
    pub thumb: Option<Color>,
    pub thumb_border: Option<Color>,
}

impl SliderColors {
    /// The four colours a live slider paints with: rail, travelled, thumb, ring.
    fn resolve(&self, theme: &Theme) -> (Color, Color, Color, Color) {
        // The rail is a filled track, not an edge, so its default is a **container**
        // rather than an outline -- and the reference names which one: the secondary
        // container, the role that carries a live-but-quiet fill and stays clear of the
        // 12 % a disabled rail lands on.
        let rail = self
            .inactive_track
            .or(theme.widgets.slider.inactive_track_color)
            .unwrap_or(theme.scheme.secondary_container);
        let filled = self
            .active_track
            .or(theme.widgets.slider.active_track_color)
            .unwrap_or(theme.primary);
        let thumb = self
            .thumb
            .or(theme.widgets.slider.thumb_color)
            .unwrap_or(Color::WHITE);
        // The ring follows the travelled track unless it is named: they are the same
        // colour in the default scheme, and a caller who recolours the track and not the
        // ring means the accent, not a hairline left behind in the old one.
        let ring = self
            .thumb_border
            .or(theme.widgets.slider.thumb_border_color)
            .unwrap_or(filled);
        (rail, filled, thumb, ring)
    }
}

impl<Msg> Slider<Msg> {
    /// Creates a slider at the given value, over `0.0..=1.0` until
    /// [`range`](Slider::range) says otherwise.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            min: 0.0,
            max: 1.0,
            width: 220.0,
            divisions: None,
            label: None,
            enabled: true,
            colors: SliderColors::default(),
            on_change: None,
            on_change_start: None,
            on_change_end: None,
        }
    }

    /// The travelled part of the track; the theme's `primary` otherwise. The thumb's
    /// ring follows it unless [`thumb_border_color`](Slider::thumb_border_color) says
    /// otherwise.
    pub fn active_color(mut self, color: Color) -> Self {
        self.colors.active_track = Some(color);
        self
    }

    /// The part of the track still to travel.
    pub fn inactive_color(mut self, color: Color) -> Self {
        self.colors.inactive_track = Some(color);
        self
    }

    /// The thumb's fill; white otherwise.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.colors.thumb = Some(color);
        self
    }

    /// The ring around the thumb; the travelled track's colour otherwise.
    pub fn thumb_border_color(mut self, color: Color) -> Self {
        self.colors.thumb_border = Some(color);
        self
    }

    /// The travel this slider covers; `0.0..=1.0` by default.
    ///
    /// Everything the application sees is in **these** units — the value it is given,
    /// the value a step lands on, what a reader is told — rather than a fraction it
    /// would have to convert back on both sides of every message. The two arguments are
    /// sorted, so a range written backwards is not a silent empty one.
    ///
    /// The value is held inside the travel rather than rejected: a caller that lowers
    /// the ceiling under a value it already had gets the ceiling, not a panic.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min.min(max);
        self.max = min.max(max);
        self
    }

    /// Splits the travel into `n` **steps**: the value snaps to `min + k·(max−min)/n`,
    /// and an arrow key moves by one of them. Without this call the travel is continuous
    /// and an arrow moves by 5 %.
    pub fn divisions(mut self, n: usize) -> Self {
        self.divisions = Some(n.max(1));
        self
    }

    /// Shows a **value tooltip** above the thumb, formatted by `label(value)` — a
    /// percentage, a price, a duration. It appears on hover or focus and reserves the
    /// room above the track, so a slider that has one is taller than one that does not.
    pub fn value_label(mut self, label: impl Fn(f32) -> String + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self
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

    /// Sent **once**, when a drag begins — before the first
    /// [`on_change`](Self::on_change), with the value the press landed on.
    ///
    /// With [`on_change_end`](Self::on_change_end) it brackets the stream.
    /// `on_change` fires on every pixel of the movement, so an application that seeks a
    /// video, writes a setting to disk or asks the network on each of them does it
    /// sixty times a second; the bracket is what lets it show a preview while the
    /// finger is down and commit when it lifts.
    pub fn on_change_start(mut self, on_start: impl Fn(f32) -> Msg + 'static) -> Self {
        self.on_change_start = Some(Box::new(on_start));
        self
    }

    /// Sent **once**, when the drag ends — after the last
    /// [`on_change`](Self::on_change), with the value it settled on.
    ///
    /// A press that never moved still gets one: it changed the value, and a caller
    /// waiting for the release would otherwise never be told it happened.
    pub fn on_change_end(mut self, on_end: impl Fn(f32) -> Msg + 'static) -> Self {
        self.on_change_end = Some(Box::new(on_end));
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

    /// The value, held inside the travel.
    fn held(&self) -> f32 {
        self.value.clamp(self.min, self.max)
    }

    /// Where the thumb sits along the track, `0..=1`. An empty range pins it at the
    /// start rather than dividing by nothing.
    fn fraction(&self) -> f32 {
        let span = self.max - self.min;
        if span <= 0.0 {
            0.0
        } else {
            ((self.held() - self.min) / span).clamp(0.0, 1.0)
        }
    }

    /// Snaps a fraction to the nearest step, if there are steps.
    fn snap(&self, fraction: f32) -> f32 {
        match self.divisions {
            Some(n) if n > 0 => (fraction * n as f32).round() / n as f32,
            _ => fraction,
        }
    }

    /// The value a fraction along the track stands for.
    fn at(&self, fraction: f32) -> f32 {
        self.min + self.snap(fraction.clamp(0.0, 1.0)) * (self.max - self.min)
    }

    /// One arrow's step, as a fraction of the travel: a division if there are
    /// divisions, otherwise [`KEY_STEP`].
    fn key_step(&self) -> f32 {
        match self.divisions {
            Some(n) if n > 0 => 1.0 / n as f32,
            _ => KEY_STEP,
        }
    }

    /// The height the control asks for: the track, plus the tooltip's zone above it
    /// when there is a tooltip to show.
    fn content_h(&self) -> f32 {
        if self.label.is_some() {
            H + TIP_H + TIP_GAP
        } else {
            H
        }
    }
}

impl<Msg> Widget<Msg> for Slider<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.content_h()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // The track lives in the **lower** `H` band: anything above it is the tooltip's
        // reserved zone, which is empty unless there is a tooltip.
        let track_top = bounds.y + bounds.height - H;
        let track_y = track_top + (H - TRACK_H) * 0.5;
        // A slider splits cleanly along the framework's one disabled rule: the part of the
        // track still to travel is a **container** (12 %), the part already travelled and
        // the thumb are **content** on it (38 %). That is the reference's own split too.
        let (rail, filled_color, thumb, ring) = if self.enabled {
            self.colors.resolve(theme)
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
        let filled = bounds.width * self.fraction();
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
            Rect::new(
                cx - THUMB * 0.5,
                track_top + (H - THUMB) * 0.5,
                THUMB,
                THUMB,
            ),
            thumb.fade(o),
            THUMB * 0.5,
            2.0,
            ring.fade(o),
        );

        // The tooltip, in the zone reserved above the track. A disabled slider shows
        // none: it is a hint about a value being changed, and this one is not.
        if let Some(label) = self.label.as_ref().filter(|_| self.enabled) {
            let reveal = if status.focused {
                o
            } else {
                status.hover_progress * o
            };
            if reveal > 0.01 {
                paint_tip(cx, bounds.y, label(self.held()), theme, reveal, scene);
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // The value survives: a reader who cannot move the slider is still owed where it
        // sits, which is the whole of what a slider says.
        // In the caller's units, and said the caller's way when it gave a formatter:
        // "42 €" is what is on screen, and a percentage of an unstated range is not an
        // answer to "where is this set".
        let held = self.held();
        let spoken = match self.label.as_ref() {
            Some(label) => label(held),
            None if self.min == 0.0 && self.max == 1.0 => {
                format!("{}%", (held * 100.0).round())
            }
            None => format!("{held}"),
        };
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Slider)
            .value(spoken)
            .range(self.min, held, self.max);
        Some(if self.enabled {
            semantics
        } else {
            semantics.disabled(true)
        })
    }

    fn draggable(&self) -> bool {
        self.enabled
    }

    fn focusable(&self) -> bool {
        self.enabled && self.on_change.is_some()
    }

    fn on_key(&self, key: &Key) -> KeyResponse<Msg> {
        // A disabled slider cannot be focused, so this should be unreachable — but a key
        // arriving from a stale focus must not move a value the caller has frozen.
        if !self.enabled {
            return KeyResponse::Ignored;
        }
        // Arrows: one step. Home/End: the two ends, which the clamp in `at` reaches
        // without either bound having to be named here.
        let delta = match key {
            Key::Left { .. } => -self.key_step(),
            Key::Right { .. } => self.key_step(),
            Key::Home { .. } => -2.0,
            Key::End { .. } => 2.0,
            _ => return KeyResponse::Ignored,
        };
        let moved = self.at(self.fraction() + delta);
        KeyResponse::Handled(self.on_change.as_ref().map(|make| make(moved)))
    }

    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        // `draggable` already says no, but a drag in flight when the caller disables the
        // slider must not land either.
        if !self.enabled {
            return None;
        }
        self.on_change.as_ref().map(|make| make(self.at(fraction)))
    }

    fn on_drag_start(&self, fraction: f32) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        // The **value**, not the fraction: a caller who asked for a range of 0..=100 is
        // owed a hundred here too, and a bracket in different units from the stream it
        // brackets would be a trap inside a signature that looks symmetrical.
        self.on_change_start
            .as_ref()
            .map(|make| make(self.at(fraction)))
    }

    fn on_drag_end(&self, fraction: f32) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        self.on_change_end
            .as_ref()
            .map(|make| make(self.at(fraction)))
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
    /// Sent once when a drag begins, before the first `on_change`.
    on_change_start: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
    /// Sent once when it ends, after the last one.
    on_change_end: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
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
            on_change_start: None,
            on_change_end: None,
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

    /// Sent **once**, when a drag begins — before the first
    /// [`on_change`](Self::on_change), with the interval as it stands.
    ///
    /// See [`Slider::on_change_start`] for why the bracket earns its place.
    pub fn on_change_start(mut self, on_start: impl Fn(f32, f32) -> Msg + 'static) -> Self {
        self.on_change_start = Some(Rc::new(on_start));
        self
    }

    /// Sent **once**, when the drag ends — after the last
    /// [`on_change`](Self::on_change), with the interval it settled on.
    pub fn on_change_end(mut self, on_end: impl Fn(f32, f32) -> Msg + 'static) -> Self {
        self.on_change_end = Some(Rc::new(on_end));
        self
    }

    /// The interval a drag reaching `fraction` would leave behind — the nearest thumb
    /// moved to it, the other one where it was.
    ///
    /// Shared by the drag and both its ends so that the three cannot disagree about
    /// which thumb the pointer was nearest.
    fn interval_at(&self, fraction: f32) -> (f32, f32) {
        let f = self.snap(fraction.clamp(0.0, 1.0));
        if f <= self.low {
            (f, self.high)
        } else if f >= self.high {
            (self.low, f)
        } else if f - self.low <= self.high - f {
            (f, self.high)
        } else {
            (self.low, f)
        }
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
    let style = tip_style(Some(theme));
    let tw = frus_text::measure_resolved(&text, &style).width;
    let bw = tw + 12.0;
    let bx = cx - bw * 0.5;
    scene.draw_rect(
        Rect::new(bx, top, bw, TIP_H),
        theme.primary.fade(o),
        TIP_H * 0.5,
        0.0,
        Color::TRANSPARENT,
    );
    let ty = top + (TIP_H - style.line_height()) * 0.5;
    scene.text(
        Point::new(bx + 6.0, ty),
        text,
        &style,
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

impl<Msg: Clone + 'static> Widget<Msg> for RangeSlider<Msg> {
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
        // The same split as the single slider, and the same secondary container.
        let (rail, span) = if self.enabled {
            (theme.scheme.secondary_container, theme.primary)
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

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // The interval survives, as the single slider's value does.
        let pct = |v: f32| (v * 100.0).round();
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Slider)
            .value(format!("{}%–{}%", pct(self.low), pct(self.high)));
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
        // The nearest thumb, bounded by the other, with no crossing.
        let (low, high) = self.interval_at(fraction);
        self.on_change.as_ref().map(|make| make(low, high))
    }

    fn on_drag_start(&self, fraction: f32) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        let (low, high) = self.interval_at(fraction);
        self.on_change_start.as_ref().map(|make| make(low, high))
    }

    fn on_drag_end(&self, fraction: f32) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        let (low, high) = self.interval_at(fraction);
        self.on_change_end.as_ref().map(|make| make(low, high))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Value(f32),
        Start(f32),
        End(f32),
        Range(f32, f32),
        RangeStart(f32, f32),
        RangeEnd(f32, f32),
    }

    /// A slider that was never asked for the bracket sends nothing at either end.
    /// The stream is what it always was.
    #[test]
    fn a_slider_that_was_not_asked_sends_no_bracket() {
        let slider = Slider::new(0.0).on_change(Msg::Value);
        assert_eq!(Widget::on_drag_start(&slider, 0.5), None);
        assert_eq!(Widget::on_drag_end(&slider, 0.5), None);
    }

    /// **The bracket is in the same units as the stream it brackets.** A caller who
    /// asked for `0..=100` gets a hundred from `on_change`, and would be owed one at
    /// each end too — a start in fractions inside a signature that looks symmetrical
    /// is a trap.
    #[test]
    fn the_bracket_speaks_in_values_not_fractions() {
        let slider = Slider::new(0.0)
            .range(0.0, 100.0)
            .on_change(Msg::Value)
            .on_change_start(Msg::Start)
            .on_change_end(Msg::End);
        assert_eq!(Widget::on_drag_start(&slider, 0.25), Some(Msg::Start(25.0)));
        assert_eq!(Widget::on_drag(&slider, 0.25), Some(Msg::Value(25.0)));
        assert_eq!(Widget::on_drag_end(&slider, 0.75), Some(Msg::End(75.0)));
    }

    /// The divisions apply at the ends too: a start or an end landing between two
    /// stops would name a value the stream can never produce.
    #[test]
    fn the_bracket_snaps_like_the_stream() {
        let slider = Slider::new(0.0)
            .divisions(4)
            .on_change(Msg::Value)
            .on_change_start(Msg::Start)
            .on_change_end(Msg::End);
        // 0.3 lies between the 0.25 and 0.5 stops, nearer the first.
        assert_eq!(Widget::on_drag(&slider, 0.3), Some(Msg::Value(0.25)));
        assert_eq!(Widget::on_drag_start(&slider, 0.3), Some(Msg::Start(0.25)));
        assert_eq!(Widget::on_drag_end(&slider, 0.3), Some(Msg::End(0.25)));
    }

    /// A disabled slider is inert at **every** end. A drag in flight when the caller
    /// freezes the value must not land its release either.
    #[test]
    fn a_disabled_slider_brackets_nothing() {
        let slider = Slider::new(0.0)
            .enabled(false)
            .on_change(Msg::Value)
            .on_change_start(Msg::Start)
            .on_change_end(Msg::End);
        assert_eq!(Widget::on_drag_start(&slider, 0.5), None);
        assert_eq!(Widget::on_drag(&slider, 0.5), None);
        assert_eq!(Widget::on_drag_end(&slider, 0.5), None);
    }

    /// A range slider brackets the **interval**, and all three agree about which thumb
    /// the pointer was nearest — they ask the same function.
    #[test]
    fn a_range_slider_brackets_the_interval() {
        let range = RangeSlider::new(0.2, 0.8)
            .on_change(Msg::Range)
            .on_change_start(Msg::RangeStart)
            .on_change_end(Msg::RangeEnd);
        // 0.3 is nearer the low thumb, so the low one moves and the high one stays.
        assert_eq!(
            Widget::on_drag_start(&range, 0.3),
            Some(Msg::RangeStart(0.3, 0.8))
        );
        assert_eq!(Widget::on_drag(&range, 0.3), Some(Msg::Range(0.3, 0.8)));
        // 0.9 is past the high thumb, so that one moves instead.
        assert_eq!(
            Widget::on_drag_end(&range, 0.9),
            Some(Msg::RangeEnd(0.2, 0.9))
        );
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
            // Quieter means closer to the surface. Since milestone 329 both tokens are
            // opaque, so there is no alpha to compare — which is the point: a disabled
            // control flattens.
            let from_surface = |c: Color| {
                (c.r - theme.scheme.surface.r).abs()
                    + (c.g - theme.scheme.surface.g).abs()
                    + (c.b - theme.scheme.surface.b).abs()
            };
            assert!(
                from_surface(fills[0]) < from_surface(fills[1]),
                "the rail is the quieter of the two"
            );
        }
    }
}

#[cfg(test)]
mod range_tests {
    use super::*;
    use crate::interaction::{Key, KeyResponse};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Value(f32),
    }

    fn value_of(msg: Option<Msg>) -> f32 {
        match msg {
            Some(Msg::Value(v)) => v,
            other => panic!("expected a value, got {other:?}"),
        }
    }

    /// The application is handed **its own** units, not a fraction it has to convert.
    #[test]
    fn a_drag_lands_in_the_caller_s_units() {
        let slider = Slider::new(20.0).range(20.0, 200.0).on_change(Msg::Value);
        assert_eq!(value_of(Widget::on_drag(&slider, 0.0)), 20.0);
        assert_eq!(value_of(Widget::on_drag(&slider, 0.5)), 110.0);
        assert_eq!(value_of(Widget::on_drag(&slider, 1.0)), 200.0);
        // Past the end of the track is the end of the range, not past it.
        assert_eq!(value_of(Widget::on_drag(&slider, 1.4)), 200.0);
    }

    /// A range written backwards is sorted rather than left empty.
    #[test]
    fn a_backwards_range_is_taken_the_way_round_it_makes_sense() {
        let slider = Slider::new(0.0).range(200.0, 20.0).on_change(Msg::Value);
        assert_eq!(value_of(Widget::on_drag(&slider, 0.0)), 20.0);
        assert_eq!(value_of(Widget::on_drag(&slider, 1.0)), 200.0);
    }

    /// A value outside the travel is held inside it, not rejected: a caller that lowers
    /// the ceiling under a value it already had gets the ceiling.
    #[test]
    fn a_value_outside_the_travel_is_held_inside_it() {
        let low = Slider::<Msg>::new(-5.0).range(0.0, 10.0);
        assert_eq!(low.fraction(), 0.0);
        let high = Slider::<Msg>::new(40.0).range(0.0, 10.0);
        assert_eq!(high.fraction(), 1.0);
    }

    /// Steps land on the divisions, in the caller's units.
    #[test]
    fn divisions_snap_the_value() {
        let slider = Slider::new(0.0)
            .range(0.0, 100.0)
            .divisions(4)
            .on_change(Msg::Value);
        assert_eq!(value_of(Widget::on_drag(&slider, 0.30)), 25.0);
        assert_eq!(value_of(Widget::on_drag(&slider, 0.40)), 50.0);
        assert_eq!(value_of(Widget::on_drag(&slider, 0.99)), 100.0);
    }

    /// An arrow moves one division when there are divisions, 5 % of the travel when
    /// there are not — and neither runs off either end.
    #[test]
    fn the_arrows_move_by_a_step() {
        let free = Slider::new(50.0).range(0.0, 100.0).on_change(Msg::Value);
        let step = |s: &Slider<Msg>, key: Key| match Widget::on_key(s, &key) {
            KeyResponse::Handled(msg) => value_of(msg),
            other => panic!("expected the key to be taken, got {other:?}"),
        };
        assert_eq!(
            step(
                &free,
                Key::Right {
                    shift: false,
                    word: false
                }
            ),
            55.0
        );
        assert_eq!(
            step(
                &free,
                Key::Left {
                    shift: false,
                    word: false
                }
            ),
            45.0
        );

        let stepped = Slider::new(50.0)
            .range(0.0, 100.0)
            .divisions(4)
            .on_change(Msg::Value);
        assert_eq!(
            step(
                &stepped,
                Key::Right {
                    shift: false,
                    word: false
                }
            ),
            75.0
        );

        let at_end = Slider::new(100.0).range(0.0, 100.0).on_change(Msg::Value);
        assert_eq!(
            step(
                &at_end,
                Key::Right {
                    shift: false,
                    word: false
                }
            ),
            100.0
        );
        assert_eq!(
            step(
                &at_end,
                Key::Home {
                    shift: false,
                    doc: false
                }
            ),
            0.0
        );
        assert_eq!(
            step(
                &at_end,
                Key::End {
                    shift: false,
                    doc: false
                }
            ),
            100.0
        );
    }

    /// It is a keyboard control now — but only when there is somebody to tell.
    #[test]
    fn it_takes_the_focus_only_when_it_can_answer() {
        assert!(!Widget::<Msg>::focusable(&Slider::<Msg>::new(0.5)));
        assert!(Widget::focusable(&Slider::new(0.5).on_change(Msg::Value)));
        assert!(!Widget::focusable(
            &Slider::new(0.5).on_change(Msg::Value).enabled(false)
        ));
    }

    /// A frozen slider answers no key, even one arriving from a stale focus.
    #[test]
    fn a_frozen_slider_answers_no_key() {
        let slider = Slider::new(0.5).on_change(Msg::Value).enabled(false);
        assert!(matches!(
            Widget::on_key(
                &slider,
                &Key::Right {
                    shift: false,
                    word: false
                }
            ),
            KeyResponse::Ignored
        ));
    }

    /// A reader is told where it is set, in the units on screen.
    #[test]
    fn a_reader_is_told_the_real_value() {
        let plain = Slider::<Msg>::new(0.25);
        let s = Widget::<Msg>::semantics(&plain).expect("a slider says where it is");
        assert_eq!(
            s.value.as_deref(),
            Some("25%"),
            "a bare 0..1 reads as a share"
        );

        let ranged = Slider::<Msg>::new(110.0).range(20.0, 200.0);
        let s = Widget::<Msg>::semantics(&ranged).expect("a slider says where it is");
        assert_eq!(s.range, Some((20.0, 110.0, 200.0)));

        let priced = Slider::<Msg>::new(110.0)
            .range(20.0, 200.0)
            .value_label(|v| format!("{v} EUR"));
        let s = Widget::<Msg>::semantics(&priced).expect("a slider says where it is");
        assert_eq!(
            s.value.as_deref(),
            Some("110 EUR"),
            "said the caller's way when it gave a formatter"
        );
    }

    /// The bubble's type is the reference's, and comes from the theme rather than from a
    /// private constant — milestone 414. The reference calls it the *value indicator* and
    /// sets it in `labelLarge`, which is 14 px **medium**: the weight is half the step, and
    /// the `TIP_SIZE: f32 = 12.0` this replaces could not have carried it at any value.
    #[test]
    fn the_value_bubble_wears_the_reference_s_step() {
        let slider = Slider::<Msg>::new(0.3).value_label(|v| format!("{v}"));
        let styles = |theme: &Theme| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &slider,
                Rect::new(0.0, 0.0, 220.0, 50.0),
                Status {
                    focused: true,
                    ..Status::default()
                },
                theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Text { size, weight, .. } => {
                        Some((*size, weight.to_u16()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let plain = Theme::default();
        assert_eq!(
            styles(&plain),
            vec![(
                plain.text.label_large.size.unwrap(),
                frus_core::FontWeight::Medium.to_u16()
            )]
        );
        // And a theme can move it, which nothing could before.
        let mut themed = Theme::default();
        themed.widgets.slider.value_indicator_text_style = Some(frus_core::TextStyle::new(25.0));
        assert_eq!(styles(&themed).first().map(|s| s.0), Some(25.0));
    }

    /// A tooltip reserves the room above the track rather than sitting on it.
    #[test]
    fn a_tooltip_makes_the_control_taller() {
        let bare = Widget::<Msg>::style(&Slider::<Msg>::new(0.5)).height;
        let tipped =
            Widget::<Msg>::style(&Slider::<Msg>::new(0.5).value_label(|v| format!("{v}"))).height;
        assert_eq!(bare, frus_layout::Dimension::Length(H));
        assert_eq!(
            tipped,
            frus_layout::Dimension::Length(H + TIP_H + TIP_GAP),
            "the tooltip's zone is above the track, not over it"
        );
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use frus_core::Primitive;

    const BRAND: Color = Color::rgb(0.0, 0.6, 0.3);
    const RAIL: Color = Color::rgb(0.9, 0.9, 0.2);

    /// (rail, travelled, thumb, ring) as painted.
    fn painted(slider: &Slider<()>, theme: &Theme) -> (Color, Color, Color, Color) {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            slider,
            Rect::new(0.0, 0.0, 220.0, H),
            Status {
                opacity: 1.0,
                ..Default::default()
            },
            theme,
            &mut scene,
        );
        let rects: Vec<(Color, Color)> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    color,
                    border_color,
                    ..
                } => Some((*color, *border_color)),
                _ => None,
            })
            .collect();
        (rects[0].0, rects[1].0, rects[2].0, rects[2].1)
    }

    /// Nothing said: what it always painted.
    #[test]
    fn the_defaults_are_what_they_were() {
        let theme = Theme::default();
        let (rail, filled, thumb, ring) = painted(&Slider::<()>::new(0.5), &theme);
        assert_eq!(rail, theme.scheme.secondary_container);
        assert_eq!(filled, theme.primary);
        assert_eq!(thumb, Color::WHITE);
        assert_eq!(ring, theme.primary);
    }

    /// The ring follows the travelled track unless it is named: they are one colour in
    /// the default scheme, and a caller who recolours the track means the accent.
    #[test]
    fn the_ring_follows_the_track_it_was_not_told_about() {
        let theme = Theme::default();
        let (_, filled, _, ring) = painted(&Slider::<()>::new(0.5).active_color(BRAND), &theme);
        assert_eq!((filled, ring), (BRAND, BRAND));
        let (_, _, _, named) = painted(
            &Slider::<()>::new(0.5)
                .active_color(BRAND)
                .thumb_border_color(RAIL),
            &theme,
        );
        assert_eq!(named, RAIL, "unless it is named");
    }

    /// The two halves of the track are separate.
    #[test]
    fn each_half_of_the_track_takes_its_own_colour() {
        let theme = Theme::default();
        let (rail, filled, thumb, _) = painted(
            &Slider::<()>::new(0.5)
                .active_color(BRAND)
                .inactive_color(RAIL)
                .thumb_color(RAIL),
            &theme,
        );
        assert_eq!((rail, filled, thumb), (RAIL, BRAND, RAIL));
    }

    /// The theme answers when the instance does not, and loses when it does.
    #[test]
    fn the_theme_answers_and_the_instance_overrules_it() {
        let mut theme = Theme::default();
        theme.widgets.slider.active_track_color = Some(RAIL);
        assert_eq!(painted(&Slider::<()>::new(0.5), &theme).1, RAIL);
        assert_eq!(
            painted(&Slider::<()>::new(0.5).active_color(BRAND), &theme).1,
            BRAND
        );
    }
}

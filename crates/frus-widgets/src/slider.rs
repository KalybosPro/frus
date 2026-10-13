//! [`Slider`]: a value slider over a range, **controlled** and draggable.

use std::rc::Rc;

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::interaction::{Key, KeyResponse, Status};
use crate::theme::Theme;
use crate::widget::Widget;
use crate::widgetstate::{WidgetState, WidgetStateProperty};

#[cfg(test)]
const H: f32 = 24.0;
/// The default keyboard step (without `divisions`): an arrow moves by 5%.
const KEY_STEP: f32 = 0.05;

/// How far a thumb sits off the track at rest, and while it is held: the reference's round
/// thumb on a slider (its slider parts, lines 680–681) and on a range slider (its range
/// slider parts, lines 789–790).
const THUMB_ELEVATION: f32 = 1.0;
const THUMB_PRESSED_ELEVATION: f32 = 6.0;

/// The shadow under a thumb whose box is `thumb`: its resting height, rising to the pressed
/// one as the press goes in.
///
/// Drawn the way a search bar draws its height — a blur of two pixels a step from four, and a
/// drop of half a pixel a step — and **not** the way a card does. That is not taste: the
/// shadow primitive's soft edge stops at its own rectangle, and around a circle this small a
/// card's wider blur leaves that rectangle's corners in the fade, so the halo comes out square.
/// At rest this one is round; while held, at six, the corners begin to show.
///
/// `told` is the widget's `(resting, pressed)`, each over the theme's and the reference's.
/// The reference's round thumb casts it enabled or not — its shape has no disabled height
/// (its slider parts, lines 747–767) — and so does this one.
fn paint_thumb_shadow(
    thumb: Rect,
    told: (Option<f32>, Option<f32>),
    status: &Status,
    theme: &Theme,
    scene: &mut Scene,
) {
    let t = &theme.widgets.slider;
    let rest = told.0.or(t.thumb_elevation).unwrap_or(THUMB_ELEVATION);
    let pressed = told
        .1
        .or(t.pressed_thumb_elevation)
        .unwrap_or(THUMB_PRESSED_ELEVATION);
    let depth = rest + (pressed - rest) * status.press_progress.clamp(0.0, 1.0);
    if depth <= 0.0 {
        return;
    }
    // The reference's shadows for this height (milestone 606).
    frus_core::paint_elevation(
        scene,
        thumb,
        frus_core::BorderRadius::uniform(thumb.width * 0.5),
        depth,
        theme.scheme.shadow.fade(status.opacity),
    );
}

/// The reference's 2023 slider (milestone 648): a 4-pixel track, 2 more for the travelled
/// part, a 20-pixel thumb, a 48-pixel halo, all inset by the halo's radius at each end.
const TRACK_HEIGHT: f32 = 4.0;
const ACTIVE_EXTRA: f32 = 2.0;
const THUMB_RADIUS: f32 = 10.0;
const OVERLAY_RADIUS: f32 = 24.0;
/// The reference's shortest track: three touch targets.
const MIN_TRACK: f32 = 144.0;
/// The default width, where the room cannot be given: what the slider has always been.
const DEFAULT_WIDTH: f32 = 220.0;

/// When a slider shows its **value indicator** — the reference's `ShowValueIndicator`.
///
/// The indicator is the label a [`Slider::value_label`] formats, in a drop over the thumb.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShowValueIndicator {
    /// While it is dragged or focused, on a slider with divisions only — the default.
    #[default]
    OnlyForDiscrete,
    /// While it is dragged or focused, on a slider without divisions only.
    OnlyForContinuous,
    /// While it is dragged or focused, whatever the slider.
    OnDrag,
    /// All the time.
    AlwaysVisible,
    /// Never.
    Never,
}

/// **What a press and a slide may do to a slider** (milestone 650) — the reference's
/// `SliderInteraction`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SliderInteraction {
    /// A press moves the thumb to it, and a slide carries it on: the default.
    #[default]
    TapAndSlide,
    /// A press moves the thumb to it; a slide does nothing.
    TapOnly,
    /// A press does nothing; a slide moves the thumb by its own length, from where it was,
    /// wherever on the track it starts.
    SlideOnly,
    /// As [`SlideOnly`](Self::SlideOnly), and only for a slide that starts on the thumb.
    SlideThumb,
}

/// A linear slider over `min..=max`, **controlled** and draggable — the reference's
/// `Slider`.
///
/// It takes the width it is offered, as the reference's does, and is as tall as its halo:
/// 48. The track is inset by the halo's radius at each end, so the thumb reaches both ends
/// without leaving the box. In a right-to-left reading direction it runs from the right.
///
/// The defaults are the reference's 2023 design: the scheme's `primary` for the
/// travelled part and the thumb, `surface_container_highest` for the rest; tick marks on
/// a slider with [`divisions`](Self::divisions); a halo on hover, focus and drag; and the
/// value, when [`value_label`](Self::value_label) formats one, in a drop above the thumb
/// while it is moved — on a slider with divisions, unless
/// [`show_value_indicator`](Self::show_value_indicator) says otherwise.
pub struct Slider<Msg = crate::callback::Callback> {
    value: f32,
    secondary: Option<f32>,
    min: f32,
    max: f32,
    width: f32,
    sized: bool,
    divisions: Option<usize>,
    /// Formats the value for the indicator over the thumb.
    label: Option<Rc<dyn Fn(f32) -> String>>,
    /// Formats the value for a screen reader, when it should hear it otherwise.
    semantic_label: Option<Rc<dyn Fn(f32) -> String>>,
    show_value_indicator: Option<ShowValueIndicator>,
    allowed_interaction: Option<SliderInteraction>,
    /// The theme's, as the layout last found it: a drag is answered without a theme.
    theme_interaction: std::cell::Cell<Option<SliderInteraction>>,
    enabled: bool,
    colors: SliderColors,
    overlay_color: Option<WidgetStateProperty<Color>>,
    thumb_elevation: Option<f32>,
    pressed_thumb_elevation: Option<f32>,
    on_change: Option<Box<dyn Fn(f32) -> Msg>>,
    /// Sent once, when a drag begins.
    on_change_start: Option<Box<dyn Fn(f32) -> Msg>>,
    /// Sent once, when it ends.
    on_change_end: Option<Box<dyn Fn(f32) -> Msg>>,
}

/// What a slider was told about its own colours; unset entries fall through to the theme
/// and then the scheme, resolved where they are painted rather than where they are built.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SliderColors {
    pub active_track: Option<Color>,
    pub inactive_track: Option<Color>,
    pub secondary_track: Option<Color>,
    pub thumb: Option<Color>,
    pub thumb_border: Option<Color>,
}

/// Everything a slider paints with, resolved.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Palette {
    active: Color,
    inactive: Color,
    secondary: Color,
    thumb: Color,
    ring: Option<Color>,
    active_tick: Color,
    inactive_tick: Color,
}

impl SliderColors {
    /// The reference's resolution, each colour the widget's, then the theme's, then the
    /// 2023 default; disabled, the theme's disabled colours or the framework's disabled rule.
    fn resolve(&self, theme: &Theme, enabled: bool) -> Palette {
        let t = &theme.widgets.slider;
        let s = &theme.scheme;
        if !enabled {
            // The travelled part and the thumb are content (38 %), the rest a container
            // (12 %), as the reference's disabled tokens are; the thumb flattened onto the
            // surface, so the track does not show through it.
            let content = disabled_content(theme);
            let container = disabled_container(theme);
            return Palette {
                active: t.disabled_active_track_color.unwrap_or(content),
                inactive: t.disabled_inactive_track_color.unwrap_or(container),
                secondary: t.disabled_inactive_track_color.unwrap_or(container),
                thumb: t.disabled_thumb_color.unwrap_or(content),
                ring: None,
                active_tick: content,
                inactive_tick: content,
            };
        }
        let active = self
            .active_track
            .or(t.active_track_color)
            .unwrap_or(s.primary);
        Palette {
            active,
            inactive: self
                .inactive_track
                .or(t.inactive_track_color)
                .unwrap_or(s.surface_container_highest),
            secondary: self
                .secondary_track
                .or(t.secondary_active_track_color)
                .unwrap_or(s.primary.fade(0.54)),
            // The thumb is the travelled part's colour unless it is named: the reference's
            // `thumbColor ?? activeColor`.
            thumb: self
                .thumb
                .or(self.active_track)
                .or(t.thumb_color)
                .unwrap_or(s.primary),
            ring: self.thumb_border.or(t.thumb_border_color),
            // The reference swaps them when a colour is given: ticks on the travelled part
            // take the inactive colour, and the others the active one.
            active_tick: self
                .inactive_track
                .or(t.active_tick_mark_color)
                .unwrap_or(s.on_primary.fade(0.38)),
            inactive_tick: self
                .active_track
                .or(t.inactive_tick_mark_color)
                .unwrap_or(s.on_surface_variant.fade(0.38)),
        }
    }
}

impl<Msg> Slider<Msg> {
    /// Creates a slider at the given value, over `0.0..=1.0` until
    /// [`range`](Slider::range) says otherwise.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            secondary: None,
            min: 0.0,
            max: 1.0,
            width: DEFAULT_WIDTH,
            sized: false,
            divisions: None,
            label: None,
            semantic_label: None,
            show_value_indicator: None,
            allowed_interaction: None,
            theme_interaction: std::cell::Cell::new(None),
            enabled: true,
            colors: SliderColors::default(),
            overlay_color: None,
            thumb_elevation: None,
            pressed_thumb_elevation: None,
            on_change: None,
            on_change_start: None,
            on_change_end: None,
        }
    }

    /// The travelled part of the track, and the thumb unless
    /// [`thumb_color`](Slider::thumb_color) names it; the scheme's `primary` otherwise.
    pub fn active_color(mut self, color: Color) -> Self {
        self.colors.active_track = Some(color);
        self
    }

    /// The part of the track still to travel; the scheme's `surface_container_highest`
    /// otherwise.
    pub fn inactive_color(mut self, color: Color) -> Self {
        self.colors.inactive_track = Some(color);
        self
    }

    /// The part between the value and the [secondary value](Self::secondary_track_value);
    /// the scheme's `primary` at 54 % otherwise.
    pub fn secondary_active_color(mut self, color: Color) -> Self {
        self.colors.secondary_track = Some(color);
        self
    }

    /// The thumb's fill; the active colour otherwise.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.colors.thumb = Some(color);
        self
    }

    /// A ring around the thumb, two pixels wide. The reference's thumb has none, and
    /// neither has this one unless it is asked for.
    pub fn thumb_border_color(mut self, color: Color) -> Self {
        self.colors.thumb_border = Some(color);
        self
    }

    /// The halo around the thumb, per state: hovered, focused, dragged. Unset, the
    /// theme's, then the active colour at 12 % when one is given, then the scheme's
    /// `primary` at 8 % hovered and 10 % focused or dragged.
    pub fn overlay_color(mut self, color: WidgetStateProperty<Color>) -> Self {
        self.overlay_color = Some(color);
        self
    }

    /// How far the thumb sits off the track at rest. Unset, the theme's, then the
    /// reference's 1; `0.0` casts no shadow.
    pub fn thumb_elevation(mut self, elevation: f32) -> Self {
        self.thumb_elevation = Some(elevation);
        self
    }

    /// How far it rises while it is held. Unset, the theme's, then the reference's 6.
    pub fn pressed_thumb_elevation(mut self, elevation: f32) -> Self {
        self.pressed_thumb_elevation = Some(elevation);
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

    /// A **second value**, shown as a lighter part of the track beyond the value: how
    /// much of a video is buffered behind where it is playing. It is not dragged.
    pub fn secondary_track_value(mut self, value: f32) -> Self {
        self.secondary = Some(value);
        self
    }

    /// Splits the travel into `n` **steps**: the value snaps to `min + k·(max−min)/n`,
    /// a tick marks each, and an arrow key moves by one of them. Without this call the
    /// travel is continuous and an arrow moves by 5 %.
    pub fn divisions(mut self, n: usize) -> Self {
        self.divisions = Some(n.max(1));
        self
    }

    /// Formats the value for the **value indicator**, the drop over the thumb — a
    /// percentage, a price, a duration. When it shows is
    /// [`show_value_indicator`](Self::show_value_indicator)'s to say. A screen reader
    /// hears it too, unless [`semantic_formatter`](Self::semantic_formatter) says
    /// otherwise.
    pub fn value_label(mut self, label: impl Fn(f32) -> String + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self
    }

    /// What a press and a slide may do. Unset, the theme's, then
    /// [`SliderInteraction::TapAndSlide`], the reference's.
    pub fn allowed_interaction(mut self, interaction: SliderInteraction) -> Self {
        self.allowed_interaction = Some(interaction);
        self
    }

    /// When the value indicator shows. Unset, the theme's, then
    /// [`ShowValueIndicator::OnlyForDiscrete`], the reference's.
    pub fn show_value_indicator(mut self, when: ShowValueIndicator) -> Self {
        self.show_value_indicator = Some(when);
        self
    }

    /// How a screen reader is told the value — the reference's `semanticFormatterCallback`.
    pub fn semantic_formatter(mut self, format: impl Fn(f32) -> String + 'static) -> Self {
        self.semantic_label = Some(Rc::new(format));
        self
    }

    /// The width, in logical pixels. Unset, the slider takes the width it is offered,
    /// and 220 where none can be offered.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.sized = true;
        self
    }

    /// A closure producing a message from the new value, in the slider's units.
    pub fn on_change<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_change: impl Fn(f32) -> R + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(crate::callback::handler1(on_change)));
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
    pub fn on_change_start<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_start: impl Fn(f32) -> R + 'static,
    ) -> Self {
        self.on_change_start = Some(Box::new(crate::callback::handler1(on_start)));
        self
    }

    /// Sent **once**, when the drag ends — after the last
    /// [`on_change`](Self::on_change), with the value it settled on.
    ///
    /// A press that never moved still gets one: it changed the value, and a caller
    /// waiting for the release would otherwise never be told it happened.
    pub fn on_change_end<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_end: impl Fn(f32) -> R + 'static,
    ) -> Self {
        self.on_change_end = Some(Box::new(crate::callback::handler1(on_end)));
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

    /// Where `value` sits along the travel, `0..=1`. An empty range pins it at the start
    /// rather than dividing by nothing.
    fn fraction_of(&self, value: f32) -> f32 {
        let span = self.max - self.min;
        if span <= 0.0 {
            0.0
        } else {
            ((value.clamp(self.min, self.max) - self.min) / span).clamp(0.0, 1.0)
        }
    }

    /// Where the thumb sits along the track, `0..=1`.
    fn fraction(&self) -> f32 {
        self.fraction_of(self.held())
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

    /// What a press and a slide may do: the widget's, the theme's, then the reference's.
    fn interaction(&self) -> SliderInteraction {
        self.allowed_interaction
            .or(self.theme_interaction.get())
            .unwrap_or_default()
    }

    fn discrete(&self) -> bool {
        self.divisions.is_some_and(|n| n > 0)
    }

    /// Whether the indicator shows in this state, and how far it has come out.
    fn indicator_reveal(&self, status: &Status, theme: &Theme) -> f32 {
        if !self.enabled || self.label.is_none() {
            return 0.0;
        }
        let when = self
            .show_value_indicator
            .or(theme.widgets.slider.show_value_indicator)
            .unwrap_or_default();
        let moved = status
            .press_progress
            .max(status.focus_progress)
            .clamp(0.0, 1.0);
        match when {
            ShowValueIndicator::AlwaysVisible => 1.0,
            ShowValueIndicator::Never => 0.0,
            ShowValueIndicator::OnlyForDiscrete if !self.discrete() => 0.0,
            ShowValueIndicator::OnlyForContinuous if self.discrete() => 0.0,
            _ => moved,
        }
    }

    /// The halo's colour in this state: the widget's, the active colour at 12 %, the
    /// theme's, then the reference's.
    fn overlay(&self, status: &Status, theme: &Theme) -> Color {
        let mut states = status.states();
        if status.interaction == crate::interaction::Interaction::Pressed {
            states = states.with(WidgetState::Dragged);
        }
        let scheme = &theme.scheme;
        self.overlay_color
            .as_ref()
            .and_then(|p| p.resolve(states).copied())
            .or(self.colors.active_track.map(|c| c.fade(0.12)))
            .or_else(|| {
                theme
                    .widgets
                    .slider
                    .overlay_color
                    .as_ref()
                    .and_then(|p| p.resolve(states).copied())
            })
            .unwrap_or_else(|| {
                if states.contains(WidgetState::Dragged) {
                    scheme.primary.fade(0.1)
                } else if states.contains(WidgetState::Hovered) {
                    scheme.primary.fade(0.08)
                } else if states.contains(WidgetState::Focused) {
                    scheme.primary.fade(0.1)
                } else {
                    Color::TRANSPARENT
                }
            })
    }

    fn track_height(&self, theme: &Theme) -> f32 {
        theme.widgets.slider.track_height.unwrap_or(TRACK_HEIGHT)
    }
}

/// Where the thumb's centre falls along a track running from `left` over `width`, for a
/// value `t` of the travel: the reference's `_calcThumbCenter`. A slider with divisions
/// keeps its thumb half a track's height inside each end, so the first and last ticks are
/// under it.
fn thumb_x(left: f32, width: f32, t: f32, discrete: bool, track_height: f32) -> f32 {
    let padding = if discrete { track_height } else { 0.0 };
    let x = left + t * (width - padding) + padding / 2.0;
    let inner = if padding > THUMB_RADIUS {
        padding / 2.0
    } else {
        0.0
    };
    x.clamp(left + inner, left + width - inner)
}

impl<Msg> Widget<Msg> for Slider<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            min_width: Dimension::Length(if self.sized {
                0.0
            } else {
                MIN_TRACK + 2.0 * OVERLAY_RADIUS
            }),
            height: Dimension::Length(2.0 * OVERLAY_RADIUS),
            ..Default::default()
        }
    }

    /// **All the room on offer** across, as the reference's takes, unless a width was
    /// given.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: !self.sized,
            vertical: false,
        }
    }

    fn soft_extent(&self) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: !self.sized,
            vertical: false,
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn debug_name(&self) -> &'static str {
        "Slider"
    }

    /// The theme's interaction, kept for the drag, which is answered without one.
    fn build_themed(&self, theme: &Theme) {
        self.theme_interaction
            .set(theme.widgets.slider.allowed_interaction);
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let rtl = theme.direction.is_rtl();
        let palette = self.colors.resolve(theme, self.enabled);
        let track_h = self.track_height(theme);
        // The track, inset by the halo's radius at each end, centred in the box.
        let left = bounds.x + OVERLAY_RADIUS;
        let width = (bounds.width - 2.0 * OVERLAY_RADIUS).max(0.0);
        let cy = bounds.y + bounds.height * 0.5;
        let along = |t: f32| if rtl { 1.0 - t } else { t };
        let discrete = self.discrete();
        let thumb = thumb_x(left, width, along(self.fraction()), discrete, track_h);

        if track_h > 0.0 {
            let half = track_h * 0.5;
            let active_half = (track_h + ACTIVE_EXTRA) * 0.5;
            let right = left + width;
            // The travelled part runs from the start, a little thicker; the rest from the
            // thumb to the end. Each reaches half a track past the thumb, under it.
            let (a0, a1) = if rtl {
                (thumb - half, right)
            } else {
                (left, thumb + half)
            };
            let (i0, i1) = if rtl {
                (left, thumb + half)
            } else {
                (thumb - half, right)
            };
            if i1 - i0 > track_h {
                scene.draw_rect(
                    Rect::new(i0, cy - half, i1 - i0, track_h),
                    palette.inactive.fade(o),
                    half,
                    0.0,
                    Color::TRANSPARENT,
                );
            }
            if a1 - a0 > track_h {
                scene.draw_rect(
                    Rect::new(a0, cy - active_half, a1 - a0, 2.0 * active_half),
                    palette.active.fade(o),
                    active_half,
                    0.0,
                    Color::TRANSPARENT,
                );
            }
            // The secondary value: from the thumb towards it, when it lies beyond.
            if let Some(secondary) = self.secondary {
                let sx = thumb_x(
                    left,
                    width,
                    along(self.fraction_of(secondary)),
                    discrete,
                    track_h,
                );
                let beyond = if rtl { sx < thumb } else { sx > thumb };
                if beyond {
                    let (s0, s1) = if rtl { (sx, thumb) } else { (thumb, sx) };
                    let radius = if rtl {
                        frus_core::BorderRadius {
                            top_left: half,
                            bottom_left: half,
                            ..frus_core::BorderRadius::ZERO
                        }
                    } else {
                        frus_core::BorderRadius {
                            top_right: half,
                            bottom_right: half,
                            ..frus_core::BorderRadius::ZERO
                        }
                    };
                    scene.draw_rect(
                        Rect::new(s0, cy - half, s1 - s0, track_h),
                        palette.secondary.fade(o),
                        radius,
                        0.0,
                        Color::TRANSPARENT,
                    );
                }
            }
        }

        // The halo, grown by hover, focus or a press.
        let activation = status
            .hover_progress
            .max(status.focus_progress)
            .max(status.press_progress)
            .clamp(0.0, 1.0);
        if self.enabled && activation > 0.0 {
            let r = OVERLAY_RADIUS * activation;
            let halo = self.overlay(&status, theme);
            if halo.a > 0.0 {
                scene.draw_rect(
                    Rect::new(thumb - r, cy - r, 2.0 * r, 2.0 * r),
                    halo.fade(o),
                    r,
                    0.0,
                    Color::TRANSPARENT,
                );
            }
        }

        // The ticks, one per division, unless they would crowd each other.
        if let Some(n) = self.divisions.filter(|n| *n > 0) {
            let tick_r = track_h / 4.0;
            let adjusted = width - track_h;
            if tick_r > 0.0 && adjusted / n as f32 >= 3.0 * 2.0 * tick_r {
                for i in 0..=n {
                    let x = left + i as f32 / n as f32 * adjusted + track_h / 2.0;
                    let travelled = if rtl { x >= thumb } else { x <= thumb };
                    let color = if travelled {
                        palette.active_tick
                    } else {
                        palette.inactive_tick
                    };
                    scene.draw_rect(
                        Rect::new(x - tick_r, cy - tick_r, 2.0 * tick_r, 2.0 * tick_r),
                        color.fade(o),
                        tick_r,
                        0.0,
                        Color::TRANSPARENT,
                    );
                }
            }
        }

        // The thumb, over its shadow.
        let thumb_box = Rect::new(
            thumb - THUMB_RADIUS,
            cy - THUMB_RADIUS,
            2.0 * THUMB_RADIUS,
            2.0 * THUMB_RADIUS,
        );
        paint_thumb_shadow(
            thumb_box,
            (self.thumb_elevation, self.pressed_thumb_elevation),
            &status,
            theme,
            scene,
        );
        let (ring_width, ring) = match palette.ring {
            Some(ring) => (2.0, ring.fade(o)),
            None => (0.0, Color::TRANSPARENT),
        };
        scene.draw_rect(
            thumb_box,
            palette.thumb.fade(o),
            THUMB_RADIUS,
            ring_width,
            ring,
        );

        // The value, in a drop over the thumb, while the slider is moved.
        let reveal = self.indicator_reveal(&status, theme);
        if let (Some(label), true) = (self.label.as_ref(), reveal > 0.0) {
            paint_drop(
                Point::new(thumb, cy),
                label(self.held()),
                theme,
                reveal,
                o,
                scene,
            );
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
        let spoken = match self.semantic_label.as_ref().or(self.label.as_ref()) {
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

    /// A drag lands on the track, inset by the halo's radius, and runs from the right in a
    /// right-to-left reading direction; how a press and a slide move the value is the
    /// [`SliderInteraction`] allowed. A slide starting on the thumb starts within its halo.
    fn drag_track(&self) -> crate::widget::DragTrack {
        use crate::widget::DragMode;
        let origin = self.fraction();
        crate::widget::DragTrack {
            inset: OVERLAY_RADIUS,
            mirrored: true,
            mode: match self.interaction() {
                SliderInteraction::TapAndSlide => DragMode::Follow,
                SliderInteraction::TapOnly => DragMode::TapOnly,
                SliderInteraction::SlideOnly => DragMode::Slide { origin },
                SliderInteraction::SlideThumb => DragMode::SlideThumb {
                    origin,
                    radius: OVERLAY_RADIUS,
                },
            },
        }
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

    /// The bracket opens on the value **as it was** before the press, as the reference's
    /// `onChangeStart` does (milestone 650): a tap is then a start at the old value and an
    /// end at the new one, and a slide that never moved the value starts and ends on it.
    ///
    /// In the caller's units, not as a fraction: a caller who asked for a range of
    /// 0..=100 is owed a hundred here too.
    fn on_drag_start(&self, _fraction: f32) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        // Snapped like everything the stream sends, as the reference's is.
        let before = self.at(self.fraction());
        self.on_change_start.as_ref().map(|make| make(before))
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

/// The reference's **drop value indicator** (`DropSliderValueIndicatorShape`): a pill
/// 28 high, as wide as the text and a margin, over a small triangle pointing at the
/// thumb's centre `at`, grown from the triangle's tip by `scale`.
fn paint_drop(at: Point, text: String, theme: &Theme, scale: f32, o: f32, scene: &mut Scene) {
    const TRIANGLE: f32 = 10.0;
    const PADDING: f32 = 8.0;
    const MIN_LABEL: f32 = 20.0;
    const RECT_H: f32 = 28.0;
    const RECT_Y: f32 = 6.0;
    const TIP_Y: f32 = 16.0;
    let style = drop_style(theme);
    let text_w = frus_text::measure_resolved(&text, &style).width;
    let rect_w = MIN_LABEL.max(text_w) + PADDING;
    // Everything is drawn about the tip, scaled from it.
    let tip = Point::new(at.x, at.y - TIP_Y);
    let s = |dx: f32, dy: f32| Point::new(tip.x + dx * scale, tip.y + dy * scale);
    let color = theme
        .widgets
        .slider
        .value_indicator_color
        .unwrap_or(theme.scheme.primary)
        .fade(o);
    let top_left = s(-rect_w / 2.0, -RECT_Y - RECT_H);
    let pill = Rect::new(top_left.x, top_left.y, rect_w * scale, RECT_H * scale);
    let path = frus_core::Path::new()
        .move_to(s(0.0, 0.0))
        .line_to(s(-TRIANGLE, -TRIANGLE))
        .line_to(s(TRIANGLE, -TRIANGLE))
        .close();
    scene.fill_path(&path, color);
    scene.draw_rect(pill, color, pill.height * 0.5, 0.0, Color::TRANSPARENT);
    // The text at full size once the drop is, centred in the pill.
    if scale >= 0.999 {
        let text_color = style.color.unwrap_or(theme.scheme.on_primary);
        scene.text(
            Point::new(
                pill.x + (pill.width - text_w) * 0.5,
                pill.y + (pill.height - style.line_height()) * 0.5,
            ),
            text,
            &style,
            text_color.fade(o),
        );
    }
}

/// The drop's type: the theme's, else the reference's 2023 `labelMedium` in `on_primary`.
fn drop_style(theme: &Theme) -> ResolvedTextStyle {
    theme
        .widgets
        .slider
        .value_indicator_text_style
        .unwrap_or_else(|| {
            crate::theme::type_scale(Some(theme))
                .label_medium
                .color(theme.scheme.on_primary)
        })
        .resolved()
}

/// The reference's smallest gap between a continuous range slider's thumbs, in logical
/// pixels: they may touch, not cross or sit on each other.
const MIN_THUMB_SEPARATION: f32 = 8.0;

/// What a range slider and its two thumbs share: the interval, its travel, its look and
/// its messages. Cloned into each thumb when the slider is built.
struct RangeState<Msg> {
    start: f32,
    end: f32,
    min: f32,
    max: f32,
    divisions: Option<usize>,
    label: Option<Rc<dyn Fn(f32) -> String>>,
    semantic_label: Option<Rc<dyn Fn(f32) -> String>>,
    show_value_indicator: Option<ShowValueIndicator>,
    enabled: bool,
    colors: SliderColors,
    overlay_color: Option<WidgetStateProperty<Color>>,
    elevations: (Option<f32>, Option<f32>),
    on_change: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
    on_change_start: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
    on_change_end: Option<Rc<dyn Fn(f32, f32) -> Msg>>,
    /// The track's length in logical pixels, as the slider's last paint found it: the
    /// thumbs' keys keep them the reference's eight pixels apart, and only a paint knows
    /// how wide eight pixels are along the travel.
    track: Rc<std::cell::Cell<f32>>,
}

impl<Msg> Clone for RangeState<Msg> {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
            min: self.min,
            max: self.max,
            divisions: self.divisions,
            label: self.label.clone(),
            semantic_label: self.semantic_label.clone(),
            show_value_indicator: self.show_value_indicator,
            enabled: self.enabled,
            colors: self.colors,
            overlay_color: self.overlay_color.clone(),
            elevations: self.elevations,
            on_change: self.on_change.clone(),
            on_change_start: self.on_change_start.clone(),
            on_change_end: self.on_change_end.clone(),
            track: self.track.clone(),
        }
    }
}

impl<Msg> RangeState<Msg> {
    /// The two values, held inside the travel and in order.
    fn held(&self) -> (f32, f32) {
        let start = self.start.clamp(self.min, self.max);
        let end = self.end.clamp(self.min, self.max);
        (start.min(end), start.max(end))
    }

    /// Where `value` sits along the travel, `0..=1`.
    fn fraction_of(&self, value: f32) -> f32 {
        let span = self.max - self.min;
        if span <= 0.0 {
            0.0
        } else {
            ((value - self.min) / span).clamp(0.0, 1.0)
        }
    }

    fn fractions(&self) -> [f32; 2] {
        let (start, end) = self.held();
        [self.fraction_of(start), self.fraction_of(end)]
    }

    fn discrete(&self) -> bool {
        self.divisions.is_some_and(|n| n > 0)
    }

    fn snap(&self, fraction: f32) -> f32 {
        match self.divisions {
            Some(n) if n > 0 => (fraction * n as f32).round() / n as f32,
            _ => fraction,
        }
    }

    /// The value a fraction along the travel stands for, snapped.
    fn at(&self, fraction: f32) -> f32 {
        self.min + self.snap(fraction.clamp(0.0, 1.0)) * (self.max - self.min)
    }

    /// The least gap between the thumbs, as a fraction of the travel: nothing with
    /// divisions, eight pixels without — the reference's `minThumbSeparation`.
    fn separation(&self) -> f32 {
        let track = self.track.get();
        if self.discrete() || track <= 0.0 {
            0.0
        } else {
            MIN_THUMB_SEPARATION / track
        }
    }

    /// The interval with thumb `thumb` (`0` the start) at `fraction`, held off the other
    /// by the least gap.
    fn moved(&self, thumb: usize, fraction: f32) -> (f32, f32) {
        let [s, e] = self.fractions();
        let gap = self.separation();
        let f = self.snap(fraction.clamp(0.0, 1.0));
        if thumb == 0 {
            (self.at(f.min(e - gap).max(0.0)), self.at(e))
        } else {
            (self.at(s), self.at(f.max(s + gap).min(1.0)))
        }
    }

    /// One arrow's step, as a fraction of the travel.
    fn key_step(&self) -> f32 {
        match self.divisions {
            Some(n) if n > 0 => 1.0 / n as f32,
            _ => KEY_STEP,
        }
    }

    /// What a reader is told a thumb's value is.
    fn spoken(&self, value: f32) -> String {
        match self.semantic_label.as_ref().or(self.label.as_ref()) {
            Some(label) => label(value),
            None if self.min == 0.0 && self.max == 1.0 => format!("{}%", (value * 100.0).round()),
            None => format!("{value}"),
        }
    }

    /// Whether the value indicators show while the slider is moved, by its policy.
    fn indicates(&self, theme: &Theme) -> bool {
        if !self.enabled || self.label.is_none() {
            return false;
        }
        match self
            .show_value_indicator
            .or(theme.widgets.slider.show_value_indicator)
            .unwrap_or_default()
        {
            ShowValueIndicator::AlwaysVisible | ShowValueIndicator::OnDrag => true,
            ShowValueIndicator::Never => false,
            ShowValueIndicator::OnlyForDiscrete => self.discrete(),
            ShowValueIndicator::OnlyForContinuous => !self.discrete(),
        }
    }

    fn always_indicates(&self, theme: &Theme) -> bool {
        self.indicates(theme)
            && self
                .show_value_indicator
                .or(theme.widgets.slider.show_value_indicator)
                == Some(ShowValueIndicator::AlwaysVisible)
    }

    /// The reference's range slider colours (its 2023 default, which is its earlier
    /// design): the span between the thumbs in `primary`, the rest `primary` at 24 %.
    fn palette(&self, theme: &Theme) -> RangePalette {
        let t = &theme.widgets.slider;
        let s = &theme.scheme;
        if !self.enabled {
            let content = disabled_content(theme);
            let container = disabled_container(theme);
            return RangePalette {
                active: t.disabled_active_track_color.unwrap_or(content),
                inactive: t.disabled_inactive_track_color.unwrap_or(container),
                thumb: t.disabled_thumb_color.unwrap_or(content),
                active_tick: content,
                inactive_tick: content,
            };
        }
        let active = self
            .colors
            .active_track
            .or(t.active_track_color)
            .unwrap_or(s.primary);
        RangePalette {
            active,
            inactive: self
                .colors
                .inactive_track
                .or(t.inactive_track_color)
                .unwrap_or(s.primary.fade(0.24)),
            thumb: self
                .colors
                .thumb
                .or(self.colors.active_track)
                .or(t.thumb_color)
                .unwrap_or(s.primary),
            active_tick: self
                .colors
                .inactive_track
                .or(t.active_tick_mark_color)
                .unwrap_or(s.on_primary.fade(0.54)),
            inactive_tick: self
                .colors
                .active_track
                .or(t.inactive_tick_mark_color)
                .unwrap_or(s.primary.fade(0.54)),
        }
    }

    /// The halo's colour: the widget's, the active colour at 12 %, the theme's, then the
    /// reference's `primary` at 12 %.
    fn halo(&self, status: &Status, theme: &Theme) -> Color {
        let states = status.states();
        self.overlay_color
            .as_ref()
            .and_then(|p| p.resolve(states).copied())
            .or(self.colors.active_track.map(|c| c.fade(0.12)))
            .or_else(|| {
                theme
                    .widgets
                    .slider
                    .overlay_color
                    .as_ref()
                    .and_then(|p| p.resolve(states).copied())
            })
            .unwrap_or_else(|| theme.scheme.primary.fade(0.12))
    }
}

/// What a range slider paints with, resolved.
struct RangePalette {
    active: Color,
    inactive: Color,
    thumb: Color,
    active_tick: Color,
    inactive_tick: Color,
}

/// A **range** slider: two thumbs bounding an interval of `min..=max`, **controlled** and
/// draggable — the reference's `RangeSlider`.
///
/// Like [`Slider`] it takes the width it is offered and is as tall as its halo, its track
/// inset by the halo's radius, running from the right in a right-to-left reading
/// direction. The defaults are the reference's: the span between the thumbs in the
/// scheme's `primary`, the rest `primary` at 24 %; two round thumbs, a stroke between them
/// when they overlap; a tick per division; and the values, when
/// [`value_label`](Self::value_label) formats them, in a tooltip over each thumb while it
/// is moved — on a slider with divisions, unless
/// [`show_value_indicator`](Self::show_value_indicator) says otherwise.
///
/// A press takes the nearer thumb and the drag moves it, never past the other: the
/// reference's eight pixels apart on a continuous slider. A press on both, where they
/// meet, waits for the first movement: leftwards takes the start. Each thumb takes the
/// keyboard's focus and tells a screen reader its own value.
pub struct RangeSlider<Msg = crate::callback::Callback> {
    state: RangeState<Msg>,
    width: f32,
    sized: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> RangeSlider<Msg> {
    /// A range slider from `start` to `end`, over `0.0..=1.0` until
    /// [`range`](Self::range) says otherwise. The two are put in order.
    pub fn new(start: f32, end: f32) -> Self {
        let mut slider = Self {
            state: RangeState {
                start: start.min(end),
                end: start.max(end),
                min: 0.0,
                max: 1.0,
                divisions: None,
                label: None,
                semantic_label: None,
                show_value_indicator: None,
                enabled: true,
                colors: SliderColors::default(),
                overlay_color: None,
                elevations: (None, None),
                on_change: None,
                on_change_start: None,
                on_change_end: None,
                track: Rc::new(std::cell::Cell::new(0.0)),
            },
            width: DEFAULT_WIDTH,
            sized: false,
            children: Vec::new(),
        };
        slider.rebuild();
        slider
    }

    fn with(mut self, change: impl FnOnce(&mut RangeState<Msg>)) -> Self {
        change(&mut self.state);
        self.rebuild();
        self
    }

    /// The travel the two values cover; `0.0..=1.0` by default. Everything the
    /// application sees is in these units.
    pub fn range(self, min: f32, max: f32) -> Self {
        self.with(|s| {
            s.min = min.min(max);
            s.max = min.max(max);
        })
    }

    /// Splits the travel into `n` steps: the values snap to them, a tick marks each, and
    /// an arrow moves a thumb by one.
    pub fn divisions(self, n: usize) -> Self {
        self.with(|s| s.divisions = Some(n.max(1)))
    }

    /// The width, in logical pixels. Unset, the slider takes the width it is offered, and
    /// 220 where none can be.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.sized = true;
        self.rebuild();
        self
    }

    /// Formats each thumb's value for its tooltip — the reference's `labels`, as one
    /// formatter rather than two strings, so it follows the values. A screen reader hears
    /// it too, unless [`semantic_formatter`](Self::semantic_formatter) says otherwise.
    pub fn value_label(self, label: impl Fn(f32) -> String + 'static) -> Self {
        self.with(|s| s.label = Some(Rc::new(label)))
    }

    /// How a screen reader is told each thumb's value.
    pub fn semantic_formatter(self, format: impl Fn(f32) -> String + 'static) -> Self {
        self.with(|s| s.semantic_label = Some(Rc::new(format)))
    }

    /// When the tooltips show. Unset, the theme's, then only on a slider with divisions.
    pub fn show_value_indicator(self, when: ShowValueIndicator) -> Self {
        self.with(|s| s.show_value_indicator = Some(when))
    }

    /// The span between the thumbs, and the thumbs; the scheme's `primary` otherwise.
    pub fn active_color(self, color: Color) -> Self {
        self.with(|s| s.colors.active_track = Some(color))
    }

    /// The track outside the span; `primary` at 24 % otherwise.
    pub fn inactive_color(self, color: Color) -> Self {
        self.with(|s| s.colors.inactive_track = Some(color))
    }

    /// The thumbs' fill; the active colour otherwise.
    pub fn thumb_color(self, color: Color) -> Self {
        self.with(|s| s.colors.thumb = Some(color))
    }

    /// The halo around a thumb, per state; the active colour at 12 % otherwise.
    pub fn overlay_color(self, color: WidgetStateProperty<Color>) -> Self {
        self.with(|s| s.overlay_color = Some(color))
    }

    /// How far a thumb sits off the track at rest. Unset, the theme's, then the
    /// reference's 1; `0.0` casts no shadow.
    pub fn thumb_elevation(self, elevation: f32) -> Self {
        self.with(|s| s.elevations.0 = Some(elevation))
    }

    /// How far it rises while it is held. Unset, the theme's, then the reference's 6.
    pub fn pressed_thumb_elevation(self, elevation: f32) -> Self {
        self.with(|s| s.elevations.1 = Some(elevation))
    }

    /// Whether the slider can be moved. Disabled it takes no drag, no key and no focus,
    /// and still shows its interval.
    pub fn enabled(self, enabled: bool) -> Self {
        self.with(|s| s.enabled = enabled)
    }

    /// A closure producing a message from the new interval, `(start, end)`, in the
    /// slider's units.
    pub fn on_change<R: crate::callback::IntoMsg<Msg>>(
        self,
        on_change: impl Fn(f32, f32) -> R + 'static,
    ) -> Self {
        let make = Rc::new(crate::callback::handler2(on_change));
        self.with(|s| s.on_change = Some(make))
    }

    /// Sent once when a drag picks a thumb, with the interval **as it was**, as the
    /// reference's `onChangeStart`.
    pub fn on_change_start<R: crate::callback::IntoMsg<Msg>>(
        self,
        on_start: impl Fn(f32, f32) -> R + 'static,
    ) -> Self {
        let make = Rc::new(crate::callback::handler2(on_start));
        self.with(|s| s.on_change_start = Some(make))
    }

    /// Sent once when the drag ends, with the interval it settled on.
    pub fn on_change_end<R: crate::callback::IntoMsg<Msg>>(
        self,
        on_end: impl Fn(f32, f32) -> R + 'static,
    ) -> Self {
        let make = Rc::new(crate::callback::handler2(on_end));
        self.with(|s| s.on_change_end = Some(make))
    }

    /// The thumbs' centres in `bounds`: the track's middle, and each thumb's place across,
    /// the start first.
    fn thumb_centres(&self, bounds: Rect, theme: &Theme) -> (f32, [f32; 2]) {
        let rtl = theme.direction.is_rtl();
        let track_h = theme.widgets.slider.track_height.unwrap_or(TRACK_HEIGHT);
        let left = bounds.x + OVERLAY_RADIUS;
        let width = (bounds.width - 2.0 * OVERLAY_RADIUS).max(0.0);
        let along = |t: f32| if rtl { 1.0 - t } else { t };
        let discrete = self.state.discrete();
        let [s, e] = self.state.fractions();
        (
            bounds.y + bounds.height * 0.5,
            [
                thumb_x(left, width, along(s), discrete, track_h),
                thumb_x(left, width, along(e), discrete, track_h),
            ],
        )
    }

    /// The thumbs, each placed at its value by an alignment across the slider's box: a
    /// 48-pixel thumb aligned at `2t − 1` has its centre at `24 + t·(width − 48)`, which is
    /// exactly where the inset track puts `t`. A directional alignment mirrors it right to
    /// left with the track.
    fn rebuild(&mut self) {
        let [s, e] = self.state.fractions();
        let thumb = |side: usize, t: f32| {
            crate::align::Aligned::new(
                frus_core::AlignmentDirectional::new(2.0 * t - 1.0, 0.0),
                RangeThumb {
                    side,
                    state: self.state.clone(),
                },
            )
        };
        let stack = crate::Stack::new()
            .fit(crate::StackFit::Expand)
            // A focused thumb's indicator stands over the slider, out of the stack.
            .clip_behavior(crate::Clip::None)
            .height(2.0 * OVERLAY_RADIUS)
            .flex(1.0)
            .layer(thumb(0, s))
            .layer(thumb(1, e));
        self.children = vec![Box::new(stack)];
    }
}

/// One of a [`RangeSlider`]'s thumbs: what takes the keyboard's focus, answers its arrows,
/// tells a screen reader its value, and shows its halo and tooltip while hovered or
/// focused. The slider draws the thumb itself, under it.
struct RangeThumb<Msg> {
    /// `0` the start, `1` the end.
    side: usize,
    state: RangeState<Msg>,
}

impl<Msg> RangeThumb<Msg> {
    fn value(&self) -> f32 {
        let (start, end) = self.state.held();
        if self.side == 0 {
            start
        } else {
            end
        }
    }
}

impl<Msg: Clone> Widget<Msg> for RangeThumb<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(2.0 * OVERLAY_RADIUS),
            height: Dimension::Length(2.0 * OVERLAY_RADIUS),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn debug_name(&self) -> &'static str {
        "RangeSliderThumb"
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        if !self.state.enabled {
            return;
        }
        let o = status.opacity;
        // The alignment puts this box's centre where a continuous track puts the value; on
        // a slider with divisions the track pulls its ends in by half its height, and the
        // thumb with them.
        let padding = if self.state.discrete() {
            theme.widgets.slider.track_height.unwrap_or(TRACK_HEIGHT)
        } else {
            0.0
        };
        let t = self.state.fractions()[self.side];
        let along = padding * (0.5 - t);
        let shift = if theme.direction.is_rtl() {
            -along
        } else {
            along
        };
        let centre = Point::new(
            bounds.x + bounds.width * 0.5 + shift,
            bounds.y + bounds.height * 0.5,
        );
        let activation = status
            .hover_progress
            .max(status.focus_progress)
            .clamp(0.0, 1.0);
        // Under the thumb, which the slider paints over its stops.
        if activation > 0.0 {
            let r = OVERLAY_RADIUS * activation;
            scene.draw_rect(
                Rect::new(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r),
                self.state.halo(&status, theme).fade(o),
                r,
                0.0,
                Color::TRANSPARENT,
            );
        }
        // Focused, its tooltip comes out, as the reference's does; the slider shows both
        // while it is dragged.
        if let Some(label) = self.state.label.as_ref() {
            let reveal = if self.state.always_indicates(theme) {
                0.0
            } else if self.state.indicates(theme) {
                status.focus_progress.clamp(0.0, 1.0)
            } else {
                0.0
            };
            if reveal > 0.0 {
                paint_rect_indicator(centre, label(self.value()), theme, reveal, o, scene);
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn focusable(&self) -> bool {
        self.state.enabled && self.state.on_change.is_some()
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let value = self.value();
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Slider)
            .value(self.state.spoken(value))
            .range(self.state.min, value, self.state.max);
        Some(if self.state.enabled {
            semantics
        } else {
            semantics.disabled(true)
        })
    }

    fn on_key(&self, key: &Key) -> KeyResponse<Msg> {
        // A disabled thumb cannot be focused, so this should be unreachable — but a key
        // arriving from a stale focus must not move a value the caller has frozen.
        if !self.state.enabled {
            return KeyResponse::Ignored;
        }
        let here = self.state.fractions()[self.side];
        let to = match key {
            Key::Left { .. } => here - self.state.key_step(),
            Key::Right { .. } => here + self.state.key_step(),
            Key::Home { .. } => -1.0,
            Key::End { .. } => 2.0,
            _ => return KeyResponse::Ignored,
        };
        let (start, end) = self.state.moved(self.side, to);
        KeyResponse::Handled(self.state.on_change.as_ref().map(|make| make(start, end)))
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for RangeSlider<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            min_width: Dimension::Length(if self.sized {
                0.0
            } else {
                MIN_TRACK + 2.0 * OVERLAY_RADIUS
            }),
            height: Dimension::Length(2.0 * OVERLAY_RADIUS),
            ..Default::default()
        }
    }

    /// **All the room on offer** across, as the reference's takes, unless a width was
    /// given.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: !self.sized,
            vertical: false,
        }
    }

    fn soft_extent(&self) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: !self.sized,
            vertical: false,
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn debug_name(&self) -> &'static str {
        "RangeSlider"
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let state = &self.state;
        let palette = state.palette(theme);
        let track_h = theme.widgets.slider.track_height.unwrap_or(TRACK_HEIGHT);
        let left = bounds.x + OVERLAY_RADIUS;
        let width = (bounds.width - 2.0 * OVERLAY_RADIUS).max(0.0);
        // What the thumbs' keys measure the least gap with.
        state.track.set(width);
        let (cy, [x0, x1]) = self.thumb_centres(bounds, theme);
        let (lo, hi) = (x0.min(x1), x0.max(x1));

        if track_h > 0.0 {
            let half = track_h * 0.5;
            let active_half = (track_h + ACTIVE_EXTRA) * 0.5;
            let right = left + width;
            // The outer parts, then the span between the thumbs, a little thicker.
            for (a, b) in [(left, lo + half), (hi - half, right)] {
                if b - a > track_h {
                    scene.draw_rect(
                        Rect::new(a, cy - half, b - a, track_h),
                        palette.inactive.fade(o),
                        half,
                        0.0,
                        Color::TRANSPARENT,
                    );
                }
            }
            if hi - lo > 0.0 {
                scene.draw_rect(
                    Rect::new(lo, cy - active_half, hi - lo, 2.0 * active_half),
                    palette.active.fade(o),
                    0.0,
                    0.0,
                    Color::TRANSPARENT,
                );
            }
        }

        // The ticks, one per division, unless they would crowd each other.
        if let Some(n) = state.divisions.filter(|n| *n > 0) {
            let tick_r = track_h / 4.0;
            let adjusted = width - track_h;
            if tick_r > 0.0 && adjusted / n as f32 >= 3.0 * 2.0 * tick_r {
                for i in 0..=n {
                    let x = left + i as f32 / n as f32 * adjusted + track_h / 2.0;
                    let color = if x >= lo && x <= hi {
                        palette.active_tick
                    } else {
                        palette.inactive_tick
                    };
                    scene.draw_rect(
                        Rect::new(x - tick_r, cy - tick_r, 2.0 * tick_r, 2.0 * tick_r),
                        color.fade(o),
                        tick_r,
                        0.0,
                        Color::TRANSPARENT,
                    );
                }
            }
        }

        // The thumb a drag took (the end, until one has) is the one on top, the one that
        // rises while held and wears the halo — the reference's last selection.
        let top = status.thumb.unwrap_or(1).min(1);
        let xs = [x0, x1];
        let press = status.press_progress.clamp(0.0, 1.0);
        if state.enabled && press > 0.0 {
            let r = OVERLAY_RADIUS * press;
            let x = xs[top];
            scene.draw_rect(
                Rect::new(x - r, cy - r, 2.0 * r, 2.0 * r),
                state.halo(&status, theme).fade(o),
                r,
                0.0,
                Color::TRANSPARENT,
            );
        }
    }

    fn paints_over(&self) -> bool {
        true
    }

    /// The thumbs, and the values over them, over the thumbs' focus stops: a stop's halo
    /// belongs under its thumb, as the reference paints it.
    fn paint_over(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let state = &self.state;
        let palette = state.palette(theme);
        let (cy, xs) = self.thumb_centres(bounds, theme);
        let [x0, x1] = xs;
        let top = status.thumb.unwrap_or(1).min(1);
        let press = status.press_progress.clamp(0.0, 1.0);

        // The thumbs over their shadows, the one on top last. Where they overlap, it wears
        // a ring in the surface colour, as the reference's does, so the two stay two.
        let overlap = (x1 - x0).abs() < 2.0 * THUMB_RADIUS;
        let resting = Status {
            press_progress: 0.0,
            ..status
        };
        for i in [1 - top, top] {
            let x = xs[i];
            let thumb = Rect::new(
                x - THUMB_RADIUS,
                cy - THUMB_RADIUS,
                2.0 * THUMB_RADIUS,
                2.0 * THUMB_RADIUS,
            );
            let held = if i == top { &status } else { &resting };
            paint_thumb_shadow(thumb, state.elevations, held, theme, scene);
            let (ring_width, ring) = if overlap && i == top {
                (1.0, theme.scheme.surface.fade(o))
            } else {
                (0.0, Color::TRANSPARENT)
            };
            scene.draw_rect(thumb, palette.thumb.fade(o), THUMB_RADIUS, ring_width, ring);
        }

        // Both tooltips while the slider is moved, as the reference's does, the one on top
        // last.
        if let Some(label) = state.label.as_ref() {
            let reveal = if state.always_indicates(theme) {
                1.0
            } else if state.indicates(theme) {
                press
            } else {
                0.0
            };
            if reveal > 0.0 {
                let (start, end) = state.held();
                let values = [start, end];
                for i in [1 - top, top] {
                    paint_rect_indicator(
                        Point::new(xs[i], cy),
                        label(values[i]),
                        theme,
                        reveal,
                        o,
                        scene,
                    );
                }
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn draggable(&self) -> bool {
        self.state.enabled && self.state.on_change.is_some()
    }

    /// The track, inset by the halo's radius and mirrored right to left, with the two
    /// thumbs on it: the shell picks the one a press takes.
    fn drag_track(&self) -> crate::widget::DragTrack {
        crate::widget::DragTrack {
            inset: OVERLAY_RADIUS,
            mirrored: true,
            mode: crate::widget::DragMode::Thumbs {
                values: self.state.fractions(),
                radius: OVERLAY_RADIUS,
            },
        }
    }

    fn on_thumb_drag(
        &self,
        thumb: usize,
        moment: crate::widget::ThumbDrag,
        fraction: f32,
    ) -> Option<Msg> {
        if !self.state.enabled {
            return None;
        }
        let state = &self.state;
        match moment {
            crate::widget::ThumbDrag::Start => {
                let (start, end) = state.held();
                let (start, end) = (
                    state.at(state.fraction_of(start)),
                    state.at(state.fraction_of(end)),
                );
                state.on_change_start.as_ref().map(|make| make(start, end))
            }
            crate::widget::ThumbDrag::Move => {
                let (start, end) = state.moved(thumb, fraction);
                state.on_change.as_ref().map(|make| make(start, end))
            }
            crate::widget::ThumbDrag::End => {
                let (start, end) = state.moved(thumb, fraction);
                state.on_change_end.as_ref().map(|make| make(start, end))
            }
        }
    }
}

/// The reference's **rectangular value indicator** (`RectangularSliderValueIndicatorShape`),
/// a range slider's default: a box with corners of 4, sixteen wider and taller than its text,
/// over an eight-pixel triangle whose point is fourteen above the thumb's centre `at`; grown
/// from that point by `scale`. In the theme's indicator colour, else the scheme's
/// `on_surface` at 60 % over its `surface` at 90 %.
fn paint_rect_indicator(
    at: Point,
    text: String,
    theme: &Theme,
    scale: f32,
    o: f32,
    scene: &mut Scene,
) {
    const TRIANGLE: f32 = 8.0;
    const PADDING: f32 = 16.0;
    const MIN_LABEL: f32 = 16.0;
    const TIP_Y: f32 = 14.0;
    let style = theme
        .widgets
        .slider
        .value_indicator_text_style
        .unwrap_or_else(|| {
            crate::theme::type_scale(Some(theme))
                .body_large
                .color(theme.scheme.on_primary)
        })
        .resolved();
    let measured = frus_text::measure_resolved(&text, &style);
    let rect_w = MIN_LABEL.max(measured.width) + 2.0 * PADDING;
    let rect_h = measured.height + PADDING;
    let tip = Point::new(at.x, at.y - TIP_Y);
    let s = |dx: f32, dy: f32| Point::new(tip.x + dx * scale, tip.y + dy * scale);
    let scheme = &theme.scheme;
    let color = theme
        .widgets
        .slider
        .value_indicator_color
        .unwrap_or_else(|| scheme.surface.fade(0.90).lerp(scheme.on_surface, 0.60))
        .fade(o);
    let top_left = s(-rect_w / 2.0, -TRIANGLE - rect_h);
    let body = Rect::new(top_left.x, top_left.y, rect_w * scale, rect_h * scale);
    let path = frus_core::Path::new()
        .move_to(s(0.0, 0.0))
        .line_to(s(-TRIANGLE, -TRIANGLE))
        .line_to(s(TRIANGLE, -TRIANGLE))
        .close();
    scene.fill_path(&path, color);
    scene.draw_rect(body, color, 4.0 * scale, 0.0, Color::TRANSPARENT);
    if scale >= 0.999 {
        let text_color = style.color.unwrap_or(scheme.on_primary);
        scene.text(
            Point::new(
                body.x + (body.width - measured.width) * 0.5,
                body.y + (body.height - measured.height) * 0.5,
            ),
            text,
            &style,
            text_color.fade(o),
        );
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
    ///
    /// The start is the value **before** the press (milestone 650), as the reference's
    /// `onChangeStart` is: a tap is a start at the old value and an end at the new one.
    #[test]
    fn the_bracket_speaks_in_values_not_fractions() {
        let slider = Slider::new(10.0)
            .range(0.0, 100.0)
            .on_change(Msg::Value)
            .on_change_start(Msg::Start)
            .on_change_end(Msg::End);
        assert_eq!(Widget::on_drag_start(&slider, 0.25), Some(Msg::Start(10.0)));
        assert_eq!(Widget::on_drag(&slider, 0.25), Some(Msg::Value(25.0)));
        assert_eq!(Widget::on_drag_end(&slider, 0.75), Some(Msg::End(75.0)));
    }

    /// The divisions apply at the ends too: a start or an end landing between two
    /// stops would name a value the stream can never produce.
    #[test]
    fn the_bracket_snaps_like_the_stream() {
        let slider = Slider::new(0.4)
            .divisions(4)
            .on_change(Msg::Value)
            .on_change_start(Msg::Start)
            .on_change_end(Msg::End);
        // 0.3 lies between the 0.25 and 0.5 stops, nearer the first; the value before,
        // 0.4, nearer the second.
        assert_eq!(Widget::on_drag(&slider, 0.3), Some(Msg::Value(0.25)));
        assert_eq!(Widget::on_drag_start(&slider, 0.3), Some(Msg::Start(0.5)));
        assert_eq!(Widget::on_drag_end(&slider, 0.3), Some(Msg::End(0.25)));
    }

    /// **Each interaction names its drag mode** (milestone 650): tap and slide follows the
    /// pointer, tap only takes the press, the two slides move from where the value is, and
    /// the thumb's within its halo. The theme's is taken when the widget names none.
    #[test]
    fn each_interaction_names_its_drag_mode() {
        use crate::widget::DragMode;
        let mode = |s: Slider<Msg>| Widget::<Msg>::drag_track(&s).mode;
        let at = || Slider::<Msg>::new(0.3);
        assert_eq!(mode(at()), DragMode::Follow, "tap and slide by default");
        assert_eq!(
            mode(at().allowed_interaction(SliderInteraction::TapOnly)),
            DragMode::TapOnly
        );
        assert_eq!(
            mode(at().allowed_interaction(SliderInteraction::SlideOnly)),
            DragMode::Slide { origin: 0.3 }
        );
        assert_eq!(
            mode(at().allowed_interaction(SliderInteraction::SlideThumb)),
            DragMode::SlideThumb {
                origin: 0.3,
                radius: 24.0
            }
        );
        let mut theme = Theme::default();
        theme.widgets.slider.allowed_interaction = Some(SliderInteraction::TapOnly);
        let themed = at();
        Widget::<Msg>::build_themed(&themed, &theme);
        assert_eq!(mode(themed), DragMode::TapOnly, "from the theme");
        let mine = at().allowed_interaction(SliderInteraction::SlideOnly);
        Widget::<Msg>::build_themed(&mine, &theme);
        assert_eq!(
            mode(mine),
            DragMode::Slide { origin: 0.3 },
            "over the theme's"
        );
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

    #[test]
    fn drag_maps_to_value() {
        let slider = Slider::new(0.0).on_change(Msg::Value);
        assert_eq!(Widget::on_drag(&slider, 0.5), Some(Msg::Value(0.5)));
        // Clamped.
        assert_eq!(Widget::on_drag(&slider, 1.5), Some(Msg::Value(1.0)));
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
            // The crisp rectangles: the thumb's shadow is painted just before the thumb.
            let fills: Vec<frus_core::Color> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Rect { color, blur, .. } if *blur == 0.0 => Some(*color),
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
    /// **A thumb stands off its track, and higher while it is held.** The reference's round
    /// thumb rests at 1 and rises to 6 as the press goes in, on a slider and on each of a
    /// range slider's two. Neither cast anything here.
    #[test]
    fn a_thumb_casts_a_shadow_that_rises_while_it_is_held() {
        // The heights the painted shadows stand for (milestone 606).
        fn heights(widget: &dyn Widget<Msg>, bounds: Rect, press: f32, theme: &Theme) -> Vec<f32> {
            let status = Status {
                opacity: 1.0,
                press_progress: press,
                ..Default::default()
            };
            let mut scene = Scene::new();
            widget.paint(bounds, status, theme, &mut scene);
            crate::shadowprobe::heights(scene.primitives())
        }
        let theme = Theme::default();
        let track = Rect::new(0.0, 0.0, 220.0, H);
        let slider = Slider::new(0.5).on_change(Msg::Value);
        assert_eq!(heights(&slider, track, 0.0, &theme), vec![1.0]);
        assert_eq!(heights(&slider, track, 1.0, &theme), vec![6.0]);
        assert_eq!(
            heights(&slider, track, 0.5, &theme),
            vec![3.5],
            "on its way up"
        );
        let grounded = Slider::new(0.5)
            .on_change(Msg::Value)
            .thumb_elevation(0.0)
            .pressed_thumb_elevation(0.0);
        assert!(heights(&grounded, track, 1.0, &theme).is_empty());

        let mut flat = Theme::default();
        flat.widgets.slider.thumb_elevation = Some(0.0);
        flat.widgets.slider.pressed_thumb_elevation = Some(0.0);
        assert!(
            heights(&slider, track, 1.0, &flat).is_empty(),
            "the theme's word"
        );
        let told = Slider::new(0.5).on_change(Msg::Value).thumb_elevation(2.0);
        assert_eq!(
            heights(&told, track, 0.0, &flat),
            vec![2.0],
            "and the slider's over it"
        );
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

    /// The drop's type is the reference's 2023 one, `labelMedium` in `on_primary`, and
    /// comes from the theme when it names one (milestones 414 and 648).
    #[test]
    fn the_value_drop_wears_the_reference_s_type() {
        let slider = Slider::<Msg>::new(0.3)
            .divisions(10)
            .value_label(|v| format!("{v}"));
        let styles = |theme: &Theme| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &slider,
                Rect::new(0.0, 0.0, 220.0, 48.0),
                Status {
                    focused: true,
                    focus_progress: 1.0,
                    ..Status::default()
                },
                theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Text {
                        size,
                        weight,
                        color,
                        ..
                    } => Some((*size, weight.to_u16(), *color)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let plain = Theme::default();
        assert_eq!(
            styles(&plain),
            vec![(
                plain.text.label_medium.size.unwrap(),
                frus_core::FontWeight::Medium.to_u16(),
                plain.scheme.on_primary
            )]
        );
        // And a theme can move it.
        let mut themed = Theme::default();
        themed.widgets.slider.value_indicator_text_style = Some(frus_core::TextStyle::new(25.0));
        assert_eq!(styles(&themed).first().map(|s| s.0), Some(25.0));
    }

    /// **As tall as its halo**, label or not: the drop is drawn over what is above, as
    /// the reference's is, and takes no room.
    #[test]
    fn the_slider_is_as_tall_as_its_halo() {
        let bare = Widget::<Msg>::style(&Slider::<Msg>::new(0.5)).height;
        let labelled =
            Widget::<Msg>::style(&Slider::<Msg>::new(0.5).value_label(|v| format!("{v}"))).height;
        assert_eq!(bare, frus_layout::Dimension::Length(48.0));
        assert_eq!(labelled, bare);
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use frus_core::Primitive;

    const BRAND: Color = Color::rgb(0.0, 0.6, 0.3);
    const RAIL: Color = Color::rgb(0.9, 0.9, 0.2);

    /// (rail, travelled, thumb, ring) as painted — the crisp rectangles, in order, and not
    /// the soft one under the thumb that is its shadow.
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
                    blur,
                    ..
                } if *blur == 0.0 => Some((*color, *border_color)),
                _ => None,
            })
            .collect();
        (rects[0].0, rects[1].0, rects[2].0, rects[2].1)
    }

    /// Nothing said: the reference's 2023 colours — `primary` travelled and for the
    /// thumb, `surface_container_highest` for the rest, and no ring.
    #[test]
    fn the_defaults_are_the_reference_s() {
        let theme = Theme::default();
        let (rail, filled, thumb, ring) = painted(&Slider::<()>::new(0.5), &theme);
        assert_eq!(rail, theme.scheme.surface_container_highest);
        assert_eq!(filled, theme.scheme.primary);
        assert_eq!(thumb, theme.scheme.primary);
        assert_eq!(ring, Color::TRANSPARENT);
    }

    /// The thumb follows the travelled track unless it is named — the reference's
    /// `thumbColor ?? activeColor` — and there is a ring only when one is asked for.
    #[test]
    fn the_thumb_follows_the_track_it_was_not_told_about() {
        let theme = Theme::default();
        let (_, filled, thumb, ring) = painted(&Slider::<()>::new(0.5).active_color(BRAND), &theme);
        assert_eq!((filled, thumb, ring), (BRAND, BRAND, Color::TRANSPARENT));
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

/// The reference's 2023 slider, painted and measured (milestone 648).
#[cfg(test)]
mod reference_tests {
    use super::*;
    use frus_core::Primitive;

    /// A box 248 wide: a track of 200 from 24, the halo's radius in.
    const BOX: Rect = Rect::new(0.0, 0.0, 248.0, 48.0);

    fn rtl() -> Theme {
        Theme {
            direction: frus_core::TextDirection::Rtl,
            ..Theme::default()
        }
    }

    fn scene_of(slider: &Slider<()>, status: Status, theme: &Theme) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(slider, BOX, status, theme, &mut scene);
        scene.primitives().to_vec()
    }

    fn rects(primitives: &[Primitive]) -> Vec<(Rect, Color)> {
        primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect, color, blur, ..
                } if *blur == 0.0 => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    /// The thumb: the crisp 20-pixel square.
    fn thumb(primitives: &[Primitive]) -> Rect {
        rects(primitives)
            .into_iter()
            .map(|(r, _)| r)
            .rfind(|r| r.width == 20.0 && r.height == 20.0)
            .expect("a thumb")
    }

    fn texts(primitives: &[Primitive]) -> usize {
        primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Text { .. }))
            .count()
    }

    fn rest() -> Status {
        Status::default()
    }

    /// **The track is inset by the halo's radius**, so the thumb reaches both ends inside
    /// the box; the travelled part is two pixels thicker than the rest.
    #[test]
    fn the_track_is_inset_and_the_travelled_part_thicker() {
        let theme = Theme::default();
        let painted = scene_of(&Slider::new(0.5), rest(), &theme);
        assert_eq!(thumb(&painted), Rect::new(114.0, 14.0, 20.0, 20.0));
        let at = |v: f32| thumb(&scene_of(&Slider::new(v), rest(), &theme)).x + 10.0;
        assert_eq!((at(0.0), at(1.0)), (24.0, 224.0), "the ends, a radius in");
        let track = rects(&painted);
        let (inactive, active) = (track[0].0, track[1].0);
        assert_eq!(inactive.height, 4.0);
        assert_eq!(active.height, 6.0);
        assert_eq!(active.x, 24.0, "from the start");
        assert_eq!(inactive.x + inactive.width, 224.0, "to the end");
    }

    /// **Right to left, it runs from the right**: a quarter is near the right end, and the
    /// travelled part is on the right.
    #[test]
    fn right_to_left_it_runs_from_the_right() {
        let painted = scene_of(&Slider::new(0.25), rest(), &rtl());
        assert_eq!(thumb(&painted).x + 10.0, 174.0);
        let track = rects(&painted);
        let (inactive, active) = (track[0].0, track[1].0);
        assert_eq!(
            active.x + active.width,
            224.0,
            "the travelled part ends at the right"
        );
        assert_eq!(inactive.x, 24.0, "the rest starts at the left");
    }

    /// **A tick per division**, the first and last under the thumb at the ends, and none
    /// when they would crowd.
    #[test]
    fn a_tick_marks_each_division() {
        let theme = Theme::default();
        let ticks = |slider: Slider<()>| {
            rects(&scene_of(&slider, rest(), &theme))
                .into_iter()
                .filter(|(r, _)| r.width == 2.0 && r.height == 2.0)
                .collect::<Vec<_>>()
        };
        let four = ticks(Slider::new(0.5).divisions(4));
        assert_eq!(four.len(), 5);
        assert_eq!(four[0].0.x + 1.0, 26.0, "half a track in");
        assert_eq!(four[4].0.x + 1.0, 222.0);
        // The travelled ticks and the rest differ.
        assert_eq!(four[0].1, theme.scheme.on_primary.fade(0.38));
        assert_eq!(four[4].1, theme.scheme.on_surface_variant.fade(0.38));
        assert!(
            ticks(Slider::new(0.5).divisions(100)).is_empty(),
            "too dense"
        );
        assert!(ticks(Slider::new(0.5)).is_empty(), "continuous");
        // The discrete thumb sits over the first and last ticks.
        let end = thumb(&scene_of(&Slider::new(1.0).divisions(4), rest(), &theme));
        assert_eq!(end.x + 10.0, 222.0);
    }

    /// **The secondary value** lightens the track from the thumb to it, when it is ahead.
    #[test]
    fn the_secondary_value_lightens_the_track_ahead() {
        let theme = Theme::default();
        let secondary = |slider: Slider<()>| {
            rects(&scene_of(&slider, rest(), &theme))
                .into_iter()
                .find(|(_, c)| *c == theme.scheme.primary.fade(0.54))
        };
        let (rect, _) =
            secondary(Slider::new(0.25).secondary_track_value(0.75)).expect("a secondary part");
        assert_eq!((rect.x, rect.x + rect.width), (74.0, 174.0));
        assert!(
            secondary(Slider::new(0.75).secondary_track_value(0.25)).is_none(),
            "behind"
        );
        let given = Color::rgb(0.2, 0.4, 0.6);
        let painted = scene_of(
            &Slider::new(0.25)
                .secondary_track_value(0.75)
                .secondary_active_color(given),
            rest(),
            &theme,
        );
        assert!(rects(&painted).iter().any(|(_, c)| *c == given));
    }

    /// **When the drop shows**: on a slider with divisions while it is moved or focused,
    /// by default; the other policies as they say; never on a disabled one.
    #[test]
    fn the_drop_shows_as_its_policy_says() {
        let theme = Theme::default();
        let focused = Status {
            focused: true,
            focus_progress: 1.0,
            ..Status::default()
        };
        let pressed = Status {
            press_progress: 1.0,
            ..Status::default()
        };
        let labelled = || Slider::<()>::new(0.5).value_label(|v| format!("{v}"));
        let shows = |s: Slider<()>, status: Status| texts(&scene_of(&s, status, &theme)) == 1;
        assert!(shows(labelled().divisions(4), focused));
        assert!(shows(labelled().divisions(4), pressed));
        assert!(!shows(labelled().divisions(4), rest()), "at rest, hidden");
        assert!(
            !shows(labelled(), focused),
            "continuous: only for discrete by default"
        );
        assert!(shows(
            labelled().show_value_indicator(ShowValueIndicator::OnlyForContinuous),
            focused
        ));
        assert!(!shows(
            labelled()
                .divisions(4)
                .show_value_indicator(ShowValueIndicator::OnlyForContinuous),
            focused
        ));
        assert!(shows(
            labelled().show_value_indicator(ShowValueIndicator::OnDrag),
            pressed
        ));
        assert!(shows(
            labelled().show_value_indicator(ShowValueIndicator::AlwaysVisible),
            rest()
        ));
        assert!(!shows(
            labelled()
                .divisions(4)
                .show_value_indicator(ShowValueIndicator::Never),
            focused
        ));
        assert!(
            !shows(labelled().divisions(4).enabled(false), focused),
            "disabled"
        );
        let mut themed = Theme::default();
        themed.widgets.slider.show_value_indicator = Some(ShowValueIndicator::AlwaysVisible);
        assert_eq!(
            texts(&scene_of(&labelled(), rest(), &themed)),
            1,
            "from the theme"
        );
    }

    /// **The drop**: a pill 28 high whose bottom is 22 above the thumb's centre, over a
    /// triangle pointing at it, in `primary`.
    #[test]
    fn the_drop_sits_over_the_thumb() {
        let theme = Theme::default();
        let slider = Slider::<()>::new(0.5)
            .show_value_indicator(ShowValueIndicator::AlwaysVisible)
            .value_label(|_| "50".to_string());
        let painted = scene_of(&slider, rest(), &theme);
        let pill = rects(&painted)
            .into_iter()
            .find(|(r, _)| r.height == 28.0)
            .expect("the pill");
        assert_eq!(pill.0.y + pill.0.height, 24.0 - 22.0);
        assert!(
            (pill.0.x + pill.0.width / 2.0 - 124.0).abs() < 0.01,
            "centred on the thumb"
        );
        assert_eq!(pill.1, theme.scheme.primary);
        assert!(
            painted.iter().any(|p| matches!(p, Primitive::Path { .. })),
            "the point"
        );
    }

    /// **The halo**: on hover, focus and press, in `primary` at 8 % hovered and 10 %
    /// pressed; none at rest, none disabled; a colour given at 12 %.
    #[test]
    fn the_halo_follows_the_state() {
        let theme = Theme::default();
        let halo = |slider: Slider<()>, status: Status| {
            rects(&scene_of(&slider, status, &theme))
                .into_iter()
                .find(|(r, _)| r.width == 48.0)
                .map(|(_, c)| c)
        };
        let hovered = Status {
            interaction: crate::interaction::Interaction::Hovered,
            hover_progress: 1.0,
            ..Status::default()
        };
        let pressed = Status {
            interaction: crate::interaction::Interaction::Pressed,
            press_progress: 1.0,
            ..Status::default()
        };
        assert_eq!(halo(Slider::new(0.5), rest()), None);
        assert_eq!(
            halo(Slider::new(0.5), hovered),
            Some(theme.scheme.primary.fade(0.08))
        );
        assert_eq!(
            halo(Slider::new(0.5), pressed),
            Some(theme.scheme.primary.fade(0.1))
        );
        assert_eq!(halo(Slider::new(0.5).enabled(false), hovered), None);
        let given = Color::rgb(0.0, 0.6, 0.3);
        assert_eq!(
            halo(Slider::new(0.5).active_color(given), hovered),
            Some(given.fade(0.12))
        );
        let property = WidgetStateProperty::new().otherwise(Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(
            halo(Slider::new(0.5).overlay_color(property), hovered),
            Some(Color::rgb(1.0, 0.0, 0.0))
        );
    }

    /// **Disabled**: the travelled part and the thumb as content, the rest as a
    /// container, the framework's disabled rule.
    #[test]
    fn a_disabled_slider_wears_the_disabled_rule() {
        let theme = Theme::default();
        let painted = scene_of(&Slider::new(0.5).enabled(false), rest(), &theme);
        let r = rects(&painted);
        assert_eq!(r[0].1, disabled_container(&theme));
        assert_eq!(r[1].1, disabled_content(&theme));
        assert_eq!(r.last().map(|x| x.1), Some(disabled_content(&theme)));
    }

    /// **It takes the width it is offered**, at least three touch targets of track, unless
    /// a width is given.
    #[test]
    fn it_takes_the_width_it_is_offered() {
        let theme = Theme::default();
        let open = Slider::<()>::new(0.5);
        assert!(Widget::<()>::fill_axes(&open, &theme).horizontal);
        assert!(Widget::<()>::soft_extent(&open).horizontal);
        assert_eq!(
            Widget::<()>::style(&open).min_width,
            frus_layout::Dimension::Length(192.0)
        );
        let fixed = Slider::<()>::new(0.5).width(120.0);
        assert!(!Widget::<()>::fill_axes(&fixed, &theme).horizontal);
        assert_eq!(
            Widget::<()>::style(&fixed).width,
            frus_layout::Dimension::Length(120.0)
        );
    }

    /// **A drag lands on the track**: inset by the halo's radius, mirrored right to left.
    #[test]
    fn a_drag_lands_on_the_inset_track() {
        let track = Widget::<()>::drag_track(&Slider::<()>::new(0.5));
        assert_eq!(track.inset, 24.0);
        assert!(track.mirrored);
        assert_eq!(track.fraction(BOX, 24.0, false), 0.0);
        assert_eq!(track.fraction(BOX, 124.0, false), 0.5);
        assert_eq!(track.fraction(BOX, 224.0, false), 1.0);
        assert_eq!(track.fraction(BOX, 74.0, true), 0.75, "from the right");
        assert_eq!(track.fraction(BOX, 0.0, false), 0.0, "held at the ends");
        assert_eq!(track.fraction(BOX, 248.0, false), 1.0);
        let whole = crate::widget::DragTrack::WHOLE;
        assert_eq!(
            whole.fraction(BOX, 62.0, true),
            0.25,
            "no opinion: left to right"
        );
        let empty = Rect::new(0.0, 0.0, 40.0, 48.0);
        assert_eq!(track.fraction(empty, 20.0, false), 0.0, "no travel");
    }

    /// **A screen reader hears the semantic formatter** over the indicator's label.
    #[test]
    fn a_reader_hears_the_semantic_formatter() {
        let slider = Slider::<()>::new(0.5)
            .value_label(|v| format!("{v}"))
            .semantic_formatter(|v| format!("{} percent", (v * 100.0).round()));
        let said = Widget::<()>::semantics(&slider).and_then(|s| s.value);
        assert_eq!(said.as_deref(), Some("50 percent"));
    }

    /// **Each drag mode moves the value as the reference's interaction does** (milestone
    /// 650), on a track of 200 from 24: following the pointer; taking the press only;
    /// sliding from where the value was by the pointer's own movement, past the ends too;
    /// and only from a press on the thumb.
    #[test]
    fn the_drag_modes_move_the_value_as_the_reference_s_do() {
        use crate::widget::{DragMode, DragTrack};
        let track = |mode| DragTrack {
            inset: 24.0,
            mirrored: true,
            mode,
        };
        let follow = track(DragMode::Follow);
        assert_eq!(
            follow.value_at(BOX, 124.0, 124.0, false, false),
            Some(0.5),
            "the press"
        );
        assert_eq!(
            follow.value_at(BOX, 124.0, 174.0, true, false),
            Some(0.75),
            "the slide"
        );
        assert_eq!(follow.released_at(BOX, 124.0, 174.0, false), 0.75);
        assert!(follow.engages(BOX, 0.0, false));

        let tap = track(DragMode::TapOnly);
        assert_eq!(tap.value_at(BOX, 124.0, 124.0, false, false), Some(0.5));
        assert_eq!(
            tap.value_at(BOX, 124.0, 174.0, true, false),
            None,
            "no slide"
        );
        assert_eq!(
            tap.released_at(BOX, 124.0, 174.0, false),
            0.5,
            "where the press set it"
        );

        let slide = track(DragMode::Slide { origin: 0.2 });
        assert_eq!(
            slide.value_at(BOX, 124.0, 124.0, false, false),
            None,
            "no jump"
        );
        assert_eq!(slide.value_at(BOX, 124.0, 174.0, true, false), Some(0.45));
        assert_eq!(slide.released_at(BOX, 124.0, 174.0, false), 0.45);
        assert_eq!(
            slide.value_at(BOX, 124.0, 1000.0, true, false),
            Some(1.0),
            "held at the end"
        );
        // A slide that starts past the end still measures its whole length.
        // 1.08 to 0.83 of the travel: a quarter back from 0.2, held at the start.
        assert_eq!(slide.value_at(BOX, 240.0, 190.0, true, false), Some(0.0));
        // Right to left, the same rightward slide lowers it.
        assert_eq!(slide.value_at(BOX, 124.0, 174.0, true, true), Some(0.0));
        let rtl_slide = track(DragMode::Slide { origin: 0.6 });
        let moved = rtl_slide.value_at(BOX, 174.0, 124.0, true, true).unwrap();
        assert!(
            (moved - 0.85).abs() < 1e-6,
            "a slide leftwards raises it right to left"
        );

        let thumb = track(DragMode::SlideThumb {
            origin: 0.2,
            radius: 24.0,
        });
        assert!(thumb.engages(BOX, 64.0, false), "on the thumb");
        assert!(thumb.engages(BOX, 88.0, false), "at the halo's edge");
        assert!(!thumb.engages(BOX, 124.0, false), "off it");
        assert!(
            thumb.engages(BOX, 184.0, true),
            "right to left, the thumb is on the right"
        );
        assert!(!thumb.engages(BOX, 64.0, true));
        assert_eq!(thumb.value_at(BOX, 70.0, 120.0, true, false), Some(0.45));
    }
}

#[cfg(test)]
mod range_slider_tests {
    use super::*;
    use crate::widget::{DragMode, DragTrack, ThumbDrag};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Range(f32, f32),
        Start(f32, f32),
        End(f32, f32),
    }

    /// A box 248 wide: a track of 200 from 24, the halo's radius in.
    const BOX: Rect = Rect::new(0.0, 0.0, 248.0, 48.0);

    fn slider(start: f32, end: f32) -> RangeSlider<Msg> {
        RangeSlider::new(start, end)
            .on_change(Msg::Range)
            .on_change_start(Msg::Start)
            .on_change_end(Msg::End)
    }

    /// The two thumbs, the start first: each the child of an alignment in the stack.
    fn thumbs(slider: &RangeSlider<Msg>) -> Vec<&dyn Widget<Msg>> {
        Widget::<Msg>::children(slider)[0]
            .children()
            .iter()
            .map(|aligned| aligned.children()[0].as_ref())
            .collect()
    }

    /// What the slider paints under its thumbs' stops and over them, in that order.
    fn paint(slider: &RangeSlider<Msg>, status: Status, theme: &Theme) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<Msg>::paint(slider, BOX, status, theme, &mut scene);
        assert!(Widget::<Msg>::paints_over(slider));
        Widget::<Msg>::paint_over(slider, BOX, status, theme, &mut scene);
        scene.primitives().to_vec()
    }

    fn rects(primitives: &[Primitive]) -> Vec<(Rect, Color)> {
        primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect, color, blur, ..
                } if *blur == 0.0 => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    /// The thumbs' squares, in the order they were painted.
    fn thumb_rects(primitives: &[Primitive]) -> Vec<Rect> {
        rects(primitives)
            .into_iter()
            .map(|(r, _)| r)
            .filter(|r| r.width == 20.0 && r.height == 20.0)
            .collect()
    }

    fn texts(primitives: &[Primitive]) -> usize {
        primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Text { .. }))
            .count()
    }

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4
    }

    fn range(msg: Option<Msg>) -> (f32, f32) {
        match msg {
            Some(Msg::Range(s, e)) => (s, e),
            other => panic!("expected a range, got {other:?}"),
        }
    }

    fn keyed(response: KeyResponse<Msg>) -> (f32, f32) {
        match response {
            KeyResponse::Handled(msg) => range(msg),
            other => panic!("expected a handled key, got {other:?}"),
        }
    }

    const RIGHT: Key = Key::Right {
        shift: false,
        word: false,
    };

    /// **The track the shell drags is the slider's own**: inset by the halo's radius,
    /// mirrored right to left, with both thumbs on it at their places along the travel —
    /// in the slider's units, held in order.
    #[test]
    fn the_drag_track_carries_both_thumbs() {
        let s = slider(30.0, 80.0).range(20.0, 120.0);
        assert!(Widget::<Msg>::draggable(&s));
        let track = Widget::<Msg>::drag_track(&s);
        assert_eq!(track.inset, 24.0);
        assert!(track.mirrored);
        assert_eq!(
            track.mode,
            DragMode::Thumbs {
                values: [0.1, 0.6],
                radius: 24.0
            }
        );
        let backwards = slider(0.9, 0.1);
        assert_eq!(
            Widget::<Msg>::drag_track(&backwards).mode,
            DragMode::Thumbs {
                values: [0.1, 0.9],
                radius: 24.0
            },
            "put in order"
        );
    }

    /// **A press takes the nearer thumb**, and one on both waits for the first movement:
    /// towards the start takes the start — leftwards, or rightwards right to left.
    #[test]
    fn a_press_picks_the_nearer_thumb() {
        let track = DragTrack {
            inset: 24.0,
            mirrored: true,
            mode: DragMode::Thumbs {
                values: [0.2, 0.8],
                radius: 24.0,
            },
        };
        // The thumbs at 64 and 184; between them the middle, 124, divides.
        assert_eq!(track.pick_thumb(BOX, 30.0, 0.0, false), Some(0));
        assert_eq!(track.pick_thumb(BOX, 123.0, 0.0, false), Some(0));
        assert_eq!(track.pick_thumb(BOX, 125.0, 0.0, false), Some(1));
        assert_eq!(track.pick_thumb(BOX, 240.0, 0.0, false), Some(1));
        // Right to left, the start is on the right.
        assert_eq!(track.pick_thumb(BOX, 184.0, 0.0, true), Some(0));
        assert_eq!(track.pick_thumb(BOX, 64.0, 0.0, true), Some(1));

        let met = DragTrack {
            mode: DragMode::Thumbs {
                values: [0.5, 0.55],
                radius: 24.0,
            },
            ..track
        };
        // At 124 and 134: a press at 129 is on both.
        assert_eq!(met.pick_thumb(BOX, 129.0, 0.0, false), None, "undecided");
        assert_eq!(met.pick_thumb(BOX, 129.0, -3.0, false), Some(0));
        assert_eq!(met.pick_thumb(BOX, 129.0, 3.0, false), Some(1));
        assert_eq!(
            met.pick_thumb(BOX, 119.0, 3.0, true),
            Some(0),
            "rightwards is towards the start, right to left"
        );
        assert_eq!(met.pick_thumb(BOX, 119.0, -3.0, true), Some(1));
        // Off one thumb's touch radius, the nearer one, whatever the movement.
        assert_eq!(met.pick_thumb(BOX, 100.0, 5.0, false), Some(0));
        let single = DragTrack {
            mode: DragMode::Follow,
            ..track
        };
        assert_eq!(single.pick_thumb(BOX, 64.0, 0.0, false), None);
        assert_eq!(met.value_at(BOX, 129.0, 174.0, true, false), Some(0.75));
    }

    /// **The drag's bracket**: the start reports the interval as it was, each movement and
    /// the end the interval with the picked thumb moved — in the slider's units, snapped.
    #[test]
    fn a_thumb_drag_reports_the_interval() {
        let s = slider(30.0, 80.0).range(20.0, 120.0).divisions(10);
        assert_eq!(
            Widget::on_thumb_drag(&s, 0, ThumbDrag::Start, 0.9),
            Some(Msg::Start(30.0, 80.0)),
            "as it was"
        );
        assert_eq!(
            Widget::on_thumb_drag(&s, 0, ThumbDrag::Move, 0.22),
            Some(Msg::Range(40.0, 80.0)),
            "the start moves, snapped"
        );
        assert_eq!(
            Widget::on_thumb_drag(&s, 1, ThumbDrag::Move, 0.87),
            Some(Msg::Range(30.0, 110.0)),
            "the end moves"
        );
        assert_eq!(
            Widget::on_thumb_drag(&s, 1, ThumbDrag::End, 1.4),
            Some(Msg::End(30.0, 120.0)),
            "held to the travel"
        );
        let dead = slider(30.0, 80.0).range(20.0, 120.0).enabled(false);
        assert!(!Widget::<Msg>::draggable(&dead));
        assert_eq!(Widget::on_thumb_drag(&dead, 0, ThumbDrag::Move, 0.5), None);
        let silent = RangeSlider::<Msg>::new(0.2, 0.8);
        assert!(!Widget::<Msg>::draggable(&silent), "nothing to tell");
    }

    /// **A thumb stops at the other**: on a slider with divisions it may meet it; without,
    /// it stays the reference's eight pixels off — measured on the track the last paint
    /// found, 200 here.
    #[test]
    fn a_thumb_never_passes_the_other() {
        // Steps fine enough that eight pixels would still show after the snap.
        let discrete = slider(0.2, 0.8).divisions(100);
        // Painted, so that it knows its track: the gap is none all the same.
        paint(&discrete, Status::default(), &Theme::default());
        let (s, e) = range(Widget::on_thumb_drag(&discrete, 0, ThumbDrag::Move, 0.95));
        assert!(close((s, e), (0.8, 0.8)), "met ({s}, {e})");

        let continuous = slider(0.2, 0.8);
        paint(&continuous, Status::default(), &Theme::default());
        let (s, e) = range(Widget::on_thumb_drag(&continuous, 0, ThumbDrag::Move, 0.95));
        assert!(close((s, e), (0.76, 0.8)), "eight pixels off ({s}, {e})");
        let (s, e) = range(Widget::on_thumb_drag(&continuous, 1, ThumbDrag::Move, 0.0));
        assert!(close((s, e), (0.2, 0.24)), "from above too ({s}, {e})");
        let (s, e) = range(Widget::on_thumb_drag(&continuous, 1, ThumbDrag::Move, 0.5));
        assert!(close((s, e), (0.2, 0.5)), "free in between ({s}, {e})");
    }

    /// **Each thumb is a stop of its own**: it takes the focus, tells a reader its value
    /// and its travel, and moves with the arrows — stopped by the other thumb.
    #[test]
    fn each_thumb_takes_the_keyboard_and_a_reader() {
        let s = slider(0.4, 0.6).divisions(10);
        let t = thumbs(&s);
        assert_eq!(t.len(), 2);
        assert!(t[0].focusable() && t[1].focusable());
        assert_eq!(Widget::<Msg>::semantics(&s), None, "the thumbs speak");
        let start = t[0].semantics().expect("the start");
        assert_eq!(start.role, frus_core::Role::Slider);
        assert_eq!(start.value.as_deref(), Some("40%"));
        assert_eq!(start.range, Some((0.0, 0.4, 1.0)));
        assert_eq!(
            t[1].semantics().and_then(|s| s.value).as_deref(),
            Some("60%")
        );

        assert!(close(keyed(t[0].on_key(&RIGHT)), (0.5, 0.6)));
        assert!(close(
            keyed(t[1].on_key(&Key::Left {
                shift: false,
                word: false
            })),
            (0.4, 0.5)
        ));
        let home = Key::Home {
            shift: false,
            doc: false,
        };
        let end = Key::End {
            shift: false,
            doc: false,
        };
        assert!(close(keyed(t[0].on_key(&home)), (0.0, 0.6)));
        assert!(close(keyed(t[0].on_key(&end)), (0.6, 0.6)), "stopped");
        assert!(close(keyed(t[1].on_key(&end)), (0.4, 1.0)));
        assert!(close(keyed(t[1].on_key(&home)), (0.4, 0.4)), "stopped");
        assert!(matches!(t[0].on_key(&Key::Enter), KeyResponse::Ignored));

        let labelled = slider(20.0, 60.0)
            .range(0.0, 100.0)
            .value_label(|v| format!("{v} °C"));
        let t = thumbs(&labelled);
        assert_eq!(
            t[1].semantics().and_then(|s| s.value).as_deref(),
            Some("60 °C")
        );
        assert_eq!(
            t[1].semantics().and_then(|s| s.range),
            Some((0.0, 60.0, 100.0)),
            "in the slider's units"
        );
        let told = labelled.semantic_formatter(|v| format!("{v} degrees"));
        assert_eq!(
            thumbs(&told)[0]
                .semantics()
                .and_then(|s| s.value)
                .as_deref(),
            Some("20 degrees")
        );
    }

    /// **Disabled, it is inert and still says where it sits**: no drag, no focus, no key.
    #[test]
    fn a_disabled_range_slider_takes_no_drag_no_key_and_no_focus() {
        let dead = slider(0.2, 0.8).enabled(false);
        assert!(!Widget::<Msg>::draggable(&dead));
        for (i, thumb) in thumbs(&dead).into_iter().enumerate() {
            assert!(!thumb.focusable(), "thumb {i} takes focus");
            assert!(
                matches!(thumb.on_key(&RIGHT), KeyResponse::Ignored),
                "thumb {i} answers an arrow"
            );
            let said = thumb.semantics().expect("still announced");
            assert!(said.disabled);
        }
        assert_eq!(
            thumbs(&dead)[0]
                .semantics()
                .and_then(|s| s.value)
                .as_deref(),
            Some("20%")
        );
    }

    /// **The reference's colours**: the span between the thumbs in `primary`, a little
    /// thicker, the rest `primary` at 24 %, the thumbs `primary`; the widget's colours over
    /// them.
    #[test]
    fn the_track_is_active_between_the_thumbs() {
        let theme = Theme::default();
        let primary = theme.scheme.primary;
        let found = rects(&paint(&slider(0.2, 0.8), Status::default(), &theme));
        let outer: Vec<_> = found
            .iter()
            .filter(|(_, c)| *c == primary.fade(0.24))
            .collect();
        assert_eq!(outer.len(), 2, "the two outer parts");
        assert_eq!(outer[0].0.x, 24.0);
        assert_eq!(outer[1].0.x + outer[1].0.width, 224.0);
        let span = found
            .iter()
            .find(|(r, c)| *c == primary && r.height == 6.0)
            .expect("the span");
        assert_eq!((span.0.x, span.0.width), (64.0, 120.0));
        assert_eq!(
            found
                .iter()
                .filter(|(r, c)| *c == primary && r.width == 20.0)
                .count(),
            2,
            "two thumbs"
        );

        let rtl = Theme {
            direction: frus_core::TextDirection::Rtl,
            ..Theme::default()
        };
        let found = rects(&paint(&slider(0.1, 0.4), Status::default(), &rtl));
        let span = found
            .iter()
            .find(|(r, c)| *c == primary && r.height == 6.0)
            .expect("the span");
        assert_eq!((span.0.x, span.0.width), (144.0, 60.0), "from the right");

        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let found = rects(&paint(
            &slider(0.2, 0.8).active_color(red).inactive_color(blue),
            Status::default(),
            &theme,
        ));
        assert!(found.iter().any(|(r, c)| *c == red && r.height == 6.0));
        assert_eq!(found.iter().filter(|(_, c)| *c == blue).count(), 2);
        assert_eq!(
            found
                .iter()
                .filter(|(r, c)| *c == red && r.width == 20.0)
                .count(),
            2,
            "the thumbs take the active colour"
        );
    }

    /// **The thumb a drag took is the one on top**: painted last, raised while held, with
    /// the halo — the end until a drag has taken one. Where the two overlap, the one on top
    /// wears a ring in the surface colour.
    #[test]
    fn the_thumb_last_taken_is_on_top() {
        let theme = Theme::default();
        let held = |thumb| Status {
            press_progress: 1.0,
            thumb,
            ..Default::default()
        };
        let s = slider(0.2, 0.8);
        let order = thumb_rects(&paint(&s, held(None), &theme));
        assert_eq!(order[1].x, 174.0, "the end on top");
        let order = thumb_rects(&paint(&s, held(Some(0)), &theme));
        assert_eq!(order[1].x, 54.0, "the start, once taken");

        let heights = |status| crate::shadowprobe::heights(&paint(&s, status, &theme));
        assert_eq!(heights(held(Some(0))), vec![1.0, 6.0], "only it rises");
        assert_eq!(heights(Status::default()), vec![1.0, 1.0]);
        let raised = slider(0.2, 0.8).thumb_elevation(3.0);
        assert_eq!(
            crate::shadowprobe::heights(&paint(&raised, Status::default(), &theme)),
            vec![3.0, 3.0]
        );

        let halo = theme.scheme.primary.fade(0.12);
        let found = rects(&paint(&s, held(Some(0)), &theme));
        let ring = found.iter().find(|(_, c)| *c == halo).expect("the halo");
        assert_eq!(ring.0, Rect::new(40.0, 0.0, 48.0, 48.0), "on the start");
        let found = rects(&paint(&s, held(None), &theme));
        let ring = found.iter().find(|(_, c)| *c == halo).expect("the halo");
        assert_eq!(
            ring.0,
            Rect::new(160.0, 0.0, 48.0, 48.0),
            "on the end, until one is taken"
        );

        let ringed = |s: &RangeSlider<Msg>| {
            paint(s, Status::default(), &theme)
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Rect { border_width, border_color, .. }
                        if *border_width == 1.0 && *border_color == theme.scheme.surface)
                })
                .count()
        };
        assert_eq!(ringed(&slider(0.5, 0.55)), 1, "overlapping");
        assert_eq!(ringed(&slider(0.2, 0.8)), 0, "apart");
    }

    /// **Both values show while it is moved**, in the reference's rectangular indicator, by
    /// the same policy as a slider's; a focused thumb shows its own.
    #[test]
    fn the_values_show_while_it_is_moved() {
        let theme = Theme::default();
        let pressed = Status {
            press_progress: 1.0,
            ..Default::default()
        };
        let labelled = |s: RangeSlider<Msg>| s.value_label(|v| format!("{v:.1}"));
        let discrete = labelled(slider(0.2, 0.8).divisions(10));
        assert_eq!(texts(&paint(&discrete, pressed, &theme)), 2, "both");
        assert_eq!(texts(&paint(&discrete, Status::default(), &theme)), 0);
        let continuous = labelled(slider(0.2, 0.8));
        assert_eq!(
            texts(&paint(&continuous, pressed, &theme)),
            0,
            "only for discrete, by default"
        );
        let always =
            labelled(slider(0.2, 0.8)).show_value_indicator(ShowValueIndicator::AlwaysVisible);
        assert_eq!(texts(&paint(&always, Status::default(), &theme)), 2);
        assert_eq!(
            texts(&paint(&slider(0.2, 0.8).divisions(10), pressed, &theme)),
            0,
            "nothing to say"
        );

        // The indicator's box: sixteen wider and taller than its text, its point fourteen
        // above the thumb's centre.
        let p = paint(&discrete, pressed, &theme);
        let indicator = theme
            .scheme
            .surface
            .fade(0.90)
            .lerp(theme.scheme.on_surface, 0.60);
        let boxes: Vec<Rect> = rects(&p)
            .into_iter()
            .filter(|(_, c)| *c == indicator)
            .map(|(r, _)| r)
            .collect();
        assert_eq!(boxes.len(), 2);
        let centre = boxes[0].x + boxes[0].width / 2.0;
        // With divisions the track pulls its ends in by half its height: 24 + 0.2 × 196 + 2.
        assert!(
            (centre - 65.2).abs() < 1e-3,
            "over the start, under: {centre}"
        );
        let bottom = boxes[0].y + boxes[0].height;
        assert!(
            (bottom - 2.0).abs() < 1e-3,
            "eight over the point: {bottom}"
        );

        // A focused thumb shows its own value, not the other's.
        let focused = Status {
            focus_progress: 1.0,
            ..Default::default()
        };
        let mut scene = Scene::new();
        thumbs(&discrete)[0].paint(Rect::new(0.0, 0.0, 48.0, 48.0), focused, &theme, &mut scene);
        assert_eq!(texts(scene.primitives()), 1);
    }

    /// **It takes the width it is offered**, as tall as its halo, at least three touch
    /// targets of track; a width given is kept.
    #[test]
    fn it_fills_the_width_it_is_offered() {
        let theme = Theme::default();
        let s = slider(0.2, 0.8);
        assert!(Widget::<Msg>::fill_axes(&s, &theme).horizontal);
        let style = Widget::<Msg>::style(&s);
        assert_eq!(style.height, Dimension::Length(48.0));
        assert_eq!(style.min_width, Dimension::Length(192.0));
        let sized = slider(0.2, 0.8).width(120.0);
        assert!(!Widget::<Msg>::fill_axes(&sized, &theme).horizontal);
        assert_eq!(Widget::<Msg>::style(&sized).width, Dimension::Length(120.0));
    }

    /// **A thumb's halo is under it** (milestone 652): the slider paints its track under
    /// the thumbs' focus stops and the thumbs over them, so a hovered stop's halo — a full
    /// disc — lies between the two, as the reference paints it.
    #[test]
    fn a_hovered_thumb_s_halo_is_under_the_thumb() {
        use crate::{build_ui, Runtime, Size};
        let theme = Theme::default();
        let root = slider(0.2, 0.8).width(248.0);
        let size = Size::new(248.0, 48.0);
        let first = build_ui(&root, size, &Runtime::default(), &theme);
        let stop = first.focus_next(None, true).expect("the start's stop");
        let mut runtime = Runtime::default();
        runtime.input.hovered = Some(stop);
        runtime.advance(1.0);
        let ui = build_ui(&root, size, &runtime, &theme);
        let found = rects(ui.scene().primitives());
        let halo = theme.scheme.primary.fade(0.12);
        let at = |wanted: &dyn Fn(&(Rect, Color)) -> bool| {
            found.iter().position(wanted).expect("painted")
        };
        let disc = at(&|(r, c)| *c == halo && r.width == 48.0);
        let thumb = at(&|(r, c)| *c == theme.scheme.primary && r.width == 20.0);
        let track = at(&|(_, c)| *c == theme.scheme.primary.fade(0.24));
        assert!(
            track < disc && disc < thumb,
            "track {track}, halo {disc}, thumb {thumb}"
        );
        let (r, _) = found[disc];
        assert!(
            (r.x + 24.0 - 64.0).abs() < 1e-3 && r.y == 0.0,
            "round the start: {r:?}"
        );
    }
}

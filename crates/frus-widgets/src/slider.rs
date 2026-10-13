//! [`Slider`]: a value slider over a range, **controlled** and draggable.

use std::rc::Rc;

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::flex::Flex;
use crate::interaction::{Key, KeyResponse, Status};
use crate::theme::Theme;
use crate::widget::Widget;
use crate::widgetstate::{WidgetState, WidgetStateProperty};

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
    /// The slider's `(resting, pressed)` thumb heights, handed down likewise.
    elevations: (Option<f32>, Option<f32>),
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
        let thumb = Rect::new(bounds.x, y, THUMB, THUMB);
        paint_thumb_shadow(thumb, self.elevations, &status, theme, scene);
        scene.draw_rect(thumb, fill.fade(o), THUMB * 0.5, border, ring.fade(o));
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
pub struct RangeSlider<Msg = crate::callback::Callback> {
    low: f32,
    high: f32,
    width: f32,
    divisions: Option<usize>,
    /// The **value tooltip** formatter: when set, a bubble above each thumb shows
    /// `label(value)` (and the height reserves the room for it).
    label: Option<Rc<dyn Fn(f32) -> String>>,
    enabled: bool,
    thumb_elevation: Option<f32>,
    pressed_thumb_elevation: Option<f32>,
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
            thumb_elevation: None,
            pressed_thumb_elevation: None,
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
    pub fn on_change<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_change: impl Fn(f32, f32) -> R + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(crate::callback::handler2(on_change)));
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

    /// How far each thumb sits off the track at rest. Unset, the theme's, then the
    /// reference's 1; `0.0` casts no shadow.
    pub fn thumb_elevation(mut self, elevation: f32) -> Self {
        self.thumb_elevation = Some(elevation);
        self.rebuild();
        self
    }

    /// How far a thumb rises while it is held. Unset, the theme's, then the reference's 6.
    pub fn pressed_thumb_elevation(mut self, elevation: f32) -> Self {
        self.pressed_thumb_elevation = Some(elevation);
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
    pub fn on_change_start<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_start: impl Fn(f32, f32) -> R + 'static,
    ) -> Self {
        self.on_change_start = Some(Rc::new(crate::callback::handler2(on_start)));
        self
    }

    /// Sent **once**, when the drag ends — after the last
    /// [`on_change`](Self::on_change), with the interval it settled on.
    pub fn on_change_end<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_end: impl Fn(f32, f32) -> R + 'static,
    ) -> Self {
        self.on_change_end = Some(Rc::new(crate::callback::handler2(on_end)));
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
            elevations: (self.thumb_elevation, self.pressed_thumb_elevation),
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

        let cell = Rect::new(0.0, 0.0, THUMB, H);
        let range = RangeSlider::new(0.25, 0.75).on_change(Msg::Range);
        for thumb in thumbs(&range) {
            assert_eq!(heights(thumb, cell, 0.0, &theme), vec![1.0]);
            assert_eq!(heights(thumb, cell, 1.0, &theme), vec![6.0]);
        }
        let raised = RangeSlider::new(0.25, 0.75)
            .on_change(Msg::Range)
            .thumb_elevation(3.0);
        for thumb in thumbs(&raised) {
            assert_eq!(
                heights(thumb, cell, 0.0, &theme),
                vec![3.0],
                "told on the slider"
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

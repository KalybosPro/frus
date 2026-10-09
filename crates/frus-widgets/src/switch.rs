//! [`Switch`]: a **controlled** toggle switch, shaped as a pill.

use frus_core::{Color, Curve, Insets, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::DISABLED_CONTAINER_OPACITY;
use crate::disabled::{disabled_container, disabled_content, disabled_mark, over_surface};
use crate::icons::IconData;
use crate::interaction::Status;
use crate::theme::{TapTarget, Theme};
use crate::widget::Widget;
use crate::widgetstate::{WidgetState, WidgetStateProperty, WidgetStates};

/// The track, at the reference's size (`switch.dart:2378`, `:2375`).
const W: f32 = 52.0;
const H: f32 = 32.0;
/// The thumb's radius **off** and **on** (`switch.dart:2354`, `:2317`). It grows as the
/// switch is flipped: off it is a dot inside an outlined track, on it is a disc on a
/// filled one, and that difference is most of what tells the two states apart at a
/// glance.
const THUMB_OFF: f32 = 8.0;
const THUMB_ON: f32 = 12.0;
/// The thumb while it is **held** (`switch.dart:2357`): larger than either end of the
/// travel, which is the squish a finger expects back. It **grows** into it — the press is
/// a progression since milestone 441, not a flag.
const THUMB_PRESSED: f32 = 14.0;
/// An end whose thumb **carries an icon** is the on-thumb's size (`switch.dart:2369`,
/// `:1067`): 16 pixels of glyph do not fit in a 16-pixel dot.
const THUMB_WITH_ICON: f32 = 12.0;
/// The thumb half way along, a pill wider than it is tall (`switch.dart:2382`): the
/// stretch of something being pulled across.
const THUMB_MIDWAY: (f32, f32) = (34.0, 22.0);
/// The glyph inside the thumb (`switch.dart:2314`).
const ICON_SIZE: f32 = 16.0;
/// The rule round an **off** track (`switch.dart:2298`).
const TRACK_OUTLINE: f32 = 2.0;
/// The halo's radius (`switch.dart:2301`).
const SPLASH: f32 = 20.0;
/// The room either side of the track (`switch.dart:2304`).
const PADDING: Insets = Insets::new(0.0, 4.0, 0.0, 4.0);
/// How long a flip takes (`switch.dart:2386`).
const TOGGLE_SECONDS: f32 = 0.3;

/// The reference's `easeOutBack` (`curves.dart:1727`): the thumb overshoots its end a
/// little and settles back, which is the snap a switch has.
fn ease_out_back() -> Curve {
    Curve::Cubic {
        x1: 0.175,
        y1: 0.885,
        x2: 0.32,
        y2: 1.275,
    }
}

/// An on/off switch.
pub struct Switch<Msg = crate::callback::Callback> {
    on: bool,
    enabled: bool,
    track_color: Option<Color>,
    inactive_track_color: Option<Color>,
    thumb_color: Option<Color>,
    inactive_thumb_color: Option<Color>,
    thumb_colors: Option<WidgetStateProperty<Color>>,
    track_colors: Option<WidgetStateProperty<Color>>,
    track_outline_color: Option<WidgetStateProperty<Color>>,
    track_outline_width: Option<f32>,
    overlay_color: Option<WidgetStateProperty<Color>>,
    splash_radius: Option<f32>,
    padding: Option<Insets>,
    thumb_icon: Option<IconData>,
    inactive_thumb_icon: Option<IconData>,
    tap_target: Option<TapTarget>,
    on_toggle: Option<Box<dyn Fn(bool) -> Msg>>,
}

impl<Msg> Switch<Msg> {
    /// Creates a switch whose state is supplied.
    pub fn new(on: bool) -> Self {
        Self {
            on,
            enabled: true,
            track_color: None,
            inactive_track_color: None,
            thumb_color: None,
            inactive_thumb_color: None,
            thumb_colors: None,
            track_colors: None,
            track_outline_color: None,
            track_outline_width: None,
            overlay_color: None,
            splash_radius: None,
            padding: None,
            thumb_icon: None,
            inactive_thumb_icon: None,
            tap_target: None,
            on_toggle: None,
        }
    }

    /// **How much room it reserves for a finger** ([`TapTarget`]). Unset, the theme's
    /// answer, which is [`Padded`](TapTarget::Padded) — at least 48 pixels either way,
    /// whatever this control paints inside it.
    pub fn tap_target(mut self, target: TapTarget) -> Self {
        self.tap_target = Some(target);
        self
    }

    /// The room this switch reserves, resolved as `caller ?? theme ?? framework`
    /// (`switch.dart:603`).
    fn reserved(&self, theme: &Theme) -> f32 {
        self.tap_target
            .or(theme.widgets.switch.tap_target)
            .unwrap_or(theme.tap_target)
            .min_side()
    }

    /// The room either side of the track: the caller's, the theme's, then 4 pixels.
    fn room(&self, theme: &Theme) -> Insets {
        self.padding
            .or(theme.widgets.switch.padding)
            .unwrap_or(PADDING)
    }

    /// The track's colour when the switch is **on**, in every state; the theme's
    /// `primary` otherwise.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// The track's colour when the switch is **off**; the scheme's
    /// `surface_container_highest` otherwise (`switch.dart:2246`), with a rule round it.
    pub fn inactive_track_color(mut self, color: Color) -> Self {
        self.inactive_track_color = Some(color);
        self
    }

    /// **A glyph inside the thumb while the switch is on** (`switch.dart:2320`).
    ///
    /// Unset, as the reference's is: a switch is legible without one. It is there for the
    /// setting that needs saying in more than colour and position — the two things a
    /// reader may not be able to tell apart — and a tick inside the thumb says *on* in a
    /// third way. The on thumb is already large enough to carry it.
    pub fn thumb_icon(mut self, icon: IconData) -> Self {
        self.thumb_icon = Some(icon);
        self
    }

    /// The same while the switch is **off** — a cross beside the tick. The off thumb
    /// grows to the on thumb's size to carry it (`switch.dart:1067`).
    pub fn inactive_thumb_icon(mut self, icon: IconData) -> Self {
        self.inactive_thumb_icon = Some(icon);
        self
    }

    /// The thumb's colour when the switch is **on**, in every state; the scheme's
    /// `on_primary` at rest otherwise (`switch.dart:2201`), `primary_container` under a
    /// pointer, the keyboard or a finger.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = Some(color);
        self
    }

    /// The thumb's colour when the switch is **off**, in every state; the scheme's
    /// `outline` at rest otherwise (`switch.dart:2212`), `on_surface_variant` under a
    /// pointer, the keyboard or a finger.
    ///
    /// The two ends are resolved apart and the thumb **travels between them**, so it is
    /// still one thumb — one that changes colour as it travels, the way the track under
    /// it does.
    pub fn inactive_thumb_color(mut self, color: Color) -> Self {
        self.inactive_thumb_color = Some(color);
        self
    }

    /// **The thumb's colour in any state** — on or off ([`WidgetState::Selected`]),
    /// hovered, focused, pressed, disabled — the reference's `thumbColor`. What it does
    /// not answer falls to [`Self::thumb_color`] and [`Self::inactive_thumb_color`], then
    /// the theme, then the reference's.
    pub fn thumb_colors(mut self, colors: WidgetStateProperty<Color>) -> Self {
        self.thumb_colors = Some(colors);
        self
    }

    /// **The track's colour in any state** — the reference's `trackColor`. See
    /// [`Self::thumb_colors`].
    pub fn track_colors(mut self, colors: WidgetStateProperty<Color>) -> Self {
        self.track_colors = Some(colors);
        self
    }

    /// **The rule round the track in any state** (`switch.dart:2251`). Unset, `outline`
    /// round an off track, none round an on one, and the disabled container colour round a
    /// disabled off one.
    pub fn track_outline_color(mut self, colors: WidgetStateProperty<Color>) -> Self {
        self.track_outline_color = Some(colors);
        self
    }

    /// **How wide that rule is.** Unset, the theme's, then 2 pixels.
    pub fn track_outline_width(mut self, width: f32) -> Self {
        self.track_outline_width = Some(width);
        self
    }

    /// **The halo** round the thumb, state by state (`switch.dart:2264`). Unset, the
    /// theme's, then `primary` round an on switch and `on_surface` round an off one, at
    /// 8 % under a pointer and 10 % focused or pressed.
    pub fn overlay_color(mut self, colors: WidgetStateProperty<Color>) -> Self {
        self.overlay_color = Some(colors);
        self
    }

    /// **The halo's radius.** Unset, the theme's, then 20.
    pub fn splash_radius(mut self, radius: f32) -> Self {
        self.splash_radius = Some(radius);
        self
    }

    /// **The room either side of the track** (`switch.dart:604`). Unset, the theme's, then
    /// 4 pixels left and right — which is what lets the halo spill past the track.
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// A closure producing a message from the new state.
    pub fn on_toggle<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_toggle: impl Fn(bool) -> R + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(crate::callback::handler1(on_toggle)));
        self
    }

    /// Whether the switch can be flipped. Disabled it is **inert** — no message, out of
    /// the tab order, announced as unavailable — and it still shows which way it is set.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The states this switch is in, at one end of its travel.
    fn states_at(&self, status: &Status, selected: bool) -> WidgetStates {
        let states = if self.enabled {
            status.states()
        } else {
            WidgetStates::EMPTY
        };
        states
            .set(WidgetState::Selected, selected)
            .set(WidgetState::Disabled, !self.enabled)
    }

    /// The thumb at one end: the caller's per-state colours, their plain colour for that
    /// end, the theme's in the same order, then the reference's (`switch.dart:961`).
    fn thumb_at(&self, theme: &Theme, states: WidgetStates) -> Color {
        let t = &theme.widgets.switch;
        let selected = states.contains(WidgetState::Selected);
        let plain = |mine: Option<Color>, inactive: Option<Color>| {
            if selected && self.enabled {
                mine
            } else {
                inactive
            }
        };
        self.thumb_colors
            .as_ref()
            .and_then(|p| p.resolve(states).copied())
            .or_else(|| plain(self.thumb_color, self.inactive_thumb_color))
            .or_else(|| {
                t.thumb_colors
                    .as_ref()
                    .and_then(|p| p.resolve(states).copied())
            })
            .or_else(|| plain(t.thumb_color, t.inactive_thumb_color))
            .unwrap_or_else(|| default_thumb(theme, states))
    }

    /// The track at one end, in the same order (`switch.dart:973`).
    fn track_at(&self, theme: &Theme, states: WidgetStates) -> Color {
        let t = &theme.widgets.switch;
        let selected = states.contains(WidgetState::Selected);
        let plain = |mine: Option<Color>, inactive: Option<Color>| {
            if selected {
                mine
            } else {
                inactive
            }
        };
        self.track_colors
            .as_ref()
            .and_then(|p| p.resolve(states).copied())
            .or_else(|| plain(self.track_color, self.inactive_track_color))
            .or_else(|| {
                t.track_colors
                    .as_ref()
                    .and_then(|p| p.resolve(states).copied())
            })
            .or_else(|| plain(t.track_color, t.inactive_track_color))
            .unwrap_or_else(|| default_track(theme, states))
    }

    /// The rule round the track at one end (`switch.dart:981`).
    fn outline_at(&self, theme: &Theme, states: WidgetStates) -> Color {
        self.track_outline_color
            .as_ref()
            .and_then(|p| p.resolve(states).copied())
            .or_else(|| {
                theme
                    .widgets
                    .switch
                    .track_outline_color
                    .as_ref()
                    .and_then(|p| p.resolve(states).copied())
            })
            .unwrap_or_else(|| {
                if states.contains(WidgetState::Selected) {
                    Color::TRANSPARENT
                } else if states.contains(WidgetState::Disabled) {
                    disabled_container(theme)
                } else {
                    theme.scheme.outline
                }
            })
    }

    /// The halo's colour with `state` added (`switch.dart:1014`).
    fn halo_at(&self, theme: &Theme, selected: bool, state: WidgetState) -> Color {
        let states = WidgetStates::of(state).set(WidgetState::Selected, selected);
        self.overlay_color
            .as_ref()
            .and_then(|p| p.resolve(states).copied())
            .or_else(|| {
                theme
                    .widgets
                    .switch
                    .overlay_color
                    .as_ref()
                    .and_then(|p| p.resolve(states).copied())
            })
            .unwrap_or_else(|| default_overlay(theme, states))
    }

    /// The thumb's size at linear progress `t` of a flip towards the switch's state
    /// (`switch.dart:1569`): off to the midway pill over the first 11 %, to the on size
    /// over the next 72 %, held for the rest — the other way about when it is turned off.
    fn thumb_size(&self, t: f32) -> (f32, f32) {
        let off = if self.inactive_thumb_icon.is_some() {
            THUMB_WITH_ICON
        } else {
            THUMB_OFF
        };
        let (off, on) = ((2.0 * off, 2.0 * off), (2.0 * THUMB_ON, 2.0 * THUMB_ON));
        let lerp =
            |a: (f32, f32), b: (f32, f32), k: f32| (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
        let early = Curve::Cubic {
            x1: 0.31,
            y1: 0.0,
            x2: 0.56,
            y2: 1.0,
        };
        let late = Curve::Cubic {
            x1: 0.2,
            y1: 0.0,
            x2: 0.0,
            y2: 1.0,
        };
        if t <= 0.0 {
            return off;
        }
        if t >= 1.0 {
            return on;
        }
        if self.on {
            if t < 0.11 {
                lerp(off, THUMB_MIDWAY, early.transform(t / 0.11))
            } else if t < 0.83 {
                lerp(THUMB_MIDWAY, on, late.transform((t - 0.11) / 0.72))
            } else {
                on
            }
        } else if t < 0.17 {
            off
        } else if t < 0.89 {
            let k = Curve::Flipped(Box::new(late)).transform((t - 0.17) / 0.72);
            lerp(off, THUMB_MIDWAY, k)
        } else {
            let k = Curve::Flipped(Box::new(early)).transform((t - 0.89) / 0.11);
            lerp(THUMB_MIDWAY, on, k)
        }
    }
}

/// **The reference's thumb** (`switch.dart:2183`): `on_primary` on, `outline` off, and
/// under a pointer, the keyboard or a finger `primary_container` on and
/// `on_surface_variant` off. Disabled, opaque, for the reason [`crate::disabled`] gives.
fn default_thumb(theme: &Theme, states: WidgetStates) -> Color {
    let c = &theme.scheme;
    let selected = states.contains(WidgetState::Selected);
    if states.contains(WidgetState::Disabled) {
        return if selected {
            disabled_mark(theme)
        } else {
            disabled_content(theme)
        };
    }
    let busy = states.contains(WidgetState::Pressed)
        || states.contains(WidgetState::Hovered)
        || states.contains(WidgetState::Focused);
    match (selected, busy) {
        (true, true) => c.primary_container,
        (true, false) => c.on_primary,
        (false, true) => c.on_surface_variant,
        (false, false) => c.outline,
    }
}

/// **The reference's track** (`switch.dart:2217`): `primary` on, `surface_container_highest`
/// off, in every state. Disabled, the flattened container on, and off the reference's 12 %
/// wash of `surface_container_highest`, which is nearly the page itself (`:2223`) —
/// resolved in sRGB for the reason [`crate::disabled::over_surface`] gives.
fn default_track(theme: &Theme, states: WidgetStates) -> Color {
    let selected = states.contains(WidgetState::Selected);
    if states.contains(WidgetState::Disabled) {
        return if selected {
            disabled_container(theme)
        } else {
            theme.scheme.surface.lerp(
                theme.scheme.surface_container_highest,
                DISABLED_CONTAINER_OPACITY,
            )
        };
    }
    if selected {
        theme.primary
    } else {
        theme.scheme.surface_container_highest
    }
}

/// **The reference's halo** (`switch.dart:2264`): `primary` round an on switch and
/// `on_surface` round an off one, 10 % pressed, 8 % under a pointer, 10 % focused.
fn default_overlay(theme: &Theme, states: WidgetStates) -> Color {
    let ink = if states.contains(WidgetState::Selected) {
        theme.primary
    } else {
        theme.scheme.on_surface
    };
    if states.contains(WidgetState::Pressed) {
        ink.with_alpha(0.1)
    } else if states.contains(WidgetState::Hovered) {
        ink.with_alpha(0.08)
    } else if states.contains(WidgetState::Focused) {
        ink.with_alpha(0.1)
    } else {
        Color::TRANSPARENT
    }
}

/// `a` to `b` by `k`, landing **exactly** on either end: a lerp's arithmetic leaves the
/// last bit off the colour it arrives at.
fn travel(a: Color, b: Color, k: f32) -> Color {
    if k <= 0.0 {
        a
    } else if k >= 1.0 {
        b
    } else {
        a.lerp(b, k)
    }
}

/// A colour that is not opaque, laid on the page first (`switch.dart:1667`), so the thumb
/// never shows the track through it.
fn opaque_on(page: Color, c: Color) -> Color {
    if c.a >= 1.0 {
        c
    } else {
        page.lerp(c.with_alpha(1.0), c.a)
    }
}

impl<Msg> Widget<Msg> for Switch<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    /// **The box is the tap target, not the track** (`switch.dart:605`): 52 wide and the
    /// room either side, and as tall as a finger. The track is painted in the middle of it.
    fn style_themed(&self, theme: &Theme) -> Style {
        let room = self.room(theme);
        Style {
            width: Dimension::Length(W + room.left + room.right),
            height: Dimension::Length(H.max(self.reserved(theme)) + room.top + room.bottom),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // `t` is the flip's linear progress, 0 off and 1 on. The reference curves it two
        // ways (`switch.dart:797`, `:1153`): the thumb's place overshoots and settles
        // (`easeOutBack`, flipped on the way back), the colours ease out on the way on and
        // in on the way off.
        let t = status.value;
        let curved = |curve: Curve| {
            // The ends are the ends, as every curve's are in the reference
            // (`Curve.transform`); a cubic's search stops a hair short of them.
            if t <= 0.0 || t >= 1.0 {
                t.clamp(0.0, 1.0)
            } else {
                curve.transform(t)
            }
        };
        let (place, mix) = if status.value_placed {
            // Under a finger, or settling from where one let go, the thumb is where the
            // finger put it: no curve (`switch.dart:873`).
            (t.clamp(0.0, 1.0), curved(Curve::ease_out()))
        } else if self.on {
            (curved(ease_out_back()), curved(Curve::ease_out()))
        } else {
            (
                curved(Curve::Flipped(Box::new(ease_out_back()))),
                curved(Curve::ease_in()),
            )
        };
        // Each end of the travel is resolved on its own and the travel runs between them,
        // so an override moves the whole animation with it.
        let on_states = self.states_at(&status, true);
        let off_states = self.states_at(&status, false);
        let track = travel(
            self.track_at(theme, off_states),
            self.track_at(theme, on_states),
            mix,
        );
        let thumb = opaque_on(
            theme.scheme.surface,
            travel(
                self.thumb_at(theme, off_states),
                self.thumb_at(theme, on_states),
                mix,
            ),
        );
        // The rule belongs to the **off** end: transparent round an on track
        // (`switch.dart:2254`), so it fades out along the travel.
        let (rule_off, rule_on) = (
            self.outline_at(theme, off_states),
            self.outline_at(theme, on_states),
        );
        let edge = if rule_on.a == 0.0 {
            rule_off.fade(1.0 - mix)
        } else if rule_off.a == 0.0 {
            rule_on.fade(mix)
        } else {
            travel(rule_off, rule_on, mix)
        };
        let edge_width = self
            .track_outline_width
            .or(theme.widgets.switch.track_outline_width)
            .unwrap_or(TRACK_OUTLINE);

        // The track, centred in the box (`switch.dart:1706`).
        let track_rect = Rect::new(
            bounds.x + (bounds.width - W) * 0.5,
            bounds.y + (bounds.height - H) * 0.5,
            W,
            H,
        );
        scene.draw_rect(track_rect, track.fade(o), H * 0.5, edge_width, edge.fade(o));

        // The thumb's centre runs half a track-height in from either end, so it stays
        // centred in the rounded cap; in right-to-left, from the right.
        let along = if theme.direction == frus_core::TextDirection::Rtl {
            1.0 - place
        } else {
            place
        };
        let cx = track_rect.x + H * 0.5 + (W - H) * along;
        let cy = track_rect.y + H * 0.5;

        // **The halo** (`switch.dart:1691`), over the track and under the thumb, round the
        // thumb's centre and wider than the track: the room either side is what it spills
        // into. Nothing round a switch that cannot be worked.
        if self.enabled {
            let radius = self
                .splash_radius
                .or(theme.widgets.switch.splash_radius)
                .unwrap_or(SPLASH);
            crate::toggleable::paint_halos(scene, Point::new(cx, cy), radius, &status, |s| {
                self.halo_at(theme, self.on, s)
            });
        }

        // The thumb: its size along the flip, swelling when held (`switch.dart:1627`) —
        // grown into over the press's own 200 ms, from wherever it has got to.
        let (mut w, mut h) = self.thumb_size(t);
        if self.enabled {
            let press = status.press_progress.clamp(0.0, 1.0);
            w += (2.0 * THUMB_PRESSED - w) * press;
            h += (2.0 * THUMB_PRESSED - h) * press;
        }
        scene.draw_rect(
            Rect::new(cx - w * 0.5, cy - h * 0.5, w, h),
            thumb.fade(o),
            h * 0.5,
            0.0,
            Color::TRANSPARENT,
        );

        // And the glyph inside it, from the end the switch is **set** to. Its colour is
        // the track's own at the off end (`switch.dart:2349`), so it reads as a hole
        // punched through the thumb rather than as a mark drawn on it.
        let icon = if self.on {
            self.thumb_icon
        } else {
            self.inactive_thumb_icon
        };
        if let Some(icon) = icon {
            let t_widget = &theme.widgets.switch;
            let ink = if !self.enabled {
                over_surface(theme, crate::disabled::DISABLED_CONTENT_OPACITY)
            } else if self.on {
                t_widget
                    .icon_color
                    .unwrap_or(theme.scheme.on_primary_container)
            } else {
                t_widget
                    .inactive_icon_color
                    .unwrap_or(theme.scheme.surface_container_highest)
            };
            let path = icon.placed(
                ICON_SIZE,
                cx - ICON_SIZE * 0.5,
                cy - ICON_SIZE * 0.5,
                theme.direction,
            );
            scene.fill_path(&path, ink.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        self.on_toggle.as_ref().map(|make| make(!self.on))
    }

    /// **The thumb follows a horizontal drag** (`switch.dart:1078`), when there is
    /// something to tell.
    fn pan_axis(&self) -> Option<crate::PanAxis> {
        (self.enabled && self.on_toggle.is_some()).then_some(crate::PanAxis::Horizontal)
    }

    /// Where the drag puts the thumb: moved by the finger's travel over the track's inner
    /// length (`switch.dart:870`), the other way in right-to-left.
    fn pan_value(&self, event: crate::PanEvent, value: f32, rtl: bool) -> Option<f32> {
        match event {
            crate::PanEvent::Start { .. } => Some(value.clamp(0.0, 1.0)),
            crate::PanEvent::Update { delta, .. } => {
                let along = delta.x / (W - H);
                let along = if rtl { -along } else { along };
                Some((value + along).clamp(0.0, 1.0))
            }
            crate::PanEvent::End { .. } => None,
        }
    }

    /// **Let go past half way, it flips** (`switch.dart:885`); short of it, the thumb goes
    /// back and nothing is said.
    fn on_value_release(&self, value: f32) -> Option<Msg> {
        if !self.enabled || (value >= 0.5) == self.on {
            return None;
        }
        self.on_toggle.as_ref().map(|make| make(!self.on))
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // Still on or off, still announced — a switch that fell silent would read as a
        // setting that had gone away rather than one that cannot be changed.
        let semantics =
            frus_core::SemanticsProperties::new(frus_core::Role::Switch).toggled(self.on);
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }

    fn anim_target(&self) -> Option<f32> {
        Some(if self.on { 1.0 } else { 0.0 })
    }

    /// The reference's 300 ms (`switch.dart:2386`).
    fn anim_duration(&self) -> f32 {
        TOGGLE_SECONDS
    }

    /// **Linear**: the value is the flip's progress, which the paint curves itself — one
    /// way for the thumb's place, another for its colours and a third for its size, as the
    /// reference curves one controller three ways.
    fn anim_curve(&self) -> frus_core::Curve {
        frus_core::Curve::Linear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disabled::{disabled_container, disabled_content, disabled_mark};
    use crate::icons::Icons;
    use crate::theme::{MIN_TAP_TARGET, SHRUNK_TAP_TARGET};
    use crate::widget::Widget;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Set(bool),
    }

    /// Each rectangle the switch painted, as `(fill, ring)`.
    fn painted(on: bool, enabled: bool, theme: &Theme) -> Vec<(Color, Color)> {
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &Switch::<Msg>::new(on).enabled(enabled),
            Rect::new(0.0, 0.0, W, H),
            Status {
                opacity: 1.0,
                value: if on { 1.0 } else { 0.0 },
                ..Default::default()
            },
            theme,
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect {
                    color,
                    border_color,
                    ..
                } => Some((*color, *border_color)),
                _ => None,
            })
            .collect()
    }

    /// Every rectangle a switch paints, with its geometry, under a given interaction.
    fn boxes(switch: &Switch<Msg>, status: Status, theme: &Theme) -> Vec<(Rect, Color)> {
        let mut scene = Scene::new();
        Widget::<Msg>::paint(switch, Rect::new(0.0, 0.0, W, H), status, theme, &mut scene);
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. } => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    /// A switch being painted at one end of its travel, in a given interaction state.
    fn state(on: bool) -> Status {
        Status {
            opacity: 1.0,
            value: if on { 1.0 } else { 0.0 },
            ..Default::default()
        }
    }

    /// **A switch answers the pointer with the reference's halo** (`switch.dart:1691`):
    /// a circle of radius 20 round the thumb, wider than the track, over the track and
    /// under the thumb — `on_surface` round an off switch, `primary` round an on one, 8 %
    /// under a pointer and 10 % pressed or focused (`switch.dart:2264`).
    #[test]
    fn a_switch_answers_the_pointer_with_a_halo() {
        let theme = Theme::default();
        let switch = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        let with = |on: bool, f: fn(&mut Status)| {
            let mut status = state(on);
            f(&mut status);
            status
        };
        let hover = |s: &mut Status| s.hover_progress = 1.0;
        let press = |s: &mut Status| s.press_progress = 1.0;
        let focus = |s: &mut Status| s.focus_progress = 1.0;
        assert_eq!(
            boxes(&switch, state(false), &theme).len(),
            2,
            "track, thumb"
        );
        let lit = boxes(&switch, with(false, hover), &theme);
        assert_eq!(lit.len(), 3, "track, halo, thumb");
        let (halo, colour) = lit[1];
        assert_eq!(colour, theme.scheme.on_surface.with_alpha(0.08));
        assert_eq!((halo.width, halo.height), (2.0 * SPLASH, 2.0 * SPLASH));
        assert!(halo.height > H, "wider than the track");
        let thumb = lit[2].0;
        assert_eq!(
            (halo.x + SPLASH, halo.y + SPLASH),
            (thumb.x + thumb.width * 0.5, thumb.y + thumb.height * 0.5),
            "round the thumb"
        );

        let on = Switch::<Msg>::new(true).on_toggle(Msg::Set);
        assert_eq!(
            boxes(&on, with(true, hover), &theme)[1].1,
            theme.primary.with_alpha(0.08)
        );
        assert_eq!(
            boxes(&on, with(true, press), &theme)[1].1,
            theme.primary.with_alpha(0.1)
        );
        assert_eq!(
            boxes(&switch, with(false, focus), &theme)[1].1,
            theme.scheme.on_surface.with_alpha(0.1)
        );

        // The caller's halo, and its radius.
        let mine = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .overlay_color(WidgetStateProperty::all(Color::rgb(1.0, 0.0, 0.0)))
            .splash_radius(12.0);
        let (halo, colour) = boxes(&mine, with(false, hover), &theme)[1];
        assert_eq!((colour, halo.width), (Color::rgb(1.0, 0.0, 0.0), 24.0));
    }

    /// **The thumb answers too** (`switch.dart:2183`): under a pointer, the keyboard or a
    /// finger it is `primary_container` on and `on_surface_variant` off.
    #[test]
    fn the_thumb_answers_the_pointer() {
        let theme = Theme::light();
        let hovered = |on: bool| Status {
            hover_progress: 1.0,
            interaction: crate::interaction::Interaction::Hovered,
            ..state(on)
        };
        let thumb = |on: bool, status: Status| {
            boxes(&Switch::<Msg>::new(on).on_toggle(Msg::Set), status, &theme)
                .last()
                .expect("a thumb")
                .1
        };
        assert_eq!(thumb(true, hovered(true)), theme.scheme.primary_container);
        assert_eq!(
            thumb(false, hovered(false)),
            theme.scheme.on_surface_variant
        );
        assert_eq!(thumb(true, state(true)), theme.scheme.on_primary);
        assert_eq!(thumb(false, state(false)), theme.scheme.outline);

        // The caller's colours, state by state, before anything else.
        let mine = WidgetStateProperty::new()
            .when(WidgetState::Hovered, Color::rgb(0.0, 0.0, 1.0))
            .when(WidgetState::Selected, Color::rgb(0.0, 1.0, 0.0));
        let painted = |on: bool, status: Status| {
            boxes(
                &Switch::<Msg>::new(on)
                    .on_toggle(Msg::Set)
                    .thumb_colors(mine.clone()),
                status,
                &theme,
            )
            .last()
            .expect("a thumb")
            .1
        };
        assert_eq!(painted(false, hovered(false)), Color::rgb(0.0, 0.0, 1.0));
        assert_eq!(painted(true, state(true)), Color::rgb(0.0, 1.0, 0.0));
        assert_eq!(
            painted(false, state(false)),
            theme.scheme.outline,
            "what it leaves unsaid is the reference's"
        );
    }

    /// **The track answers in any state** the caller names, and the rule round it too.
    #[test]
    fn the_track_and_its_rule_are_the_callers() {
        let theme = Theme::light();
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let switch = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .track_colors(WidgetStateProperty::all(red))
            .track_outline_color(WidgetStateProperty::all(blue))
            .track_outline_width(3.0);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &switch,
            Rect::new(0.0, 0.0, W, H),
            state(false),
            &theme,
            &mut scene,
        );
        match scene.primitives()[0] {
            frus_core::Primitive::Rect {
                color,
                border_color,
                border_width,
                ..
            } => assert_eq!((color, border_color, border_width), (red, blue, 3.0)),
            _ => panic!("the track is a rectangle"),
        }
        // Unsaid, the reference's 2 (`switch.dart:2298`).
        let plain = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &plain,
            Rect::new(0.0, 0.0, W, H),
            state(false),
            &theme,
            &mut scene,
        );
        match scene.primitives()[0] {
            frus_core::Primitive::Rect { border_width, .. } => assert_eq!(border_width, 2.0),
            _ => panic!("the track is a rectangle"),
        }
    }

    /// And a switch that cannot be worked does not light, in any state: a state layer is
    /// the promise of an interaction.
    #[test]
    fn a_disabled_switch_does_not_light() {
        let theme = Theme::default();
        let dead = Switch::<Msg>::new(false).on_toggle(Msg::Set).enabled(false);
        for status in [
            Status {
                hover_progress: 1.0,
                ..state(false)
            },
            Status {
                focus_progress: 1.0,
                ..state(false)
            },
            Status {
                press_progress: 1.0,
                ..state(false)
            },
        ] {
            assert_eq!(
                boxes(&dead, status, &theme).len(),
                2,
                "track and thumb only"
            );
        }
    }

    /// The thumb's radius, at the end of the travel it was painted at.
    fn thumb_radius(switch: &Switch<Msg>, status: Status, theme: &Theme) -> f32 {
        boxes(switch, status, theme)
            .last()
            .map(|(rect, _)| rect.width * 0.5)
            .expect("a switch paints a thumb")
    }

    /// **An end whose thumb carries a glyph is the larger one** (`switch.dart:1067`):
    /// sixteen pixels of glyph do not fit in a sixteen-pixel dot. The on thumb is that
    /// size already, so only an off glyph grows the off thumb.
    #[test]
    fn a_thumb_that_carries_a_glyph_is_the_larger_one() {
        let theme = Theme::default();
        let bare = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        assert_eq!(thumb_radius(&bare, state(false), &theme), THUMB_OFF);

        let ticked = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .thumb_icon(Icons::CHECK);
        assert_eq!(
            thumb_radius(&ticked, state(false), &theme),
            THUMB_OFF,
            "an on glyph leaves the off thumb alone"
        );
        let crossed = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .inactive_thumb_icon(Icons::CLOSE);
        assert_eq!(
            thumb_radius(&crossed, state(false), &theme),
            THUMB_WITH_ICON
        );
        assert_eq!(
            thumb_radius(&crossed, state(true), &theme),
            THUMB_ON,
            "and the on thumb was already that size"
        );
    }

    /// The glyph itself: drawn only at the end that has one, in the colour that end names.
    #[test]
    fn the_glyph_is_drawn_at_the_end_that_has_one() {
        let theme = Theme::default();
        let glyph = |switch: &Switch<Msg>, on: bool| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                switch,
                Rect::new(0.0, 0.0, W, H),
                state(on),
                &theme,
                &mut scene,
            );
            scene.primitives().iter().find_map(|p| match p {
                frus_core::Primitive::Path { fill, .. } => *fill,
                _ => None,
            })
        };
        let ticked = Switch::<Msg>::new(true)
            .on_toggle(Msg::Set)
            .thumb_icon(Icons::CHECK);
        assert_eq!(
            glyph(&ticked, true),
            Some(theme.scheme.on_primary_container)
        );
        // The end a switch is **set** to decides which glyph it carries, as the reference
        // resolves `thumbIcon` from the state rather than from the animation: a switch
        // that is off and names no off icon carries none.
        let unticked = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .thumb_icon(Icons::CHECK);
        assert_eq!(glyph(&unticked, false), None, "nothing at the other end");

        let crossed = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .thumb_icon(Icons::CHECK)
            .inactive_thumb_icon(Icons::CLOSE);
        assert_eq!(
            glyph(&crossed, false),
            Some(theme.scheme.surface_container_highest),
            "off, the glyph takes the track's own colour: a hole, not a mark"
        );
    }

    /// **A held thumb swells** (`switch.dart:2357`), past either end of the travel.
    #[test]
    fn a_held_thumb_swells() {
        let theme = Theme::default();
        let switch = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        let pressed = Status {
            press_progress: 1.0,
            ..state(false)
        };
        assert_eq!(thumb_radius(&switch, pressed, &theme), THUMB_PRESSED);
        // Past **both** ends of the travel, which is what makes it read as a press rather
        // than as the switch having moved.
        for end in [false, true] {
            assert!(
                thumb_radius(&switch, pressed, &theme) > thumb_radius(&switch, state(end), &theme),
                "not past the {end} end"
            );
        }
    }

    /// And it **grows** into it (milestone 441): half way through the press the thumb is
    /// half way there, where it used to arrive whole on the first frame the finger was
    /// down and leave whole on the first frame it was not.
    #[test]
    fn a_held_thumb_grows_into_it() {
        let theme = Theme::default();
        let switch = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        let at = |p: f32| {
            thumb_radius(
                &switch,
                Status {
                    press_progress: p,
                    ..state(false)
                },
                &theme,
            )
        };
        assert_eq!(at(0.0), THUMB_OFF, "untouched");
        assert_eq!(at(1.0), THUMB_PRESSED, "held");
        assert!(
            (at(0.5) - (THUMB_OFF + THUMB_PRESSED) * 0.5).abs() < 0.01,
            "half way = {}",
            at(0.5)
        );
    }

    /// The height a switch asks the layout for, under a given theme.
    fn reserved_height(switch: &Switch<Msg>, theme: &Theme) -> f32 {
        match Widget::<Msg>::style_themed(switch, theme).height {
            Dimension::Length(h) => h,
            other => panic!("a switch asks for a length, not {other:?}"),
        }
    }

    /// **A switch's box is its tap target, not its track** (milestone 442).
    ///
    /// The track is 32 pixels tall and a finger is not. The reference lays a switch out
    /// at 52 × 48 and paints the track in the middle (`switch.dart:605`); this asked the
    /// layout for the track and nothing else, so the area a click could land in was the
    /// track exactly.
    #[test]
    fn a_switch_reserves_room_for_a_finger() {
        let theme = Theme::default();
        assert_eq!(
            reserved_height(&Switch::<Msg>::new(false).on_toggle(Msg::Set), &theme),
            MIN_TAP_TARGET
        );
        assert_eq!(
            reserved_height(
                &Switch::<Msg>::new(false)
                    .on_toggle(Msg::Set)
                    .tap_target(TapTarget::ShrinkWrap),
                &theme
            ),
            SHRUNK_TAP_TARGET,
            "and a caller may ask for the smaller answer"
        );

        // The theme answers for a switch that has not said, and the widget theme sits
        // between the two.
        let dense = Theme {
            tap_target: TapTarget::ShrinkWrap,
            ..Theme::default()
        };
        let plain = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        assert_eq!(reserved_height(&plain, &dense), SHRUNK_TAP_TARGET);
        let mut mixed = dense;
        mixed.widgets.switch.tap_target = Some(TapTarget::Padded);
        assert_eq!(reserved_height(&plain, &mixed), MIN_TAP_TARGET);
    }

    /// And **nothing it paints moves**: the track is centred in the room, and the thumb
    /// stays centred on the track.
    #[test]
    fn the_track_is_centred_in_the_room() {
        let theme = Theme::default();
        let switch = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &switch,
            Rect::new(0.0, 0.0, W, MIN_TAP_TARGET),
            state(false),
            &theme,
            &mut scene,
        );
        let rects: Vec<Rect> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect();

        let track = rects[0];
        assert_eq!(track.height, H, "the track keeps its own height");
        assert!(
            (track.y - (MIN_TAP_TARGET - H) * 0.5).abs() < 0.01,
            "and sits in the middle of the room: {track:?}"
        );
        let thumb = *rects.last().expect("a switch paints a thumb");
        assert!(
            (thumb.y + thumb.height * 0.5 - MIN_TAP_TARGET * 0.5).abs() < 0.01,
            "the thumb is centred on the track: {thumb:?}"
        );
    }

    /// **A switch that can be flipped takes a horizontal drag** (`switch.dart:1078`); one
    /// that cannot, or has no one to tell, takes none, so a drag there is the page's.
    #[test]
    fn a_live_switch_takes_a_horizontal_drag() {
        let live = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        assert_eq!(
            Widget::<Msg>::pan_axis(&live),
            Some(crate::PanAxis::Horizontal)
        );
        assert_eq!(Widget::<Msg>::pan_axis(&live.enabled(false)), None);
        assert_eq!(Widget::<Msg>::pan_axis(&Switch::<Msg>::new(false)), None);
    }

    /// **The thumb moves by the finger's travel over the track's inner length**
    /// (`switch.dart:875`) — 20 pixels end to end — the other way in right-to-left, and
    /// never past an end.
    #[test]
    fn the_drag_moves_the_thumb_by_the_finger() {
        let switch = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        let update = |dx: f32| crate::PanEvent::Update {
            local: frus_core::Point::new(0.0, 0.0),
            delta: frus_core::Point::new(dx, 0.0),
        };
        let start = crate::PanEvent::Start {
            local: frus_core::Point::new(0.0, 0.0),
        };
        let value = |e, v, rtl| Widget::<Msg>::pan_value(&switch, e, v, rtl);
        assert_eq!(value(start, 0.0, false), Some(0.0), "held where it is");
        assert_eq!(value(update(5.0), 0.0, false), Some(0.25));
        assert_eq!(value(update(5.0), 0.5, true), Some(0.25), "mirrored");
        assert_eq!(
            value(update(50.0), 0.5, false),
            Some(1.0),
            "not past the end"
        );
        assert_eq!(value(update(-50.0), 0.5, false), Some(0.0), "nor the start");
        let end = crate::PanEvent::End {
            velocity: frus_core::Point::new(0.0, 0.0),
        };
        assert_eq!(value(end, 0.7, false), None, "the release is not a move");
    }

    /// **Let go past half way it flips, short of it nothing is said** (`switch.dart:885`).
    #[test]
    fn let_go_past_half_way_it_flips() {
        let off = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        assert_eq!(
            Widget::<Msg>::on_value_release(&off, 0.5),
            Some(Msg::Set(true))
        );
        assert_eq!(Widget::<Msg>::on_value_release(&off, 0.49), None);
        let on = Switch::<Msg>::new(true).on_toggle(Msg::Set);
        assert_eq!(
            Widget::<Msg>::on_value_release(&on, 0.3),
            Some(Msg::Set(false))
        );
        assert_eq!(Widget::<Msg>::on_value_release(&on, 0.5), None);
        assert_eq!(
            Widget::<Msg>::on_value_release(&off.enabled(false), 1.0),
            None,
            "a disabled switch never flips"
        );
    }

    /// **A placed value is painted straight** (`switch.dart:873`): the thumb is where the
    /// finger put it, not where the overshooting curve would take that value.
    #[test]
    fn a_placed_thumb_is_where_the_finger_put_it() {
        let theme = Theme::default();
        let switch = Switch::<Msg>::new(true).on_toggle(Msg::Set);
        let centre = |placed: bool| {
            let thumb = boxes(
                &switch,
                Status {
                    value: 0.7,
                    value_placed: placed,
                    ..state(true)
                },
                &theme,
            )
            .last()
            .expect("a thumb")
            .0;
            thumb.x + thumb.width * 0.5
        };
        assert!((centre(true) - (H * 0.5 + (W - H) * 0.7)).abs() < 1e-4);
        assert!(
            centre(false) > centre(true),
            "the curve would have overshot"
        );
    }

    #[test]
    fn click_toggles() {
        assert_eq!(
            Widget::on_click(&Switch::new(false).on_toggle(Msg::Set)),
            Some(Msg::Set(true))
        );
    }

    #[test]
    fn a_disabled_switch_is_inert_but_still_says_which_way_it_is_set() {
        let dead = Switch::new(true).on_toggle(Msg::Set).enabled(false);
        assert_eq!(Widget::on_click(&dead), None, "the press goes nowhere");
        assert!(!Widget::<Msg>::focusable(&dead), "out of the tab order");
        let semantics = Widget::<Msg>::semantics(&dead).expect("still announced");
        assert!(semantics.disabled, "and announced as unavailable");
        assert_eq!(semantics.toggled, frus_core::Toggled::True, "still on");
    }

    /// The switch is the control that takes **both** halves of the rule at once, which is
    /// the argument that the split is container-against-content rather than one rule per
    /// widget. If these two ever collapse to the same opacity, that argument is gone.
    #[test]
    fn a_disabled_switch_takes_both_halves_of_the_rule() {
        for theme in [Theme::dark(), Theme::light()] {
            let off = painted(false, false, &theme);
            let ((track, ring), thumb) = (off[0], off[1].0);
            // Since milestone 428 the **off** track is the reference's
            // `surfaceContainerHighest` wash, which is near enough the page to be nothing;
            // the container half of the rule is the ring round it. The switch still shows
            // both halves at once, which is the whole argument — the container is the
            // pill's edge rather than its fill while it is off.
            assert_eq!(ring, disabled_container(&theme), "the ring is a container");
            assert_eq!(
                thumb,
                disabled_content(&theme),
                "the thumb is content on it"
            );
            // Quieter means closer to the surface: since milestone 329 both tokens are
            // opaque, which is the flattening this rule asks for.
            let from_surface = |c: Color| {
                (c.r - theme.scheme.surface.r).abs()
                    + (c.g - theme.scheme.surface.g).abs()
                    + (c.b - theme.scheme.surface.b).abs()
            };
            assert!(
                from_surface(ring) < from_surface(thumb),
                "and the container is the quieter of the two"
            );
            assert!(
                from_surface(track) < from_surface(ring),
                "with the wash inside it quieter still"
            );

            // Flipped on, the track *is* the flattened container and the thumb sits on
            // it, so it punches through opaquely rather than adding a third translucent
            // layer — and the ring goes, a filled track having no edge.
            let on = painted(true, false, &theme);
            assert_eq!(on[0].0, disabled_container(&theme), "a filled track");
            assert_eq!(on[1].0, disabled_mark(&theme), "an opaque thumb");
            assert_eq!(on[0].1.a, 0.0, "and no ring round it");
        }
    }

    #[test]
    fn a_live_switch_is_untouched_by_any_of_it() {
        let theme = Theme::dark();
        let live = painted(true, true, &theme);
        assert_ne!(live[0].0, disabled_container(&theme));
        assert_ne!(live[1].0, disabled_content(&theme));
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::widget::Widget;

    const BRAND: Color = Color::rgb(0.0, 0.6, 0.3);
    const RAIL: Color = Color::rgb(0.9, 0.9, 0.2);
    const KNOB: Color = Color::rgb(0.1, 0.1, 0.9);

    /// The (track, thumb, ring) a switch painted.
    fn painted(switch: &Switch<()>, on: bool, theme: &Theme) -> (Color, Color, Color) {
        at(switch, if on { 1.0 } else { 0.0 }, theme)
    }

    /// The same, anywhere along the travel.
    fn at(switch: &Switch<()>, t: f32, theme: &Theme) -> (Color, Color, Color) {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            switch,
            Rect::new(0.0, 0.0, W, H),
            Status {
                opacity: 1.0,
                value: t,
                ..Default::default()
            },
            theme,
            &mut scene,
        );
        let rects: Vec<(Color, Color)> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect {
                    color,
                    border_color,
                    ..
                } => Some((*color, *border_color)),
                _ => None,
            })
            .collect();
        (rects[0].0, rects[1].0, rects[0].1)
    }

    /// Nothing said: the four colours the reference names, and the ring that only the
    /// off end has.
    #[test]
    fn the_defaults_are_the_reference_s() {
        let theme = Theme::default();
        // Both ends are the arrival of a lerp, so they land on the colour rather than
        // matching it bit for bit — see `lands_on`.
        let (track, thumb, ring) = painted(&Switch::<()>::new(true), true, &theme);
        lands_on(track, theme.primary); // `switch.dart:2235`
        lands_on(thumb, theme.scheme.on_primary); // `switch.dart:2201`
        assert_eq!(ring.a, 0.0, "a filled track has no edge (`:2254`)");

        let (track, thumb, ring) = painted(&Switch::<()>::new(false), false, &theme);
        lands_on(track, theme.scheme.surface_container_highest); // `switch.dart:2246`
        lands_on(thumb, theme.scheme.outline); // `switch.dart:2212`
        lands_on(ring, theme.scheme.outline); // `switch.dart:2259`
        assert_eq!(ring.a, 1.0, "and it is drawn");
    }

    /// The **ring fades out along the travel**, which is the animated form of the
    /// reference's either-or: an edge round an empty track, none round a full one.
    #[test]
    fn the_ring_belongs_to_the_off_end_alone() {
        let theme = Theme::default();
        let switch = Switch::<()>::new(false);
        let alpha = |t: f32| at(&switch, t, &theme).2.a;
        assert_eq!(alpha(0.0), 1.0, "fully drawn off");
        assert_eq!(alpha(1.0), 0.0, "gone on");
        // Turned off, the colours ease in (`switch.dart:1156`).
        assert!(
            (alpha(0.5) - (1.0 - Curve::ease_in().transform(0.5))).abs() < 1e-6,
            "and part drawn halfway, rather than snapping at one end"
        );
    }

    /// The **thumb grows** as the switch is flipped — a dot inside an outlined track
    /// becoming a disc on a filled one (`switch.dart:2354`, `:2317`). It is most of what
    /// tells the two states apart at a glance, and it is the half a colour change alone
    /// would have missed.
    #[test]
    fn the_thumb_grows_as_it_travels() {
        let theme = Theme::default();
        let switch = Switch::<()>::new(false);
        let thumb = |t: f32| {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &switch,
                Rect::new(0.0, 0.0, W, H),
                Status {
                    opacity: 1.0,
                    value: t,
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            match scene.primitives()[1] {
                frus_core::Primitive::Rect { rect, .. } => rect,
                _ => panic!("the thumb is a rectangle"),
            }
        };
        assert_eq!(thumb(0.0).width, THUMB_OFF * 2.0);
        assert_eq!(thumb(1.0).width, THUMB_ON * 2.0);
        // Centred in the rounded cap at both ends, so it never breaks the pill.
        assert!((thumb(0.0).x - (H * 0.5 - THUMB_OFF)).abs() < 1e-4);
        assert!((thumb(1.0).x + thumb(1.0).width - (W - H * 0.5 + THUMB_ON)).abs() < 1e-4);
        // Half way, it is pulled into a pill wider than it is tall (`switch.dart:2382`).
        let mid = thumb(0.5);
        assert!(mid.width > mid.height, "a pill: {mid:?}");
        assert!(mid.width <= THUMB_MIDWAY.0 && mid.height <= THUMB_MIDWAY.1);
    }

    /// **The flip is the reference's**: 300 ms (`switch.dart:2386`), the thumb overshooting
    /// its end and settling back (`easeOutBack`, `:800`), and through the midway pill.
    #[test]
    fn the_flip_overshoots_and_settles() {
        let theme = Theme::default();
        let on = Switch::<()>::new(true);
        assert_eq!(Widget::<()>::anim_duration(&on), 0.3);
        assert_eq!(
            Widget::<()>::anim_curve(&on),
            Curve::Linear,
            "curved in the paint"
        );
        let centre = |switch: &Switch<()>, t: f32| {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                switch,
                Rect::new(0.0, 0.0, W, H),
                Status {
                    opacity: 1.0,
                    value: t,
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            match scene.primitives()[1] {
                frus_core::Primitive::Rect { rect, .. } => rect.x + rect.width * 0.5,
                _ => panic!("the thumb is a rectangle"),
            }
        };
        let end = W - H * 0.5;
        assert!(centre(&on, 0.7) > end, "past the end on the way on");
        assert!((centre(&on, 1.0) - end).abs() < 1e-4, "and back on it");
        let off = Switch::<()>::new(false);
        assert!(centre(&off, 0.3) < H * 0.5, "past the start on the way off");
        // Thumb sizes along the way on: off, the pill at 11 %, on by 83 %.
        let size = |t: f32| on.thumb_size(t);
        let early = size(0.05);
        assert!(
            early.0 > early.1 && early.0 > 2.0 * THUMB_OFF,
            "already stretching towards the pill: {early:?}"
        );
        assert_eq!(size(0.11), THUMB_MIDWAY);
        assert_eq!(size(0.9), (2.0 * THUMB_ON, 2.0 * THUMB_ON));
        assert_eq!(off.thumb_size(0.89), THUMB_MIDWAY, "and back the other way");
        assert_eq!(off.thumb_size(0.1), (2.0 * THUMB_OFF, 2.0 * THUMB_OFF));
    }

    /// **The box is the reference's**: the track and 4 pixels either side
    /// (`switch.dart:604`), as the caller or the theme may change.
    #[test]
    fn the_box_has_room_either_side() {
        let theme = Theme::default();
        let width =
            |switch: &Switch<()>, theme: &Theme| match Widget::<()>::style_themed(switch, theme)
                .width
            {
                Dimension::Length(w) => w,
                other => panic!("a length, not {other:?}"),
            };
        assert_eq!(width(&Switch::<()>::new(false), &theme), 60.0);
        assert_eq!(
            width(&Switch::<()>::new(false).padding(Insets::ZERO), &theme),
            W
        );
        let mut roomy = Theme::default();
        roomy.widgets.switch.padding = Some(Insets::new(0.0, 10.0, 0.0, 10.0));
        assert_eq!(width(&Switch::<()>::new(false), &roomy), 72.0);
    }

    /// Each end of the travel takes its own colour.
    #[test]
    fn each_end_takes_its_own_colour() {
        let theme = Theme::default();
        let switch = Switch::<()>::new(true)
            .track_color(BRAND)
            .inactive_track_color(RAIL)
            .thumb_color(KNOB);
        let (track, thumb, _) = painted(&switch, true, &theme);
        lands_on(track, BRAND);
        lands_on(thumb, KNOB);
        lands_on(painted(&switch, false, &theme).0, RAIL);
    }

    /// The mix happens between the two **resolved** ends, so an override moves the whole
    /// animation rather than being a colour the switch passes through.
    #[test]
    fn the_travel_runs_between_the_two_overrides() {
        let theme = Theme::default();
        let switch = Switch::<()>::new(true)
            .track_color(BRAND)
            .inactive_track_color(RAIL);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &switch,
            Rect::new(0.0, 0.0, W, H),
            Status {
                opacity: 1.0,
                value: 0.5,
                ..Default::default()
            },
            &theme,
            &mut scene,
        );
        let track = match scene.primitives()[0] {
            frus_core::Primitive::Rect { color, .. } => color,
            _ => panic!("the track is a rectangle"),
        };
        // Turned on, the colours ease out (`switch.dart:1155`).
        assert_eq!(
            track,
            RAIL.lerp(BRAND, Curve::ease_out().transform(0.5)),
            "between the two ends"
        );
    }

    /// The two ends of the thumb are **two colours**, not one.
    ///
    /// They used to be one: an unsaid off thumb followed the on one, on the reasoning
    /// that a switch is a single thumb sliding rather than two swapping places. The
    /// reasoning holds and the conclusion did not — the reference resolves both ends and
    /// interpolates between them, so it is still one thumb, one that changes colour as it
    /// travels. Saying the on colour therefore no longer says the off one.
    #[test]
    fn the_two_ends_of_the_thumb_are_two_colours() {
        let theme = Theme::default();
        let switch = Switch::<()>::new(false).thumb_color(KNOB);
        lands_on(painted(&switch, false, &theme).1, theme.scheme.outline);
        lands_on(painted(&switch, true, &theme).1, KNOB);
        let both = Switch::<()>::new(false)
            .thumb_color(KNOB)
            .inactive_thumb_color(RAIL);
        lands_on(painted(&both, false, &theme).1, RAIL);
    }

    /// A colour the track lands on at the end of its travel. It is a **lerp** to get
    /// there, so the arrival is within a rounding step of the colour asked for rather
    /// than bit-identical to it — which is why this compares by eye rather than by bits.
    fn lands_on(got: Color, want: Color) {
        let off = (got.r - want.r)
            .abs()
            .max((got.g - want.g).abs())
            .max((got.b - want.b).abs());
        assert!(off < 1e-4, "{got:?} is not {want:?}");
    }

    /// The theme answers when the instance does not, and loses when it does.
    #[test]
    fn the_theme_answers_and_the_instance_overrules_it() {
        let mut theme = Theme::default();
        theme.widgets.switch.track_color = Some(RAIL);
        lands_on(painted(&Switch::<()>::new(true), true, &theme).0, RAIL);
        lands_on(
            painted(&Switch::<()>::new(true).track_color(BRAND), true, &theme).0,
            BRAND,
        );
    }
}

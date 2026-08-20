//! [`Switch`]: a **controlled** toggle switch, shaped as a pill.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_container, disabled_content, disabled_mark};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const W: f32 = 44.0;
const H: f32 = 24.0;
const MARGIN: f32 = 3.0;

/// An on/off switch.
pub struct Switch<Msg> {
    on: bool,
    enabled: bool,
    track_color: Option<Color>,
    inactive_track_color: Option<Color>,
    thumb_color: Option<Color>,
    inactive_thumb_color: Option<Color>,
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
            on_toggle: None,
        }
    }

    /// The track's colour when the switch is **on**; the theme's `primary` otherwise.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// The track's colour when the switch is **off**; the theme's border otherwise.
    pub fn inactive_track_color(mut self, color: Color) -> Self {
        self.inactive_track_color = Some(color);
        self
    }

    /// The thumb's colour when the switch is **on**; white otherwise.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = Some(color);
        self
    }

    /// The thumb's colour when the switch is **off**. Unset it follows the on colour,
    /// which is what a switch looks like: one thumb sliding, not two swapping places.
    pub fn inactive_thumb_color(mut self, color: Color) -> Self {
        self.inactive_thumb_color = Some(color);
        self
    }

    /// A closure producing a message from the new state.
    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
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
}

impl<Msg> Widget<Msg> for Switch<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(W),
            height: Dimension::Length(H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // `t` is the switch's animated position: 0 is off, 1 is on.
        let t = status.value;
        // A switch is the control that takes **both** halves of the disabled rule: the
        // track is a container (12 %), the thumb is content drawn on it (38 %), and when
        // the thumb sits on that flattened track it punches through opaquely instead.
        // Each end of the travel is resolved on its own -- the caller's word, then the
        // theme's, then the scheme's -- and `t` interpolates between the two ends rather
        // than between two already-resolved colours. An override therefore moves the
        // whole animation with it instead of being a colour the switch passes through.
        let (track, thumb) = if self.enabled {
            let on_track = self
                .track_color
                .or(theme.widgets.switch.track_color)
                .unwrap_or(theme.primary);
            let off_track = self
                .inactive_track_color
                .or(theme.widgets.switch.inactive_track_color)
                .unwrap_or(theme.border);
            let on_thumb = self
                .thumb_color
                .or(theme.widgets.switch.thumb_color)
                .unwrap_or(Color::WHITE);
            let off_thumb = self
                .inactive_thumb_color
                .or(theme.widgets.switch.inactive_thumb_color)
                .unwrap_or(on_thumb);
            (off_track.lerp(on_track, t), off_thumb.lerp(on_thumb, t))
        } else {
            (
                disabled_container(theme),
                if self.on {
                    disabled_mark(theme)
                } else {
                    disabled_content(theme)
                },
            )
        };
        scene.draw_rect(bounds, track.fade(o), H * 0.5, 0.0, Color::TRANSPARENT);

        let d = H - MARGIN * 2.0;
        let off_x = bounds.x + MARGIN;
        let on_x = bounds.x + W - MARGIN - d;
        let thumb_x = off_x + (on_x - off_x) * t;
        scene.draw_rect(
            Rect::new(thumb_x, bounds.y + MARGIN, d, d),
            thumb.fade(o),
            d * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        self.on_toggle.as_ref().map(|make| make(!self.on))
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // Still on or off, still announced — a switch that fell silent would read as a
        // setting that had gone away rather than one that cannot be changed.
        let semantics = frus_core::Semantics::new(frus_core::Role::Switch).toggled(self.on);
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }

    fn anim_target(&self) -> Option<f32> {
        Some(if self.on { 1.0 } else { 0.0 })
    }

    /// The thumb glides smoothly, accelerating then braking, rather than at a constant
    /// speed — a switch's standard implicit animation.
    fn anim_curve(&self) -> frus_core::Curve {
        frus_core::Curve::ease_in_out()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disabled::{disabled_container, disabled_content, disabled_mark};
    use crate::widget::Widget;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Set(bool),
    }

    fn painted(on: bool, enabled: bool, theme: &Theme) -> Vec<frus_core::Color> {
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
                frus_core::Primitive::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
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
            let (track, thumb) = (off[0], off[1]);
            assert_eq!(
                track,
                disabled_container(&theme),
                "the track is a container"
            );
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
                from_surface(track) < from_surface(thumb),
                "and the container is the quieter of the two"
            );

            // Flipped on, the thumb is sitting *on* that flattened track, so it punches
            // through opaquely rather than adding a third translucent layer.
            let on = painted(true, false, &theme);
            assert_eq!(on[0], disabled_container(&theme), "the same track");
            assert_eq!(on[1], disabled_mark(&theme), "an opaque thumb");
        }
    }

    #[test]
    fn a_live_switch_is_untouched_by_any_of_it() {
        let theme = Theme::dark();
        let live = painted(true, true, &theme);
        assert_ne!(live[0], disabled_container(&theme));
        assert_ne!(live[1], disabled_content(&theme));
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::widget::Widget;

    const BRAND: Color = Color::rgb(0.0, 0.6, 0.3);
    const RAIL: Color = Color::rgb(0.9, 0.9, 0.2);
    const KNOB: Color = Color::rgb(0.1, 0.1, 0.9);

    /// The (track, thumb) a switch painted, in order.
    fn painted(switch: &Switch<()>, on: bool, theme: &Theme) -> (Color, Color) {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            switch,
            Rect::new(0.0, 0.0, W, H),
            Status {
                opacity: 1.0,
                value: if on { 1.0 } else { 0.0 },
                ..Default::default()
            },
            theme,
            &mut scene,
        );
        let rects: Vec<Color> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        (rects[0], rects[1])
    }

    /// Nothing said: what it always painted.
    #[test]
    fn the_defaults_are_what_they_were() {
        let theme = Theme::default();
        let (track, thumb) = painted(&Switch::<()>::new(true), true, &theme);
        assert_eq!(track, theme.primary);
        assert_eq!(thumb, Color::WHITE);
        let (off, _) = painted(&Switch::<()>::new(false), false, &theme);
        assert_eq!(off, theme.border);
    }

    /// Each end of the travel takes its own colour.
    #[test]
    fn each_end_takes_its_own_colour() {
        let theme = Theme::default();
        let switch = Switch::<()>::new(true)
            .track_color(BRAND)
            .inactive_track_color(RAIL)
            .thumb_color(KNOB);
        let (track, thumb) = painted(&switch, true, &theme);
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
        assert_eq!(track, RAIL.lerp(BRAND, 0.5), "halfway between the two ends");
    }

    /// An off thumb follows the on one: a switch is one thumb sliding, not two.
    #[test]
    fn an_unsaid_off_thumb_follows_the_on_one() {
        let theme = Theme::default();
        let switch = Switch::<()>::new(false).thumb_color(KNOB);
        lands_on(painted(&switch, false, &theme).1, KNOB);
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

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
    on_toggle: Option<Box<dyn Fn(bool) -> Msg>>,
}

impl<Msg> Switch<Msg> {
    /// Creates a switch whose state is supplied.
    pub fn new(on: bool) -> Self {
        Self {
            on,
            enabled: true,
            on_toggle: None,
        }
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
        let (track, thumb) = if self.enabled {
            (theme.border.lerp(theme.primary, t), Color::WHITE)
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
            assert!(
                track.a < thumb.a,
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

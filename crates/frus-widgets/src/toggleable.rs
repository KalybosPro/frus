//! What the checkbox and the radio share, as the reference's `ToggleableStateMixin` and
//! `ToggleablePainter` share it (`material/toggleable.dart`): the halo round the control
//! under a pointer, the keyboard or a finger, and its colours (milestone 635).

use frus_core::{Color, Point, Rect, Scene};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widgetstate::{WidgetState, WidgetStates};

/// The halo's radius, unless told otherwise (`kRadialReactionRadius`).
pub(crate) const SPLASH: f32 = 20.0;

/// **The reference's halo colours** for a checkbox and a radio, Material 3
/// (`checkbox.dart:998`, `radio.dart:977`): under a pointer 8 %, focused or pressed 10 %;
/// round an unchosen control `on_surface` under a pointer and focused and `primary`
/// pressed, round a chosen one the other way about. Nothing at rest.
pub(crate) fn overlay(theme: &Theme, states: WidgetStates) -> Color {
    let c = &theme.scheme;
    let pick = |press: Color, hover: Color, focus: Color| {
        if states.contains(WidgetState::Pressed) {
            press.with_alpha(0.1)
        } else if states.contains(WidgetState::Hovered) {
            hover.with_alpha(0.08)
        } else if states.contains(WidgetState::Focused) {
            focus.with_alpha(0.1)
        } else {
            Color::TRANSPARENT
        }
    };
    if states.contains(WidgetState::Selected) {
        pick(c.on_surface, c.primary, c.primary)
    } else {
        pick(c.primary, c.on_surface, c.on_surface)
    }
}

/// **Paints the halo** round `centre`: the hovered and focused ones fading in by their
/// progress, the pressed one growing by its own (the reference's radial reaction).
/// `colour_for` answers the halo's colour with one more state added.
pub(crate) fn paint_halos(
    scene: &mut Scene,
    centre: Point,
    radius: f32,
    status: &Status,
    colour_for: impl Fn(WidgetState) -> Color,
) {
    let o = status.opacity;
    let mut halo = |state: WidgetState, amount: f32, r: f32| {
        if amount <= 0.0 || r <= 0.0 {
            return;
        }
        let c = colour_for(state);
        if c.a > 0.0 {
            scene.draw_rect(
                Rect::new(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r),
                c.with_alpha(c.a * amount.clamp(0.0, 1.0)).fade(o),
                r,
                0.0,
                Color::TRANSPARENT,
            );
        }
    };
    halo(WidgetState::Hovered, status.hover_progress, radius);
    halo(WidgetState::Focused, status.focus_progress, radius);
    let press = status.press_progress.clamp(0.0, 1.0);
    halo(WidgetState::Pressed, 1.0, radius * press);
}

#[cfg(test)]
mod tests {
    use super::overlay;
    use crate::theme::Theme;
    use crate::widgetstate::{WidgetState, WidgetStates};

    /// **The halo's colour, state by state** (`checkbox.dart:998`, `radio.dart:977`): a
    /// press round a chosen control is `on_surface`, round an unchosen one `primary`; a
    /// pointer the other way about.
    #[test]
    fn the_halo_colours_are_the_reference_s() {
        let theme = Theme::dark();
        let c = &theme.scheme;
        let on = |s: WidgetState| overlay(&theme, WidgetState::Selected | s);
        let off = |s: WidgetState| overlay(&theme, WidgetStates::of(s));
        assert_eq!(on(WidgetState::Pressed), c.on_surface.with_alpha(0.1));
        assert_eq!(on(WidgetState::Hovered), c.primary.with_alpha(0.08));
        assert_eq!(on(WidgetState::Focused), c.primary.with_alpha(0.1));
        assert_eq!(off(WidgetState::Pressed), c.primary.with_alpha(0.1));
        assert_eq!(off(WidgetState::Hovered), c.on_surface.with_alpha(0.08));
        assert_eq!(off(WidgetState::Focused), c.on_surface.with_alpha(0.1));
        assert_eq!(
            overlay(&theme, WidgetStates::EMPTY).a,
            0.0,
            "nothing at rest"
        );
    }
}

//! [`Switch`]: a **controlled** toggle switch, shaped as a pill.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::DISABLED_CONTAINER_OPACITY;
use crate::disabled::{disabled_container, disabled_content, disabled_mark, over_surface};
use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

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
/// A thumb **carrying an icon** is the on-thumb's size at both ends (`switch.dart:2369`):
/// 16 pixels of glyph do not fit in a 16-pixel dot, and a switch whose thumb changed size
/// only when it had something to show would be two different switches.
const THUMB_WITH_ICON: f32 = 12.0;
/// The glyph inside the thumb (`switch.dart:2314`).
const ICON_SIZE: f32 = 16.0;
/// The grid the icon paths are drawn on.
const ICON_GRID: f32 = 24.0;
/// The rule round an **off** track (`switch.dart:2298`).
const TRACK_OUTLINE: f32 = 2.0;

/// An on/off switch.
pub struct Switch<Msg> {
    on: bool,
    enabled: bool,
    track_color: Option<Color>,
    inactive_track_color: Option<Color>,
    thumb_color: Option<Color>,
    inactive_thumb_color: Option<Color>,
    thumb_icon: Option<Icons>,
    inactive_thumb_icon: Option<Icons>,
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
            thumb_icon: None,
            inactive_thumb_icon: None,
            on_toggle: None,
        }
    }

    /// The track's colour when the switch is **on**; the theme's `primary` otherwise.
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
    /// third way.
    ///
    /// Giving either end an icon makes **both** thumbs the on-thumb's size
    /// (`switch.dart:2369`), because a switch whose thumb changed size only when it had
    /// something to show would be two different switches.
    pub fn thumb_icon(mut self, icon: Icons) -> Self {
        self.thumb_icon = Some(icon);
        self
    }

    /// The same while the switch is **off** — a cross beside the tick. See
    /// [`Self::thumb_icon`].
    pub fn inactive_thumb_icon(mut self, icon: Icons) -> Self {
        self.inactive_thumb_icon = Some(icon);
        self
    }

    /// The thumb's colour when the switch is **on**; the scheme's `on_primary`
    /// otherwise (`switch.dart:2201`) — the content colour of the track it sits on.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = Some(color);
        self
    }

    /// The thumb's colour when the switch is **off**; the scheme's `outline` otherwise
    /// (`switch.dart:2212`).
    ///
    /// It used to follow the on colour, on the reasoning that a switch is one thumb
    /// sliding rather than two swapping places. The reasoning was right and the
    /// conclusion was not: the reference resolves both ends and **interpolates between
    /// them**, so it is still one thumb — one that changes colour as it travels, the way
    /// the track under it does.
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
        let (track, thumb, edge) = if self.enabled {
            let on_track = self
                .track_color
                .or(theme.widgets.switch.track_color)
                .unwrap_or(theme.primary);
            let off_track = self
                .inactive_track_color
                .or(theme.widgets.switch.inactive_track_color)
                .unwrap_or(theme.scheme.surface_container_highest);
            let on_thumb = self
                .thumb_color
                .or(theme.widgets.switch.thumb_color)
                .unwrap_or(theme.scheme.on_primary);
            let off_thumb = self
                .inactive_thumb_color
                .or(theme.widgets.switch.inactive_thumb_color)
                .unwrap_or(theme.scheme.outline);
            (
                off_track.lerp(on_track, t),
                off_thumb.lerp(on_thumb, t),
                theme.scheme.outline,
            )
        } else {
            (
                if self.on {
                    // A disabled **on** track is the flattened container
                    // (`switch.dart:2221`).
                    disabled_container(theme)
                } else {
                    // A disabled **off** one is not: the reference washes
                    // `surfaceContainerHighest` over the page at the same 12 %
                    // (`switch.dart:2223`), which is nearly the page itself — a faint
                    // ring with almost nothing inside. `disabled_container` is the
                    // *ring's* colour, and filling the pill with it would draw the
                    // opposite. Resolved in sRGB for the reason
                    // [`crate::disabled::over_surface`] gives.
                    theme.scheme.surface.lerp(
                        theme.scheme.surface_container_highest,
                        DISABLED_CONTAINER_OPACITY,
                    )
                },
                if self.on {
                    disabled_mark(theme)
                } else {
                    disabled_content(theme)
                },
                disabled_container(theme),
            )
        };
        // The rule belongs to the **off** end alone. A filled track needs no edge, and the
        // reference returns a transparent one for a switch that is on whether it is
        // available or not (`switch.dart:2254`) — so fading it out along the travel *is*
        // that rule, written as the animation it already was.
        scene.draw_rect(
            bounds,
            track.fade(o),
            H * 0.5,
            TRACK_OUTLINE,
            edge.fade((1.0 - t) * o),
        );

        // Both ends of the travel put the thumb's centre half a track-height in from their
        // own edge, so it stays centred in the rounded cap whichever size it is.
        let icon = if self.on {
            self.thumb_icon
        } else {
            self.inactive_thumb_icon
        };
        // **A thumb that carries a glyph is the on-thumb's size at both ends**
        // (`switch.dart:2369`): sixteen pixels of glyph do not fit in a sixteen-pixel dot.
        // The rule is about the switch, not about this end of it — either icon sets both,
        // or a switch would change size when it was flipped for a reason that has nothing
        // to do with being flipped.
        let carries_an_icon = self.thumb_icon.is_some() || self.inactive_thumb_icon.is_some();
        let off_r = if carries_an_icon {
            THUMB_WITH_ICON
        } else {
            THUMB_OFF
        };
        let mut r = off_r + (THUMB_ON - off_r) * t;
        // Held, it swells past either end (`switch.dart:2357`) — the squish a finger
        // expects back, **grown** into over the press's own 200 ms rather than jumped to.
        // It is measured from wherever the thumb has got to, so a switch held mid-travel
        // swells from where it is instead of snapping back to an end first.
        if self.enabled {
            r += (THUMB_PRESSED - r) * status.press_progress.clamp(0.0, 1.0);
        }
        let cx = bounds.x + H * 0.5 + (W - H) * t;
        let cy = bounds.y + H * 0.5;

        // **The state layer**, which this had none of. The reference paints the toggle's
        // radial reaction over the track and under the thumb (`switch.dart:2264`); here it
        // is the theme's one rule, resolved opaquely from the track it stands on — a
        // translucent circle would blend in linear light and paint at something other than
        // the number it names (milestones 329, 437).
        //
        // **The ink is the track's content colour, where the reference's is `primary`.**
        // That is not a disagreement about the role: the reference's reaction circle is
        // wider than its track and spills onto the page, so `primary` over the page is
        // visible at either end. This one is bounded by the switch's own box, so its
        // ground is the track — and `primary` lerped over a `primary` track is that track
        // again. A state layer takes the content colour of what it stands on, which is
        // what [`Theme::state_layer`] asks for and what makes it visible at both ends.
        //
        // Nothing at all on a disabled switch: a state layer is the promise of an
        // interaction, and there is none.
        if self.enabled {
            let ink = theme.scheme.on_surface.lerp(theme.scheme.on_primary, t);
            let layer = theme.state_layer(track, ink, &status);
            if layer != track {
                let reach = H * 0.5;
                scene.draw_rect(
                    Rect::new(cx - reach, cy - reach, reach * 2.0, reach * 2.0),
                    layer.fade(o),
                    reach,
                    0.0,
                    Color::TRANSPARENT,
                );
            }
        }

        scene.draw_rect(
            Rect::new(cx - r, cy - r, r * 2.0, r * 2.0),
            thumb.fade(o),
            r,
            0.0,
            Color::TRANSPARENT,
        );

        // And the glyph inside it. Its colour is the track's own at the off end
        // (`switch.dart:2349`), so it reads as a hole punched through the thumb rather
        // than as a mark drawn on it.
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
            let path = icon
                .path()
                .scaled(ICON_SIZE / ICON_GRID)
                .translated(cx - ICON_SIZE * 0.5, cy - ICON_SIZE * 0.5);
            scene.fill_path(&path, ink.fade(o));
        }
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

    /// **A switch answers the pointer** (milestone 440).
    ///
    /// It had no state layer at all: hovering one, focusing it, holding it — nothing
    /// changed. The reference paints the toggle's radial reaction over the track and under
    /// the thumb (`switch.dart:2264`); here it is the theme's one rule, resolved opaquely
    /// from the track it stands on, because a translucent circle would blend in linear
    /// light and paint at something other than the number it names.
    #[test]
    fn a_switch_answers_the_pointer() {
        let theme = Theme::default();
        let switch = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        let hovered = Status {
            hover_progress: 1.0,
            ..state(false)
        };
        assert_eq!(
            boxes(&switch, state(false), &theme).len(),
            2,
            "track, thumb"
        );
        let lit = boxes(&switch, hovered, &theme);
        assert_eq!(lit.len(), 3, "track, layer, thumb");

        let (rect, color) = lit[1];
        assert_eq!(color.a, 1.0, "resolved here, not handed over as an alpha");
        assert_eq!(
            color,
            theme.state_layer(
                theme.scheme.surface_container_highest,
                theme.scheme.on_surface,
                &hovered
            ),
            "the theme's rule, over the track it stands on"
        );
        // Centred on the thumb, and inside the switch either way it is set.
        assert!(
            rect.x >= 0.0 && rect.x + rect.width <= W,
            "the layer left the switch: {rect:?}"
        );
        assert!(
            (rect.height - H).abs() < 0.01,
            "and it reaches the track's full height"
        );
    }

    /// Its ink is **the track's content colour**, which the travel interpolates between
    /// like everything else here.
    ///
    /// The reference names `primary` at the on end (`switch.dart:2266`), which works there
    /// because its reaction circle is wider than its track and spills onto the page. This
    /// one is bounded by the switch's box, so `primary` over a `primary` track would be
    /// that track again — the layer would vanish exactly where a pointer is most likely to
    /// be.
    #[test]
    fn the_layer_is_the_accent_at_one_end_and_the_ink_at_the_other() {
        let theme = Theme::default();
        let switch = Switch::<Msg>::new(true).on_toggle(Msg::Set);
        let hovered = |on: bool| Status {
            hover_progress: 1.0,
            ..state(on)
        };
        let off = boxes(&switch, hovered(false), &theme)[1].1;
        let on = boxes(&switch, hovered(true), &theme)[1].1;
        assert_eq!(
            on,
            theme.state_layer(theme.primary, theme.scheme.on_primary, &hovered(true)),
            "the accent track, tinted with what is legible on it"
        );
        assert_ne!(off, on, "the two ends do not light the same way");
        assert_ne!(on, theme.primary, "and it is visible against the track");
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

    /// **A thumb that carries a glyph is the larger one at both ends**
    /// (`switch.dart:2369`): sixteen pixels of glyph do not fit in a sixteen-pixel dot.
    ///
    /// And the rule is about the *switch*, not about the end it is at — giving only the on
    /// end an icon still grows the off thumb, or the switch would change size when flipped
    /// for a reason that has nothing to do with being flipped.
    #[test]
    fn a_thumb_that_carries_a_glyph_is_the_larger_one() {
        let theme = Theme::default();
        let bare = Switch::<Msg>::new(false).on_toggle(Msg::Set);
        assert_eq!(thumb_radius(&bare, state(false), &theme), THUMB_OFF);

        let ticked = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .thumb_icon(Icons::Check);
        assert_eq!(
            thumb_radius(&ticked, state(false), &theme),
            THUMB_WITH_ICON,
            "the on end's icon grows the off thumb too"
        );
        assert_eq!(
            thumb_radius(&ticked, state(true), &theme),
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
            .thumb_icon(Icons::Check);
        assert_eq!(
            glyph(&ticked, true),
            Some(theme.scheme.on_primary_container)
        );
        // The end a switch is **set** to decides which glyph it carries, as the reference
        // resolves `thumbIcon` from the state rather than from the animation: a switch
        // that is off and names no off icon carries none.
        let unticked = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .thumb_icon(Icons::Check);
        assert_eq!(glyph(&unticked, false), None, "nothing at the other end");

        let crossed = Switch::<Msg>::new(false)
            .on_toggle(Msg::Set)
            .thumb_icon(Icons::Check)
            .inactive_thumb_icon(Icons::Close);
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
        assert!(
            (alpha(0.5) - 0.5).abs() < 1e-6,
            "and half drawn halfway, rather than snapping at one end"
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
        assert_eq!(track, RAIL.lerp(BRAND, 0.5), "halfway between the two ends");
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

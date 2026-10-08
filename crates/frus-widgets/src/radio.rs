//! [`RadioGroup`]: a group of radio buttons, with one option selected.

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene, TextStyle};

use crate::widgetstate::{WidgetState, WidgetStates};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::disabled_content;
use crate::interaction::Status;
use crate::theme::{TapTarget, Theme};
use crate::widget::Widget;

/// The ring's radius, to the middle of its stroke (`radio.dart:31`).
const OUTER: f32 = 8.0;
/// The dot's radius once chosen (`radio.dart:32`).
const INNER: f32 = 4.5;
/// The ring's stroke.
const STROKE: f32 = 2.0;
/// How long the dot takes to grow (the reference's toggle animation).
const TOGGLE_SECONDS: f32 = 0.2;
/// The label's size beside a radio, where nothing says otherwise.
const LABEL_SIZE: f32 = 18.0;

/// One radio option, internal to the group.
/// **One radio button.**
///
/// It existed before this, as a private `Radio` a [`RadioGroup`] built for each of
/// its labels, and there was no way to have one on its own. The reference's `Radio` is a
/// widget in its own right — which is what lets a radio sit in a list row, in a table
/// cell, or anywhere else the group's fixed column of labels is the wrong shape.
///
/// ```
/// # use frus_widgets::Radio;
/// # #[derive(Clone)] enum Msg { Pick }
/// Radio::<Msg>::new(true).label("Every day").on_select(Msg::Pick);
/// ```
///
/// A radio does **not** know how to turn itself off: the reference's takes a value and a
/// group value and reports the value it stands for, and turning one off is choosing
/// another. So this reports one message when pressed and says nothing about what the
/// answer becomes.
pub struct Radio<Msg = crate::callback::Callback> {
    label: String,
    selected: bool,
    size: f32,
    /// The group's colours, handed down with everything else it decides.
    colors: RadioColors,
    /// The group's availability, handed down. An option that stayed live under a disabled
    /// group would be the whole group, since a group is only ever its options.
    enabled: bool,
    /// The group's answer on how much room to reserve for a finger, handed down.
    tap_target: Option<TapTarget>,
    /// How compact it is; the theme's radio density, then the theme's.
    visual_density: Option<crate::VisualDensity>,
    /// The halo's radius.
    splash_radius: Option<f32>,
    on_click: Option<Msg>,
}

impl<Msg> Radio<Msg> {
    /// A radio, chosen or not. It says nothing and answers nothing until it is given a
    /// [`label`](Self::label) and an [`on_select`](Self::on_select).
    pub fn new(selected: bool) -> Self {
        Self {
            label: String::new(),
            selected,
            size: LABEL_SIZE,
            colors: RadioColors::default(),
            enabled: true,
            tap_target: None,
            visual_density: None,
            splash_radius: None,
            on_click: None,
        }
    }

    /// The words beside it. Empty by default, which is what a radio in a row that has its
    /// own title wants — see [`RadioListTile`](crate::RadioListTile).
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The label's size, in pixels.
    #[must_use]
    pub fn label_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// What to say when it is pressed. A radio with no message is inert.
    #[must_use]
    pub fn on_select(mut self, message: impl Into<Msg>) -> Self {
        self.on_click = Some(message.into());
        self
    }

    /// Whether it can be pressed.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The ring and the dot while it is the chosen one, over the theme's and `primary`.
    #[must_use]
    pub fn selected_color(mut self, color: Color) -> Self {
        self.colors.selected = Some(color);
        self
    }

    /// The ring while it is not, at rest.
    #[must_use]
    pub fn border_color(mut self, color: Color) -> Self {
        self.colors.border = Some(color);
        self
    }

    /// And the ring while it is not, under a finger, a pointer or focus.
    #[must_use]
    pub fn active_border_color(mut self, color: Color) -> Self {
        self.colors.active_border = Some(color);
        self
    }

    /// The label's colour.
    #[must_use]
    pub fn label_color(mut self, color: Color) -> Self {
        self.colors.label = Some(color);
        self
    }

    /// **How compact it is.** Unset, the theme's radio density, then the theme's own: a
    /// radio follows the theme's density, as the reference's does (`radio.dart:1008`).
    #[must_use]
    pub fn visual_density(mut self, density: crate::VisualDensity) -> Self {
        self.visual_density = Some(density);
        self
    }

    /// **The halo's radius** under a pointer, the keyboard or a finger. Unset, the
    /// theme's, then 20.
    #[must_use]
    pub fn splash_radius(mut self, radius: f32) -> Self {
        self.splash_radius = Some(radius);
        self
    }

    /// How much room it reserves for the finger that works it.
    #[must_use]
    pub fn tap_target(mut self, target: TapTarget) -> Self {
        self.tap_target = Some(target);
        self
    }

    /// The label's style, **resolved once** so that the number the box is measured with is
    /// the number the glyphs are drawn at. Resolving is the single place the reader's font
    /// setting is applied (milestone 403).
    fn label_style(&self) -> ResolvedTextStyle {
        TextStyle::new(self.size).resolved()
    }

    /// The square the ring sits in: 48 px padded or 40 shrink-wrapped, moved by the
    /// density (`radio.dart:738`).
    fn square(&self, theme: &Theme) -> f32 {
        let target = self
            .tap_target
            .or(theme.widgets.radio.tap_target)
            .unwrap_or(theme.tap_target);
        let side = match target {
            TapTarget::Padded => crate::MIN_TAP_TARGET,
            TapTarget::ShrinkWrap => crate::MIN_TAP_TARGET - 8.0,
        };
        let density = self
            .visual_density
            .or(theme.widgets.radio.visual_density)
            .unwrap_or_else(|| theme.visual_density());
        (side + density.base_size_adjustment().1).max(2.0 * (OUTER + STROKE * 0.5))
    }

    /// The ring's and the dot's colour for these states (`radio.dart:943`): the chosen
    /// colour once chosen, `on_surface` under a pointer, the keyboard or a finger and
    /// `on_surface_variant` at rest otherwise, `on_surface` at 38 % disabled.
    fn fill(&self, theme: &Theme, states: WidgetStates) -> Color {
        let c = self.colors;
        if states.contains(WidgetState::Disabled) {
            return disabled_content(theme);
        }
        if states.contains(WidgetState::Selected) {
            return c
                .selected
                .or(theme.widgets.radio.selected_color)
                .unwrap_or(theme.scheme.primary);
        }
        let resting = c.border.or(theme.widgets.radio.border_color);
        let active = states.contains(WidgetState::Pressed)
            || states.contains(WidgetState::Hovered)
            || states.contains(WidgetState::Focused);
        if active {
            c.active_border
                .or(theme.widgets.radio.active_border_color)
                .or(resting)
                .unwrap_or(theme.scheme.on_surface)
        } else {
            resting.unwrap_or(theme.scheme.on_surface_variant)
        }
    }

    /// The states it is in.
    fn states(&self, status: &Status) -> WidgetStates {
        let states = if self.enabled {
            status.states()
        } else {
            WidgetStates::of(WidgetState::Disabled)
        };
        states.set(WidgetState::Selected, self.selected)
    }
}

impl<Msg: Clone> Widget<Msg> for Radio<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    /// **A square a finger can hit**, 48 px at the standard density with the ring in its
    /// middle (`radio.dart:738`), and the label after it.
    fn style_themed(&self, theme: &Theme) -> Style {
        let square = self.square(theme);
        let style = self.label_style();
        let label_w = if self.label.is_empty() {
            0.0
        } else {
            frus_text::measure_resolved(&self.label, &style).width
        };
        Style {
            width: Dimension::Length((square + label_w).ceil()),
            height: Dimension::Length(style.line_height().max(square).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let square = self.square(theme);
        let centre = Point::new(bounds.x + square * 0.5, bounds.y + bounds.height * 0.5);
        let states = self.states(&status);

        if self.enabled {
            let radius = self
                .splash_radius
                .or(theme.widgets.radio.splash_radius)
                .unwrap_or(crate::toggleable::SPLASH);
            let overlay = theme.widgets.radio.overlay_color.as_ref();
            crate::toggleable::paint_halos(scene, centre, radius, &status, |state| {
                let s = states.set(state, true);
                overlay
                    .and_then(|p| p.resolve(s).copied())
                    .unwrap_or_else(|| crate::toggleable::overlay(theme, s))
            });
        }

        // The ring, its colour moving from the unchosen to the chosen one, and the dot
        // growing from nothing to 4.5 px as the radio is chosen (`radio.dart:849`).
        let t = status.value.clamp(0.0, 1.0);
        let off = self.fill(theme, states.set(WidgetState::Selected, false));
        let on = self.fill(theme, states.set(WidgetState::Selected, true));
        // At either end, the colour itself: a lerp to 1 is not exactly its target.
        let ring = if t >= 1.0 {
            on
        } else if t <= 0.0 {
            off
        } else {
            off.lerp(on, t)
        };
        // A stroke centred on the 8 px circle: the box is the circle's outer edge, and the
        // stroke is drawn inside it.
        let edge = OUTER + STROKE * 0.5;
        scene.draw_rect(
            Rect::new(centre.x - edge, centre.y - edge, 2.0 * edge, 2.0 * edge),
            Color::TRANSPARENT,
            edge,
            STROKE,
            ring.fade(o),
        );
        if t > 0.0 {
            let r = INNER * t;
            scene.draw_rect(
                Rect::new(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r),
                ring.fade(o),
                r,
                0.0,
                Color::TRANSPARENT,
            );
        }

        if !self.label.is_empty() {
            let colour = if self.enabled {
                self.colors
                    .label
                    .or(theme.widgets.radio.label_color)
                    .unwrap_or(theme.on_surface)
            } else {
                disabled_content(theme)
            };
            let style = self.label_style();
            scene.text(
                Point::new(
                    bounds.x + square,
                    bounds.y + (bounds.height - style.line_height()) * 0.5,
                ),
                self.label.clone(),
                &style,
                colour.fade(o),
            );
        }
    }

    /// Chosen or not: choosing grows the dot over a fifth of a second.
    fn anim_target(&self) -> Option<f32> {
        Some(if self.selected { 1.0 } else { 0.0 })
    }

    fn anim_duration(&self) -> f32 {
        TOGGLE_SECONDS
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        self.on_click.clone()
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // A radio group had no semantics at all before this: a reader was told nothing
        // about which option was chosen. Adding the disabled announcement without the
        // announcement it qualifies would have been announcing an absence.
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::RadioButton)
            .label(self.label.clone())
            .toggled(self.selected);
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// What a [`RadioGroup`] was told about its own colours, handed to each option.
///
/// Unset entries fall through to the theme and then the scheme, resolved where they are
/// painted rather than here: a group built under one theme and rendered under another --
/// which [`crate::Themed`] makes ordinary -- must take the theme it is *painted* in.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct RadioColors {
    selected: Option<Color>,
    border: Option<Color>,
    active_border: Option<Color>,
    label: Option<Color>,
}

/// A single-selection group of radio buttons.
pub struct RadioGroup<Msg = crate::callback::Callback> {
    selected: usize,
    size: f32,
    gap: f32,
    enabled: bool,
    colors: RadioColors,
    tap_target: Option<TapTarget>,
    on_select: Box<dyn Fn(usize) -> Msg>,
    /// The labels as given. The options are **derived** from these, so that a builder
    /// called after them — `enabled`, and any that follow — still reaches every option
    /// rather than only the ones added afterwards.
    labels: Vec<String>,
    options: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> RadioGroup<Msg> {
    /// Creates a group: the selected index plus an `index -> message` closure.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            size: 18.0,
            gap: 8.0,
            enabled: true,
            colors: RadioColors::default(),
            tap_target: None,
            on_select: Box::new(on_select),
            labels: Vec::new(),
            options: Vec::new(),
        }
    }

    /// The ring and the dot of the **chosen** option; the theme's `primary` otherwise.
    pub fn selected_color(mut self, color: Color) -> Self {
        self.colors.selected = Some(color);
        self.rebuild();
        self
    }

    /// The ring of an option that is not chosen, at rest.
    ///
    /// Set on its own it also becomes the ring under a pointer or focus, unless
    /// [`active_border_color`](RadioGroup::active_border_color) says otherwise.
    pub fn border_color(mut self, color: Color) -> Self {
        self.colors.border = Some(color);
        self.rebuild();
        self
    }

    /// That ring under a pointer, a finger or focus.
    pub fn active_border_color(mut self, color: Color) -> Self {
        self.colors.active_border = Some(color);
        self.rebuild();
        self
    }

    /// The labels' colour; the theme's `on_surface` otherwise.
    pub fn label_color(mut self, color: Color) -> Self {
        self.colors.label = Some(color);
        self.rebuild();
        self
    }

    /// Adds an option, in order.
    pub fn option(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    /// Whether the group can be chosen from. Disabled it is **inert** — no message, out
    /// of the tab order, announced as unavailable — and it still shows which option is
    /// chosen.
    ///
    /// It disables the **whole** group; a single unavailable option among live ones is
    /// not expressible yet. See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// **How much room it reserves for a finger** ([`TapTarget`]). Unset, the theme's
    /// answer, which is [`Padded`](TapTarget::Padded) — at least 48 pixels either way,
    /// whatever this control paints in the middle of it.
    ///
    /// It reaches every option, as the group's other answers do.
    pub fn tap_target(mut self, target: TapTarget) -> Self {
        self.tap_target = Some(target);
        self.rebuild();
        self
    }

    /// Rebuilds the options from the labels, so that the order of the builders does not
    /// change what comes out.
    fn rebuild(&mut self) {
        self.options = self
            .labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                Box::new(Radio {
                    label: label.clone(),
                    selected: index == self.selected,
                    size: self.size,
                    colors: self.colors,
                    enabled: self.enabled,
                    tap_target: self.tap_target,
                    visual_density: None,
                    splash_radius: None,
                    on_click: Some((self.on_select)(index)),
                }) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for RadioGroup<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            gap: self.gap,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.options
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::Interaction;
    use crate::theme::{MIN_TAP_TARGET, SHRUNK_TAP_TARGET};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pick(usize),
    }

    /// **A 20-pixel ring is not something a finger can be asked to hit** (milestone 442).
    ///
    /// The reference lays a radio out inside a 48-pixel square whatever it paints in the
    /// middle (`radio.dart:734`). Here the target is the whole option, words included,
    /// because a `Radio` carries its label where the reference's radio does not.
    #[test]
    fn an_option_reserves_room_for_a_finger() {
        // A phone's theme: a radio follows the theme's density, compact on a desktop.
        let theme = Theme::dark().with_platform(frus_core::TargetPlatform::Android);
        let option = |target: Option<TapTarget>| Radio::<Msg> {
            label: "Daily".into(),
            selected: false,
            size: 18.0,
            colors: RadioColors::default(),
            enabled: true,
            tap_target: target,
            visual_density: None,
            splash_radius: None,
            on_click: None,
        };
        let height =
            |option: &Radio<Msg>, theme: &Theme| match Widget::<Msg>::style_themed(option, theme)
                .height
            {
                Dimension::Length(h) => h,
                other => panic!("{other:?}"),
            };
        assert_eq!(height(&option(None), &theme), MIN_TAP_TARGET);
        assert_eq!(
            height(&option(Some(TapTarget::ShrinkWrap)), &theme),
            SHRUNK_TAP_TARGET
        );

        // The group hands its answer to every option, whenever it is said.
        let group = RadioGroup::new(0, Msg::Pick)
            .option("Daily")
            .tap_target(TapTarget::ShrinkWrap);
        for child in Widget::<Msg>::children(&group) {
            assert_eq!(
                match child.style_themed(&theme).height {
                    Dimension::Length(h) => h,
                    other => panic!("{other:?}"),
                },
                SHRUNK_TAP_TARGET
            );
        }
    }

    /// And the label is centred down that room. It used to be drawn at the top of the
    /// bounds, which was the same sentence while the bounds *were* the line.
    #[test]
    fn the_label_is_centred_in_the_room() {
        let theme = Theme::dark();
        let option = Radio::<Msg> {
            label: "Daily".into(),
            selected: false,
            size: 18.0,
            colors: RadioColors::default(),
            enabled: true,
            tap_target: None,
            visual_density: None,
            splash_radius: None,
            on_click: None,
        };
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &option,
            Rect::new(0.0, 0.0, 200.0, MIN_TAP_TARGET),
            Status::default(),
            &theme,
            &mut scene,
        );
        let at = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { position, .. } => Some(*position),
                _ => None,
            })
            .expect("an option paints its label");
        let line = option.label_style().line_height();
        assert!(
            (at.y - (MIN_TAP_TARGET - line) * 0.5).abs() < 0.01,
            "the label sits in the middle: {at:?}"
        );
    }

    /// An unselected ring, state by state — the same rule as a checkbox's box, and for
    /// the same reason: it is the mark, not a container's edge. See
    /// `checkbox::tests::an_unticked_box_is_a_mark_not_a_container_edge`.
    #[test]
    fn an_unselected_ring_is_a_mark_not_a_container_edge() {
        let theme = Theme::dark();
        let ring = |selected: bool, interaction, focused| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &Radio {
                    label: "Daily".into(),
                    selected,
                    size: 18.0,
                    colors: RadioColors::default(),
                    enabled: true,
                    tap_target: None,
                    visual_density: None,
                    splash_radius: None,
                    on_click: Some(Msg::Pick(0)),
                },
                Rect::new(0.0, 0.0, 120.0, 20.0),
                Status {
                    opacity: 1.0,
                    interaction,
                    focused,
                    // Settled: the dot grown or gone.
                    value: if selected { 1.0 } else { 0.0 },
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    // The ring is the stroked circle; a halo has no stroke.
                    frus_core::Primitive::Rect {
                        border_color,
                        border_width,
                        ..
                    } if *border_width > 0.0 => Some(*border_color),
                    _ => None,
                })
                .expect("the ring")
        };
        assert_eq!(
            ring(false, Interaction::None, false),
            theme.scheme.on_surface_variant,
            "at rest"
        );
        assert_eq!(
            ring(false, Interaction::Hovered, false),
            theme.scheme.on_surface,
            "hovered"
        );
        assert_eq!(
            ring(false, Interaction::None, true),
            theme.scheme.on_surface,
            "focused"
        );
        // The chosen one is the accent in every state, which is what the reference does.
        for interaction in [
            Interaction::None,
            Interaction::Hovered,
            Interaction::Pressed,
        ] {
            assert_eq!(ring(true, interaction, false), theme.primary, "selected");
        }
        assert_ne!(
            ring(false, Interaction::None, false),
            theme.scheme.outline,
            "a ring is a mark, not a container's edge"
        );
    }

    /// **The reference's ring and dot** (`radio.dart:849`): a ring of radius 8 drawn with
    /// a 2 px stroke centred on it, in the middle of the square, and a dot of 4.5 grown
    /// with the animation.
    #[test]
    fn the_ring_and_the_dot_are_the_reference_s() {
        let theme = Theme::dark().with_platform(frus_core::TargetPlatform::Android);
        let painted = |value: f32| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &Radio::<Msg>::new(true),
                Rect::new(0.0, 0.0, 48.0, 48.0),
                Status {
                    opacity: 1.0,
                    value,
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Rect {
                        rect, border_width, ..
                    } => Some((*rect, *border_width)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let settled = painted(1.0);
        assert_eq!(
            settled[0],
            (Rect::new(15.0, 15.0, 18.0, 18.0), 2.0),
            "the ring"
        );
        assert_eq!(
            settled[1].0,
            Rect::new(19.5, 19.5, 9.0, 9.0),
            "the dot, 4.5 round"
        );
        let half = painted(0.5);
        assert_eq!(half[1].0, Rect::new(21.75, 21.75, 4.5, 4.5), "half grown");
        assert_eq!(painted(0.0).len(), 1, "no dot before it is chosen");
    }

    fn group(enabled_first: bool) -> RadioGroup<Msg> {
        // The same group built in both orders. `enabled` before the options and `enabled`
        // after them must come out identical — the reason the options are derived from
        // the labels rather than frozen as each one is added.
        if enabled_first {
            RadioGroup::new(1, Msg::Pick)
                .enabled(false)
                .option("Daily")
                .option("Weekly")
        } else {
            RadioGroup::new(1, Msg::Pick)
                .option("Daily")
                .option("Weekly")
                .enabled(false)
        }
    }

    #[test]
    fn picking_an_option_reports_its_index() {
        let live = RadioGroup::new(0, Msg::Pick)
            .option("Daily")
            .option("Weekly");
        let options = Widget::children(&live);
        assert_eq!(options.len(), 2);
        assert_eq!(options[1].on_click(), Some(Msg::Pick(1)));
    }

    #[test]
    fn a_disabled_group_disables_every_option_whichever_order_it_was_built_in() {
        for first in [true, false] {
            let dead = group(first);
            let options = Widget::children(&dead);
            assert_eq!(options.len(), 2, "built in order {first}");
            for (i, option) in options.iter().enumerate() {
                assert_eq!(option.on_click(), None, "option {i} still answers");
                assert!(!option.focusable(), "option {i} is still in the tab order");
                let semantics = option.semantics().expect("still announced");
                assert!(semantics.disabled, "option {i} does not say it is disabled");
            }
            // And the chosen one is still legible to a reader who cannot change it.
            assert_eq!(
                options[1].semantics().unwrap().toggled,
                frus_core::Toggled::True
            );
        }
    }

    /// The trap the rebuild exists for: with the options frozen as they were added,
    /// `.enabled(false)` at the end of the chain would have reached none of them, and the
    /// group would have looked disabled to a reader of the call site and answered every
    /// tap.
    #[test]
    fn the_builder_order_does_not_change_what_comes_out() {
        let before = Widget::children(&group(true))
            .iter()
            .map(|o| o.focusable())
            .collect::<Vec<_>>();
        let after = Widget::children(&group(false))
            .iter()
            .map(|o| o.focusable())
            .collect::<Vec<_>>();
        assert_eq!(before, after);
        assert_eq!(after, vec![false, false]);
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::interaction::Interaction;
    use crate::widget::Widget;
    use frus_core::Primitive;

    const BRAND: Color = Color::rgb(0.0, 0.6, 0.3);
    const MARK: Color = Color::rgb(0.9, 0.9, 0.2);

    /// The ring's colour, then the dot's if the option is the chosen one.
    fn painted(
        group: &RadioGroup<usize>,
        index: usize,
        status: Status,
        theme: &Theme,
    ) -> Vec<Color> {
        let option = &Widget::<usize>::children(group)[index];
        let mut scene = Scene::new();
        // Settled: these groups choose their first option.
        let status = Status {
            value: if index == 0 { 1.0 } else { 0.0 },
            ..status
        };
        option.paint(Rect::new(0.0, 0.0, 200.0, 48.0), status, theme, &mut scene);
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    border_width,
                    border_color,
                    color,
                    ..
                } => Some(if *border_width > 0.0 {
                    *border_color
                } else {
                    *color
                }),
                Primitive::Text { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
    }

    fn group() -> RadioGroup<usize> {
        RadioGroup::new(0, |i| i).option("One").option("Two")
    }

    fn rest() -> Status {
        Status {
            opacity: 1.0,
            ..Default::default()
        }
    }

    /// Nothing said: what it always painted.
    #[test]
    fn the_defaults_are_what_they_were() {
        let theme = Theme::default();
        let chosen = painted(&group(), 0, rest(), &theme);
        assert_eq!(chosen[0], theme.primary, "the ring of the chosen option");
        assert_eq!(chosen[1], theme.primary, "and its dot");
        let other = painted(&group(), 1, rest(), &theme);
        assert_eq!(other[0], theme.scheme.on_surface_variant);
    }

    /// The chosen ring and dot take one colour; the labels take another.
    #[test]
    fn the_chosen_option_takes_its_colour() {
        let theme = Theme::default();
        let g = group().selected_color(BRAND).label_color(MARK);
        let chosen = painted(&g, 0, rest(), &theme);
        assert_eq!((chosen[0], chosen[1]), (BRAND, BRAND));
        assert_eq!(*chosen.last().expect("the label"), MARK);
    }

    /// One ring colour covers both states, as the checkbox's outline does.
    #[test]
    fn one_ring_colour_covers_both_states() {
        let theme = Theme::default();
        let hovered = Status {
            opacity: 1.0,
            interaction: Interaction::Hovered,
            ..Default::default()
        };
        let g = group().border_color(BRAND);
        assert_eq!(painted(&g, 1, rest(), &theme)[0], BRAND);
        assert_eq!(painted(&g, 1, hovered, &theme)[0], BRAND);

        let both = group().border_color(BRAND).active_border_color(MARK);
        assert_eq!(painted(&both, 1, hovered, &theme)[0], MARK);
    }

    /// A builder called after the options still reaches every one of them.
    #[test]
    fn a_colour_set_last_still_reaches_the_options() {
        let theme = Theme::default();
        let g = RadioGroup::new(0, |i: usize| i)
            .option("One")
            .selected_color(BRAND);
        assert_eq!(painted(&g, 0, rest(), &theme)[0], BRAND);
    }

    /// The theme answers when the instance does not, and loses when it does.
    #[test]
    fn the_theme_answers_and_the_instance_overrules_it() {
        let mut theme = Theme::default();
        theme.widgets.radio.selected_color = Some(MARK);
        assert_eq!(painted(&group(), 0, rest(), &theme)[0], MARK);
        assert_eq!(
            painted(&group().selected_color(BRAND), 0, rest(), &theme)[0],
            BRAND
        );
    }
}

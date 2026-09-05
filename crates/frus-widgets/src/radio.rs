//! [`RadioGroup`]: a group of radio buttons, with one option selected.

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::disabled_content;
use crate::interaction::{Interaction, Status};
use crate::theme::{TapTarget, Theme};
use crate::widget::Widget;

const DOT: f32 = 20.0;
const GAP: f32 = 10.0;
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
/// Radio::new(true).label("Every day").on_select(Msg::Pick);
/// ```
///
/// A radio does **not** know how to turn itself off: the reference's takes a value and a
/// group value and reports the value it stands for, and turning one off is choosing
/// another. So this reports one message when pressed and says nothing about what the
/// answer becomes.
pub struct Radio<Msg> {
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
    pub fn on_select(mut self, message: Msg) -> Self {
        self.on_click = Some(message);
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

    /// The room this option reserves, resolved as `group ?? theme ?? framework`
    /// (`radio.dart:734`).
    fn reserved(&self, theme: &Theme) -> f32 {
        self.tap_target
            .or(theme.widgets.radio.tap_target)
            .unwrap_or(theme.tap_target)
            .min_side()
    }
}

impl<Msg: Clone> Widget<Msg> for Radio<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    /// **A 20-pixel ring is not something a finger can be asked to hit** — the reference
    /// lays a radio out inside a 48-pixel square whatever it paints in the middle
    /// (`radio.dart:734`). As with a checkbox, the target is the whole control: an option
    /// with words beside it is already wider than the minimum.
    fn style_themed(&self, theme: &Theme) -> Style {
        let least = self.reserved(theme);
        let style = self.label_style();
        let line = style.line_height().max(DOT);
        let label_w = frus_text::measure_resolved(&self.label, &style).width;
        Style {
            width: Dimension::Length((DOT + GAP + label_w).max(least).ceil()),
            height: Dimension::Length(line.max(least).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let c = self.colors;
        let cy = bounds.y + (bounds.height - DOT) * 0.5;
        let outer = Rect::new(bounds.x, cy, DOT, DOT);
        let label_style = self.label_style();
        // A radio has no container: the ring and the dot *are* the control, so both take
        // the content opacity — the reference disables its fill at 38 % whether the option
        // is the chosen one or not.
        //
        // An **unselected** ring is not `outline` either, for the same reason a checkbox's
        // box is not: it is the mark itself, so it takes an *on* colour. The reference
        // resolves it `on_surface_variant` at rest and the full `on_surface` under a
        // finger, a pointer or focus. Milestone 332.
        //
        // A caller who names one ring colour means the ring, so the pointer state falls
        // back to the resting override before it falls back to the scheme.
        let resting = c.border.or(theme.widgets.radio.border_color);
        let chosen = c
            .selected
            .or(theme.widgets.radio.selected_color)
            .unwrap_or(theme.primary);
        let (ring, dot, label) = if self.enabled {
            (
                if self.selected {
                    chosen
                } else if status.interaction != Interaction::None || status.focused {
                    c.active_border
                        .or(theme.widgets.radio.active_border_color)
                        .or(resting)
                        .unwrap_or(theme.scheme.on_surface)
                } else {
                    resting.unwrap_or(theme.scheme.on_surface_variant)
                },
                chosen,
                c.label
                    .or(theme.widgets.radio.label_color)
                    .unwrap_or(theme.on_surface),
            )
        } else {
            let dead = disabled_content(theme);
            (dead, dead, dead)
        };
        scene.draw_rect(outer, theme.surface.fade(o), DOT * 0.5, 2.0, ring.fade(o));
        if self.selected {
            let inner = DOT * 0.5;
            let pad = (DOT - inner) * 0.5;
            scene.draw_rect(
                Rect::new(outer.x + pad, outer.y + pad, inner, inner),
                dot.fade(o),
                inner * 0.5,
                0.0,
                Color::TRANSPARENT,
            );
        }
        // Centred down the ring's own height. It used to be drawn at the top of the
        // bounds, which was the same sentence while the bounds *were* the line.
        scene.text(
            Point::new(
                bounds.x + DOT + GAP,
                bounds.y + (bounds.height - label_style.line_height()) * 0.5,
            ),
            self.label.clone(),
            &label_style,
            label.fade(o),
        );
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
pub struct RadioGroup<Msg> {
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
        let theme = Theme::dark();
        let option = |target: Option<TapTarget>| Radio::<Msg> {
            label: "Daily".into(),
            selected: false,
            size: 18.0,
            colors: RadioColors::default(),
            enabled: true,
            tap_target: target,
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
                    on_click: Some(Msg::Pick(0)),
                },
                Rect::new(0.0, 0.0, 120.0, 20.0),
                Status {
                    opacity: 1.0,
                    interaction,
                    focused,
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { border_color, .. } => Some(*border_color),
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
        option.paint(Rect::new(0.0, 0.0, 200.0, DOT), status, theme, &mut scene);
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

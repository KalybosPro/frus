//! [`Checkbox`]: a **controlled** checkbox, its state coming from the application.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_content, disabled_mark};
use crate::interaction::{Interaction, Status};
use crate::theme::Theme;
use crate::widget::Widget;

const BOX: f32 = 20.0;
const GAP: f32 = 10.0;

/// A checkbox, with an optional label.
pub struct Checkbox<Msg> {
    checked: bool,
    label: Option<String>,
    size: f32,
    enabled: bool,
    fill_color: Option<Color>,
    check_color: Option<Color>,
    border_color: Option<Color>,
    active_border_color: Option<Color>,
    radius: Option<f32>,
    label_color: Option<Color>,
    on_toggle: Option<Box<dyn Fn(bool) -> Msg>>,
}

impl<Msg> Checkbox<Msg> {
    /// Creates a checkbox whose checked state is supplied.
    pub fn new(checked: bool) -> Self {
        Self {
            checked,
            label: None,
            size: 18.0,
            enabled: true,
            fill_color: None,
            check_color: None,
            border_color: None,
            active_border_color: None,
            radius: None,
            label_color: None,
            on_toggle: None,
        }
    }

    /// The box's fill when it is **ticked**; the theme's `primary` otherwise.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    /// The tick drawn on that fill; the theme's `on_primary` otherwise.
    pub fn check_color(mut self, color: Color) -> Self {
        self.check_color = Some(color);
        self
    }

    /// The outline when the box is **not** ticked and at rest.
    ///
    /// Set on its own it also becomes the colour under a pointer or focus, unless
    /// [`active_border_color`](Checkbox::active_border_color) says otherwise: a caller
    /// who names one outline colour means the outline, not half of it.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// The outline under a pointer, a finger or focus. The reference resolves this side
    /// per state, and an outline that did not answer at all would look inert.
    pub fn active_border_color(mut self, color: Color) -> Self {
        self.active_border_color = Some(color);
        self
    }

    /// The box's corner radius.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    /// The label's colour; the theme's `on_surface` otherwise.
    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = Some(color);
        self
    }

    /// The corner radius actually used.
    fn corner(&self, theme: &Theme) -> f32 {
        self.radius.or(theme.widgets.checkbox.radius).unwrap_or(5.0)
    }

    /// Adds a label on the right.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Whether the box can be ticked. Disabled it is **inert** — no message, out of the
    /// tab order, announced as unavailable — and it still shows whether it is ticked,
    /// because read-only is not invisible.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A closure producing a message from the new state, checked or not.
    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    fn label_width(&self) -> f32 {
        match &self.label {
            Some(text) => GAP + frus_text::measure(text, self.size).width,
            None => 0.0,
        }
    }
}

impl<Msg> Widget<Msg> for Checkbox<Msg> {
    fn style(&self) -> Style {
        let line = frus_text::line_height(self.size).max(BOX);
        Style {
            width: Dimension::Length((BOX + self.label_width()).ceil()),
            height: Dimension::Length(line.ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let box_y = bounds.y + (bounds.height - BOX) * 0.5;
        let box_rect = Rect::new(bounds.x, box_y, BOX, BOX);

        if self.checked {
            // Disabled and ticked: the box flattens to `on_surface` at 38 % and the tick
            // punches through in `surface`. A translucent tick on a translucent box would
            // land within a few percent of it and vanish.
            let (fill, tick) = if self.enabled {
                (
                    self.fill_color
                        .or(theme.widgets.checkbox.fill_color)
                        .unwrap_or(theme.primary),
                    self.check_color
                        .or(theme.widgets.checkbox.check_color)
                        .unwrap_or(theme.on_primary),
                )
            } else {
                (disabled_content(theme), disabled_mark(theme))
            };
            scene.draw_rect(
                box_rect,
                fill.fade(o),
                self.corner(theme),
                0.0,
                Color::TRANSPARENT,
            );
            scene.text(
                Point::new(box_rect.x + 3.0, box_rect.y + 1.0),
                "✓".to_string(),
                self.size,
                tick.fade(o),
            );
        } else {
            // Unticked, the outline *is* the control — the mark rather than a container —
            // so it takes the content opacity, as the reference's does.
            //
            // And it is **not** `outline`. The reference resolves this side per state:
            // `on_surface_variant` at rest, the full `on_surface` under a finger, a
            // pointer or focus, and `on_surface` at 38 % when disabled. An unselected
            // checkbox is a mark, and a mark is drawn in an *on* colour; `outline` is for
            // the edge of a container, which this is not. Milestone 332.
            //
            // A caller who names one outline colour means the outline, so the pointer
            // state falls back to the resting override before it falls back to the
            // scheme -- otherwise a green checkbox would turn grey under a finger.
            let resting = self.border_color.or(theme.widgets.checkbox.border_color);
            let border = if !self.enabled {
                disabled_content(theme)
            } else if status.interaction != Interaction::None || status.focused {
                self.active_border_color
                    .or(theme.widgets.checkbox.active_border_color)
                    .or(resting)
                    .unwrap_or(theme.scheme.on_surface)
            } else {
                resting.unwrap_or(theme.scheme.on_surface_variant)
            };
            scene.draw_rect(
                box_rect,
                theme.surface.fade(o),
                self.corner(theme),
                2.0,
                border.fade(o),
            );
        }

        if let Some(label) = &self.label {
            let color = if self.enabled {
                self.label_color
                    .or(theme.widgets.checkbox.label_color)
                    .unwrap_or(theme.on_surface)
            } else {
                disabled_content(theme)
            };
            scene.text(
                Point::new(bounds.x + BOX + GAP, bounds.y),
                label.clone(),
                self.size,
                color.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        self.on_toggle.as_ref().map(|make| make(!self.checked))
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // Still ticked or not, still announced: a reader who cannot change the answer is
        // still owed it.
        let mut s = frus_core::Semantics::new(frus_core::Role::CheckBox).toggled(self.checked);
        s = if self.enabled {
            s.clickable()
        } else {
            s.disabled(true)
        };
        if let Some(label) = &self.label {
            s = s.label(label.clone());
        }
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Set(bool),
    }

    #[test]
    fn click_toggles() {
        let unchecked = Checkbox::new(false).on_toggle(Msg::Set);
        assert_eq!(Widget::on_click(&unchecked), Some(Msg::Set(true)));
        let checked = Checkbox::new(true).on_toggle(Msg::Set);
        assert_eq!(Widget::on_click(&checked), Some(Msg::Set(false)));
    }

    #[test]
    fn a_disabled_box_is_inert_but_still_says_whether_it_is_ticked() {
        let dead = Checkbox::new(true).on_toggle(Msg::Set).enabled(false);
        assert_eq!(Widget::on_click(&dead), None, "the press goes nowhere");
        assert!(!Widget::<Msg>::focusable(&dead), "out of the tab order");
        let semantics = Widget::<Msg>::semantics(&dead).expect("still announced");
        assert!(semantics.disabled, "and announced as unavailable");
        // The answer survives: read-only is not invisible.
        assert_eq!(semantics.toggled, frus_core::Toggled::True);
    }

    /// An unselected box's side, state by state. The reference resolves it as
    /// `on_surface_variant` at rest and the full `on_surface` under a finger, a pointer or
    /// focus — an unselected checkbox is a **mark**, so it takes an *on* colour. Ours was
    /// `outline`, the role for the edge of a container, since it was written.
    #[test]
    fn an_unticked_box_is_a_mark_not_a_container_edge() {
        let theme = Theme::dark();
        let side = |status: Status| {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &Checkbox::<Msg>::new(false),
                Rect::new(0.0, 0.0, 20.0, 20.0),
                status,
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
                .expect("the box")
        };
        let at = |interaction, focused| Status {
            opacity: 1.0,
            interaction,
            focused,
            ..Default::default()
        };
        assert_eq!(
            side(at(Interaction::None, false)),
            theme.scheme.on_surface_variant,
            "at rest"
        );
        for (name, status) in [
            ("hovered", at(Interaction::Hovered, false)),
            ("pressed", at(Interaction::Pressed, false)),
            ("focused", at(Interaction::None, true)),
        ] {
            assert_eq!(side(status), theme.scheme.on_surface, "{name}");
        }
        // And it is no longer the container-edge role, which is what it used to be.
        assert_ne!(
            side(at(Interaction::None, false)),
            theme.scheme.outline,
            "an unselected box is a mark, not a container's edge"
        );
    }

    #[test]
    fn a_disabled_tick_does_not_disappear_into_its_box() {
        // Both are drawn from `on_surface`; if the tick took the content opacity too it
        // would land within a few percent of the 38 % box behind it and vanish. It punches
        // through in `surface` instead, and this is the assertion that says so.
        for theme in [Theme::dark(), Theme::light()] {
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &Checkbox::<Msg>::new(true).enabled(false),
                Rect::new(0.0, 0.0, 20.0, 20.0),
                Status {
                    opacity: 1.0,
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            let box_fill = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { color, .. } => Some(*color),
                    _ => None,
                })
                .expect("the box");
            let tick = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { color, .. } => Some(*color),
                    _ => None,
                })
                .expect("the tick");
            let against = |c: frus_core::Color| {
                (c.r - theme.scheme.surface.r).abs()
                    + (c.g - theme.scheme.surface.g).abs()
                    + (c.b - theme.scheme.surface.b).abs()
            };
            // The tick is the surface punching through, so it is *at* the surface while
            // the fill it sits on is a measurable way off it. Since milestone 329 the
            // disabled tokens resolve to opaque colours, so the two are told apart by
            // where they sit rather than by an alpha.
            assert!(
                against(tick) < 0.01,
                "the tick is the surface punching through: {tick:?}"
            );
            assert!(
                against(box_fill) > 0.1,
                "and the box it is inside is not: {box_fill:?}"
            );
        }
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::widget::Widget;
    use frus_core::Primitive;

    const BRAND: Color = Color::rgb(0.0, 0.6, 0.3);
    const MARK: Color = Color::rgb(0.9, 0.9, 0.2);

    /// The box: its fill, its border colour and its radius.
    fn box_of(cb: &Checkbox<()>, status: Status, theme: &Theme) -> (Color, Color, f32) {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            cb,
            Rect::new(0.0, 0.0, 200.0, BOX),
            status,
            theme,
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect {
                    color,
                    border_color,
                    radius,
                    ..
                } => Some((*color, *border_color, radius.top_left)),
                _ => None,
            })
            .expect("the box is painted")
    }

    /// The tick's colour, or the label's — whichever text came out.
    fn text_color(cb: &Checkbox<()>, theme: &Theme) -> Vec<Color> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            cb,
            Rect::new(0.0, 0.0, 200.0, BOX),
            Status {
                opacity: 1.0,
                ..Default::default()
            },
            theme,
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
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
        let (fill, _, radius) = box_of(&Checkbox::<()>::new(true), rest(), &theme);
        assert_eq!(fill, theme.primary);
        assert_eq!(radius, 5.0);
        let (_, border, _) = box_of(&Checkbox::<()>::new(false), rest(), &theme);
        assert_eq!(border, theme.scheme.on_surface_variant);
    }

    /// A ticked box takes its fill and its tick; the corner takes its radius.
    #[test]
    fn a_ticked_box_takes_its_colours() {
        let theme = Theme::default();
        let cb = Checkbox::<()>::new(true)
            .fill_color(BRAND)
            .check_color(MARK)
            .radius(2.0);
        let (fill, _, radius) = box_of(&cb, rest(), &theme);
        assert_eq!((fill, radius), (BRAND, 2.0));
        assert_eq!(text_color(&cb, &theme), vec![MARK]);
    }

    /// A caller who names one outline colour means the outline: the pointer state falls
    /// back to it before it falls back to the scheme. Otherwise a green checkbox would
    /// turn grey the moment a finger came near it.
    #[test]
    fn one_outline_colour_covers_both_states() {
        let theme = Theme::default();
        let hovered = Status {
            opacity: 1.0,
            interaction: Interaction::Hovered,
            ..Default::default()
        };
        let cb = Checkbox::<()>::new(false).border_color(BRAND);
        assert_eq!(box_of(&cb, rest(), &theme).1, BRAND);
        assert_eq!(box_of(&cb, hovered, &theme).1, BRAND, "and under a finger");

        let both = Checkbox::<()>::new(false)
            .border_color(BRAND)
            .active_border_color(MARK);
        assert_eq!(box_of(&both, rest(), &theme).1, BRAND);
        assert_eq!(box_of(&both, hovered, &theme).1, MARK, "unless it is named");
    }

    /// The theme answers when the instance does not, and loses when it does.
    #[test]
    fn the_theme_answers_and_the_instance_overrules_it() {
        let mut theme = Theme::default();
        theme.widgets.checkbox.fill_color = Some(MARK);
        assert_eq!(box_of(&Checkbox::<()>::new(true), rest(), &theme).0, MARK);
        assert_eq!(
            box_of(&Checkbox::<()>::new(true).fill_color(BRAND), rest(), &theme).0,
            BRAND
        );
    }

    /// The label answers too.
    #[test]
    fn the_label_takes_its_colour() {
        let theme = Theme::default();
        let cb = Checkbox::<()>::new(false).label("Ready").label_color(BRAND);
        assert_eq!(text_color(&cb, &theme), vec![BRAND]);
    }
}

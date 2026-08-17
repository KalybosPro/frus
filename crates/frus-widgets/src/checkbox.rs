//! [`Checkbox`]: a **controlled** checkbox, its state coming from the application.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_content, disabled_mark};
use crate::interaction::Status;
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
            on_toggle: None,
        }
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
                (theme.primary, theme.on_primary)
            } else {
                (disabled_content(theme), disabled_mark(theme))
            };
            scene.draw_rect(box_rect, fill.fade(o), 5.0, 0.0, Color::TRANSPARENT);
            scene.text(
                Point::new(box_rect.x + 3.0, box_rect.y + 1.0),
                "✓".to_string(),
                self.size,
                tick.fade(o),
            );
        } else {
            // Unticked, the outline *is* the control — the mark rather than a container —
            // so it takes the content opacity, as the reference's does.
            let border = if self.enabled {
                theme.border
            } else {
                disabled_content(theme)
            };
            scene.draw_rect(box_rect, theme.surface.fade(o), 5.0, 2.0, border.fade(o));
        }

        if let Some(label) = &self.label {
            let color = if self.enabled {
                theme.on_surface
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

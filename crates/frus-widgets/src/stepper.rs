//! [`Stepper`]: a **controlled** numeric field with **−/value/+** buttons.

use frus_core::{Rect, Scene};
use frus_layout::{Align, FlexDirection, Style};

use crate::disabled::disabled_content;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

/// An incremental numeric picker.
pub struct Stepper<Msg> {
    value: i32,
    min: i32,
    max: i32,
    step: i32,
    enabled: bool,
    on_change: Box<dyn Fn(i32) -> Msg>,
    /// `[−, value, +]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Stepper<Msg> {
    /// Creates a picker: the current value plus a message for each new value.
    pub fn new(value: i32, on_change: impl Fn(i32) -> Msg + 'static) -> Self {
        let mut stepper = Self {
            value,
            min: i32::MIN,
            max: i32::MAX,
            step: 1,
            enabled: true,
            on_change: Box::new(on_change),
            children: Vec::new(),
        };
        stepper.rebuild();
        stepper
    }

    /// Clamps the value to `[min, max]`.
    pub fn range(mut self, min: i32, max: i32) -> Self {
        self.min = min;
        self.max = max;
        self.rebuild();
        self
    }

    /// Whether the value can be changed. Disabled, **both** buttons are inert and the
    /// value greys with them - and it is still readable, because read-only is not
    /// invisible.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Sets the increment step, at least 1.
    pub fn step(mut self, step: i32) -> Self {
        self.step = step.max(1);
        self.rebuild();
        self
    }

    /// (Re)builds the three children from the value and the bounds.
    fn rebuild(&mut self) {
        let dec = (self.value - self.step).clamp(self.min, self.max);
        let inc = (self.value + self.step).clamp(self.min, self.max);
        // **A button at its bound is disabled**, not merely clamped. Until milestone 324
        // both stayed live at the ends of the range and emitted the value the picker was
        // already showing: a control that looks pressable, is pressable, and does nothing.
        // Now the end of the range is visible before it is reached.
        let can_dec = self.enabled && self.value > self.min;
        let can_inc = self.enabled && self.value < self.max;

        self.children = vec![
            Box::new(
                crate::IconButton::glyph("−")
                    .label("Less")
                    .variant(crate::IconButtonVariant::Outlined)
                    .icon_size(20.0)
                    .enabled(can_dec)
                    .on_press((self.on_change)(dec)),
            ),
            // The value's colour needs the **ambient** theme, and a stepper is assembled
            // before it has one. That is exactly what milestone 319's `ThemeBuilder` was
            // built for, and this is its second consumer: reaching for `Theme::default()`
            // here would paint a dark-theme grey into a light-theme application.
            Box::new({
                let (text, enabled) = (self.value.to_string(), self.enabled);
                crate::ThemeBuilder::new(move |theme| {
                    let value = Text::new(text).size(18.0);
                    if enabled {
                        value
                    } else {
                        value.color(disabled_content(theme))
                    }
                })
            }),
            Box::new(
                crate::IconButton::glyph("+")
                    .label("More")
                    .variant(crate::IconButtonVariant::Outlined)
                    .icon_size(20.0)
                    .enabled(can_inc)
                    .on_press((self.on_change)(inc)),
            ),
        ];
    }
}

impl<Msg: Clone> Widget<Msg> for Stepper<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            gap: 12.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // The picker said nothing to a reader before this; its value is the whole of what
        // it carries.
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Slider)
            .value(self.value.to_string())
            .range(self.min as f32, self.value as f32, self.max as f32);
        Some(if self.enabled {
            semantics
        } else {
            semantics.disabled(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Set(i32),
    }

    #[test]
    fn buttons_emit_stepped_values() {
        let stepper = Stepper::new(5, Msg::Set).range(0, 10).step(2);
        let children = Widget::<Msg>::children(&stepper);
        assert_eq!(children[0].on_click(), Some(Msg::Set(3))); // −
        assert_eq!(children[2].on_click(), Some(Msg::Set(7))); // +
    }

    #[test]
    fn values_are_clamped_to_range() {
        // A step that would overshoot lands on the bound. This used to be asserted *at*
        // the bound, where the button emitted the value already showing; milestone 324
        // disables it there instead, so the clamp is checked one step short of the top —
        // which is the only place it can still be observed.
        let near_max = Stepper::new(9, Msg::Set).range(0, 10).step(3);
        let children = Widget::<Msg>::children(&near_max);
        assert_eq!(children[2].on_click(), Some(Msg::Set(10))); // + clamped to max
        assert_eq!(children[0].on_click(), Some(Msg::Set(6))); // −
    }

    #[test]
    fn a_disabled_picker_is_inert_but_still_readable() {
        let dead = Stepper::new(5, Msg::Set).range(0, 10).enabled(false);
        let children = Widget::<Msg>::children(&dead);
        assert_eq!(children[0].on_click(), None, "minus still answers");
        assert_eq!(children[2].on_click(), None, "plus still answers");
        assert!(!children[0].focusable(), "minus is still in the tab order");
        assert!(!children[2].focusable(), "plus is still in the tab order");
        let semantics = Widget::<Msg>::semantics(&dead).expect("still announced");
        assert!(semantics.disabled);
        assert_eq!(semantics.value.as_deref(), Some("5"), "the value survives");
    }

    /// A button at the end of its range used to stay live and emit the value already
    /// shown - pressable, and doing nothing. The bound is visible now.
    #[test]
    fn a_button_at_its_bound_is_disabled_rather_than_merely_clamped() {
        let at_max = Stepper::new(10, Msg::Set).range(0, 10);
        let kids = Widget::<Msg>::children(&at_max);
        assert_eq!(kids[2].on_click(), None, "plus at the top does nothing");
        assert!(!kids[2].focusable(), "and Tab does not stop there");
        assert_eq!(kids[0].on_click(), Some(Msg::Set(9)), "minus still works");

        let at_min = Stepper::new(0, Msg::Set).range(0, 10);
        let kids = Widget::<Msg>::children(&at_min);
        assert_eq!(kids[0].on_click(), None, "minus at the bottom does nothing");
        assert_eq!(kids[2].on_click(), Some(Msg::Set(1)), "plus still works");
    }

    /// The value's colour comes from the **ambient** theme, through milestone 319's
    /// `ThemeBuilder`. Building it against `Theme::default()` at assembly time would paint
    /// a dark-theme grey into a light-theme application, and nothing in a dark-themed
    /// golden would ever show it.
    #[test]
    fn the_disabled_value_takes_the_ambient_theme_not_a_default_one() {
        let colour_in = |theme: &Theme, enabled: bool| {
            let stepper = Stepper::new(5, Msg::Set).range(0, 10).enabled(enabled);
            let value = &Widget::<Msg>::children(&stepper)[1];
            // What the layout pass does on the way down, before anything reads children.
            value.build_themed(theme);
            let mut scene = Scene::new();
            value.children()[0].paint(
                Rect::new(0.0, 0.0, 40.0, 24.0),
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
                .find_map(|p| match p {
                    frus_core::Primitive::Text { color, .. } => Some(*color),
                    _ => None,
                })
                .expect("the value is text")
        };
        for theme in [Theme::dark(), Theme::light()] {
            assert_eq!(colour_in(&theme, false), disabled_content(&theme));
            assert_ne!(
                colour_in(&theme, true),
                disabled_content(&theme),
                "a live value is not greyed"
            );
        }
        // And the two themes disagree, which is the whole point of deferring it.
        assert_ne!(
            colour_in(&Theme::dark(), false),
            colour_in(&Theme::light(), false)
        );
    }
}

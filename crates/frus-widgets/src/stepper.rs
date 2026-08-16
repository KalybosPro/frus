//! [`Stepper`]: a **controlled** numeric field with **−/value/+** buttons.

use frus_core::{Rect, Scene};
use frus_layout::{Align, FlexDirection, Style};

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
        self.children = vec![
            Box::new(
                crate::IconButton::glyph("−")
                    .label("Less")
                    .variant(crate::IconButtonVariant::Outlined)
                    .icon_size(20.0)
                    .on_press((self.on_change)(dec)),
            ),
            Box::new(Text::new(self.value.to_string()).size(18.0)),
            Box::new(
                crate::IconButton::glyph("+")
                    .label("More")
                    .variant(crate::IconButtonVariant::Outlined)
                    .icon_size(20.0)
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
        let at_max = Stepper::new(10, Msg::Set).range(0, 10).step(3);
        let children = Widget::<Msg>::children(&at_max);
        assert_eq!(children[2].on_click(), Some(Msg::Set(10))); // + clamped to max
        assert_eq!(children[0].on_click(), Some(Msg::Set(7))); // −
    }
}

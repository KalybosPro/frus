//! [`Carousel`]: ‹ › arrows around a **current slide** the application supplies, so
//! it is controlled. Only one slide is realised at a time.

use frus_core::{Rect, Scene};
use frus_layout::{Align, FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::flex::Flex;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A navigation arrow, disabled when `message` is `None`.
fn arrow<Msg: Clone + 'static>(label: &str, message: Option<Msg>) -> Box<dyn Widget<Msg>> {
    let mut button = Button::new(label).variant(Variant::Outlined).size(16.0);
    if let Some(message) = message {
        button = button.on_press(message);
    }
    Box::new(button)
}

/// A controlled carousel.
pub struct Carousel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Carousel<Msg> {
    /// Creates the carousel: slide `index` of `count`, `on_change(i)` on the arrows,
    /// and the current slide's content — the app knows what to show for `index`.
    pub fn new(
        index: usize,
        count: usize,
        on_change: impl Fn(usize) -> Msg + 'static,
        slide: impl Widget<Msg> + 'static,
    ) -> Self {
        let prev = arrow("‹", (index > 0).then(|| on_change(index - 1)));
        let next = arrow("›", (index + 1 < count).then(|| on_change(index + 1)));
        Self {
            children: vec![prev, Box::new(Flex::column().flex(1.0).child(slide)), next],
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Carousel<Msg> {
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
    use crate::Text;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
    }

    #[test]
    fn arrows_bounded_and_emit_change() {
        // Slide 0 of 3: ‹ disabled, › goes to 1.
        let first = Carousel::new(0, 3, Msg::Go, Text::new("A"));
        let c = Widget::<Msg>::children(&first);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].on_click(), None); // ‹ disabled at the start
        assert_eq!(c[2].on_click(), Some(Msg::Go(1))); // › goes to slide 1

        // Slide 2 of 3: ‹ goes to 1, › disabled.
        let last = Carousel::new(2, 3, Msg::Go, Text::new("C"));
        let c = Widget::<Msg>::children(&last);
        assert_eq!(c[0].on_click(), Some(Msg::Go(1)));
        assert_eq!(c[2].on_click(), None);
    }
}

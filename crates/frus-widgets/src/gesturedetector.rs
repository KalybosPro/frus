//! [`GestureDetector`]: taps, double taps, long presses and secondary clicks on anything.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// **Answers gestures on its child** with the application's messages: a tap, a double tap,
/// a long press, a secondary click (the right mouse button).
///
/// It draws nothing and changes nothing about the layout. The child is what is seen, and
/// the detector hears the gestures that land inside it. A detector that wants a tap gives
/// the child none of its own look — no ink, no hover — which is what
/// [`InkWell`](crate::InkWell) and the buttons are for.
///
/// ```
/// use frus_widgets::{GestureDetector, Text};
///
/// #[derive(Clone)]
/// enum Msg { Open, Like, Menu }
///
/// let _photo: GestureDetector<Msg> = GestureDetector::new(Text::new("photo"))
///     .on_tap(Msg::Open)
///     .on_double_tap(Msg::Like)
///     .on_secondary_tap(Msg::Menu);
/// ```
///
/// **A tap waits when a double tap is possible.** With both set, a first tap is not a tap
/// until 300 ms have gone by without a second one, as on every platform: the second tap of a
/// double tap is not also two taps. With only a tap set, it answers at once.
pub struct GestureDetector<Msg = crate::callback::Callback> {
    children: Vec<Box<dyn Widget<Msg>>>,
    tap: Option<Msg>,
    double_tap: Option<Msg>,
    long_press: Option<Msg>,
    secondary_tap: Option<Msg>,
}

impl<Msg> GestureDetector<Msg> {
    /// Listens for gestures on `child`. Nothing is answered until a gesture is given a message.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            children: vec![Box::new(child)],
            tap: None,
            double_tap: None,
            long_press: None,
            secondary_tap: None,
        }
    }

    /// A **tap**: a press and a release in the same place, with a finger or the main mouse
    /// button.
    pub fn on_tap(mut self, message: impl Into<Msg>) -> Self {
        self.tap = Some(message.into());
        self
    }

    /// A **double tap**: two taps in quick succession, close together.
    pub fn on_double_tap(mut self, message: impl Into<Msg>) -> Self {
        self.double_tap = Some(message.into());
        self
    }

    /// A **long press**: a press held still for half a second. The release that follows is
    /// not also a tap.
    pub fn on_long_press(mut self, message: impl Into<Msg>) -> Self {
        self.long_press = Some(message.into());
        self
    }

    /// A **secondary tap**: a click of the secondary mouse button, usually the right one,
    /// which on a desktop opens a context menu.
    pub fn on_secondary_tap(mut self, message: impl Into<Msg>) -> Self {
        self.secondary_tap = Some(message.into());
        self
    }
}

impl<Msg: Clone> Widget<Msg> for GestureDetector<Msg> {
    fn style(&self) -> Style {
        // A pass-through, as a clip is: the box is its child's.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // It draws nothing: what it listens on is its child.
    }

    fn on_click(&self) -> Option<Msg> {
        self.tap.clone()
    }

    fn on_double_tap(&self) -> Option<Msg> {
        self.double_tap.clone()
    }

    fn on_long_press(&self) -> Option<Msg> {
        self.long_press.clone()
    }

    fn on_secondary_tap(&self) -> Option<Msg> {
        self.secondary_tap.clone()
    }

    fn debug_name(&self) -> &'static str {
        "GestureDetector"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Keyed, Runtime};
    use frus_core::{Point, Size};

    /// **Every gesture is the detector's to answer**, through any wrapper around it.
    #[test]
    fn a_wrapped_detector_still_answers() {
        let detector = GestureDetector::<u8>::new(Container::new())
            .on_tap(1)
            .on_double_tap(2)
            .on_long_press(3)
            .on_secondary_tap(4);
        let wrapped = Keyed::new(9u64, detector);
        assert_eq!(wrapped.on_click(), Some(1));
        assert_eq!(wrapped.on_double_tap(), Some(2));
        assert_eq!(wrapped.on_long_press(), Some(3));
        assert_eq!(wrapped.on_secondary_tap(), Some(4));
    }

    /// **A detector with no tap of its own is still hit** by a press, so that the shell can
    /// ask it for the double or secondary tap; and one with nothing at all is not.
    #[test]
    fn a_detector_without_a_tap_is_still_a_target() {
        let size = Size::new(100.0, 100.0);
        let rt = Runtime::default();
        let theme = Theme::default();
        let block = || Container::<u8>::new().width(100.0).height(100.0);
        let double = GestureDetector::new(block()).on_double_tap(2);
        let ui = build_ui(&double, size, &rt, &theme);
        assert!(ui.hit(Point::new(50.0, 50.0)).is_some());
        let secondary = GestureDetector::new(block()).on_secondary_tap(4);
        let ui = build_ui(&secondary, size, &rt, &theme);
        assert!(ui.hit(Point::new(50.0, 50.0)).is_some());
        let nothing = GestureDetector::new(block());
        let ui = build_ui(&nothing, size, &rt, &theme);
        assert!(ui.hit(Point::new(50.0, 50.0)).is_none());
    }
}

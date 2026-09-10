//! What the pointer is over — kept as a **place**, and asked of each frame.
//!
//! The hovered widget used to be a [`WidgetId`] set when a pointer moved and never
//! otherwise. That had two faults, and a device showed both at once: a "Back" tooltip stood
//! over the status bar of a screen nobody had pressed anything on (milestone 505).
//!
//! - **A finger hovered after it lifted.** Touch events went through the same path as the
//!   mouse, so the last place a finger touched stayed hovered for good. The reference has no
//!   hover for a finger at all; here a press needs the hover — a press only shows while the
//!   pointer is still over what it pressed — so a finger hovers **while it is down** and
//!   nothing once it lifts.
//! - **An id outlived its frame.** A widget id is a position in the tree, so when the tree
//!   changed under a still pointer — a tap that opens a new screen — the id named whatever
//!   now stood at that position: there, the back button, which the avatar tapped on the
//!   screen before had shared it with. The reference asks again after every frame which
//!   regions are under the mouse; so does this, from the point, against the frame just built.

use frus_widgets::{Point, Ui, WidgetId};

use crate::gesture::PointerKind;

/// Where the pointer that hovers is — `None` when there is none.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Hover {
    at: Option<Point>,
}

impl Hover {
    /// Follows one pointer event. A mouse is always somewhere once it has moved, pressed or
    /// not; a finger is somewhere only while it touches.
    pub(crate) fn event(&mut self, kind: PointerKind, position: Point, touch: bool) {
        self.at = match (kind, touch) {
            (PointerKind::Up | PointerKind::Cancel, true) => None,
            _ => Some(position),
        };
    }

    /// The mouse left the window: nothing in it is under the pointer any more.
    pub(crate) fn left(&mut self) {
        self.at = None;
    }

    /// The widget under the pointer **in this frame**.
    pub(crate) fn target<Msg: Clone>(&self, ui: &Ui<Msg>) -> Option<WidgetId> {
        self.at.and_then(|point| ui.hit(point))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::{build_ui, BackButton, Container, Flex, Runtime, Size, Theme};

    const BUTTON: Point = Point::new(24.0, 24.0);

    /// A frame with a button at the top-left, or pushed down by `offset`.
    fn frame(offset: f32) -> Ui<()> {
        let root = Flex::column()
            .width(200.0)
            .height(400.0)
            .child(Container::new().height(offset))
            .child(BackButton::new().on_press(()));
        build_ui(
            &root,
            Size::new(200.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        )
    }

    /// A finger hovers what it touches **from the moment it is down**, without having to
    /// move: a press only shows while its widget is also the hovered one, and a tap has no
    /// move in it.
    #[test]
    fn a_finger_hovers_what_it_touches_from_the_moment_it_is_down() {
        let ui = frame(0.0);
        let mut hover = Hover::default();
        hover.event(PointerKind::Down, BUTTON, true);
        assert!(hover.target(&ui).is_some(), "the button under the finger");
    }

    /// **A finger that has lifted hovers nothing** — which is what left a tooltip standing
    /// on the next screen.
    #[test]
    fn a_finger_that_has_lifted_hovers_nothing() {
        let ui = frame(0.0);
        for ending in [PointerKind::Up, PointerKind::Cancel] {
            let mut hover = Hover::default();
            hover.event(PointerKind::Down, BUTTON, true);
            hover.event(ending, BUTTON, true);
            assert_eq!(hover.target(&ui), None, "after {ending:?}");
        }
    }

    /// A mouse is still there after its button is released: releasing is not leaving.
    #[test]
    fn a_mouse_still_hovers_after_its_button_is_released() {
        let ui = frame(0.0);
        let mut hover = Hover::default();
        hover.event(PointerKind::Down, BUTTON, false);
        hover.event(PointerKind::Up, BUTTON, false);
        assert!(hover.target(&ui).is_some());
        hover.left();
        assert_eq!(hover.target(&ui), None, "until it leaves the window");
    }

    /// **The hover is asked of the frame, not remembered.** The same still pointer over two
    /// frames: in the first the button is under it, in the second it has moved away, and
    /// what stood at the button's position in the tree is not what is under the pointer.
    #[test]
    fn the_hover_is_asked_of_each_frame_not_remembered() {
        let mut hover = Hover::default();
        hover.event(PointerKind::Move, BUTTON, false);
        assert!(hover.target(&frame(0.0)).is_some(), "over the button");
        assert_eq!(
            hover.target(&frame(200.0)),
            None,
            "the button moved from under a still pointer"
        );
    }
}

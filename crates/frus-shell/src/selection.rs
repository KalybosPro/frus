//! The handles of a touch selection (milestone 511): which one a finger takes, and
//! where dragging it leaves the selection.
//!
//! Pure, like the recogniser in `gesture.rs`: the geometry comes from the field
//! ([`frus_widgets::Widget::selection_handles`]) and the text position under the finger
//! from its own hit test, so what is left here is the rule between the two.

use frus_widgets::{Edit, Point, SelectionHandle};

/// The side of the square a finger can take a handle in, centred on the handle. A
/// 22 px handle is smaller than a fingertip, so it is given the platform's minimum
/// touch target instead (the reference's `kMinInteractiveDimension`).
const TOUCH_TARGET: f32 = 48.0;

/// Which of the two handles a finger holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handle {
    Start,
    End,
}

impl Handle {
    /// Its place in the pair a field answers with — start, then end.
    pub fn index(self) -> usize {
        match self {
            Handle::Start => 0,
            Handle::End => 1,
        }
    }
}

/// The handle a press at `at` takes, if any: the one whose touch target holds it, and
/// the nearer of the two when a short selection puts both targets under the finger.
pub(crate) fn grab(handles: &[SelectionHandle; 2], at: Point) -> Option<Handle> {
    let reach = |handle: &SelectionHandle| {
        let centre_x = handle.rect.x + handle.rect.width * 0.5;
        let centre_y = handle.rect.y + handle.rect.height * 0.5;
        let (dx, dy) = (at.x - centre_x, at.y - centre_y);
        let half = TOUCH_TARGET * 0.5;
        (dx.abs() <= half && dy.abs() <= half).then_some(dx * dx + dy * dy)
    };
    match (reach(&handles[0]), reach(&handles[1])) {
        (Some(start), Some(end)) if end < start => Some(Handle::End),
        (Some(_), _) => Some(Handle::Start),
        (None, Some(_)) => Some(Handle::End),
        (None, None) => None,
    }
}

/// The selection once `handle` is dragged to the text position `to`: that end moves and
/// the other stays where it is. `None` when the move is refused — the two ends never
/// meet nor cross (`text_selection.dart:866`), so a selection dragged small stays a
/// selection rather than turning, under the finger, into a caret or into its mirror.
///
/// The end being dragged is the caret: it is what the field keeps in view when its
/// content is wider than its box.
pub(crate) fn drag(edit: Edit, handle: Handle, to: usize) -> Option<Edit> {
    let (start, end) = edit.selection_range()?;
    let (fixed, moving) = match handle {
        Handle::Start if to < end => (end, to),
        Handle::End if to > start => (start, to),
        _ => return None,
    };
    Some(Edit {
        cursor: moving,
        anchor: Some(fixed),
        composing: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::Rect;

    /// Two handles as a field places them: the start one hanging to the left of `from`,
    /// the end one to the right of `to`, both below a line whose middle is at y = 10.
    fn pair(from: f32, to: f32) -> [SelectionHandle; 2] {
        let handle = |x: f32, start: bool| SelectionHandle {
            rect: Rect::new(if start { x - 22.0 } else { x }, 20.0, 22.0, 22.0),
            line_center: Point::new(x, 10.0),
        };
        [handle(from, true), handle(to, false)]
    }

    fn selected(anchor: usize, cursor: usize) -> Edit {
        Edit {
            cursor,
            anchor: Some(anchor),
            composing: None,
        }
    }

    #[test]
    fn a_finger_takes_the_handle_it_lands_on() {
        let handles = pair(50.0, 150.0);
        // On each disc.
        assert_eq!(grab(&handles, Point::new(39.0, 31.0)), Some(Handle::Start));
        assert_eq!(grab(&handles, Point::new(161.0, 31.0)), Some(Handle::End));
        // Beside one, but inside its touch target: a fingertip is bigger than a handle.
        assert_eq!(
            grab(&handles, Point::new(39.0, 31.0 + 20.0)),
            Some(Handle::Start)
        );
        // Past it, and between the two: the text, not a handle.
        assert_eq!(grab(&handles, Point::new(39.0, 31.0 + 30.0)), None);
        assert_eq!(grab(&handles, Point::new(100.0, 31.0)), None);
    }

    #[test]
    fn when_both_are_in_reach_the_nearer_one_is_taken() {
        // A one-letter selection: the discs' centres 32 px apart, so between x = 97 and
        // x = 113 a finger is inside both touch targets.
        let handles = pair(100.0, 110.0);
        assert_eq!(grab(&handles, Point::new(100.0, 31.0)), Some(Handle::Start));
        assert_eq!(grab(&handles, Point::new(110.0, 31.0)), Some(Handle::End));
    }

    #[test]
    fn dragging_a_handle_moves_its_own_end_only() {
        // "hello world" with "world" selected, whichever way it was made.
        for edit in [selected(6, 11), selected(11, 6)] {
            let wider = drag(edit, Handle::Start, 3).expect("the start moves left");
            assert_eq!(wider.selection_range(), Some((3, 11)));
            let shorter = drag(edit, Handle::End, 8).expect("the end moves left");
            assert_eq!(shorter.selection_range(), Some((6, 8)));
        }
    }

    #[test]
    fn the_end_being_dragged_is_the_caret() {
        let edit = drag(selected(6, 11), Handle::Start, 3).expect("a move");
        assert_eq!((edit.cursor, edit.anchor), (3, Some(11)));
        let edit = drag(selected(6, 11), Handle::End, 9).expect("a move");
        assert_eq!((edit.cursor, edit.anchor), (9, Some(6)));
    }

    #[test]
    fn the_ends_never_meet_nor_cross() {
        let edit = selected(6, 11);
        assert_eq!(drag(edit, Handle::Start, 11), None, "onto the other end");
        assert_eq!(drag(edit, Handle::Start, 13), None, "past it");
        assert_eq!(drag(edit, Handle::End, 6), None);
        assert_eq!(drag(edit, Handle::End, 2), None);
        // One short of the other end is still a selection.
        assert_eq!(
            drag(edit, Handle::Start, 10).and_then(|e| e.selection_range()),
            Some((10, 11))
        );
    }

    #[test]
    fn a_composition_does_not_survive_a_handle() {
        let mut edit = selected(6, 11);
        edit.composing = Some((6, 11));
        assert_eq!(drag(edit, Handle::End, 9).map(|e| e.composing), Some(None));
    }

    #[test]
    fn without_a_selection_there_is_nothing_to_drag() {
        let caret = Edit {
            cursor: 4,
            anchor: None,
            composing: None,
        };
        assert_eq!(drag(caret, Handle::End, 8), None);
    }
}

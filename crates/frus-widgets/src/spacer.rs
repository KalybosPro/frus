//! [`Spacer`]: room between two children of a row or a column, and nothing else.
//!
//! It is a gap that **takes what is left** rather than a fixed one, which is the
//! difference between a header whose two ends stay at the two ends and one whose right
//! half drifts as its left half changes. A fixed `SizedBox` between them is a guess about
//! the parent's width; a `Spacer` is not a guess.
//!
//! ```ignore
//! Flex::row()
//!     .child(avatar)
//!     .child(Spacer::new())        // ← pushes the rest to the far end
//!     .child(other_avatars)
//! ```
//!
//! **Two of them centre a thing**, which is the second reason it exists: a `Spacer` either
//! side of a child, at whatever ratio, places that child anywhere along the axis without
//! anyone measuring anything.
//!
//! It is [`Expanded`](crate::Expanded) around nothing
//! (`spacer.dart:58`) — the same three properties, without the child. Written out here
//! rather than as that composition because a wrapper with no content is a wrapper whose
//! whole implementation is the box, and one struct is less to read than two.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Flexible empty space along a row's or a column's main axis.
pub struct Spacer {
    flex: f32,
}

impl Spacer {
    /// A gap that takes the whole of what is left (a flex factor of 1).
    pub fn new() -> Self {
        Self { flex: 1.0 }
    }

    /// Its share of the spare room, when there is more than one flexible child: two
    /// spacers at `2.0` and `1.0` split it two to one, and a child between them lands a
    /// third of the way along (`spacer.dart:47`).
    ///
    /// The reference asserts the factor is at least one, its being an `int` there. This
    /// takes an `f32` and does not: a spacer at `0.5` beside one at `1.0` is a two-thirds
    /// mark, which is a thing to want and which whole numbers can only express by making
    /// them larger.
    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Spacer {
    /// The three properties [`Expanded`](crate::Expanded) sets, with no child to set them
    /// around: a basis of nothing, the grow factor, and **no automatic minimum** — the one
    /// that is always forgotten, and without which a flex item is floored at its own
    /// content. A spacer has no content, so that floor is zero either way; it is written
    /// out so that the box is the same box, and stays the same box if the two ever have to
    /// be compared.
    fn style(&self) -> Style {
        Style {
            flex_grow: self.flex,
            flex_shrink: self.flex,
            flex_basis: Dimension::Length(0.0),
            min_width: Dimension::Length(0.0),
            min_height: Dimension::Length(0.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    /// Nothing. A spacer that painted would not be a spacer.
    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The box, and the part of it that is easy to leave out: `flex_grow` alone floors a
    /// flex item at its content, so the three properties go together or none of them
    /// works. See [`crate::Expanded`], where the same omission was a real bug.
    #[test]
    fn a_spacer_is_a_flexible_box_with_no_floor() {
        let style = Widget::<()>::style(&Spacer::new());
        assert_eq!(style.flex_grow, 1.0);
        assert_eq!(style.flex_basis, Dimension::Length(0.0));
        assert_eq!(style.min_width, Dimension::Length(0.0));
        assert_eq!(style.min_height, Dimension::Length(0.0));
    }

    /// And the share is the caller's, in whatever fraction they want it.
    #[test]
    fn two_spacers_split_the_room_in_the_ratio_they_were_given() {
        assert_eq!(Widget::<()>::style(&Spacer::new().flex(2.0)).flex_grow, 2.0);
        assert_eq!(Widget::<()>::style(&Spacer::new().flex(0.5)).flex_grow, 0.5);
    }

    /// It draws nothing, which is the whole contract.
    #[test]
    fn a_spacer_paints_nothing() {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &Spacer::new(),
            Rect::new(0.0, 0.0, 100.0, 20.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert!(scene.primitives().is_empty());
    }
}

//! [`Scroll`]: a vertically scrollable container.
//!
//! It occupies a fixed-size viewport; its single child is laid out at free height by
//! the driver, then clipped to the viewport and translated by the scroll offset, which
//! the runtime retains, keyed by identity.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::physics::ScrollPhysics;
use crate::theme::Theme;
use crate::widget::Widget;

/// A [`Scroll`]'s scrolling axis, or axes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
    /// Both, which is a **convenience** rather than a third kind of area: a scrollable
    /// elsewhere has one axis and one only, and two of them means one nested inside the
    /// other. This is that pair collapsed into a single node, and a gesture reads it as
    /// the pair: one drag moves one axis, decided by the direction the finger went in and
    /// held to the end of the gesture. A page that scrolls down does not drift sideways
    /// while it does it.
    Both,
}

impl Axis {
    pub(crate) fn free_x(self) -> bool {
        matches!(self, Axis::Horizontal | Axis::Both)
    }
    pub(crate) fn free_y(self) -> bool {
        matches!(self, Axis::Vertical | Axis::Both)
    }
}

/// A scrollable container.
pub struct Scroll<Msg> {
    width: Dimension,
    height: Dimension,
    /// Was the width **set** explicitly? If not, in flex mode the width must not
    /// serve as a basis; see [`Scroll::style`].
    width_explicit: bool,
    /// Was the height **set** explicitly? The same question for the vertical axis.
    height_explicit: bool,
    flex_grow: f32,
    axis: Axis,
    /// Edge and fling behaviour, when this area wants one of its own; `None`
    /// follows the application, which follows the platform.
    physics: Option<ScrollPhysics>,
    reverse: bool,
    content: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Scroll<Msg> {
    /// Creates a scrollable area, its viewport 200 px tall by default.
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Length(200.0),
            width_explicit: false,
            height_explicit: false,
            flex_grow: 0.0,
            axis: Axis::Vertical,
            physics: None,
            reverse: false,
            content: Vec::new(),
        }
    }

    /// Chooses the scrolling axis or axes; vertical by default.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Scrolls **from the far end**: the content is anchored to the bottom of a
    /// vertical viewport (the right of a horizontal one) and the area starts there.
    ///
    /// It is what a conversation wants. Two things follow from it that nothing else
    /// gives you:
    ///
    /// - content **shorter** than the viewport sits at the bottom rather than the top;
    /// - the area **stays** at the end when content arrives, because offsets are
    ///   measured from the end. A view resting at the newest message goes on resting
    ///   there, however many messages are appended.
    ///
    /// Nothing changes for the user's hand: a finger pushes the content the way it
    /// moves, in either direction, and the scrollbar's thumb rests where the content is.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Overrides how this area behaves at its edges and after a fling.
    ///
    /// Left unset, it follows the application's choice, which itself defaults to
    /// what the running platform does — so setting this is for the cases where the
    /// content wants a particular feel, not for making an app feel native.
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
    }

    /// Sets the viewport width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self.width_explicit = true;
        self
    }

    /// Sets the viewport height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self.height_explicit = true;
        self
    }

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Sets the scrollable content.
    pub fn child(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.content.clear();
        self.content.push(Box::new(content));
        self
    }
}

impl<Msg> Default for Scroll<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for Scroll<Msg> {
    fn style(&self) -> Style {
        // In **flex** mode (fill then scroll), an axis dimension that was not set
        // explicitly must not serve as a basis: the default height of 200 would
        // otherwise stay a flexible basis and the viewport would not fill the parent's
        // remaining space — it would take `+200` of free room to grow. So we set it to
        // `Auto`, a basis of 0, and let `flex_grow` **fill**.
        let filling = self.flex_grow > 0.0;
        let height = if filling && !self.height_explicit && self.axis.free_y() {
            Dimension::Auto
        } else {
            self.height
        };
        let width = if filling && !self.width_explicit && self.axis.free_x() {
            Dimension::Auto
        } else {
            self.width
        };
        Style {
            width,
            height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.content
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // The viewport itself is transparent: only the content is drawn.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        self.content.first().map(|child| child.as_ref())
    }

    fn scroll_axis(&self) -> Axis {
        self.axis
    }

    fn scroll_physics(&self) -> Option<ScrollPhysics> {
        self.physics
    }

    fn scroll_reverse(&self) -> bool {
        self.reverse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    const MARK: Color = Color::rgb(1.0, 0.0, 0.0);

    /// A scroll 100 tall holding `content_h` of content, at `offset`; returns the
    /// rectangle of the marker drawn at the very end of that content.
    fn last_mark(scroll: Scroll<()>, offset: f32) -> Rect {
        let mut runtime = Runtime::default();
        let ui = build_ui(
            &scroll,
            Size::new(100.0, 100.0),
            &runtime,
            &Theme::default(),
        );
        let id = ui.scroll_regions()[0].id;
        runtime.scroll.insert(id, (0.0, offset));
        let ui = build_ui(
            &scroll,
            Size::new(100.0, 100.0),
            &runtime,
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { color, rect, .. } if *color == MARK => Some(*rect),
                _ => None,
            })
            .next_back()
            .expect("the marker is painted")
    }

    /// A column of `n` 40 px rows, the last one marked.
    fn content(n: usize) -> Container<()> {
        let mut column = crate::Flex::<()>::column();
        for i in 0..n {
            let colour = if i + 1 == n {
                MARK
            } else {
                Color::rgb(0.0, 0.0, 1.0)
            };
            column = column.child(Container::<()>::new().height(40.0).color(colour));
        }
        Container::<()>::new().child(column)
    }

    /// Content shorter than the viewport sits at the **top** normally and at the
    /// **bottom** reversed — the first of the two things reversing is for.
    #[test]
    fn short_content_sits_at_the_end_when_reversed() {
        let plain = Scroll::<()>::new()
            .width(100.0)
            .height(100.0)
            .child(content(1));
        assert_eq!(last_mark(plain, 0.0).y, 0.0, "at the top");

        let reversed = Scroll::<()>::new()
            .width(100.0)
            .height(100.0)
            .reverse()
            .child(content(1));
        let r = last_mark(reversed, 0.0);
        assert_eq!(r.y + r.height, 100.0, "against the bottom: {r:?}");
    }

    /// Offset 0 is the **end** of the content when reversed, and it stays the end
    /// however much content there is — which is what keeps a conversation resting on
    /// its newest message as messages arrive.
    #[test]
    fn offset_zero_is_the_end_and_stays_there() {
        for rows in [4, 10, 40] {
            let reversed = Scroll::<()>::new()
                .width(100.0)
                .height(100.0)
                .reverse()
                .child(content(rows));
            let r = last_mark(reversed, 0.0);
            assert_eq!(
                r.y + r.height,
                100.0,
                "{rows} rows, still resting at the end: {r:?}"
            );
        }
    }

    /// Scrolling away from zero walks **back** through the content, which is up the
    /// screen for the newest-last layout a reversed view has.
    #[test]
    fn a_positive_offset_moves_back_through_the_content() {
        let scroll = || {
            Scroll::<()>::new()
                .width(100.0)
                .height(100.0)
                .reverse()
                .child(content(10))
        };
        let at_rest = last_mark(scroll(), 0.0);
        let scrolled = last_mark(scroll(), 60.0);
        assert_eq!(
            scrolled.y - at_rest.y,
            60.0,
            "the end has moved down and out of view"
        );
    }

    /// The one thing that is *not* reversed: what the user's hand does. A push in a
    /// direction moves the content that way in both, and only the arithmetic between
    /// the push and the number changes sign.
    #[test]
    fn a_push_moves_the_content_the_way_it_pushes() {
        let region = |reverse: bool| {
            let mut s = Scroll::<()>::new().width(100.0).height(100.0);
            if reverse {
                s = s.reverse();
            }
            let s = s.child(content(10));
            let ui = build_ui(
                &s,
                Size::new(100.0, 100.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scroll_regions()[0]
        };
        // A finger pushing the content **down** by 10 px.
        assert_eq!(region(false).offset_delta((0.0, 10.0)).1, -10.0);
        assert_eq!(region(true).offset_delta((0.0, 10.0)).1, 10.0);
        // Opposite numbers, and in both cases "the content went down": one counts from
        // the top and the other from the bottom.
    }
}

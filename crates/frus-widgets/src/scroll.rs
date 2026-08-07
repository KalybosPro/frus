//! [`Scroll`]: a vertically scrollable container.
//!
//! It occupies a fixed-size viewport; its single child is laid out at free height by
//! the driver, then clipped to the viewport and translated by the scroll offset, which
//! the runtime retains, keyed by identity.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A [`Scroll`]'s scrolling axis, or axes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
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
            content: Vec::new(),
        }
    }

    /// Chooses the scrolling axis or axes; vertical by default.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
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
}

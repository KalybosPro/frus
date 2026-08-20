//! [`Flex`]: a container that lays its children out in a row or a column, with
//! distribution (justify) and alignment (align).

use frus_core::{Insets, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A flex container, row or column. It paints no decoration of its own.
pub struct Flex<Msg> {
    direction: FlexDirection,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    flex_shrink: f32,
    justify: Justify,
    align: Align,
    padding: Insets,
    gap: f32,
    wrap: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Flex<Msg> {
    /// A container laying its children out horizontally.
    pub fn row() -> Self {
        Self::with_direction(FlexDirection::Row)
    }

    /// A container laying its children out vertically.
    pub fn column() -> Self {
        Self::with_direction(FlexDirection::Column)
    }

    fn with_direction(direction: FlexDirection) -> Self {
        Self {
            direction,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            justify: Justify::Start,
            align: Align::Stretch,
            padding: Insets::ZERO,
            gap: 0.0,
            wrap: false,
            children: Vec::new(),
        }
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Sets the height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// The flex grow factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// How much of a row's deficit this box absorbs. The default is `0.0` — the
    /// reference's rule, where an inflexible child is never squeezed and a row that does
    /// not fit overflows and says so.
    ///
    /// `shrink(1.0)` asks for flexbox's behaviour instead: give way rather than let the
    /// row run over. It is the right answer for a box whose size is a preference rather
    /// than a requirement, and the wrong one for fixed chrome — an icon button at the end
    /// of a row should keep its width however long the label beside it grows.
    pub fn shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = shrink;
        self
    }

    /// This box never shrinks — the default said out loud, kept because a layout that
    /// depends on it reads better for saying so. See [`Self::shrink`].
    pub fn no_shrink(self) -> Self {
        self.shrink(0.0)
    }

    /// How the children are distributed along the main axis.
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// How the children are aligned on the cross axis.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Uniform padding, in logical pixels.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::uniform(padding);
        self
    }

    /// Padding per side: top, right, bottom, left.
    pub fn padding_each(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Insets::new(top, right, bottom, left);
        self
    }

    /// Spacing between children, in logical pixels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Turns **wrapping** on: children that overflow the main axis move to a new
    /// line, a responsive reflow. See also [`Wrap`] as a named entry point.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Adds a child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Adds an **already boxed** child — useful when children are built dynamically,
    /// in a `Vec<Box<dyn Widget>>`, and already erased to the dynamic type.
    pub fn child_boxed(mut self, child: Box<dyn Widget<Msg>>) -> Self {
        self.children.push(child);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Flex<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_direction: self.direction,
            justify: self.justify,
            align: self.align,
            padding: self.padding,
            margin: Insets::ZERO,
            aspect_ratio: None,
            gap: self.gap,
            flex_wrap: self.wrap,
            grid_columns: None,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A flex container is transparent: no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A named entry point for a **wrapping** row (flex-wrap).
///
/// `Wrap::new().gap(8.0).child(a).child(b)…` — the children flow over several lines
/// according to the available width, with no breakpoint. It returns a [`Flex`], a row
/// with `wrap` turned on, so all its settings remain available.
pub struct Wrap;

impl Wrap {
    /// A row with wrapping turned on.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<Msg>() -> Flex<Msg> {
        Flex::row().wrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_sets_flex_wrap_in_style() {
        let plain = Flex::<()>::row();
        assert!(!Widget::<()>::style(&plain).flex_wrap);

        let wrapped = Wrap::new::<()>().child(Flex::<()>::row());
        assert!(Widget::<()>::style(&wrapped).flex_wrap);
    }
}

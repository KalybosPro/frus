//! [`FractionallySizedBox`]: sizes its box to a **fraction** of the space the
//! parent offers.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Takes a **fraction** of the parent's size on each axis that is set:
/// `width_factor(0.5)` = half the parent's width, `height_factor(0.25)` = a
/// quarter of its height. An axis left **unset** follows its content (its
/// natural size). The child fills the box (stretch / `flex`).
///
/// Rather than constraining the child, the box **sizes itself** as a percentage
/// of the parent (via `Dimension::Percent`). In the common case — a child that
/// fills — that gives the same visual result, and it fits the flex model.
pub struct FractionallySizedBox<Msg> {
    width_factor: Option<f32>,
    height_factor: Option<f32>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> FractionallySizedBox<Msg> {
    /// Creates a fractional box with no factor (both axes follow the content for
    /// as long as neither is set).
    pub fn new() -> Self {
        Self {
            width_factor: None,
            height_factor: None,
            children: Vec::new(),
        }
    }

    /// Fraction `0.0..=1.0` of the parent's **width** (clamped to `>= 0`).
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.width_factor = Some(factor.max(0.0));
        self
    }

    /// Fraction `0.0..=1.0` of the parent's **height** (clamped to `>= 0`).
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.height_factor = Some(factor.max(0.0));
        self
    }

    /// Sets the child, which fills the fractional box.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for FractionallySizedBox<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for FractionallySizedBox<Msg> {
    fn style(&self) -> Style {
        // A factor that is set → a dimension as a percentage of the parent; otherwise
        // `Auto` (the axis follows its content).
        let dim = |factor: Option<f32>| match factor {
            Some(f) => Dimension::Percent(f),
            None => Dimension::Auto,
        };
        Style {
            width: dim(self.width_factor),
            height: dim(self.height_factor),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure layout widget: no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;
    use frus_core::{Color, Primitive, Size};

    /// `width_factor(0.5)` in a column 100 wide → a box 50 wide; the child that
    /// fills paints a background about 50 wide.
    #[test]
    fn width_factor_takes_a_fraction_of_the_parent() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            FractionallySizedBox::new()
                .width_factor(0.5)
                .child(Container::new().flex(1.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .expect("the child's red background");
        assert!((rect.width - 50.0).abs() < 0.5, "half the width: {rect:?}");
    }

    /// `height_factor(0.25)` takes a quarter of the parent's height: in a column
    /// 200 tall, the box is 50 tall.
    #[test]
    fn height_factor_takes_a_fraction_of_the_parent() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .height(200.0)
            .child(
                FractionallySizedBox::new()
                    .height_factor(0.25)
                    .child(Container::new().flex(1.0).color(red)),
            );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .expect("the child's red background");
        assert!(
            (rect.height - 50.0).abs() < 0.5,
            "a quarter of the height: {rect:?}"
        );
    }
}

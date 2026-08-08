//! [`AspectRatio`]: constrains its child to a given **width-to-height ratio**.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Imposes a `width / height` ratio on its box: the box **takes the full available
/// width**, then derives its height from the ratio (`height = width / ratio`). In a
/// column 100 wide, `AspectRatio::new(2.0)` gives a 100×50 box and
/// `AspectRatio::new(0.5)` a 100×200 one.
///
/// The ratio follows the usual convention (taffy's included): `width / height`
/// — `2.0` is twice as wide as it is tall, `0.5` twice as tall as it is wide.
///
/// The child inherits the box: it stretches in height, along the cross axis, and
/// fills the width if it grows (`flex`) — typically an image or a solid background.
pub struct AspectRatio<Msg> {
    ratio: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> AspectRatio<Msg> {
    /// Creates a box at the given `width / height` ratio, clamped to `> 0`.
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.max(f32::MIN_POSITIVE),
            children: Vec::new(),
        }
    }

    /// Sets the child constrained to the ratio.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for AspectRatio<Msg> {
    fn style(&self) -> Style {
        // The box takes the parent's full width, a dimension taffy **knows**, and the
        // ratio derives the height from it. A width that is merely "stretched", through
        // align stretch, is not enough for taffy to derive the other axis.
        Style {
            width: Dimension::Percent(1.0),
            aspect_ratio: Some(self.ratio),
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

    /// In a column 100 wide, an `AspectRatio(2.0)` gives a 100×50 box — full width,
    /// derived height — and its child, which fills, paints a background of about
    /// 100×50.
    #[test]
    fn derives_free_dimension_from_ratio() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(AspectRatio::new(2.0).child(Container::new().flex(1.0).color(red)));
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
            (rect.width - 100.0).abs() < 0.5 && (rect.height - 50.0).abs() < 0.5,
            "rapport 2.0 → 100×50 : {rect:?}"
        );
    }

    /// A ratio below 1 makes the box **taller than it is wide**: `0.5` in a column of
    /// 100 gives 100×200.
    #[test]
    fn ratio_below_one_is_taller_than_wide() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(AspectRatio::new(0.5).child(Container::new().flex(1.0).color(red)));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 400.0), &rt, &theme);
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
            (rect.width - 100.0).abs() < 0.5 && (rect.height - 200.0).abs() < 0.5,
            "rapport 0.5 → 100×200 : {rect:?}"
        );
    }
}

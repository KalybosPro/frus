//! [`RotatedBox`]: rotates its child by a whole **quarter turn** — and, unlike
//! [`crate::Transform`], the rotation **affects layout** (the box swaps width
//! and height for an odd number of quarters).

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Rotates its child by `quarter_turns` **quarter turns** (90° each, clockwise),
/// **changing the layout**: for an odd number of quarters, the box presented to
/// the parent has the child's **width and height swapped** (a vertical label in
/// a sidebar, a rotated chart axis label…).
///
/// The child is measured at its **natural** size, centred, then rotated about the
/// centre of the box — hit-testing counter-rotates the point (as `Transform` does).
/// A negative `quarter_turns` turns the other way; only the remainder modulo 4 counts.
pub struct RotatedBox<Msg> {
    quarter_turns: i32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> RotatedBox<Msg> {
    /// Tourne l'enfant de `quarter_turns` quarts de tour horaires.
    pub fn new(quarter_turns: i32) -> Self {
        Self {
            quarter_turns,
            children: Vec::new(),
        }
    }

    /// Sets the rotated child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for RotatedBox<Msg> {
    fn style(&self) -> Style {
        // The real box (dimensions swapped for an odd quarter) is computed at layout
        // time from the child's natural size (see `build_layout`).
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure rotation widget: no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn rotated_quarter_turns(&self) -> Option<i32> {
        Some(self.quarter_turns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex};
    use frus_core::{Color, Primitive, Size};

    /// A quarter turn **swaps** the box's width and height: an 80×20 child occupies
    /// a 20×80 box in the column (the next sibling starts at y=80).
    #[test]
    fn odd_quarter_turn_swaps_the_layout_box() {
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = Flex::<()>::column()
            .child(
                RotatedBox::new(1).child(
                    Container::new()
                        .width(80.0)
                        .height(20.0)
                        .color(Color::rgb(0.3, 0.3, 0.3)),
                ),
            )
            .child(Container::new().width(40.0).height(30.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 400.0), &rt, &theme);
        let sibling_y = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => {
                    Some(rect.y)
                }
                _ => None,
            })
            .expect("the green sibling");
        assert!(
            (sibling_y - 80.0).abs() < 0.5,
            "rotated box = 20×80 → the sibling follows at y=80: {sibling_y}"
        );
    }

    /// A half turn (2 quarters) **does not change** the box's dimensions.
    #[test]
    fn even_quarter_turn_keeps_the_box() {
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = Flex::<()>::column()
            .child(
                RotatedBox::new(2).child(
                    Container::new()
                        .width(80.0)
                        .height(20.0)
                        .color(Color::rgb(0.3, 0.3, 0.3)),
                ),
            )
            .child(Container::new().width(40.0).height(30.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 400.0), &rt, &theme);
        let sibling_y = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => {
                    Some(rect.y)
                }
                _ => None,
            })
            .expect("the green sibling");
        assert!(
            (sibling_y - 20.0).abs() < 0.5,
            "box unchanged 80×20 → sibling at y=20: {sibling_y}"
        );
    }

    /// The rotation emits a **rotated layer** (an off-diagonal linear part).
    #[test]
    fn emits_a_rotated_layer() {
        let root = RotatedBox::<()>::new(1).child(
            Container::new()
                .width(80.0)
                .height(20.0)
                .color(Color::rgb(0.3, 0.3, 0.3)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 400.0), &rt, &theme);
        let m = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer {
                    transform: Some(t), ..
                } => Some(t.affine),
                _ => None,
            })
            .expect("a rotated layer");
        // +90°: linear part ≈ [0, 1, -1, 0].
        assert!(
            m.m[0].abs() < 1e-3 && (m.m[1] - 1.0).abs() < 1e-3,
            "rotation +90° : {:?}",
            m.m
        );
    }
}

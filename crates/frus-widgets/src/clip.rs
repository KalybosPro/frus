//! [`ClipRRect`] and [`ClipOval`]: clip their child to a **shape** (rounded
//! corners, an ellipse) at paint time.

use frus_core::{BorderRadius, ClipShape, Path, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Clips its child to a **rounded rectangle** — a radius **per corner**. The subtree
/// is painted in a layer whose shape erases whatever overflows the corners
/// (antialiased edges) — the building block of a thumbnail, a squircle avatar, a card
/// with soft corners (or only the top rounded, like a rising sheet) whose content (an
/// image, a gradient…) hugs the rounding exactly.
///
/// A pass-through in layout: the box takes the size the parent gives it (like its
/// child), and the rounding is **inscribed** in that box. Each radius is clamped to
/// half the smaller dimension (beyond that the corners meet — a stadium).
///
/// ```ignore
/// ClipRRect::new(12.0).child(Image::asset("photo.png"))              // uniform
/// ClipRRect::rounded(BorderRadius::top(16.0)).child(header)          // top rounded
/// ```
pub struct ClipRRect<Msg> {
    radius: BorderRadius,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ClipRRect<Msg> {
    /// Clips the child to a rounded rectangle of `radius` logical pixels, **uniform**
    /// across all four corners.
    pub fn new(radius: f32) -> Self {
        Self::rounded(BorderRadius::uniform(radius))
    }

    /// Clips the child to a rounded rectangle **per corner** (distinct radii).
    pub fn rounded(radius: BorderRadius) -> Self {
        Self {
            radius,
            children: Vec::new(),
        }
    }

    /// Sets the clipped child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ClipRRect<Msg> {
    fn style(&self) -> Style {
        // A pass-through: the box takes its size from the context, like the child;
        // the clipping only acts at paint time (see `clip_shape`).
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure clipping widget: no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn clip_shape(&self) -> Option<ClipShape> {
        Some(ClipShape::RRect(self.radius.clamped()))
    }
}

/// Clips its child to an **ellipse** inscribed in its box (a circle if the box is
/// square): the building block of a round avatar, a dot, a circular gauge whose
/// content is cropped to the disc. Same layout rules as [`ClipRRect`] (a
/// pass-through, with the shape inscribed in the box).
///
/// ```ignore
/// ClipOval::new().child(Image::asset("avatar.png"))
/// ```
pub struct ClipOval<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ClipOval<Msg> {
    /// Clips the child to the ellipse inscribed in the box.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Sets the clipped child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for ClipOval<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for ClipOval<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn clip_shape(&self) -> Option<ClipShape> {
        Some(ClipShape::Oval)
    }
}

/// Clips its child to an **arbitrary path**: the subtree is painted in a layer where
/// a **mask** (the path, rendered by the GPU) erases everything outside it — stars,
/// pointed cut-outs, speech bubbles, free-form shapes, with antialiased edges.
///
/// The path is given in **local coordinates** (the origin at the top-left corner of
/// the widget's box); the walk offsets it to the screen position. A layout
/// pass-through, like [`ClipRRect`].
///
/// ```ignore
/// // A diamond inscribed in a 100×100 box.
/// let diamond = Path::new()
///     .move_to(Point::new(50.0, 0.0))
///     .line_to(Point::new(100.0, 50.0))
///     .line_to(Point::new(50.0, 100.0))
///     .line_to(Point::new(0.0, 50.0))
///     .close();
/// ClipPath::new(diamond).child(Image::asset("photo.png"))
/// ```
pub struct ClipPath<Msg> {
    path: Path,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ClipPath<Msg> {
    /// Clips the child to `path` (coordinates local to the box).
    pub fn new(path: Path) -> Self {
        Self {
            path,
            children: Vec::new(),
        }
    }

    /// Sets the clipped child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ClipPath<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn clip_path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;
    use frus_core::{Color, Point, Primitive, Size};

    /// Wrapping the scene in a `ClipRRect`: a layer with an `RRect` shape is emitted,
    /// and the child's background is painted **inside** it (in the layer's primitives).
    #[test]
    fn clip_rrect_wraps_child_in_a_rounded_layer() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(ClipRRect::new(8.0).child(Container::new().width(40.0).height(40.0).color(red)));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let layer = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer {
                    clip_shape,
                    primitives,
                    ..
                } => Some((clip_shape.clone(), primitives.clone())),
                _ => None,
            })
            .expect("a clipping layer");
        assert_eq!(
            layer.0,
            ClipShape::RRect(BorderRadius::uniform(8.0)),
            "forme arrondie de rayon 8"
        );
        assert!(
            layer
                .1
                .iter()
                .any(|p| matches!(p, Primitive::Rect { color, .. } if color.r > 0.5)),
            "the child's red background is painted in the layer"
        );
    }

    /// `ClipOval` emits a layer with an `Oval` shape.
    #[test]
    fn clip_oval_emits_an_oval_layer() {
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(ClipOval::new().child(Container::new().width(40.0).height(40.0).color(blue)));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let shape = ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Layer { clip_shape, .. } => Some(clip_shape.clone()),
            _ => None,
        });
        assert_eq!(shape, Some(ClipShape::Oval), "forme ellipse");
    }

    /// `ClipPath` emits a layer with a `Path` shape, **offset to the screen position**
    /// of the box (the local path is translated by the box's origin).
    #[test]
    fn clip_path_emits_a_translated_path_layer() {
        // A 40×40 diamond in local coordinates.
        let diamond = Path::new()
            .move_to(Point::new(20.0, 0.0))
            .line_to(Point::new(40.0, 20.0))
            .line_to(Point::new(20.0, 40.0))
            .line_to(Point::new(0.0, 20.0))
            .close();
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .padding(10.0)
            .child(
                ClipPath::new(diamond).child(
                    Container::new()
                        .width(40.0)
                        .height(40.0)
                        .color(Color::rgb(1.0, 0.0, 0.0)),
                ),
            );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let shape = ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Layer {
                clip_shape: ClipShape::Path(path),
                ..
            } => Some(path.clone()),
            _ => None,
        });
        let path = shape.expect("a path clipping layer");
        // The local vertex (20, 0) is translated by the box's origin (padding 10)
        // → (30, 10) on screen.
        let first = path.verbs().first().copied().expect("at least one verb");
        match first {
            frus_core::PathVerb::MoveTo(p) => assert!(
                (p.x - 30.0).abs() < 0.6 && (p.y - 10.0).abs() < 0.6,
                "vertex offset on screen to (30, 10): {p:?}"
            ),
            other => panic!("expected MoveTo as the first verb, got {other:?}"),
        }
    }

    /// Clipping is a layout **pass-through**: a sibling placed after a clipped child
    /// keeps its position (the `ClipRRect` does not grow its box).
    #[test]
    fn clip_is_layout_passthrough() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(ClipRRect::new(6.0).child(Container::new().height(20.0).color(red)))
            .child(Container::new().height(20.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        // The 2nd child follows at y = 20 (the clipped child takes 20px of height).
        let green_y = ui
            .scene()
            .primitives()
            .iter()
            .flat_map(|p| match p {
                Primitive::Layer { primitives, .. } => primitives.clone(),
                other => vec![other.clone()],
            })
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => {
                    Some(rect.y)
                }
                _ => None,
            })
            .expect("the 2nd child's green background");
        assert!(
            (green_y - 20.0).abs() < 0.5,
            "sibling in its layout place: y = {green_y}"
        );
    }
}

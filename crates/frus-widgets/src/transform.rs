//! [`Transform`]: offsets its child at **paint** time, without touching layout.

use frus_core::{Alignment, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Transforms its child **at paint time** (both rendering **and** hit-testing),
/// without changing layout: the siblings do not move, and the child may overflow
/// its box, since there is no clipping.
///
/// Two transformations, each combinable with a `Tween` read in `view()` to
/// **animate**:
/// - **`translate(dx, dy)`** — offsets the subtree (a dot in a corner, an entry
///   that slides in, an error shake…).
/// - **`scale(factor)`** / **`scale_xy(sx, sy)`** — scales the subtree about a
///   pivot (the centre by default), uniformly or **per axis** (stretching,
///   flattening): a button's "pop" effect, a thumbnail zoom.
/// - **`rotate(radians)`** — rotates the subtree about a pivot (the centre by
///   default): a needle, a chevron that flips, a spinner.
///
/// Scale and rotation (and their composition) are melted into **a single affine
/// matrix** carried by a transformed composited layer; hit-testing applies the
/// **inverse** matrix to the point. They **compose** within one widget through the
/// chainers `and_translate`, `and_scale` / `and_scale_xy`, `and_rotate` — applied in
/// the order translation → scale → rotation (the translation innermost, through the
/// child offset). So `Transform::scale(1.5).and_rotate(0.2)` enlarges **and**
/// rotates, exactly, with no composition approximation.
pub struct Transform<Msg> {
    dx: f32,
    dy: f32,
    /// `(sx, sy, pivot)` — `None` = no scaling.
    scale: Option<(f32, f32, Alignment)>,
    /// `(angle_radians, pivot)` — `None` = no rotation.
    rotate: Option<(f32, Alignment)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Transform<Msg> {
    /// Offsets the child by `(dx, dy)` logical pixels (x to the right, y downwards),
    /// without touching layout.
    pub fn translate(dx: f32, dy: f32) -> Self {
        Self {
            dx,
            dy,
            scale: None,
            rotate: None,
            children: Vec::new(),
        }
    }

    /// Scales the child by `factor` **about its centre**, without touching layout
    /// (`1.0` = neutral, `2.0` = double, `0.5` = half).
    pub fn scale(factor: f32) -> Self {
        Self::scale_xy_from(factor, factor, Alignment::CENTER)
    }

    /// Scales the child **per axis** (`sx` horizontal, `sy` vertical) about its
    /// centre — stretching or flattening. `scale_xy(2.0, 1.0)` doubles the width
    /// while keeping the height.
    pub fn scale_xy(sx: f32, sy: f32) -> Self {
        Self::scale_xy_from(sx, sy, Alignment::CENTER)
    }

    /// Like [`Transform::scale`], but about a `pivot` (an anchor within the box:
    /// `Alignment::TOP_LEFT` pins the top-left corner, and so on).
    pub fn scale_from(factor: f32, pivot: Alignment) -> Self {
        Self::scale_xy_from(factor, factor, pivot)
    }

    /// Per-axis scaling about a `pivot` — the most general form.
    pub fn scale_xy_from(sx: f32, sy: f32, pivot: Alignment) -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            scale: Some((sx, sy, pivot)),
            rotate: None,
            children: Vec::new(),
        }
    }

    /// Rotates the child by `radians` (clockwise) **about its centre**, without
    /// touching layout.
    pub fn rotate(radians: f32) -> Self {
        Self::rotate_from(radians, Alignment::CENTER)
    }

    /// Like [`Transform::rotate`], but about a `pivot` (an anchor within the box).
    pub fn rotate_from(radians: f32, pivot: Alignment) -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            scale: None,
            rotate: Some((radians, pivot)),
            children: Vec::new(),
        }
    }

    /// **Adds** a translation to the current transformation (composition):
    /// `Transform::scale(1.2).and_translate(0, -4)` enlarges *and* moves up.
    pub fn and_translate(mut self, dx: f32, dy: f32) -> Self {
        self.dx = dx;
        self.dy = dy;
        self
    }

    /// **Adds** a uniform scaling (about the centre) to the current
    /// transformation.
    pub fn and_scale(self, factor: f32) -> Self {
        self.and_scale_xy(factor, factor)
    }

    /// **Adds** a per-axis scaling (about the centre) to the current transformation.
    pub fn and_scale_xy(mut self, sx: f32, sy: f32) -> Self {
        self.scale = Some((sx, sy, Alignment::CENTER));
        self
    }

    /// **Adds** a rotation (about the centre) to the current transformation:
    /// `Transform::scale(1.5).and_rotate(0.2)` enlarges *and* rotates.
    pub fn and_rotate(mut self, radians: f32) -> Self {
        self.rotate = Some((radians, Alignment::CENTER));
        self
    }

    /// Sets the transformed child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Transform<Msg> {
    fn style(&self) -> Style {
        // A pass-through: the box takes its size from the context, like the child;
        // the offset only acts at paint time (see `transform_translate`).
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure transformation widget: no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn transform_translate(&self) -> Option<(f32, f32)> {
        ((self.dx != 0.0) || (self.dy != 0.0)).then_some((self.dx, self.dy))
    }

    fn transform_scale(&self) -> Option<(f32, f32, Alignment)> {
        self.scale
    }

    fn transform_rotate(&self) -> Option<(f32, Alignment)> {
        self.rotate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;
    use frus_core::{Color, Primitive, Size};

    /// `Transform::translate(30, 10)` offsets the (20×20) child at paint time: its
    /// background, normally at the top left, is painted at about (30, 10).
    #[test]
    fn translate_offsets_the_child_at_paint() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::translate(30.0, 10.0)
                .child(Container::new().width(20.0).height(20.0).color(red)),
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
            .expect("le fond rouge de l'enfant");
        assert!(
            (rect.x - 30.0).abs() < 0.5 && (rect.y - 10.0).abs() < 0.5,
            "offset to (30, 10): {rect:?}"
        );
    }

    /// The offset is **purely visual**: a sibling placed after a transformed child
    /// keeps its layout position (the Transform neither grows nor moves its box).
    /// Here the 2nd child stays at `y = 20`, despite the 1st being offset 50
    /// vertically.
    #[test]
    fn translate_does_not_affect_layout() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(
                Transform::translate(0.0, 50.0)
                    .child(Container::new().flex(1.0).height(20.0).color(red)),
            )
            .child(Container::new().height(20.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let green_y = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => {
                    Some(rect.y)
                }
                _ => None,
            })
            .expect("the 2nd child's green background");
        // The 1st child takes 20px of height in layout (its offset of 50 is visual):
        // the 2nd child follows at y = 20.
        assert!(
            (green_y - 20.0).abs() < 0.5,
            "sibling in its layout place: y = {green_y}"
        );
    }

    /// Extracts the [`Affine`] of the scene's transformed layer (the subtree is
    /// painted flat *inside* that layer; the transformation is carried by its matrix).
    fn layer_affine<Msg: Clone>(ui: &crate::ui::Ui<Msg>) -> frus_core::Affine {
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer {
                    transform: Some(t), ..
                } => Some(t.affine),
                _ => None,
            })
            .expect("a transformed layer")
    }

    /// `Transform::scale(2.0)` carries a matrix that doubles (a linear part of 2) and
    /// leaves **the child's centre (10, 10) fixed**.
    #[test]
    fn scale_grows_the_child_about_its_center() {
        use frus_core::Point;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::scale(2.0).child(Container::new().width(20.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!(
            (m.m[0] - 2.0).abs() < 1e-3 && (m.m[3] - 2.0).abs() < 1e-3,
            "×2 : {:?}",
            m.m
        );
        let c = m.apply(Point::new(10.0, 10.0));
        assert!(
            (c.x - 10.0).abs() < 0.5 && (c.y - 10.0).abs() < 0.5,
            "centre fixe : {c:?}"
        );
    }

    /// `scale_from(2.0, TOP_LEFT)` scales about the child's top-left corner: that
    /// corner (0, 0) stays fixed.
    #[test]
    fn scale_from_pins_the_pivot_corner() {
        use frus_core::{Alignment, Point};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::scale_from(2.0, Alignment::TOP_LEFT)
                .child(Container::new().width(20.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!(
            (m.m[0] - 2.0).abs() < 1e-3 && (m.m[3] - 2.0).abs() < 1e-3,
            "×2 : {:?}",
            m.m
        );
        let c = m.apply(Point::new(0.0, 0.0));
        assert!(
            c.x.abs() < 0.5 && c.y.abs() < 0.5,
            "coin haut-gauche fixe : {c:?}"
        );
    }

    /// `Transform::scale_xy(3.0, 1.0)` carries a **per-axis** scale matrix (×3 in x,
    /// ×1 in y), with the centre fixed.
    #[test]
    fn scale_xy_stretches_per_axis() {
        use frus_core::Point;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(200.0).child(
            Transform::scale_xy(3.0, 1.0)
                .child(Container::new().width(20.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!(
            (m.m[0] - 3.0).abs() < 1e-3 && (m.m[3] - 1.0).abs() < 1e-3,
            "×3 en x, ×1 en y : {:?}",
            m.m
        );
        let c = m.apply(Point::new(10.0, 10.0));
        assert!(
            (c.x - 10.0).abs() < 0.5 && (c.y - 10.0).abs() < 0.5,
            "centre fixe : {c:?}"
        );
    }

    /// `Transform::rotate(π/2)` carries a pure rotation matrix (a linear part of
    /// `[0, 1, -1, 0]`) that leaves the 40×20 child's centre fixed → (20, 10).
    #[test]
    fn rotate_emits_a_rotated_layer() {
        use frus_core::Point;
        use std::f32::consts::FRAC_PI_2;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::rotate(FRAC_PI_2)
                .child(Container::new().width(40.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!(
            m.m[0].abs() < 1e-3
                && (m.m[1] - 1.0).abs() < 1e-3
                && (m.m[2] + 1.0).abs() < 1e-3
                && m.m[3].abs() < 1e-3,
            "rotation +90° : {:?}",
            m.m
        );
        let c = m.apply(Point::new(20.0, 10.0));
        assert!(
            (c.x - 20.0).abs() < 0.5 && (c.y - 10.0).abs() < 0.5,
            "centre fixe : {c:?}"
        );
    }

    /// **Composition**: `scale(2.0).and_rotate(π/2)` melts the two into **a single**
    /// matrix = rotation ∘ scale: the linear part is `[0, 2, -2, 0]` (magnitude 2 =
    /// the scaling, off-diagonal = the rotation).
    #[test]
    fn scale_and_rotate_compose() {
        use std::f32::consts::FRAC_PI_2;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::scale(2.0)
                .and_rotate(FRAC_PI_2)
                .child(Container::new().width(20.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!(
            m.m[0].abs() < 1e-3
                && (m.m[1] - 2.0).abs() < 1e-3
                && (m.m[2] + 2.0).abs() < 1e-3
                && m.m[3].abs() < 1e-3,
            "rotation ∘ scale: {:?}",
            m.m
        );
    }

    /// Under an **axis-aligned** transformation (pure scaling), the **focus** targets
    /// follow too: a point outside the flat button but inside its enlarged image is
    /// focusable, and its focus rectangle is about 2× wider.
    #[test]
    fn axis_aligned_transform_scales_the_focus_rect() {
        use frus_core::Point;
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let flat_ui = crate::ui::build_ui(
            &crate::Flex::<i32>::column()
                .width(200.0)
                .child(crate::Button::new("Ok").on_press(1)),
            Size::new(200.0, 200.0),
            &rt,
            &theme,
        );
        let flat = flat_ui
            .focus_hit(Point::new(2.0, 2.0))
            .expect("a focusable button")
            .1;
        let cy = flat.y + flat.height / 2.0;
        // A point just right of the flat button (200 wide): outside its target.
        let probe = Point::new(flat.x + flat.width + 2.0, cy);
        assert!(
            flat_ui.focus_hit(probe).is_none(),
            "outside the flat button"
        );

        let scaled_ui = crate::ui::build_ui(
            &crate::Flex::<i32>::column()
                .width(200.0)
                .child(Transform::scale(2.0).child(crate::Button::new("Ok").on_press(1))),
            Size::new(200.0, 200.0),
            &rt,
            &theme,
        );
        let (_, r) = scaled_ui
            .focus_hit(probe)
            .expect("the button's enlarged image covers the point");
        assert!(
            (r.width - flat.width * 2.0).abs() < 1.0,
            "rectangle de focus ~2× : {} vs {}",
            r.width,
            flat.width * 2.0
        );
    }

    /// Hit-testing **counter-rotates** the point: a click at a clickable child's
    /// *rotated* position reaches it, while its original (unrotated) position no
    /// longer does. A 40×20 child rotated +90° about (20, 10): the internal point
    /// (35, 10) appears on screen at (20, 25).
    #[test]
    fn rotate_hit_test_counter_rotates_the_point() {
        use frus_core::Point;
        use std::f32::consts::FRAC_PI_2;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<i32>::column().width(100.0).child(
            Transform::rotate(FRAC_PI_2).child(
                Container::new()
                    .width(40.0)
                    .height(20.0)
                    .color(red)
                    .on_click(7),
            ),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        // On screen, the internal point (35, 10) is painted at (20, 25) after rotation.
        assert!(
            ui.hit(Point::new(20.0, 25.0)).is_some(),
            "a click on the rotated position"
        );
        // The original (unrotated) position no longer covers the child.
        assert!(
            ui.hit(Point::new(35.0, 10.0)).is_none(),
            "l'ancienne position rate"
        );
    }
}

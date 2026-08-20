//! [`FittedBox`]: scales its child to **fit** its box according to a [`BoxFit`] —
//! and, unlike [`crate::Transform`], the scale follows from the **layout** (the
//! size of the box) instead of being set by hand.

use frus_core::{BoxFit, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Scales its child to **fit** its own box according to a [`BoxFit`] (like CSS
/// `object-fit`), then centres it. The child is measured at its **natural** size;
/// the scale factor follows from that — hence the effect on layout (unlike
/// `Transform`, where the scale is set by hand).
///
/// Ideal for making intrinsically sized content (text, an icon, a drawing) **fill**
/// or **fit** a given frame with no manual arithmetic. The box needs a size (as
/// [`crate::SingleChildScrollView`] does): a fixed `width`/`height`, or `flex`.
///
/// ```ignore
/// FittedBox::new(BoxFit::Contain).width(120.0).height(40.0).child(Text::new("Big"))
/// ```
pub struct FittedBox<Msg> {
    fit: BoxFit,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> FittedBox<Msg> {
    /// Fits the child according to `fit` (the usual default: [`BoxFit::Contain`]).
    pub fn new(fit: BoxFit) -> Self {
        Self {
            fit,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            children: Vec::new(),
        }
    }

    /// Fixed box width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixed box height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Sets the fitted child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for FittedBox<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure fitting widget: no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn fitted(&self) -> Option<BoxFit> {
        Some(self.fit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex};
    use frus_core::{Color, Primitive, Size};

    /// `BoxFit::Fill` stretches the child to **fill** the box: the layer's matrix
    /// carries the per-axis scale (200/40 = 5 in x, 100/20 = 5 in y here → square: 5,5).
    #[test]
    fn fill_scales_child_to_the_box() {
        let root = Flex::<()>::column().width(200.0).child(
            FittedBox::new(BoxFit::Fill)
                .width(200.0)
                .height(100.0)
                .child(
                    Container::new()
                        .width(40.0)
                        .height(20.0)
                        .color(Color::rgb(0.3, 0.3, 0.3)),
                ),
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
            .expect("a fitted layer");
        assert!(
            (m.m[0] - 5.0).abs() < 1e-2 && (m.m[3] - 5.0).abs() < 1e-2,
            "Fill scale 5×5: {:?}",
            m.m
        );
    }

    /// `BoxFit::Contain` preserves the aspect ratio: the smallest factor that fits.
    /// A 40×20 child in 200×100 → min(5, 5) = 5 (square), and stays centred.
    #[test]
    fn contain_preserves_aspect() {
        let root = Flex::<()>::column().width(200.0).child(
            FittedBox::new(BoxFit::Contain)
                .width(200.0)
                .height(100.0)
                .child(
                    Container::new()
                        .width(40.0)
                        .height(40.0)
                        .color(Color::rgb(0.3, 0.3, 0.3)),
                ),
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
            .expect("a fitted layer");
        // A square 40×40 child in 200×100 → min(5, 2.5) = 2.5, uniform.
        assert!(
            (m.m[0] - 2.5).abs() < 1e-2 && (m.m[3] - 2.5).abs() < 1e-2,
            "Contain scale 2.5×2.5: {:?}",
            m.m
        );
    }
}

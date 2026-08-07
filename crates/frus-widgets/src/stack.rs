//! [`Stack`]: overlays its children in the **same box**, z-layering them. Every layer
//! fills the stack and the last one sits on top. It underpins the internal overlays: a
//! scrim over content, a badge in a corner through an aligned layer, and so on.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A container of overlaid layers.
pub struct Stack<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    layers: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Stack<Msg> {
    /// Creates an empty stack.
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            layers: Vec::new(),
        }
    }

    /// Fixe la largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la hauteur, en pixels logiques.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Facteur d'expansion flex sur l'axe principal du parent.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Adds a layer, on top of the previous ones.
    pub fn layer(mut self, layer: impl Widget<Msg> + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }
}

impl<Msg> Default for Stack<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for Stack<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.layers
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn stack(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    #[test]
    fn layers_overlap_in_same_box() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let stack = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .layer(Container::<()>::new().width(100.0).height(100.0).color(red))
            .layer(Container::<()>::new().width(40.0).height(40.0).color(blue));
        let ui = build_ui(
            &stack,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );

        let rect_of = |c: Color| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, rect, .. } if *color == c => Some(*rect),
                    _ => None,
                })
                .expect("the layer is present")
        };
        // Both layers share the same origin, being overlaid.
        assert_eq!(rect_of(red).origin(), rect_of(blue).origin());
    }
}

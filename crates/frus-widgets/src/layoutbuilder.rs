//! [`LayoutBuilder`]: builds its content **from the box actually available** — the
//! most powerful responsive primitive, since a component adapts wherever it is placed,
//! in a narrow sidebar or full screen, and not merely to the window.
//!
//! As with a virtualised list item, the content is **built on the fly** every frame, so
//! it has **no retained state**: hover and clicks work, but persistent keyboard focus
//! and deferred overlays do not.
//!
//! The sizing contract: `LayoutBuilder` is a **layout leaf**, so its own size comes
//! from its style, as a box's does, and not from its content. On the cross axis of a
//! `Stretch` parent the width is already the parent's; set the height, or `flex`, to
//! give it a usable box.

use frus_core::{Rect, Scene, Size};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Construit son enfant depuis la taille disponible (`taille → widget`).
pub struct LayoutBuilder<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    build: Box<dyn Fn(Size) -> Box<dyn Widget<Msg>>>,
}

impl<Msg> LayoutBuilder<Msg> {
    /// Builds from a `size → widget` factory. The size received is the **real box**
    /// allocated to this `LayoutBuilder`, in logical px.
    pub fn new<W: Widget<Msg> + 'static>(build: impl Fn(Size) -> W + 'static) -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            build: Box::new(move |size| Box::new(build(size)) as Box<dyn Widget<Msg>>),
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
}

impl<Msg> Widget<Msg> for LayoutBuilder<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        Some(&*self.build)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, SizeClass};
    use frus_core::{Color, Primitive};

    #[test]
    fn receives_its_real_box() {
        use std::cell::Cell;
        use std::rc::Rc;

        let seen = Rc::new(Cell::new(Size::new(0.0, 0.0)));
        let probe = seen.clone();
        let lb = LayoutBuilder::<()>::new(move |size| {
            probe.set(size);
            Container::<()>::new().color(Color::rgb(1.0, 0.0, 0.0))
        })
        .width(320.0)
        .height(48.0);

        let _ui = build_ui(
            &lb,
            Size::new(800.0, 600.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The factory receives the LayoutBuilder's box, not the window's.
        assert_eq!(seen.get(), Size::new(320.0, 48.0));
    }

    #[test]
    fn adapts_content_to_available_width() {
        // Picks a number of tiles from the box's size class.
        let count_tiles = |window_w: f32| {
            let lb = LayoutBuilder::<()>::new(|size| {
                let tiles = match SizeClass::from_width(size.width) {
                    SizeClass::Compact => 1,
                    SizeClass::Medium => 2,
                    SizeClass::Expanded => 3,
                };
                let mut row = crate::Flex::<()>::row();
                for _ in 0..tiles {
                    row = row.child(
                        Container::<()>::new()
                            .width(10.0)
                            .height(10.0)
                            .color(Color::WHITE),
                    );
                }
                row
            })
            .height(20.0);
            // The root column is sized, so the stretch gives the LayoutBuilder the
            // window's width and its box reflects the available width.
            let root = crate::Flex::<()>::column()
                .width(window_w)
                .height(200.0)
                .child(lb);
            let ui = build_ui(
                &root,
                Size::new(window_w, 200.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { color, .. } if *color == Color::WHITE))
                .count()
        };
        assert_eq!(count_tiles(400.0), 1);
        assert_eq!(count_tiles(700.0), 2);
        assert_eq!(count_tiles(1000.0), 3);
    }
}

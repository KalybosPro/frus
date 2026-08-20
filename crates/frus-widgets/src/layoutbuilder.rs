//! [`LayoutBuilder`]: builds its content **from the box actually available** — the
//! most powerful responsive primitive, since a component adapts wherever it is placed,
//! in a narrow sidebar or full screen, and not merely to the window.
//!
//! As with a virtualised list item, the content is **built on the fly** every frame, so
//! it has **no retained state**: hover and clicks work, but persistent keyboard focus
//! and deferred overlays do not.
//!
//! The sizing contract: **as big as what it built**, on any axis its style leaves open.
//! That is the reference's rule — `size = constraints.constrain(child.size)` — and it is
//! what lets a `LayoutBuilder` be dropped into a column without having to guess a height
//! for it first.
//!
//! It works because the widget is a **measured** leaf rather than a plain one: the
//! layout engine calls back during the computation, with the space actually available,
//! at the same point the reference runs its layout callback. The closure builds the
//! subtree, lays it out in a tree of its own, and returns its size.
//!
//! Three consequences worth knowing:
//!
//! - **A pinned axis stays pinned.** `width`, `height` and `flex` still win, because a
//!   dimension the style already gives is one the engine never asks about.
//! - **The closure runs more than once a frame** — once for the measurement, once for
//!   the paint, and again for each intrinsic question the engine asks. It must therefore
//!   be cheap and free of side effects, which it must be anyway: it is called with no
//!   retained state.
//! - **A subtree holding one is not layout-cached.** The relayout cache reuses geometry
//!   when a fingerprint of the styles and the structure has not moved, and a closure has
//!   no fingerprint: two frames that look identical to it can still want different
//!   boxes. So the root is recomputed every frame rather than risk a stale one.

use frus_core::{Rect, Scene, Size};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Builds its child from the available size (`size → widget`).
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

    /// Flex growth factor along the parent's main axis.
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

    /// The reference's rule: with no height of its own, the box is as tall as what the
    /// closure built. Before milestone 355 this laid out 0 px tall and the widget's own
    /// documentation told you to go and find a number for it.
    #[test]
    fn it_is_as_big_as_what_it_built() {
        let lb = LayoutBuilder::<()>::new(|_| {
            Container::<()>::new()
                .height(60.0)
                .color(Color::rgb(1.0, 0.0, 0.0))
        });
        let root = crate::Flex::<()>::column()
            .width(200.0)
            .height(400.0)
            .child(lb);
        let ui = build_ui(
            &root,
            Size::new(200.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let painted = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.9 && color.g < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the built content is painted");
        assert_eq!(
            painted.height, 60.0,
            "the box is its content's: {painted:?}"
        );
    }

    /// A sibling after it is pushed down by what it built — which is the point: the box
    /// is real, not a zero-height hole the rest of the column lays out through.
    #[test]
    fn what_it_built_pushes_its_siblings_down() {
        let column = crate::Flex::<()>::column()
            .width(200.0)
            .height(400.0)
            .child(LayoutBuilder::<()>::new(|_| {
                Container::<()>::new().height(60.0)
            }))
            .child(
                Container::<()>::new()
                    .height(20.0)
                    .color(Color::rgb(0.0, 1.0, 0.0)),
            );
        let ui = build_ui(
            &column,
            Size::new(200.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let sibling = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.9 && color.r < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the sibling is painted");
        assert_eq!(sibling.y, 60.0, "it starts below the built content");
    }

    /// An axis the style pins is still the style's: the engine never asks about a
    /// dimension it already knows, so every existing `height(...)` behaves as it did.
    #[test]
    fn a_pinned_axis_is_still_the_style_s() {
        let lb = LayoutBuilder::<()>::new(|_| {
            Container::<()>::new()
                .height(60.0)
                .color(Color::rgb(1.0, 0.0, 0.0))
        })
        .height(120.0);
        let root = crate::Flex::<()>::column()
            .width(200.0)
            .height(400.0)
            .child(lb)
            .child(
                Container::<()>::new()
                    .height(20.0)
                    .color(Color::rgb(0.0, 1.0, 0.0)),
            );
        let ui = build_ui(
            &root,
            Size::new(200.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let sibling = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.9 && color.r < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the sibling is painted");
        assert_eq!(sibling.y, 120.0, "the style's 120, not the content's 60");
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

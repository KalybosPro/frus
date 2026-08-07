//! [`List`]: a **virtualised** list — only the **visible** items are built, laid
//! out and painted. Essential for large lists (thousands of rows): the per-frame
//! cost is proportional to the visible items, not to the total.
//!
//! Contract (v1): **fixed** item height, **vertical** scrolling. Items are built
//! on demand by an `index → widget` closure, so they have **no retained state**
//! and take no keyboard focus (you cannot retain the state of an item that does
//! not exist off screen) — perfect for display (logs, tables, long lists), and
//! still clickable and hoverable while visible.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The description of a virtualised list, exposed to the render driver.
pub struct VirtualList<'a, Msg> {
    /// Total number of items.
    pub count: usize,
    /// The (logical) height of one item.
    pub item_height: f32,
    /// Builds one item per index.
    pub build: &'a dyn Fn(usize) -> Box<dyn Widget<Msg>>,
}

/// A virtualised list with a fixed item height.
pub struct List<Msg> {
    count: usize,
    item_height: f32,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    build: Box<dyn Fn(usize) -> Box<dyn Widget<Msg>>>,
}

impl<Msg> List<Msg> {
    /// Creates a list of `count` items of height `item_height`, each item being
    /// built on demand by `build(index)`.
    pub fn new<W: Widget<Msg> + 'static>(
        count: usize,
        item_height: f32,
        build: impl Fn(usize) -> W + 'static,
    ) -> Self {
        Self {
            count,
            item_height,
            width: Dimension::Auto,
            height: Dimension::Length(200.0),
            flex_grow: 0.0,
            build: Box::new(move |index| Box::new(build(index)) as Box<dyn Widget<Msg>>),
        }
    }

    /// Sets the viewport width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Sets the viewport height, in logical pixels.
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

impl<Msg> Widget<Msg> for List<Msg> {
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

    fn virtual_list(&self) -> Option<VirtualList<'_, Msg>> {
        Some(VirtualList {
            count: self.count,
            item_height: self.item_height,
            build: &*self.build,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    #[test]
    fn only_visible_items_are_built() {
        use std::cell::Cell;
        use std::rc::Rc;

        // Counts how many items the list builds.
        let built = Rc::new(Cell::new(0usize));
        let counter = built.clone();
        let list = List::<()>::new(5000, 40.0, move |_i| {
            counter.set(counter.get() + 1);
            Container::<()>::new()
                .width(180.0)
                .height(40.0)
                .color(Color::rgb(1.0, 0.0, 0.0))
        })
        .width(200.0)
        .height(200.0);

        let _ui = build_ui(
            &list,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Viewport 200 / item 40 = 5 visible (+ maybe 1 of margin) — never 5000.
        assert!(
            built.get() <= 8,
            "only the visible items are built: {}",
            built.get()
        );
        assert!(
            built.get() >= 5,
            "at least the visible window: {}",
            built.get()
        );
    }

    #[test]
    fn scroll_max_covers_full_content() {
        let list = List::<()>::new(100, 40.0, |_i| Container::<()>::new().height(40.0))
            .width(200.0)
            .height(200.0);
        let ui = build_ui(
            &list,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Content = 100×40 = 4000; viewport 200 → max vertical scroll 3800.
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1);
        assert_eq!(maxes[0].2, 3800.0);
    }

    #[test]
    fn builds_a_scene() {
        let list = List::<()>::new(50, 30.0, |i| {
            Container::<()>::new().height(30.0).color(if i % 2 == 0 {
                Color::rgb(0.2, 0.2, 0.2)
            } else {
                Color::rgb(0.3, 0.3, 0.3)
            })
        })
        .width(200.0)
        .height(120.0);
        let ui = build_ui(
            &list,
            Size::new(200.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let rects = ui
            .scene()
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { .. }))
            .count();
        assert!(rects > 0);
    }
}

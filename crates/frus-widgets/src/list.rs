//! [`List`]: a **virtualised** list — only the **visible** items are built, laid
//! out and painted. Essential for large lists (thousands of rows): the per-frame
//! cost is proportional to the visible items, not to the total.
//!
//! Contract (v1): **fixed** item height, **vertical** scrolling. Items are built
//! on demand by an `index → widget` closure, so they have **no retained state**
//! and take no keyboard focus (you cannot retain the state of an item that does
//! not exist off screen) — perfect for display (logs, tables, long lists), and
//! still clickable and hoverable while visible.

use frus_core::{Insets, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::physics::ScrollPhysics;
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
    physics: Option<ScrollPhysics>,
    reverse: bool,
    /// Room around the items, inside the viewport; see [`List::padding`].
    padding: Insets,
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
            physics: None,
            reverse: false,
            padding: Insets::ZERO,
            build: Box::new(move |index| Box::new(build(index)) as Box<dyn Widget<Msg>>),
        }
    }

    /// Builds **from the bottom**: item 0 sits at the bottom of the viewport, item 1
    /// above it, and the list starts resting there.
    ///
    /// The other half of a conversation, and the half [`crate::Scroll::reverse`] cannot
    /// give you: a scroll can anchor its content to the end, but only a list decides
    /// which end an *index* is. With index 0 the newest message, adding one keeps every
    /// other item exactly where it was — the view does not jump, and nothing has to be
    /// renumbered.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Insets the content, **inside** the viewport: the padding scrolls with what it
    /// surrounds rather than shrinking the window onto it.
    ///
    /// That is the whole distinction, and it is the reference's: a scroll area padded at
    /// the bottom has that room *at the end of its content*, reachable only by scrolling
    /// to it, which is what a floating button hovering over the last row needs. Room
    /// taken out of the viewport instead would sit there permanently and the last row
    /// would still slide under the button.
    ///
    /// Along the cross axis it simply insets the content, which stays the width of the
    /// viewport less the two sides.
    ///
    /// A **reversed** list keeps the sides where they look: the bottom inset is at the
    /// bottom, which is also the end the items start from, so it is the one the first
    /// item clears.
    ///
    /// ```
    /// # use frus_widgets::{Container, List};
    /// List::<()>::new(200, 56.0, |_| Container::<()>::new()).padding(16.0);
    /// ```
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::uniform(padding);
        self
    }

    /// The same, one side at a time.
    pub fn padding_each(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Insets::new(top, right, bottom, left);
        self
    }

    /// Overrides how the list behaves at its edges and after a fling; see
    /// [`crate::scroll::Scroll::physics`].
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
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

    fn scroll_physics(&self) -> Option<ScrollPhysics> {
        self.physics
    }

    fn scroll_reverse(&self) -> bool {
        self.reverse
    }

    fn scroll_padding(&self) -> Insets {
        self.padding
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

    /// An item that sets no width **fills the list**, as the reference's does: a list
    /// hands its children a box rather than asking them how big they would like to be.
    ///
    /// Asked instead, a row hugs whatever is in it, and a list of coloured rows paints a
    /// column of chips down the left rather than rows across the list. A device showed
    /// exactly that in milestone 349.
    #[test]
    fn an_item_is_handed_the_list_s_width() {
        let list = List::<()>::new(20, 40.0, |i| {
            Container::<()>::new()
                .height(40.0)
                .color(Color::rgb(1.0, 0.0, 0.0))
                .child(crate::Text::new(format!("Row {i}")))
        })
        .width(200.0)
        .height(200.0);
        let ui = build_ui(
            &list,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let row = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.9 && color.g < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("a row is painted");
        assert_eq!(row.width, 200.0, "the row is as wide as the list: {row:?}");
    }

    /// Item 0 sits at the **bottom** of a reversed list, and item 1 above it — which is
    /// what lets index 0 be the newest message.
    #[test]
    fn a_reversed_list_builds_from_the_bottom() {
        let colour = |i: usize| {
            if i == 0 {
                Color::rgb(1.0, 0.0, 0.0)
            } else {
                Color::rgb(0.0, 0.0, 1.0)
            }
        };
        let list = List::<()>::new(3, 40.0, move |i| {
            Container::<()>::new().height(40.0).color(colour(i))
        })
        .reverse()
        .width(100.0)
        .height(200.0);
        let ui = build_ui(
            &list,
            Size::new(100.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let rects: Vec<Rect> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.b > 0.9 || color.r > 0.9 => {
                    Some(*rect)
                }
                _ => None,
            })
            .collect();
        let first = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.9 && color.b < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("item 0 is painted");
        assert_eq!(
            first.y + first.height,
            200.0,
            "item 0 at the bottom: {first:?}"
        );
        assert_eq!(rects.len(), 3, "all three fit");
        // And item 1 is directly above it.
        let above = rects
            .iter()
            .filter(|r| r.y < first.y)
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("something above");
        assert_eq!(above.y + above.height, first.y);
    }

    /// Three items of 40 in a viewport of 200 do not fill it, and a reversed list puts
    /// them against the bottom rather than leaving them at the top.
    #[test]
    fn a_short_reversed_list_rests_at_the_bottom() {
        let plain = List::<()>::new(3, 40.0, |_| {
            Container::<()>::new()
                .height(40.0)
                .color(Color::rgb(1.0, 0.0, 0.0))
        })
        .width(100.0)
        .height(200.0);
        let top_of = |list: &List<()>| {
            let ui = build_ui(
                list,
                Size::new(100.0, 200.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if color.r > 0.9 => Some(rect.y),
                    _ => None,
                })
                .fold(f32::MAX, f32::min)
        };
        assert_eq!(top_of(&plain), 0.0);

        let reversed = List::<()>::new(3, 40.0, |_| {
            Container::<()>::new()
                .height(40.0)
                .color(Color::rgb(1.0, 0.0, 0.0))
        })
        .reverse()
        .width(100.0)
        .height(200.0);
        assert_eq!(top_of(&reversed), 80.0, "200 - 3 x 40");
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

#[cfg(test)]
mod padding_tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    const MARK: Color = Color::rgb(1.0, 0.0, 0.0);

    /// The rectangles the list painted for its items, in the order they came out.
    fn items(list: &List<()>, size: Size) -> Vec<Rect> {
        build_ui(list, size, &Runtime::default(), &Theme::default())
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if *color == MARK => Some(*rect),
                _ => None,
            })
            .collect()
    }

    fn list(count: usize) -> List<()> {
        List::<()>::new(count, 40.0, |_| {
            Container::<()>::new().height(40.0).color(MARK)
        })
        .width(100.0)
        .height(200.0)
    }

    /// The leading inset pushes item 0 in, and the sides inset every item.
    #[test]
    fn the_first_item_clears_the_leading_inset() {
        let plain = items(&list(5), Size::new(100.0, 200.0));
        assert_eq!((plain[0].y, plain[0].x, plain[0].width), (0.0, 0.0, 100.0));

        let padded = items(
            &list(5).padding_each(12.0, 8.0, 24.0, 8.0),
            Size::new(100.0, 200.0),
        );
        assert_eq!(padded[0].y, 12.0, "item 0 starts after the top inset");
        assert_eq!(padded[0].x, 8.0, "and inside the left one");
        assert_eq!(padded[0].width, 84.0, "the sides come off the width");
        assert_eq!(padded[1].y, 52.0, "the pitch is unchanged");
    }

    /// The room is **inside** the viewport and scrolls with the items, so it is added to
    /// what there is to scroll rather than taken out of the window.
    #[test]
    fn the_far_inset_is_reachable_rather_than_lost() {
        let bare = build_ui(
            &list(5),
            Size::new(100.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // 5 x 40 = 200 of content in a 200 viewport: nothing to scroll.
        assert_eq!(bare.scroll_regions()[0].max_y, 0.0);

        let padded = build_ui(
            &list(5).padding_each(12.0, 0.0, 24.0, 0.0),
            Size::new(100.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(
            padded.scroll_regions()[0].max_y,
            36.0,
            "both insets joined the content"
        );
    }

    /// A reversed list starts at the bottom, so the **bottom** inset is the one item 0
    /// clears — and it is still at the bottom, where it looks.
    #[test]
    fn a_reversed_list_clears_its_bottom_inset() {
        let padded = items(
            &list(3).reverse().padding_each(12.0, 0.0, 24.0, 0.0),
            Size::new(100.0, 200.0),
        );
        let first = padded
            .iter()
            .copied()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("item 0 is painted");
        assert_eq!(
            first.y + first.height,
            176.0,
            "the bottom of item 0 sits one bottom inset above the viewport's"
        );
    }

    /// The window is still only what fits: a leading inset moves it, it does not widen it.
    #[test]
    fn the_padding_does_not_widen_the_window() {
        let built = items(&list(5000).padding(16.0), Size::new(100.0, 200.0));
        assert!(built.len() <= 8, "only the visible items: {}", built.len());
    }
}

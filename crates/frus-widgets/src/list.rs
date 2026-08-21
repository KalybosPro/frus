//! [`ListView`]: a **virtualised** list — only the **visible** items are built, laid
//! out and painted. Essential for large lists (thousands of rows): the per-frame
//! cost is proportional to the visible items, not to the total.
//!
//! Contract: a **fixed** item extent along the axis the list runs — down the screen by
//! default, across it with [`ListView::axis`]. Items are built
//! on demand by an `index → widget` closure, so they have **no retained state**
//! and take no keyboard focus (you cannot retain the state of an item that does
//! not exist off screen) — perfect for display (logs, tables, long lists), and
//! still clickable and hoverable while visible.

use frus_core::{Insets, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::physics::ScrollPhysics;
use crate::scroll::Axis;
use crate::theme::Theme;
use crate::widget::Widget;

/// The description of a virtualised list, exposed to the render driver.
pub struct VirtualList<'a, Msg> {
    /// Total number of items.
    pub count: usize,
    /// The (logical) extent of one item **along the axis the list scrolls**: its
    /// height down a vertical list, its width across a horizontal one.
    pub item_extent: f32,
    /// Which way the list runs.
    pub axis: Axis,
    /// Builds one item per index.
    pub build: &'a dyn Fn(usize) -> Box<dyn Widget<Msg>>,
}

/// A virtualised list with a fixed item extent.
pub struct ListView<Msg> {
    count: usize,
    item_extent: f32,
    axis: Axis,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    physics: Option<ScrollPhysics>,
    reverse: bool,
    /// Room around the items, inside the viewport; see [`ListView::padding`].
    padding: Insets,
    build: Box<dyn Fn(usize) -> Box<dyn Widget<Msg>>>,
}

impl<Msg> ListView<Msg> {
    /// Creates a list of `count` items of extent `item_extent`, each item being
    /// built on demand by `build(index)`.
    ///
    /// The extent is measured **along the axis the list scrolls** — a height by
    /// default, a width once [`ListView::axis`] says so. The cross axis is handed to
    /// the item whole.
    pub fn new<W: Widget<Msg> + 'static>(
        count: usize,
        item_extent: f32,
        build: impl Fn(usize) -> W + 'static,
    ) -> Self {
        Self {
            count,
            item_extent,
            axis: Axis::Vertical,
            width: Dimension::Auto,
            height: Dimension::Length(200.0),
            flex_grow: 0.0,
            physics: None,
            reverse: false,
            padding: Insets::ZERO,
            build: Box::new(move |index| Box::new(build(index)) as Box<dyn Widget<Msg>>),
        }
    }

    /// Which way the list runs: down the screen (the default) or across it.
    ///
    /// A row of cards, a strip of thumbnails, a shelf of covers — all of them are this
    /// list turned on its side, and all of them want the same virtualisation, since a
    /// shelf of two hundred covers is as much work as a column of two hundred rows.
    ///
    /// A list virtualises along **one** axis: it knows where item `n` starts because
    /// every item is the same extent in that one direction. [`Axis::Both`] has no
    /// meaning here and is read as vertical; a surface that scrolls both ways is
    /// [`crate::SingleChildScrollView`], which does not virtualise and does not need to.
    ///
    /// The extent given to [`ListView::new`] follows the axis — a height down, a width
    /// across — and so does [`ListView::reverse`], which is why it is the same builder
    /// rather than two.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Builds **from the far end**: item 0 sits at the end the axis finishes at — the
    /// bottom of a vertical list, the trailing edge of a horizontal one — item 1 before
    /// it, and the list starts resting there.
    ///
    /// The other half of a conversation, and the half [`crate::SingleChildScrollView::reverse`] cannot
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
    /// # use frus_widgets::{Container, ListView};
    /// ListView::<()>::new(200, 56.0, |_| Container::<()>::new()).padding(16.0);
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
    /// [`crate::scroll::SingleChildScrollView::physics`].
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

impl<Msg> Widget<Msg> for ListView<Msg> {
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
            item_extent: self.item_extent,
            axis: self.axis,
            build: &*self.build,
        })
    }

    fn scroll_axis(&self) -> Axis {
        self.axis
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
        let list = ListView::<()>::new(5000, 40.0, move |_i| {
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
        let list = ListView::<()>::new(20, 40.0, |i| {
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
        let list = ListView::<()>::new(3, 40.0, move |i| {
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
        let plain = ListView::<()>::new(3, 40.0, |_| {
            Container::<()>::new()
                .height(40.0)
                .color(Color::rgb(1.0, 0.0, 0.0))
        })
        .width(100.0)
        .height(200.0);
        let top_of = |list: &ListView<()>| {
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

        let reversed = ListView::<()>::new(3, 40.0, |_| {
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
        let list = ListView::<()>::new(100, 40.0, |_i| Container::<()>::new().height(40.0))
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
        let list = ListView::<()>::new(50, 30.0, |i| {
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
    fn items(list: &ListView<()>, size: Size) -> Vec<Rect> {
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

    fn list(count: usize) -> ListView<()> {
        ListView::<()>::new(count, 40.0, |_| {
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

/// A list turned on its side. Everything a vertical list does, across.
#[cfg(test)]
mod across_tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive, Rect};

    const MARK: Color = Color::rgb(0.0, 1.0, 0.0);

    /// The rectangles a horizontal list painted, in the order they came out.
    fn shelf(list: &ListView<()>, size: Size) -> Vec<Rect> {
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

    /// A shelf of `count` cards, 50 wide, in a 200x100 viewport.
    fn shelf_of(count: usize) -> ListView<()> {
        ListView::<()>::new(count, 50.0, |_| Container::<()>::new().color(MARK))
            .axis(Axis::Horizontal)
            .width(200.0)
            .height(100.0)
    }

    /// Cards run **across**, each one item-extent wide, side by side from the left.
    #[test]
    fn the_items_run_across_rather_than_down() {
        let cards = shelf(&shelf_of(3), Size::new(200.0, 100.0));
        assert_eq!(cards.len(), 3);
        for (i, card) in cards.iter().enumerate() {
            assert_eq!(card.x, i as f32 * 50.0, "card {i} sits along the row");
            assert_eq!(card.y, 0.0, "and none of them moves down");
            assert_eq!(card.width, 50.0, "the extent is a width here");
        }
    }

    /// The **cross** axis is handed over whole, exactly as a vertical list hands over
    /// its width. A card whose height nobody set is as tall as the shelf, rather than
    /// hugging whatever is inside it -- which is milestone 351's finding, in the other
    /// direction.
    #[test]
    fn an_item_is_handed_the_shelf_s_height() {
        let cards = shelf(&shelf_of(2), Size::new(200.0, 100.0));
        assert_eq!(cards[0].height, 100.0);
    }

    /// **Virtualised**, which is the whole point of doing this here rather than in a
    /// `Flex` inside a scroll: a shelf of two hundred covers builds the four that show.
    #[test]
    fn only_the_visible_cards_are_built() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let built = Rc::new(RefCell::new(Vec::new()));
        let seen = built.clone();
        let list = ListView::<()>::new(200, 50.0, move |i| {
            seen.borrow_mut().push(i);
            Container::<()>::new().color(MARK)
        })
        .axis(Axis::Horizontal)
        .width(200.0)
        .height(100.0);
        let _ = build_ui(
            &list,
            Size::new(200.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(
            *built.borrow(),
            vec![0, 1, 2, 3],
            "four fit, four are built"
        );
    }

    /// The scrollable it registers runs along **x**, and leaves **y** alone. Getting
    /// this the wrong way round is the bug that would let a shelf scroll vertically and
    /// not horizontally, which is worse than not scrolling at all.
    #[test]
    fn it_scrolls_along_x_and_not_along_y() {
        let ui = build_ui(
            &shelf_of(10),
            Size::new(200.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Ten cards of 50 is 500, in a 200-wide window.
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1);
        assert_eq!(maxes[0].1, 300.0, "it scrolls along x");
        assert_eq!(maxes[0].2, 0.0, "and not along y");
    }

    /// Padding follows the axis: the leading inset pushes the first card along, and the
    /// cross insets take height off every card rather than width.
    #[test]
    fn the_insets_follow_the_axis() {
        let padded = shelf(
            &shelf_of(3).padding_each(10.0, 4.0, 6.0, 8.0),
            Size::new(200.0, 100.0),
        );
        assert_eq!(padded[0].x, 8.0, "the left inset leads");
        assert_eq!(padded[0].y, 10.0, "the top inset is a cross inset here");
        assert_eq!(
            padded[0].height,
            100.0 - 10.0 - 6.0,
            "top and bottom come off"
        );
        assert_eq!(padded[0].width, 50.0, "the extent is untouched by padding");
    }

    /// Reversed, a shelf starts at its **trailing** edge: card 0 sits at the right of
    /// the viewport, card 1 to its left. The same conversation a reversed column has
    /// with the bottom of the screen.
    #[test]
    fn a_reversed_shelf_starts_at_the_far_edge() {
        let cards = shelf(&shelf_of(3).reverse(), Size::new(200.0, 100.0));
        assert_eq!(
            cards[0].x + cards[0].width,
            200.0,
            "card 0 ends at the edge"
        );
        assert_eq!(cards[1].x + cards[1].width, 150.0, "card 1 is before it");
    }

    /// It actually **scrolls**. Everything above is layout at rest; this drives the
    /// offset the shell would set and watches the window move.
    ///
    /// A shelf that lays out correctly and does not move is the failure worth guarding
    /// against, because every assertion up to here would still pass.
    #[test]
    fn scrolling_it_moves_the_window_along_x() {
        let mut runtime = Runtime::default();
        runtime
            .scroll
            .insert(crate::interaction::WidgetId::ROOT, (120.0, 0.0));
        let list = shelf_of(10);
        let cards: Vec<Rect> =
            build_ui(&list, Size::new(200.0, 100.0), &runtime, &Theme::default())
                .scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if *color == MARK => Some(*rect),
                    _ => None,
                })
                .collect();
        // 120 in, at 50 a card: cards 2..=5 are the ones on screen, and card 2 starts
        // 20 px before the left edge.
        assert_eq!(cards.first().map(|c| c.x), Some(-20.0));
        assert!(
            cards.iter().all(|c| c.y == 0.0),
            "and nothing drifted down the cross axis"
        );
    }

    /// And a vertical list is untouched by any of it -- the default is what it was.
    #[test]
    fn a_list_that_says_nothing_still_runs_down() {
        let rows = shelf(
            &ListView::<()>::new(3, 50.0, |_| Container::<()>::new().color(MARK))
                .width(200.0)
                .height(100.0),
            Size::new(200.0, 100.0),
        );
        assert_eq!(rows[0].y, 0.0);
        assert_eq!(rows[1].y, 50.0, "the second is below, not beside");
        assert_eq!(rows[0].width, 200.0, "and the width is handed over whole");
    }
}

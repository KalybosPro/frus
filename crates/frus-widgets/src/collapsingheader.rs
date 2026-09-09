//! [`CollapsingHeader`]: a bar that is tall at the top of a page and shrinks to a plain
//! toolbar as the page scrolls, then stays there.
//!
//! The composition [`ScrollOverlay`](crate::ScrollOverlay) exists for, and the one every
//! application writes: a header with a picture or an oversized title in it, a long list
//! under it, and the first turning into the second as the second moves.
//!
//! ```ignore
//! CollapsingHeader::new(list, |state| header(state.fraction))
//!     .collapsed_height(56.0)
//!     .build()
//! ```
//!
//! ## Where the header's room comes from
//!
//! **The body's own top padding**, which already means "room at the top of this list"
//! and already scrolls with the content. So the expanded height is stated once, in the
//! place that was going to state it anyway, and the header and the space it needs cannot
//! drift apart:
//!
//! ```text
//! ListView::new(…).padding_each(220.0, 0.0, 0.0, 0.0)   // the header's room
//! ```
//!
//! [`CollapsingHeader::expanded_height`] overrides it for a body that has other reasons
//! for its padding, and a body that scrolls with no room at the top gets a header that is
//! already collapsed — which is what a zero-height expansion means and not a bug.
//!
//! ## What it does not do
//!
//! It does not make a page out of a list, a grid and another list scrolling as one. That
//! wants a viewport laying out a sequence of scrollable pieces, each given a remaining
//! extent and each reporting what it consumed, and it is a much larger piece of work —
//! see the roadmap. This is the answer to the two results that do not need one, chosen as
//! an answer rather than as a down payment on that.

use frus_core::{Rect, Size};

use crate::container::Container;
use crate::rowcolumn::Column;
use crate::scrolloverlay::ScrollOverlay;
use crate::widget::Widget;

/// How far collapsed the header is, handed to its builder every frame it moves.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HeaderState {
    /// `0.0` fully expanded, `1.0` fully collapsed. **What a header should be written
    /// against**: a size, an opacity or a corner interpolated on this reads the same
    /// whatever the two heights happen to be, and survives somebody changing them.
    pub fraction: f32,
    /// The height the header is being drawn at, in logical pixels.
    pub height: f32,
    /// How wide it is — the page, not the title.
    ///
    /// Handed over because a header **has** to state it: a box left to size itself is as
    /// wide as its own content, which is a band with the list showing either side of it.
    /// The first golden of this widget was exactly that, and no test measuring heights
    /// could have seen it.
    pub width: f32,
    /// The height it has when nothing has scrolled.
    pub expanded_height: f32,
    /// The height it stops at, or `0` for a header that goes entirely.
    pub collapsed_height: f32,
}

/// A header over a scroll region, interpolated between two heights by the region's offset.
pub struct CollapsingHeader<Msg> {
    body: Box<dyn Widget<Msg>>,
    build: Box<dyn Fn(HeaderState) -> Box<dyn Widget<Msg>>>,
    expanded: Option<f32>,
    collapsed: f32,
    pinned: bool,
}

/// The height a collapsed header stops at, unless told otherwise: the same toolbar
/// height an application bar has, because what it collapses **to** is a toolbar.
pub const COLLAPSED_HEIGHT: f32 = crate::appbar::APP_BAR_HEIGHT;

impl<Msg: Clone + 'static> CollapsingHeader<Msg> {
    /// A header built from `build`, over the scroll region `body`.
    ///
    /// The expanded height is the body's own top padding; see the module documentation
    /// for why that is the number rather than a second one here.
    pub fn new<W: Widget<Msg> + 'static>(
        body: impl Widget<Msg> + 'static,
        build: impl Fn(HeaderState) -> W + 'static,
    ) -> Self {
        Self {
            body: Box::new(body),
            build: Box::new(move |state| Box::new(build(state)) as Box<dyn Widget<Msg>>),
            expanded: None,
            collapsed: COLLAPSED_HEIGHT,
            pinned: true,
        }
    }

    /// The height when nothing has scrolled, overriding the body's top padding.
    pub fn expanded_height(mut self, height: f32) -> Self {
        self.expanded = Some(height.max(0.0));
        self
    }

    /// The height it stops shrinking at. Ignored by [`CollapsingHeader::floating`].
    pub fn collapsed_height(mut self, height: f32) -> Self {
        self.collapsed = height.max(0.0);
        self
    }

    /// **Goes entirely** instead of stopping at a toolbar, leaving the whole window to
    /// the content — a reading view, where the chrome is worth its room only while
    /// somebody is choosing what to read.
    pub fn floating(mut self) -> Self {
        self.pinned = false;
        self
    }

    /// The widget: the body with the header drawn over it.
    pub fn build(self) -> ScrollOverlay<Msg> {
        // The room the body reserved at its top **is** the expanded height, unless the
        // caller said otherwise. One number rather than two that have to agree.
        let expanded = self
            .expanded
            .unwrap_or_else(|| self.body.scroll_padding().top);
        let collapsed = if self.pinned {
            self.collapsed.min(expanded)
        } else {
            0.0
        };
        // Never nought: a header whose two heights are the same is pinned at one height,
        // and dividing by the difference would make the fraction meaningless rather than
        // constant.
        let range = (expanded - collapsed).max(1.0);
        let build = self.build;
        ScrollOverlay::new(self.body, move |(_, y), box_: Size| {
            // The header gives way pixel for pixel with the content until it reaches its
            // floor. Not a curve: a header that moved at some other rate than the thing
            // under it reads as two pages sliding over each other.
            let height = (expanded - y).max(collapsed);
            let state = HeaderState {
                fraction: ((expanded - height) / range).clamp(0.0, 1.0),
                height,
                width: box_.width,
                expanded_height: expanded,
                collapsed_height: collapsed,
            };
            // A floating header that has gone is **gone**, not a box of no height: an
            // empty column draws nothing and registers nothing, where a zero-height
            // header would still be asked to paint itself every frame.
            let content: Box<dyn Widget<Msg>> = if height <= 0.0 {
                Box::new(Column::new())
            } else {
                Box::new(
                    Container::new()
                        .width(box_.width)
                        .height(height)
                        .child(build(state)),
                )
            };
            // Top of the box and no taller than it needs to be: the rest of the overlay
            // is empty, so everything below the header belongs to the list — a finger
            // there scrolls, and a row there is clickable.
            //
            // **The width is stated, not inherited**, which is why the builder is handed
            // its box at all. A header is a band across the page, and a box left to size
            // itself is as wide as its own title — a header with the list showing either
            // side of it, which is exactly what the first golden of this caught and what
            // no test measuring heights could have.
            Column::new().child(content)
        })
    }
}

/// The header's own box, at the origin — for a caller placing something inside it by hand.
pub fn header_rect(state: HeaderState) -> Rect {
    Rect::new(0.0, 0.0, state.width, state.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::WidgetId;
    use crate::ui::child_id;
    use crate::{build_ui, ListView, Runtime, Size, Theme};
    use frus_core::{Color, Primitive};

    /// A list of a hundred rows reserving 200 px at its top, under a header that paints a
    /// block whose height *is* the header's — so the picture states the number.
    fn screen(expanded: f32, pinned: bool) -> ScrollOverlay<()> {
        let list = ListView::<()>::new(100, 40.0, |_| {
            Container::<()>::new().height(40.0).color(Color::WHITE)
        })
        .width(300.0)
        .height(400.0)
        .padding_each(expanded, 0.0, 0.0, 0.0);
        let header = CollapsingHeader::new(list, |state| {
            Container::<()>::new()
                .width(state.width)
                .height(state.height)
                .color(Color::rgb(1.0, 0.0, 0.0))
        })
        .collapsed_height(60.0);
        if pinned { header } else { header.floating() }.build()
    }

    /// The height of the red block: what the header was actually drawn at.
    fn drawn(expanded: f32, pinned: bool, offset: f32) -> Option<f32> {
        let mut runtime = Runtime::default();
        let tree = screen(expanded, pinned);
        let body = child_id(WidgetId::ROOT, 0, tree.children()[0].as_ref());
        runtime.scroll.insert(body, (0.0, offset));
        let tree = screen(expanded, pinned);
        let ui = build_ui(&tree, Size::new(300.0, 400.0), &runtime, &Theme::dark());
        ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if color.r > 0.9 && color.g < 0.1 => {
                Some(rect.height)
            }
            _ => None,
        })
    }

    /// The header's drawn box, not just its height: a header is a **band across the
    /// page**, and the first golden of this one was a block the width of its own title
    /// with the list showing either side of it. Heights alone never saw it.
    fn drawn_box(expanded: f32, offset: f32) -> Option<frus_core::Rect> {
        let mut runtime = Runtime::default();
        let tree = screen(expanded, true);
        let body = child_id(WidgetId::ROOT, 0, tree.children()[0].as_ref());
        runtime.scroll.insert(body, (0.0, offset));
        let tree = screen(expanded, true);
        let ui = build_ui(&tree, Size::new(300.0, 400.0), &runtime, &Theme::dark());
        ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if color.r > 0.9 && color.g < 0.1 => Some(*rect),
            _ => None,
        })
    }

    #[test]
    fn the_header_is_a_band_across_the_whole_page() {
        let open = drawn_box(200.0, 0.0).expect("drawn");
        assert_eq!(
            open.width, 300.0,
            "a header narrower than the page is a header with the list beside it"
        );
        assert_eq!(open.x, 0.0, "and it starts at the edge");
        // Still true once it has given way: the width is not a function of the height.
        let shut = drawn_box(200.0, 400.0).expect("drawn");
        assert_eq!(shut.width, 300.0);
    }

    #[test]
    fn the_expanded_height_is_the_room_the_body_reserved() {
        // The number is stated once, where it already had to be stated, so the header and
        // the space under it cannot disagree.
        assert_eq!(drawn(200.0, true, 0.0), Some(200.0));
        assert_eq!(drawn(140.0, true, 0.0), Some(140.0));
    }

    #[test]
    fn it_gives_way_pixel_for_pixel_and_then_stops() {
        // Anything else reads as two pages sliding over each other.
        assert_eq!(drawn(200.0, true, 50.0), Some(150.0));
        assert_eq!(drawn(200.0, true, 140.0), Some(60.0));
        // Its floor, and it stays there however far the list goes.
        assert_eq!(drawn(200.0, true, 300.0), Some(60.0));
        assert_eq!(drawn(200.0, true, 3000.0), Some(60.0));
    }

    #[test]
    fn a_floating_header_goes_entirely() {
        assert_eq!(drawn(200.0, false, 120.0), Some(80.0));
        assert_eq!(
            drawn(200.0, false, 260.0),
            None,
            "past its own height there is nothing left to draw"
        );
    }

    #[test]
    fn the_fraction_is_what_a_header_should_be_written_against() {
        // A header interpolating on this reads the same whatever the two heights are,
        // which is what makes it survive somebody changing them.
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let record = std::rc::Rc::clone(&seen);
        let list = ListView::<()>::new(10, 40.0, |_| Container::<()>::new().height(40.0))
            .width(300.0)
            .height(400.0)
            .padding_each(200.0, 0.0, 0.0, 0.0);
        let tree = CollapsingHeader::new(list, move |state| {
            record.borrow_mut().push((state.fraction, state.height));
            Container::<()>::new()
                .width(state.width)
                .height(state.height)
        })
        .collapsed_height(100.0)
        .build();
        let mut runtime = Runtime::default();
        let body = child_id(WidgetId::ROOT, 0, tree.children()[0].as_ref());
        runtime.scroll.insert(body, (0.0, 50.0));
        build_ui(&tree, Size::new(300.0, 400.0), &runtime, &Theme::dark());
        let (fraction, height) = *seen.borrow().first().expect("the header was built");
        assert!((height - 150.0).abs() < 0.01, "{height}");
        assert!(
            (fraction - 0.5).abs() < 0.01,
            "halfway between 200 and 100 is half collapsed: {fraction}"
        );
    }

    #[test]
    fn a_collapsed_height_taller_than_the_expanded_one_is_held_to_it() {
        // A header cannot collapse *upwards*, and the number that gives way is the one
        // the caller typed rather than the one the body reserved.
        assert_eq!(drawn(50.0, true, 0.0), Some(50.0));
        assert_eq!(drawn(50.0, true, 200.0), Some(50.0));
    }
}

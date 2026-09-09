//! **Where a scroll region is**, and **where an application is asking it to go**.
//!
//! Two halves of one gap. A scroll offset lives in the [`Runtime`](crate::Runtime),
//! written by the finger, the fling and the spring; until this module there was no way
//! for an application to read one and no way to move one. So a "back to top" button, a
//! chat that opens at its newest message, a bar that fades in as the page goes down and
//! a list that loads more when its end comes into view were all inexpressible.
//!
//! Neither half is a controller. The reference hands the application an object that
//! *owns* the offset and is asked for it; here the offset belongs to the runtime, a view
//! is rebuilt from state every frame, and an application speaks in messages. So the
//! reading half is a **notification** — [`Widget::on_scroll`](crate::Widget::on_scroll),
//! a message when the offset changes — and the commanding half is a **request** —
//! [`ScrollTo`], carried by a command and named by the same key a focus request uses.

use frus_core::Size;

use crate::physics::ScrollMetrics;
use crate::ui::Scrollable;

/// Where a scroll region is, and how much room it has, at the moment it is reported.
///
/// Handed to [`Widget::on_scroll`](crate::Widget::on_scroll). Everything an application
/// could want to know about a position is derivable from these three, and they are the
/// three the layout already knows — no measurement is done to produce one.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ScrollPosition {
    /// The offsets `(x, y)` the region is resting or moving at. Measured from the end
    /// its axis starts at, so a reversed list at 0 is at its newest end.
    pub offset: (f32, f32),
    /// The largest offsets the content may rest at, per axis. `0` = that axis has
    /// nothing to scroll.
    pub max: (f32, f32),
    /// The window the content is seen through.
    pub viewport: Size,
}

impl ScrollPosition {
    /// The position of `area`, resting at `offset`.
    pub fn of(area: &Scrollable, offset: (f32, f32)) -> Self {
        Self {
            offset,
            max: (area.max_x, area.max_y),
            viewport: Size {
                width: area.viewport.width,
                height: area.viewport.height,
            },
        }
    }

    /// The horizontal axis, as the physics states it.
    pub fn x(&self) -> ScrollMetrics {
        ScrollMetrics::new(self.offset.0, self.max.0, self.viewport.width)
    }

    /// The vertical axis, as the physics states it.
    pub fn y(&self) -> ScrollMetrics {
        ScrollMetrics::new(self.offset.1, self.max.1, self.viewport.height)
    }

    /// How far down the content is, `0.0..=1.0`.
    ///
    /// **Zero when there is nothing to scroll**, which is the only answer that is not a
    /// lie: a list shorter than its window is neither at its beginning nor at its end,
    /// and a progress rule over it should read empty rather than full.
    pub fn fraction_y(&self) -> f32 {
        fraction(self.offset.1, self.max.1)
    }

    /// How far across the content is, `0.0..=1.0`. See [`ScrollPosition::fraction_y`].
    pub fn fraction_x(&self) -> f32 {
        fraction(self.offset.0, self.max.0)
    }

    /// Pixels of content left below the window; `0` at the end, and `0` when there is
    /// nothing to scroll.
    ///
    /// **This is what a load-more asks**: `remaining_y() < 400.0` is "the end is
    /// coming", said in the unit the reader is moving in rather than as a percentage,
    /// which means something quite different in a list of ten and a list of ten
    /// thousand.
    pub fn remaining_y(&self) -> f32 {
        remaining(self.offset.1, self.max.1)
    }

    /// Pixels of content left past the trailing edge. See [`ScrollPosition::remaining_y`].
    pub fn remaining_x(&self) -> f32 {
        remaining(self.offset.0, self.max.0)
    }
}

/// `offset / max`, clamped, and 0 where there is no room to move.
fn fraction(offset: f32, max: f32) -> f32 {
    if max <= 0.0 {
        0.0
    } else {
        (offset / max).clamp(0.0, 1.0)
    }
}

/// What is left of an axis past `offset`, never negative and never more than the axis.
fn remaining(offset: f32, max: f32) -> f32 {
    (max - offset).clamp(0.0, max.max(0.0))
}

/// One axis of a [`ScrollTo`]: an offset, or an end named rather than measured.
///
/// `End` exists because the number it stands for is not knowable where the request is
/// written: `update` has the state, and the largest offset a region may rest at is a
/// fact about a layout that has not happened yet. Naming the end and letting the frame
/// resolve it is the difference between "the bottom" and a number that was the bottom
/// one frame ago.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ScrollTarget {
    /// Offset 0 — the end the axis **starts** at. On a reversed list that is the end it
    /// opened at, which is the point of reversing it.
    Start,
    /// As far as the content goes.
    End,
    /// This offset exactly, clamped to what the content actually has.
    Offset(f32),
}

impl ScrollTarget {
    /// The offset this names, on an axis whose largest is `max`.
    pub fn resolve(self, max: f32) -> f32 {
        let max = max.max(0.0);
        match self {
            ScrollTarget::Start => 0.0,
            ScrollTarget::End => max,
            ScrollTarget::Offset(offset) => offset.clamp(0.0, max),
        }
    }
}

/// **A request to move a scroll region**, addressed to one axis or both.
///
/// `None` on an axis leaves that axis where it is, which is what lets one type serve a
/// vertical page, a sideways strip and the rare surface that scrolls both ways without a
/// caller ever writing a coordinate it did not mean.
///
/// Carried by a command, resolved against the frame that follows it, and **refused by a
/// region a finger is holding** — an offset has one owner at a time, and the finger is
/// the one the reader can see.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct ScrollTo {
    /// Where the horizontal axis is asked to go; `None` leaves it alone.
    pub x: Option<ScrollTarget>,
    /// Where the vertical axis is asked to go; `None` leaves it alone.
    pub y: Option<ScrollTarget>,
    /// Spring to it (what every constructor here sets) rather than arrive there.
    pub animate: bool,
}

impl ScrollTo {
    /// To offset `offset` down the page, with an animation.
    pub fn y(offset: f32) -> Self {
        Self {
            y: Some(ScrollTarget::Offset(offset)),
            animate: true,
            ..Self::default()
        }
    }

    /// To offset `offset` across, with an animation.
    pub fn x(offset: f32) -> Self {
        Self {
            x: Some(ScrollTarget::Offset(offset)),
            animate: true,
            ..Self::default()
        }
    }

    /// Back to where the vertical axis starts — the "back to top" button.
    pub fn start() -> Self {
        Self {
            y: Some(ScrollTarget::Start),
            animate: true,
            ..Self::default()
        }
    }

    /// To the far end of the vertical axis, wherever the content has got to.
    pub fn end() -> Self {
        Self {
            y: Some(ScrollTarget::End),
            animate: true,
            ..Self::default()
        }
    }

    /// Also moves the horizontal axis to `target`.
    pub fn and_x(mut self, target: ScrollTarget) -> Self {
        self.x = Some(target);
        self
    }

    /// Also moves the vertical axis to `target`.
    pub fn and_y(mut self, target: ScrollTarget) -> Self {
        self.y = Some(target);
        self
    }

    /// **Arrive** rather than travel: no animation, the offset is simply there.
    ///
    /// What restoring a reader's place wants. An animation there would be a list
    /// scrolling itself in front of somebody who has just come back and is looking for
    /// where they left off, and the reference draws the same line between its two verbs.
    pub fn instant(mut self) -> Self {
        self.animate = false;
        self
    }

    /// Nothing to do: neither axis was named.
    pub fn is_empty(&self) -> bool {
        self.x.is_none() && self.y.is_none()
    }

    /// The offsets this request means for `area`, starting from `current`.
    pub fn resolve(&self, area: &Scrollable, current: (f32, f32)) -> (f32, f32) {
        (
            self.x.map_or(current.0, |t| t.resolve(area.max_x)),
            self.y.map_or(current.1, |t| t.resolve(area.max_y)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::WidgetId;
    use frus_core::Rect;

    pub(super) fn area(max_x: f32, max_y: f32) -> Scrollable {
        Scrollable {
            id: WidgetId::from_u64(1),
            viewport: Rect {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 500.0,
            },
            max_x,
            max_y,
            physics: None,
            refresh: None,
            page: None,
            reverse_x: false,
            reverse_y: false,
            keep_visible: None,
            host: None,
        }
    }

    #[test]
    fn a_fraction_of_nothing_is_zero_and_not_one() {
        // A list shorter than its window is not "fully scrolled": a progress rule over
        // it must read empty, or every short page would claim to be finished.
        let position = ScrollPosition::of(&area(0.0, 0.0), (0.0, 0.0));
        assert_eq!(position.fraction_y(), 0.0);
        assert_eq!(position.fraction_x(), 0.0);
        assert_eq!(position.remaining_y(), 0.0);
    }

    #[test]
    fn a_fraction_is_clamped_through_an_overscroll() {
        // Dragged past the edge, the offset is out of range; the fraction is not.
        let position = ScrollPosition::of(&area(0.0, 1000.0), (0.0, -80.0));
        assert_eq!(position.fraction_y(), 0.0);
        let position = ScrollPosition::of(&area(0.0, 1000.0), (0.0, 1080.0));
        assert_eq!(position.fraction_y(), 1.0);
        assert_eq!(
            position.remaining_y(),
            0.0,
            "past the end is not more to go"
        );
    }

    #[test]
    fn remaining_is_measured_in_pixels_the_reader_moves_in() {
        let position = ScrollPosition::of(&area(0.0, 1000.0), (0.0, 700.0));
        assert_eq!(position.remaining_y(), 300.0);
        assert_eq!(position.y().max, 1000.0);
        assert_eq!(position.y().viewport, 500.0);
    }

    #[test]
    fn an_end_is_resolved_against_the_frame_and_not_the_request() {
        // The whole reason `End` is a name and not a number: what it means changes as
        // the content does, and the request outlives the state it was written in.
        assert_eq!(ScrollTarget::End.resolve(1200.0), 1200.0);
        assert_eq!(ScrollTarget::End.resolve(2400.0), 2400.0);
        assert_eq!(ScrollTarget::Start.resolve(2400.0), 0.0);
    }

    #[test]
    fn an_offset_is_clamped_to_what_the_content_has() {
        assert_eq!(ScrollTarget::Offset(9999.0).resolve(300.0), 300.0);
        assert_eq!(ScrollTarget::Offset(-40.0).resolve(300.0), 0.0);
        // A region with nothing to scroll takes every request to 0 rather than refusing
        // it: the request is satisfied, there simply is nowhere else to be.
        assert_eq!(ScrollTarget::Offset(120.0).resolve(0.0), 0.0);
    }

    #[test]
    fn an_unnamed_axis_is_left_alone() {
        let area = area(400.0, 1000.0);
        let request = ScrollTo::start();
        assert_eq!(request.resolve(&area, (250.0, 800.0)), (250.0, 0.0));
        let both = ScrollTo::start().and_x(ScrollTarget::End);
        assert_eq!(both.resolve(&area, (250.0, 800.0)), (400.0, 0.0));
    }

    #[test]
    fn a_request_animates_unless_it_is_told_not_to() {
        assert!(ScrollTo::end().animate);
        assert!(!ScrollTo::end().instant().animate);
        assert!(ScrollTo::default().is_empty());
        assert!(!ScrollTo::y(10.0).is_empty());
    }
}

/// The rules of [`crate::Runtime::scroll_changes`] and [`crate::Runtime::scroll_to`],
/// driven through a real runtime — the reporting and the commanding, which are the two
/// halves this module exists for.
#[cfg(test)]
mod runtime_tests {
    use super::tests::area;
    use super::*;
    use crate::runtime::{Runtime, ScrollBallistic};

    /// Every region reports every change — the default a widget that declares
    /// `on_scroll` and nothing else gets.
    fn every(_: crate::interaction::WidgetId) -> f32 {
        0.0
    }

    #[test]
    fn a_region_appearing_at_the_top_is_not_a_movement() {
        // Told on the first frame of every screen, an application would answer, and it
        // already knew: a list nobody has touched is at the top.
        let mut runtime = Runtime::default();
        let regions = [area(0.0, 1000.0)];
        assert!(runtime.scroll_changes(&regions, every).is_empty());
        // And having been recorded silently, it does not go off on the next frame either.
        assert!(runtime.scroll_changes(&regions, every).is_empty());
    }

    #[test]
    fn a_region_appearing_somewhere_else_is_news() {
        // Restored, or opened at a box it was asked to keep in view. The application did
        // not put it there and has no other way of finding out.
        let mut runtime = Runtime::default();
        let regions = [area(0.0, 1000.0)];
        runtime.scroll.insert(regions[0].id, (0.0, 320.0));
        let changes = runtime.scroll_changes(&regions, every);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.offset, (0.0, 320.0));
        assert_eq!(changes[0].1.max, (0.0, 1000.0));
    }

    #[test]
    fn a_region_that_has_not_moved_says_nothing() {
        let mut runtime = Runtime::default();
        let regions = [area(0.0, 1000.0)];
        runtime.scroll.insert(regions[0].id, (0.0, 320.0));
        assert_eq!(runtime.scroll_changes(&regions, every).len(), 1);
        assert!(runtime.scroll_changes(&regions, every).is_empty());
    }

    #[test]
    fn a_grain_measures_from_the_last_report_and_not_the_last_frame() {
        // The failure this rule exists to stop: a slow drag creeping the whole way down
        // a list, three pixels a frame, without ever travelling a whole grain **in one
        // frame** — and so never reported at all. Measured from what was last said, the
        // fourth frame crosses ten and the report arrives.
        let mut runtime = Runtime::default();
        let regions = [area(0.0, 1000.0)];
        let id = regions[0].id;
        // Still under the finger: not at rest, so only the grain can let a report out.
        runtime.hold_scroll(id);
        runtime.scroll_changes(&regions, |_| 10.0);
        for step in 1..=3 {
            runtime.scroll.insert(id, (0.0, 3.0 * step as f32));
            assert!(
                runtime.scroll_changes(&regions, |_| 10.0).is_empty(),
                "3 px is under the grain"
            );
        }
        runtime.scroll.insert(id, (0.0, 12.0));
        let changes = runtime.scroll_changes(&regions, |_| 10.0);
        assert_eq!(changes.len(), 1, "12 px from the last report is over it");
        assert_eq!(changes[0].1.offset, (0.0, 12.0));
    }

    #[test]
    fn a_grain_never_swallows_where_it_came_to_rest() {
        // A coarse grain costs frames, never accuracy: the last step is reported however
        // short it was, or the application would be left believing a list stopped
        // somewhere it did not.
        let mut runtime = Runtime::default();
        let regions = [area(0.0, 1000.0)];
        let id = regions[0].id;
        runtime.hold_scroll(id);
        runtime.scroll_changes(&regions, |_| 100.0);
        runtime.scroll.insert(id, (0.0, 7.0));
        assert!(runtime.scroll_changes(&regions, |_| 100.0).is_empty());
        // The finger lets go and nothing is driving it any more.
        runtime.release_scroll(id);
        let changes = runtime.scroll_changes(&regions, |_| 100.0);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.offset, (0.0, 7.0));
    }

    #[test]
    fn a_region_that_has_gone_is_forgotten() {
        // Otherwise the map grows by one entry per screen ever opened, and a region
        // whose identity came round again would be measured against a stale offset.
        let mut runtime = Runtime::default();
        let regions = [area(0.0, 1000.0)];
        runtime.scroll.insert(regions[0].id, (0.0, 40.0));
        runtime.scroll_changes(&regions, every);
        assert!(runtime.scroll_reported.contains_key(&regions[0].id));
        runtime.scroll_changes(&[], every);
        assert!(runtime.scroll_reported.is_empty());
    }

    #[test]
    fn an_animated_request_moves_the_target_and_a_jump_moves_the_offset() {
        let mut runtime = Runtime::default();
        let area = area(0.0, 1000.0);
        runtime.scroll.insert(area.id, (0.0, 900.0));

        assert!(runtime.scroll_to(&area, ScrollTo::start()));
        assert_eq!(runtime.scroll_target.get(&area.id), Some(&(0.0, 0.0)));
        assert_eq!(
            runtime.scroll.get(&area.id),
            Some(&(0.0, 900.0)),
            "an animated request does not teleport: the spring runs"
        );

        assert!(runtime.scroll_to(&area, ScrollTo::start().instant()));
        assert_eq!(runtime.scroll.get(&area.id), Some(&(0.0, 0.0)));
        assert!(
            !runtime.scroll_target.contains_key(&area.id),
            "there is nothing left to spring towards"
        );
    }

    #[test]
    fn an_end_is_resolved_against_the_region_it_reaches() {
        let mut runtime = Runtime::default();
        let area = area(0.0, 1240.0);
        assert!(runtime.scroll_to(&area, ScrollTo::end().instant()));
        assert_eq!(runtime.scroll.get(&area.id), Some(&(0.0, 1240.0)));
    }

    #[test]
    fn a_finger_refuses_a_request() {
        // An offset has one owner at a time, and while a hand is on the content that
        // owner is the hand. A list that jumped out from under a finger because a timer
        // fired would be the framework overruling the reader.
        let mut runtime = Runtime::default();
        let area = area(0.0, 1000.0);
        runtime.scroll.insert(area.id, (0.0, 500.0));
        runtime.hold_scroll(area.id);
        assert!(!runtime.scroll_to(&area, ScrollTo::start().instant()));
        assert_eq!(runtime.scroll.get(&area.id), Some(&(0.0, 500.0)));
        runtime.release_scroll(area.id);
        assert!(runtime.scroll_to(&area, ScrollTo::start().instant()));
    }

    #[test]
    fn a_request_catches_a_fling() {
        // A fling is where the list was going; the request is where it is now being
        // asked to be, and it is the more recent statement.
        let mut runtime = Runtime::default();
        let area = area(0.0, 1000.0);
        runtime
            .scroll_ballistic
            .insert(area.id, ScrollBallistic::default());
        assert!(runtime.scroll_to(&area, ScrollTo::start()));
        assert!(!runtime.scroll_ballistic.contains_key(&area.id));
    }

    #[test]
    fn a_request_naming_no_axis_does_nothing_at_all() {
        let mut runtime = Runtime::default();
        let area = area(0.0, 1000.0);
        assert!(!runtime.scroll_to(&area, ScrollTo::default()));
        assert!(runtime.scroll_target.is_empty());
        assert!(runtime.scroll.is_empty());
    }
}

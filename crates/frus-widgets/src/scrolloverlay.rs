//! [`ScrollOverlay`]: something drawn **over** a scroll region, built from where that
//! region has got to.
//!
//! A bar that is tall with a picture in it at the top of a page and shrinks to a plain
//! toolbar as you scroll; a rule that fills as the page goes down; a shadow that appears
//! under a header the moment anything has scrolled beneath it. All of them are the same
//! shape: a thing over a list, redrawn as the list moves.
//!
//! ## Why this rather than a message
//!
//! [`Widget::on_scroll`] can already tell an application where
//! a region is, and an application could keep that in its state and build a header from
//! it. It would work, and it would be wrong for this: the offset would go out as a
//! message, come back as a rebuild of the **whole** view, and arrive a frame late — a
//! header that lags the list it is attached to by one frame, at every frame of every
//! fling.
//!
//! The division is worth stating plainly, because both exist and they are not
//! substitutes:
//!
//! - **`on_scroll` is for what changes the application's state** — loading the next page,
//!   remembering where somebody was. It reports the extents as well as the offset, it
//!   costs a message, and what it changes is *true* between frames.
//! - **`ScrollOverlay` is for what changes only the picture.** It is handed the offset
//!   during the frame that draws it, so nothing lags and nothing round-trips, and it
//!   knows nothing but the offset — see below.
//!
//! ## What it is handed, and what it is not
//!
//! **The offset, and its own box** — the same box a [`LayoutBuilder`](crate::LayoutBuilder)
//! is handed, and for the same reason: an overlay that does not know how wide it is cannot
//! be a band across the page, and a box left to size itself is as wide as its title.
//!
//! **Not the extents.** How far the content *can* scroll is a fact about a layout that has
//! not finished — this builder runs inside it — and a number handed over here would be the
//! previous frame's without being able to say so. An overlay that genuinely needs "how near
//! the end are we" reads it through `on_scroll`, which is measured, and keeps it in the
//! application's state where a measured thing belongs.
//!
//! ## The price, which is the same one `LayoutBuilder` pays
//!
//! The overlay is **rebuilt every frame the region moves**, so it has **no retained
//! state**: hover and clicks work, persistent keyboard focus and deferred overlays do
//! not. Put a menu in the body, not in the overlay. The body itself is an ordinary child
//! and keeps everything.
//!
//! ```ignore
//! ScrollOverlay::new(list, |(_, y), box_| {
//!     let collapsed = (y / 160.0).clamp(0.0, 1.0);
//!     Column::new().child(header(collapsed, box_.width))
//! })
//! ```

use frus_core::{Rect, Scene, Size};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// What an overlay is built by: `offset → widget`, run once per frame the region moves.
///
/// Named because it appears in [`Widget::scroll_overlay`]
/// as well, and a signature spelled out twice is a signature that can be spelled
/// differently twice.
pub type OverlayBuilder<Msg> = dyn Fn((f32, f32), Size) -> Box<dyn Widget<Msg>>;

/// A scroll region with something drawn over it that knows where it has got to.
///
/// The region is the **child**, which is the whole of the identity design: the offset
/// this reads is its own child's, so nothing has to be named, looked up, or kept in step.
/// A builder floating free of the region it watches would need one of those three, and
/// each of them can go stale without saying so.
pub struct ScrollOverlay<Msg> {
    /// The scroll region, as the one-element slice [`Widget::children`] hands back.
    body: Vec<Box<dyn Widget<Msg>>>,
    /// `(offset, box) → overlay`, run once per frame the region moves.
    build: Box<OverlayBuilder<Msg>>,
}

impl<Msg> ScrollOverlay<Msg> {
    /// Draws `build(offset)` over `body`, a scroll region.
    ///
    /// The offset is `(x, y)`, measured from the end each axis starts at — the same
    /// numbers [`crate::ScrollPosition::offset`] carries, and the same ones a reversed
    /// list counts from its newest end.
    ///
    /// `body` does not have to be a scroll region: one that is not simply never moves,
    /// and the overlay is built at `(0, 0)` for ever. That is a useless composition
    /// rather than a broken one, which is the right way round.
    pub fn new<W: Widget<Msg> + 'static>(
        body: impl Widget<Msg> + 'static,
        build: impl Fn((f32, f32), Size) -> W + 'static,
    ) -> Self {
        Self {
            body: vec![Box::new(body)],
            build: Box::new(move |offset, size| {
                Box::new(build(offset, size)) as Box<dyn Widget<Msg>>
            }),
        }
    }

    /// The body, for the walk that needs it before the children slice is handed out.
    fn inner(&self) -> &dyn Widget<Msg> {
        self.body[0].as_ref()
    }
}

impl<Msg: 'static> Widget<Msg> for ScrollOverlay<Msg> {
    /// **The body's box, exactly.** An overlay is drawn over something, so the something
    /// is what decides the shape: a wrapper with a box of its own would put the region
    /// somewhere other than where the caller placed it, and the overlay would be over the
    /// wrong rectangle.
    fn style(&self) -> Style {
        self.inner().style()
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.inner().style_themed(theme)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.body
    }

    /// Nothing of its own: everything drawn here is drawn by the body or by the overlay.
    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "ScrollOverlay"
    }

    fn scroll_overlay(&self) -> Option<&OverlayBuilder<Msg>> {
        Some(&*self.build)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::WidgetId;
    use crate::ui::child_id;
    use crate::{build_ui, Container, ListView, Runtime, Size};
    use frus_core::{Color, Primitive};

    /// A list of a hundred rows in a 200-tall window, with a bar over it whose colour
    /// says how far down the list has got.
    fn tree(runtime: &Runtime) -> (ScrollOverlay<()>, WidgetId) {
        let list = ListView::<()>::new(100, 40.0, |_| {
            Container::<()>::new().height(40.0).color(Color::WHITE)
        })
        .width(300.0)
        .height(200.0);
        let overlay = ScrollOverlay::new(list, |(_, y), _| {
            // A block whose height is the offset, so the picture states the number.
            Container::<()>::new()
                .width(300.0)
                .height((y / 10.0).clamp(1.0, 100.0))
                .color(Color::rgb(1.0, 0.0, 0.0))
        });
        let _ = runtime;
        let id = WidgetId::ROOT;
        (overlay, id)
    }

    /// The height of the red block, which is this test's readout of the offset the
    /// overlay was built with.
    fn marker_height(runtime: &Runtime) -> Option<f32> {
        let (overlay, _) = tree(runtime);
        let size = Size::new(300.0, 200.0);
        let ui = build_ui(&overlay, size, runtime, &crate::Theme::dark());
        ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if color.r > 0.9 && color.g < 0.1 => {
                Some(rect.height)
            }
            _ => None,
        })
    }

    #[test]
    fn an_overlay_is_built_from_where_the_body_has_got_to() {
        // Nothing scrolled: the builder sees zero and the block is at its floor.
        let mut runtime = Runtime::default();
        assert_eq!(marker_height(&runtime), Some(1.0));

        // The region's offset is the body's, keyed by the body's own identity — which is
        // the point of the region being the child rather than something named.
        let (overlay, id) = tree(&runtime);
        let body = child_id(id, 0, overlay.children()[0].as_ref());
        runtime.scroll.insert(body, (0.0, 400.0));
        assert_eq!(
            marker_height(&runtime),
            Some(40.0),
            "the overlay follows the offset without anything being told"
        );
    }

    #[test]
    fn the_body_is_still_an_ordinary_scroll_region() {
        // The wrapper must not swallow what makes the body a region: it is registered,
        // it has somewhere to go, and it is the identity the offset is keyed by.
        let runtime = Runtime::default();
        let (overlay, id) = tree(&runtime);
        let size = Size::new(300.0, 200.0);
        let ui = build_ui(&overlay, size, &runtime, &crate::Theme::dark());
        let body = child_id(id, 0, overlay.children()[0].as_ref());
        let area = ui
            .scroll_region(body)
            .expect("the body is registered as a scroll region in its own right");
        assert!(
            (area.max_y - (100.0 * 40.0 - 200.0)).abs() < 1.0,
            "a hundred rows of forty in a window of two hundred: {}",
            area.max_y
        );
    }

    #[test]
    fn the_overlay_is_drawn_over_the_body_and_not_under_it() {
        // Order is the whole of what "overlay" means, and it is also what makes a button
        // in the overlay clickable: the hit registry is read last-first.
        let runtime = Runtime::default();
        let (overlay, _) = tree(&runtime);
        let size = Size::new(300.0, 200.0);
        let ui = build_ui(&overlay, size, &runtime, &crate::Theme::dark());
        let prims = ui.scene().primitives();
        let last_row = prims.iter().rposition(
            |p| matches!(p, Primitive::Rect { color, .. } if color.r > 0.9 && color.g > 0.9),
        );
        let marker = prims.iter().position(
            |p| matches!(p, Primitive::Rect { color, .. } if color.r > 0.9 && color.g < 0.1),
        );
        assert!(
            matches!((last_row, marker), (Some(row), Some(marker)) if marker > row),
            "the overlay comes after every row: {last_row:?} then {marker:?}"
        );
    }
}

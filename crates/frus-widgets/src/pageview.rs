//! [`PageView`]: a scrollable that comes to rest **on a page**, never between two.
//!
//! Onboarding screens, an image gallery, the tabs of a phone home screen: the
//! content is a sequence of full-viewport panels, and the only resting positions
//! are the panels themselves. Everything else — the drag, the edges, the glow —
//! is the ordinary scrollable machinery; what a page view changes is the single
//! moment the finger lifts, where a fling is replaced by a spring to one page.
//!
//! Like [`crate::List`], it is **virtualised**: pages are built by an
//! `index → widget` closure and only the ones on screen exist. A hundred-page
//! walkthrough costs the same per frame as a two-page one, and — the other side
//! of the same coin — a page has **no retained state** while it is off screen.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::physics::ScrollPhysics;
use crate::scroll::Axis;
use crate::theme::Theme;
use crate::widget::Widget;

/// How a scrollable snaps to pages, as the shell and the runtime see it.
///
/// This is what turns an ordinary scroll region into a paged one: it travels on
/// the region's [`crate::Scrollable`] description, so the release path can ask
/// "is this area paged?" without knowing anything about the widget.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PageSnap {
    /// Distance from one page to the next, in logical pixels.
    pub extent: f32,
    /// How many pages there are.
    pub count: usize,
    /// The page the application is asking for, applied on the first frame and on
    /// every change; see [`PageView::page`].
    pub requested: usize,
    /// Does the view page sideways?
    pub horizontal: bool,
}

impl PageSnap {
    /// The offset at which `index` rests.
    pub fn offset_of(&self, index: usize) -> f32 {
        index as f32 * self.extent
    }

    /// The page an `offset` reads as — the one a reader would say they are on.
    pub fn page_at(&self, offset: f32) -> usize {
        let last = self.count.saturating_sub(1);
        (crate::physics::page_of(offset, self.extent).round().max(0.0) as usize).min(last)
    }
}

/// The description of a paged view, exposed to the render driver.
pub struct PagedView<'a, Msg> {
    /// Total number of pages.
    pub count: usize,
    /// Which way the pages are laid out.
    pub axis: Axis,
    /// The fraction of the viewport one page occupies along that axis.
    pub viewport_fraction: f32,
    /// The page asked for by the application.
    pub requested: usize,
    /// Builds one page per index.
    pub build: &'a dyn Fn(usize) -> Box<dyn Widget<Msg>>,
}

impl<Msg> PagedView<'_, Msg> {
    /// The snapping description for a viewport of this size.
    pub fn snap(&self, viewport: Rect) -> PageSnap {
        let horizontal = !matches!(self.axis, Axis::Vertical);
        let along = if horizontal {
            viewport.width
        } else {
            viewport.height
        };
        PageSnap {
            extent: (along * self.viewport_fraction).max(1.0),
            count: self.count,
            requested: self.requested,
            horizontal,
        }
    }
}

/// A scrollable that rests only on page boundaries.
pub struct PageView<Msg> {
    count: usize,
    axis: Axis,
    viewport_fraction: f32,
    requested: usize,
    width: Dimension,
    height: Dimension,
    /// Was the width **set** explicitly? In flex mode an unset dimension on the
    /// paging axis must not serve as a basis; see [`PageView::style`].
    width_explicit: bool,
    /// Was the height **set** explicitly? The same question for the other axis.
    height_explicit: bool,
    flex_grow: f32,
    physics: Option<ScrollPhysics>,
    build: Box<dyn Fn(usize) -> Box<dyn Widget<Msg>>>,
    on_page_changed: Option<Box<dyn Fn(usize) -> Msg>>,
}

impl<Msg> PageView<Msg> {
    /// Creates a view of `count` pages, each built on demand by `build(index)`.
    ///
    /// Pages run **horizontally** and fill the viewport, which is what a page view
    /// is nine times out of ten; [`PageView::axis`] and
    /// [`PageView::viewport_fraction`] change both.
    pub fn new<W: Widget<Msg> + 'static>(count: usize, build: impl Fn(usize) -> W + 'static) -> Self {
        Self {
            count,
            axis: Axis::Horizontal,
            viewport_fraction: 1.0,
            requested: 0,
            width: Dimension::Auto,
            height: Dimension::Length(200.0),
            width_explicit: false,
            height_explicit: false,
            flex_grow: 0.0,
            physics: None,
            build: Box::new(move |index| Box::new(build(index)) as Box<dyn Widget<Msg>>),
            on_page_changed: None,
        }
    }

    /// Lays the pages out along `axis`. Only [`Axis::Horizontal`] and
    /// [`Axis::Vertical`] mean anything here: a page view pages one way.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// The fraction of the viewport one page occupies; `1.0` — the default — makes
    /// each page fill it.
    ///
    /// Below one, the neighbouring pages show at the edges, which is how a carousel
    /// says "there is more this way" without a hint or an arrow. The value is what
    /// the pages are measured *and* snapped by, so the two cannot disagree.
    pub fn viewport_fraction(mut self, fraction: f32) -> Self {
        self.viewport_fraction = fraction.max(0.05);
        self
    }

    /// The page to show.
    ///
    /// Applied when the view first appears — so this is also the initial page — and
    /// again **whenever the value changes**, gliding across to it. Between changes
    /// the finger has the offset: a view that re-asserted its page every frame
    /// could not be swiped at all. An application that never sets it gets an
    /// uncontrolled view starting at page 0.
    pub fn page(mut self, page: usize) -> Self {
        self.requested = page;
        self
    }

    /// The message sent when the page a reader would name changes — mid-drag, as
    /// soon as the rounding tips, not once the motion has settled. A title above a
    /// gallery should follow the picture, not trail it.
    pub fn on_page_changed(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_page_changed = Some(Box::new(message));
        self
    }

    /// Overrides how the view behaves at its edges; see [`crate::Scroll::physics`].
    /// The **fling** is a page view's own business either way.
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
    }

    /// Sets the viewport width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self.width_explicit = true;
        self
    }

    /// Sets the viewport height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self.height_explicit = true;
        self
    }

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }
}

impl<Msg> Widget<Msg> for PageView<Msg> {
    fn style(&self) -> Style {
        // Filling then paging: a default dimension that was never asked for must not
        // stand as a flex **basis**, or the view would need `+200` of free room
        // before it grew at all. The same trap as [`crate::Scroll::style`], and the
        // same answer: an `Auto` basis of 0, and `flex_grow` does the filling.
        let filling = self.flex_grow > 0.0;
        let width = if filling && !self.width_explicit {
            Dimension::Auto
        } else {
            self.width
        };
        let height = if filling && !self.height_explicit {
            Dimension::Auto
        } else {
            self.height
        };
        Style {
            width,
            height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // The viewport is transparent: only the pages are drawn.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn page_view(&self) -> Option<PagedView<'_, Msg>> {
        Some(PagedView {
            count: self.count,
            axis: self.axis,
            viewport_fraction: self.viewport_fraction,
            requested: self.requested,
            build: &*self.build,
        })
    }

    fn on_page_changed(&self, page: usize) -> Option<Msg> {
        self.on_page_changed.as_ref().map(|make| make(page))
    }

    fn scroll_axis(&self) -> Axis {
        self.axis
    }

    fn scroll_physics(&self) -> Option<ScrollPhysics> {
        self.physics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};
    use std::cell::Cell;
    use std::rc::Rc;

    fn view(built: Rc<Cell<usize>>) -> PageView<()> {
        PageView::new(50, move |index| {
            built.set(built.get() + 1);
            Container::new().color(Color::rgb(index as f32 / 50.0, 0.0, 0.0))
        })
        .width(300.0)
        .height(400.0)
    }

    fn ui_of(widget: PageView<()>, runtime: &Runtime) -> crate::Ui<()> {
        build_ui(
            &widget,
            Size::new(300.0, 400.0),
            runtime,
            &Theme::default(),
        )
    }

    #[test]
    fn only_the_pages_on_screen_are_built() {
        let built = Rc::new(Cell::new(0));
        let runtime = Runtime::default();
        ui_of(view(built.clone()), &runtime);
        // Page 0 fills the viewport; page 1 starts exactly at its right edge, so it
        // is not on screen yet. Fifty pages, one built.
        assert_eq!(built.get(), 1);
    }

    #[test]
    fn a_half_scrolled_view_shows_both_neighbours() {
        let built = Rc::new(Cell::new(0));
        let mut runtime = Runtime::default();
        let widget = view(built.clone());
        let id = crate::interaction::WidgetId::ROOT;
        runtime.scroll.insert(id, (150.0, 0.0));
        ui_of(widget, &runtime);
        assert_eq!(built.get(), 2);
    }

    #[test]
    fn the_content_is_as_long_as_the_pages_put_together() {
        let runtime = Runtime::default();
        let ui = ui_of(view(Rc::new(Cell::new(0))), &runtime);
        let area = ui.scroll_regions().first().copied().expect("a scroll region");
        // 50 pages of 300 px in a 300 px viewport: 49 pages' worth of travel.
        assert_eq!(area.max_x, 49.0 * 300.0);
        assert_eq!(area.max_y, 0.0);
        let snap = area.page.expect("a paged region");
        assert_eq!(snap.extent, 300.0);
        assert_eq!(snap.count, 50);
        assert!(snap.horizontal);
    }

    #[test]
    fn narrower_pages_let_the_next_one_show() {
        let runtime = Runtime::default();
        let ui = ui_of(
            view(Rc::new(Cell::new(0))).viewport_fraction(0.8),
            &runtime,
        );
        let area = ui.scroll_regions().first().copied().expect("a scroll region");
        let snap = area.page.expect("a paged region");
        assert_eq!(snap.extent, 240.0);
        // The travel stops with the last page's right edge on the viewport's.
        assert_eq!(area.max_x, 50.0 * 240.0 - 300.0);
    }

    #[test]
    fn a_vertical_view_pages_downwards() {
        let runtime = Runtime::default();
        let ui = ui_of(view(Rc::new(Cell::new(0))).axis(Axis::Vertical), &runtime);
        let area = ui.scroll_regions().first().copied().expect("a scroll region");
        let snap = area.page.expect("a paged region");
        assert!(!snap.horizontal);
        assert_eq!(snap.extent, 400.0);
        assert_eq!(area.max_x, 0.0);
        assert_eq!(area.max_y, 49.0 * 400.0);
    }

    #[test]
    fn a_page_is_painted_where_the_offset_says() {
        let mut runtime = Runtime::default();
        let widget = PageView::new(3, |_| Container::new().color(Color::rgb(1.0, 0.0, 0.0)))
            .width(300.0)
            .height(400.0);
        let id = crate::interaction::WidgetId::ROOT;
        runtime.scroll.insert(id, (300.0, 0.0));
        let ui = ui_of(widget, &runtime);
        // Page 1 is scrolled exactly into place: its box starts at the viewport's
        // left edge, not at 300.
        let left = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } if rect.width > 100.0 => Some(rect.x),
                _ => None,
            })
            .next()
            .expect("a page");
        assert!(left.abs() < 0.5, "page 1 drawn at {left}");
    }

    /// The whole loop, as the shell drives it: build, drag, release, settle.
    ///
    /// Everything below the shell's pointer plumbing is exercised here — the extent
    /// read off the viewport, the release, the spring, and the page reported — which
    /// is as close to the gesture as a test without a window gets.
    #[test]
    fn a_short_flick_turns_the_page_and_says_so() {
        let mut runtime = Runtime::default();
        let id = crate::interaction::WidgetId::ROOT;
        let physics = ScrollPhysics::Clamping;

        let area = {
            let ui = ui_of(view(Rc::new(Cell::new(0))), &runtime);
            ui.scroll_regions()[0]
        };
        // Opening on page 0 is not a change to report.
        assert!(runtime.page_changes(&[area]).is_empty());

        // The finger drags a fifth of a page across and throws.
        runtime.scroll.insert(id, (60.0, 0.0));
        assert!(runtime.fling_scroll(area, physics, (700.0, 0.0)));
        let mut reported = Vec::new();
        for _ in 0..240 {
            runtime.advance_scroll(&[area], physics, 1.0 / 60.0);
            reported.extend(runtime.page_changes(&[area]));
        }

        assert_eq!(reported, vec![(id, 1)], "reported once, on the way");
        let (x, _) = runtime.scroll[&id];
        assert!((x - 300.0).abs() < 1.0, "settled between pages, at {x}");
    }

    #[test]
    fn the_page_a_reader_would_name_is_the_nearest_one() {
        let snap = PageSnap {
            extent: 300.0,
            count: 3,
            requested: 0,
            horizontal: true,
        };
        assert_eq!(snap.page_at(0.0), 0);
        assert_eq!(snap.page_at(149.0), 0);
        assert_eq!(snap.page_at(151.0), 1);
        assert_eq!(snap.offset_of(2), 600.0);
        // An overscroll is still the page it is pulling away from.
        assert_eq!(snap.page_at(-40.0), 0);
        assert_eq!(snap.page_at(9_000.0), 2);
    }
}

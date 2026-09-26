//! [`MouseRegion`]: the pointer entering, moving over and leaving a part of the screen, and
//! the cursor it shows there.

use frus_core::Point;

use crate::interaction::Cursor;
use crate::widget::Widget;

/// How a region the mouse enters and leaves behaves, as the shell asks it of the frame
/// (milestone 583).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HoverRegion {
    /// The cursor shown while the pointer is over the region, unless something inside it
    /// asks for one of its own. `None` leaves the cursor to what is around it.
    pub cursor: Option<Cursor>,
    /// Whether the region **hides the regions behind it**: those it is not inside are not
    /// entered while the pointer is over it. The regions it is inside always are.
    pub opaque: bool,
}

/// One moment of the pointer over a region, as the shell reports it through
/// [`Widget::on_hover_event`](crate::Widget::on_hover_event). Every position is in the
/// region's own coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HoverEvent {
    /// The pointer came into the region — or the region came under a pointer that had not
    /// moved, when the screen changed around it.
    Enter {
        /// Where the pointer is.
        local: Point,
    },
    /// The pointer moved inside the region, with no button held.
    Move {
        /// Where the pointer is now.
        local: Point,
    },
    /// The pointer left the region, or the region went from under it.
    Exit {
        /// Where the pointer is, which is outside the region.
        local: Point,
    },
}

/// **Hears the pointer** coming into its child, moving over it and leaving it, and sets the
/// **cursor** shown while it is there.
///
/// A transparent wrapper: it is its child, laid out and painted exactly as the child would be
/// without it.
///
/// ```
/// use frus_widgets::{Cursor, MouseRegion, Text};
///
/// #[derive(Clone)]
/// enum Msg { Show, Hide }
///
/// let _card: MouseRegion<Msg> = MouseRegion::new(Text::new("Hover me"))
///     .on_enter(|_| Msg::Show)
///     .on_exit(|_| Msg::Hide)
///     .cursor(Cursor::Pointer);
/// ```
///
/// **Every region under the pointer is in**, the innermost first: a region inside another
/// is entered with it. A region is **opaque** by default, as the reference's is: the regions
/// *behind* it — overlapping it without containing it — are left while the pointer is over
/// it. [`opaque(false)`](Self::opaque) lets them be entered too.
///
/// A finger is over a region **while it touches** and not after: there is no hover without a
/// mouse. So a finger sends an enter when it goes down and an exit when it lifts, and no moves.
pub struct MouseRegion<Msg = crate::callback::Callback> {
    inner: Box<dyn Widget<Msg>>,
    region: HoverRegion,
    enter: Option<Box<dyn Fn(Point) -> Msg>>,
    hover: Option<Box<dyn Fn(Point) -> Msg>>,
    exit: Option<Box<dyn Fn(Point) -> Msg>>,
}

impl<Msg> MouseRegion<Msg> {
    /// A region over `child`: opaque, with no cursor of its own, hearing nothing yet.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(child),
            region: HoverRegion {
                cursor: None,
                opaque: true,
            },
            enter: None,
            hover: None,
            exit: None,
        }
    }

    /// The pointer came in: `build` is given where it is, in the region's coordinates.
    pub fn on_enter(mut self, build: impl Fn(Point) -> Msg + 'static) -> Self {
        self.enter = Some(Box::new(build));
        self
    }

    /// The pointer moved inside, with no button held: `build` is given where it is now.
    pub fn on_hover(mut self, build: impl Fn(Point) -> Msg + 'static) -> Self {
        self.hover = Some(Box::new(build));
        self
    }

    /// The pointer left: `build` is given where it is, now outside.
    pub fn on_exit(mut self, build: impl Fn(Point) -> Msg + 'static) -> Self {
        self.exit = Some(Box::new(build));
        self
    }

    /// The cursor shown while the pointer is over the region, unless something inside it
    /// asks for another — a field's text cursor, a button's hand.
    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.region.cursor = Some(cursor);
        self
    }

    /// Whether the regions behind this one are left while the pointer is over it. On by
    /// default.
    pub fn opaque(mut self, opaque: bool) -> Self {
        self.region.opaque = opaque;
        self
    }

    /// A region is not a box: its child's own, unchanged.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(hover MouseRegion {
    /// This one, unless the child is a region itself: then the child is the region at this
    /// place, and this one is not a second one.
    fn hover_region(&self) -> Option<HoverRegion> {
        self.inner.hover_region().or(Some(self.region))
    }

    fn on_hover_event(&self, event: HoverEvent) -> Option<Msg> {
        if self.inner.hover_region().is_some() {
            return self.inner.on_hover_event(event);
        }
        match event {
            HoverEvent::Enter { local } => self.enter.as_ref().map(|build| build(local)),
            HoverEvent::Move { local } => self.hover.as_ref().map(|build| build(local)),
            HoverEvent::Exit { local } => self.exit.as_ref().map(|build| build(local)),
        }
    }

    /// Forwarded, as for every transparent wrapper: a region is not an identity, a place, a
    /// theme, a surface, a form, nor a selection area.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
    fn autofill_group(&self) -> bool {
        self.inner.autofill_group()
    }
    fn area_toolbar(
        &self,
        context: crate::ToolbarContext,
    ) -> Option<Option<&dyn crate::widget::Widget<Msg>>> {
        self.inner.area_toolbar(context)
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, GestureDetector, Keyed, Runtime, Theme};
    use frus_core::Size;

    fn block() -> Container<u8> {
        Container::new().width(100.0).height(100.0)
    }

    /// **A region and a detector wrap each other either way round** and each still answers
    /// for itself; a wrapper around a region forwards it.
    #[test]
    fn a_region_and_a_detector_compose() {
        let region_outside =
            MouseRegion::new(GestureDetector::new(block()).on_tap(1)).on_enter(|_| 2);
        assert_eq!(Widget::on_click(&region_outside), Some(1));
        let enter = HoverEvent::Enter {
            local: Point::new(0.0, 0.0),
        };
        assert_eq!(region_outside.on_hover_event(enter), Some(2));

        let detector_outside =
            GestureDetector::new(MouseRegion::new(block()).on_exit(|_| 3)).on_tap(1);
        let exit = HoverEvent::Exit {
            local: Point::new(0.0, 0.0),
        };
        assert_eq!(Widget::on_click(&detector_outside), Some(1));
        assert_eq!(detector_outside.on_hover_event(exit), Some(3));
        assert!(detector_outside.hover_region().is_some());

        let keyed = Keyed::new(4u64, MouseRegion::new(block()).cursor(Cursor::Move));
        assert_eq!(
            keyed.hover_region().and_then(|region| region.cursor),
            Some(Cursor::Move)
        );
        assert_eq!(Widget::<u8>::hover_region(&block()), None);
    }

    /// **The regions under a point come innermost first, and survive a replayed frame**:
    /// read again from the paint cache, a region inside a repaint boundary is still found.
    #[test]
    fn the_regions_under_a_point_come_innermost_first_even_from_the_cache() {
        let tree = MouseRegion::new(
            Container::<u8>::new()
                .repaint_boundary()
                .width(200.0)
                .height(200.0)
                .child(MouseRegion::new(block()).opaque(false)),
        );
        let size = Size::new(200.0, 200.0);
        let rt = Runtime::default();
        let theme = Theme::default();
        let first = build_ui(&tree, size, &rt, &theme);
        let second = build_ui(&tree, size, &rt, &theme);
        assert_eq!(rt.paint_cache.borrow().last_frame_stats().0, 1, "replayed");
        for ui in [&first, &second] {
            let inside = ui.hover_regions_at(Point::new(20.0, 20.0));
            assert_eq!(inside.len(), 2, "both");
            assert_eq!(inside[0].1.width, 100.0, "the inner one first");
            let outside = ui.hover_regions_at(Point::new(150.0, 150.0));
            assert_eq!(outside.len(), 1, "only the outer one");
            assert_eq!(
                ui.hover_region_rect(outside[0].0).map(|r| r.width),
                Some(200.0)
            );
        }
    }
}

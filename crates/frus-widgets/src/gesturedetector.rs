//! [`GestureDetector`]: taps, double taps, long presses, secondary clicks and drags on anything.

use frus_core::Point;

use crate::widget::Widget;

/// **Answers gestures on its child** with the application's messages: a tap, a double tap,
/// a long press, a secondary click (the right mouse button), and a drag.
///
/// It is a transparent wrapper: it **is** its child, laid out and painted exactly as the child
/// would be without it, and it hears the gestures that land inside the child. A detector that
/// wants a tap gives the child none of its own look — no ink, no hover — which is what
/// [`InkWell`](crate::InkWell) and the buttons are for.
///
/// ```
/// use frus_widgets::{GestureDetector, Text};
///
/// #[derive(Clone)]
/// enum Msg { Open, Like, Menu }
///
/// let _photo: GestureDetector<Msg> = GestureDetector::new(Text::new("photo"))
///     .on_tap(Msg::Open)
///     .on_double_tap(Msg::Like)
///     .on_secondary_tap(Msg::Menu);
/// ```
///
/// **A tap waits when a double tap is possible.** With both set, a first tap is not a tap
/// until 300 ms have gone by without a second one, as on every platform: the second tap of a
/// double tap is not also two taps. With only a tap set, it answers at once.
///
/// **What the child answers itself, it keeps.** A detector around a button does not take the
/// button's tap: the innermost answer wins, as in the reference, and the detector answers
/// only the gestures the child has no answer to.
pub struct GestureDetector<Msg = crate::callback::Callback> {
    inner: Box<dyn Widget<Msg>>,
    tap: Option<Message<Msg>>,
    double_tap: Option<Message<Msg>>,
    long_press: Option<Message<Msg>>,
    secondary_tap: Option<Message<Msg>>,
    pan_axis: PanAxis,
    pan_start: Option<Box<dyn Fn(Point) -> Msg>>,
    pan_update: Option<Box<dyn Fn(Point, Point) -> Msg>>,
    pan_end: Option<Box<dyn Fn(Point) -> Msg>>,
}

/// A message the detector sends, made afresh each time it is asked for.
type Message<Msg> = Box<dyn Fn() -> Msg>;

/// Which way a [`GestureDetector`]'s drag may go (milestone 582).
///
/// The reference has three families of callbacks for this — pan, horizontal drag, vertical
/// drag — and forbids mixing them; here it is one family and a direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanAxis {
    /// Any direction: a pan.
    #[default]
    Free,
    /// Left and right only: the vertical part of the movement is not reported, and a vertical
    /// movement inside a scroll is the scroll's.
    Horizontal,
    /// Up and down only.
    Vertical,
}

/// One moment of a drag on a [`GestureDetector`], as the shell reports it through
/// [`Widget::on_pan`](crate::Widget::on_pan).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanEvent {
    /// The drag began: the pointer went down at `local` (in the detector's own coordinates)
    /// and has since moved far enough to be a drag rather than a tap.
    Start {
        /// Where the pointer went down.
        local: Point,
    },
    /// The pointer moved by `delta` and is now at `local`. Along a [`PanAxis`] that is not
    /// free, the other component of `delta` is zero.
    Update {
        /// Where the pointer is now, in the detector's own coordinates.
        local: Point,
        /// How far it moved since the last update, in logical pixels.
        delta: Point,
    },
    /// The pointer lifted, moving at `velocity` (logical pixels per second).
    End {
        /// The release velocity, masked to the axis like the updates.
        velocity: Point,
    },
}

impl<Msg> GestureDetector<Msg> {
    /// Listens for gestures on `child`. Nothing is answered until a gesture is given a message.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(child),
            tap: None,
            double_tap: None,
            long_press: None,
            secondary_tap: None,
            pan_axis: PanAxis::Free,
            pan_start: None,
            pan_update: None,
            pan_end: None,
        }
    }

    /// A detector is not a box: its child's own, unchanged.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

/// The message `message` will send, each time it is asked for.
fn keep<Msg: Clone + 'static>(message: impl Into<Msg>) -> Option<Message<Msg>> {
    let message = message.into();
    Some(Box::new(move || message.clone()))
}

impl<Msg: Clone + 'static> GestureDetector<Msg> {
    /// A **tap**: a press and a release in the same place, with a finger or the main mouse
    /// button.
    pub fn on_tap(mut self, message: impl Into<Msg>) -> Self {
        self.tap = keep(message);
        self
    }

    /// A **double tap**: two taps in quick succession, close together.
    pub fn on_double_tap(mut self, message: impl Into<Msg>) -> Self {
        self.double_tap = keep(message);
        self
    }

    /// A **long press**: a press held still for half a second. The release that follows is
    /// not also a tap.
    pub fn on_long_press(mut self, message: impl Into<Msg>) -> Self {
        self.long_press = keep(message);
        self
    }

    /// A **secondary tap**: a click of the secondary mouse button, usually the right one,
    /// which on a desktop opens a context menu.
    pub fn on_secondary_tap(mut self, message: impl Into<Msg>) -> Self {
        self.secondary_tap = keep(message);
        self
    }

    /// A **drag** has begun: `build` is given where the pointer went down, in the detector's
    /// own coordinates. A drag begins once the pointer has moved far enough to not be a tap.
    pub fn on_pan_start(mut self, build: impl Fn(Point) -> Msg + 'static) -> Self {
        self.pan_start = Some(Box::new(build));
        self
    }

    /// The pointer moved during a drag: `build` is given where it is now, in the detector's
    /// own coordinates, and how far it moved since the last update.
    pub fn on_pan_update(mut self, build: impl Fn(Point, Point) -> Msg + 'static) -> Self {
        self.pan_update = Some(Box::new(build));
        self
    }

    /// The drag ended: `build` is given the velocity the pointer lifted at, in logical pixels
    /// per second.
    pub fn on_pan_end(mut self, build: impl Fn(Point) -> Msg + 'static) -> Self {
        self.pan_end = Some(Box::new(build));
        self
    }

    /// Keeps the drag to one direction — the reference's horizontal and vertical drags. The
    /// other component of each movement is not reported, and inside a scroll that runs the
    /// other way the drag no longer competes with it. Free, a pan, by default.
    pub fn pan_axis(mut self, axis: PanAxis) -> Self {
        self.pan_axis = axis;
        self
    }
}

crate::transparent::forward_transparent!(gestures GestureDetector {
    /// The child's tap, or the detector's.
    fn on_click(&self) -> Option<Msg> {
        self.inner
            .on_click()
            .or_else(|| self.tap.as_ref().map(|message| message()))
    }

    fn on_long_press(&self) -> Option<Msg> {
        self.inner
            .on_long_press()
            .or_else(|| self.long_press.as_ref().map(|message| message()))
    }

    fn on_double_tap(&self) -> Option<Msg> {
        self.inner
            .on_double_tap()
            .or_else(|| self.double_tap.as_ref().map(|message| message()))
    }

    fn on_secondary_tap(&self) -> Option<Msg> {
        self.inner
            .on_secondary_tap()
            .or_else(|| self.secondary_tap.as_ref().map(|message| message()))
    }

    /// The child's drag, if it has one; the detector's otherwise.
    fn pan_axis(&self) -> Option<PanAxis> {
        self.inner.pan_axis().or_else(|| {
            (self.pan_start.is_some() || self.pan_update.is_some() || self.pan_end.is_some())
                .then_some(self.pan_axis)
        })
    }

    fn on_pan(&self, event: PanEvent) -> Option<Msg> {
        if self.inner.pan_axis().is_some() {
            return self.inner.on_pan(event);
        }
        match event {
            PanEvent::Start { local } => self.pan_start.as_ref().map(|build| build(local)),
            PanEvent::Update { local, delta } => {
                self.pan_update.as_ref().map(|build| build(local, delta))
            }
            PanEvent::End { velocity } => self.pan_end.as_ref().map(|build| build(velocity)),
        }
    }

    /// Forwarded, as for every transparent wrapper: a detector is not an identity, a place, a
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
    use crate::{build_ui, Container, Flex, Keyed, Runtime, Theme};
    use frus_core::{Point, Size};

    /// **Every gesture is the detector's to answer**, through any wrapper around it.
    #[test]
    fn a_wrapped_detector_still_answers() {
        let detector = GestureDetector::<u8>::new(Container::new())
            .on_tap(1)
            .on_double_tap(2)
            .on_long_press(3)
            .on_secondary_tap(4);
        let wrapped = Keyed::new(9u64, detector);
        assert_eq!(wrapped.on_click(), Some(1));
        assert_eq!(wrapped.on_double_tap(), Some(2));
        assert_eq!(wrapped.on_long_press(), Some(3));
        assert_eq!(wrapped.on_secondary_tap(), Some(4));
    }

    /// **A detector with no tap of its own is still hit** by a press, so that the shell can
    /// ask it for the double or secondary tap; and one with nothing at all is not.
    #[test]
    fn a_detector_without_a_tap_is_still_a_target() {
        let size = Size::new(100.0, 100.0);
        let rt = Runtime::default();
        let theme = Theme::default();
        let block = || Container::<u8>::new().width(100.0).height(100.0);
        let double = GestureDetector::new(block()).on_double_tap(2);
        let ui = build_ui(&double, size, &rt, &theme);
        assert!(ui.hit(Point::new(50.0, 50.0)).is_some());
        let secondary = GestureDetector::new(block()).on_secondary_tap(4);
        let ui = build_ui(&secondary, size, &rt, &theme);
        assert!(ui.hit(Point::new(50.0, 50.0)).is_some());
        let nothing = GestureDetector::new(block());
        let ui = build_ui(&nothing, size, &rt, &theme);
        assert!(ui.hit(Point::new(50.0, 50.0)).is_none());
    }

    /// **A detector drags only once told what a drag sends**, in the way it was told, and
    /// every moment of the drag reaches its builder — through any wrapper (milestone 582).
    #[test]
    fn a_drag_is_answered_through_a_wrapper() {
        let plain = GestureDetector::<u8>::new(Container::new()).on_tap(1);
        assert_eq!(Widget::pan_axis(&plain), None, "a tap is not a drag");
        let detector = GestureDetector::<f32>::new(Container::new())
            .on_pan_start(|at| at.x)
            .on_pan_update(|_, delta| delta.y)
            .on_pan_end(|velocity| velocity.x + 1000.0)
            .pan_axis(PanAxis::Vertical);
        let wrapped = Keyed::new(3u64, detector);
        assert_eq!(wrapped.pan_axis(), Some(PanAxis::Vertical));
        let at = Point::new(7.0, 9.0);
        assert_eq!(wrapped.on_pan(PanEvent::Start { local: at }), Some(7.0));
        let moved = PanEvent::Update {
            local: at,
            delta: Point::new(0.0, -4.0),
        };
        assert_eq!(wrapped.on_pan(moved), Some(-4.0));
        let end = PanEvent::End {
            velocity: Point::new(5.0, 0.0),
        };
        assert_eq!(wrapped.on_pan(end), Some(1005.0));
        // Only an update told: the other moments send nothing.
        let updates = GestureDetector::<f32>::new(Container::new()).on_pan_update(|_, d| d.x);
        assert_eq!(Widget::pan_axis(&updates), Some(PanAxis::Free));
        assert_eq!(
            Widget::on_pan(&updates, PanEvent::Start { local: at }),
            None
        );
    }

    /// **A detector that drags is found under the pointer**, the innermost first, and still
    /// found when its frame is replayed from the paint cache.
    #[test]
    fn a_dragging_detector_is_found_even_from_the_cache() {
        let size = Size::new(200.0, 200.0);
        let rt = Runtime::default();
        let theme = Theme::default();
        let inner = GestureDetector::new(Container::<u8>::new().width(50.0).height(50.0))
            .on_pan_update(|_, _| 2);
        let tree = GestureDetector::new(
            Container::<u8>::new()
                .repaint_boundary()
                .width(200.0)
                .height(200.0)
                .child(inner),
        )
        .on_pan_update(|_, _| 1);
        let first = build_ui(&tree, size, &rt, &theme);
        let second = build_ui(&tree, size, &rt, &theme);
        assert_eq!(rt.paint_cache.borrow().last_frame_stats().0, 1, "replayed");
        for ui in [&first, &second] {
            let (outer, _) = ui.pan_at(Point::new(150.0, 150.0)).expect("the outer one");
            let (inner, rect) = ui.pan_at(Point::new(20.0, 20.0)).expect("the inner one");
            assert_ne!(outer, inner, "the innermost wins");
            assert_eq!(rect.width, 50.0);
        }
        let plain = GestureDetector::new(Container::<u8>::new().width(50.0).height(50.0)).on_tap(1);
        assert!(build_ui(&plain, size, &rt, &theme)
            .pan_at(Point::new(5.0, 5.0))
            .is_none());
    }

    /// **A detector changes nothing about the layout**: its child comes out exactly as it
    /// would without it — a box with no width of its own stretched across a column, as a bare
    /// one is. (A detector that was a box of its own gave such a child no width at all, and
    /// the pad in the demo's About page drew nothing: milestone 582, seen on the phone.)
    #[test]
    fn a_detector_leaves_the_layout_alone() {
        let red = frus_core::Color::rgb(1.0, 0.0, 0.0);
        let pad = || Container::<u8>::new().height(40.0).color(red);
        let size = Size::new(300.0, 200.0);
        let rt = Runtime::default();
        let theme = Theme::default();
        let bare = Flex::<u8>::column().width(300.0).child(pad());
        let detected = Flex::<u8>::column()
            .width(300.0)
            .child(GestureDetector::new(pad()).on_tap(1));
        let reds = |ui: &crate::Ui<u8>| -> Vec<frus_core::Rect> {
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Rect { rect, color, .. } if *color == red => Some(*rect),
                    _ => None,
                })
                .collect()
        };
        let without = reds(&build_ui(&bare, size, &rt, &theme));
        let with = reds(&build_ui(&detected, size, &rt, &theme));
        assert_eq!(without.first().map(|r| r.width), Some(300.0), "{without:?}");
        assert_eq!(with, without);
    }

    /// **What the child answers itself, it keeps**: a detector around something that takes a
    /// tap answers only what the child does not.
    #[test]
    fn the_child_answers_first() {
        let child = Container::<u8>::new().on_click(7u8);
        let detector = GestureDetector::new(child).on_tap(1).on_double_tap(2);
        assert_eq!(detector.on_click(), Some(7), "the child's tap");
        assert_eq!(
            Widget::on_double_tap(&detector),
            Some(2),
            "the detector's double tap"
        );
    }
}

//! [`Listener`]: the pointer's raw events on a part of the screen, and [`OrientationBuilder`].

use frus_core::{Orientation, Point, Size};

use crate::layoutbuilder::LayoutBuilder;
use crate::widget::Widget;

/// What happened to the pointer, as a [`Listener`] hears it (milestone 586).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListenerEventKind {
    /// A finger touched, or a mouse button went down, inside the listener.
    Down,
    /// The pointer that went down inside moved — wherever it is now.
    Move,
    /// The pointer that went down inside lifted — wherever it is now.
    Up,
    /// The gesture was interrupted: the application went to the background, the touch was
    /// taken back by the system.
    Cancel,
    /// A mouse moved inside the listener with no button held.
    Hover,
}

/// One raw event of the pointer, in the listener's own coordinates (milestone 586).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListenerEvent {
    /// What happened.
    pub kind: ListenerEventKind,
    /// Where the pointer is, in the listener's coordinates — outside its box for a move or a
    /// release that left it.
    pub local: Point,
    /// Whether the pointer is a finger rather than a mouse.
    pub touch: bool,
}

/// **Hears the pointer's raw events** on its child: down, move, up, cancel, and a mouse's
/// hover — the reference's `Listener`.
///
/// Unlike a [`GestureDetector`](crate::GestureDetector), it does not recognise anything and
/// competes with nothing: a scroll under the finger still scrolls, a button still takes its
/// tap, and the listener hears every event all the same. A pointer that went down inside it
/// is followed **wherever it goes** until it lifts. That is what a drawing surface, a custom
/// slider or a gesture of the application's own is built on.
///
/// ```
/// use frus_widgets::{Container, Listener, ListenerEventKind};
///
/// #[derive(Clone)]
/// enum Msg { Ink(f32, f32), Lift }
///
/// let _pad: Listener<Msg> = Listener::new(Container::new().width(300.0).height(200.0), |event| {
///     match event.kind {
///         ListenerEventKind::Down | ListenerEventKind::Move => {
///             Some(Msg::Ink(event.local.x, event.local.y))
///         }
///         ListenerEventKind::Up | ListenerEventKind::Cancel => Some(Msg::Lift),
///         ListenerEventKind::Hover => None,
///     }
/// });
/// ```
///
/// A transparent wrapper: it is its child, laid out and painted exactly as the child would be
/// without it. Every listener under the pointer hears a press, the innermost first.
pub struct Listener<Msg = crate::callback::Callback> {
    inner: Box<dyn Widget<Msg>>,
    on_event: Box<dyn Fn(ListenerEvent) -> Option<Msg>>,
}

impl<Msg> Listener<Msg> {
    /// Hears the pointer on `child`: `on_event` answers each event with a message, or with
    /// nothing.
    pub fn new(
        child: impl Widget<Msg> + 'static,
        on_event: impl Fn(ListenerEvent) -> Option<Msg> + 'static,
    ) -> Self {
        Self {
            inner: Box::new(child),
            on_event: Box::new(on_event),
        }
    }

    /// A listener is not a box: its child's own, unchanged.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(listener Listener {
    /// This one — and a child that is a listener too is the same place, heard by both.
    fn pointer_listener(&self) -> bool {
        true
    }

    fn on_pointer_event(&self, event: ListenerEvent) -> Vec<Msg> {
        let mut messages = self.inner.on_pointer_event(event);
        messages.extend((self.on_event)(event));
        messages
    }

    /// Forwarded, as for every transparent wrapper: a listener is not an identity, a place, a
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

/// Builds from **the orientation of the box it is given** — the reference's
/// `OrientationBuilder`: landscape when the box is wider than tall, portrait otherwise.
///
/// The box, not the window: a panel beside a list on a wide window is portrait if it is
/// taller than wide, whatever the window is. It takes all the room on offer, as an
/// [`Aligned`](crate::Aligned) does, so that the box it decides on is that room and not the
/// size of what it built; along a row's or a column's own direction, shared with other
/// children, the room is only its content's. It is a [`LayoutBuilder`], with what that says
/// about state: the content is built on the fly from the box, and keeps none.
///
/// ```
/// use frus_core::Orientation;
/// use frus_widgets::{column, row, text, OrientationBuilder};
///
/// let _gallery: frus_widgets::LayoutBuilder<()> = OrientationBuilder::new(|orientation| {
///     match orientation {
///         Orientation::Landscape => row![text("a"), text("b")],
///         Orientation::Portrait => column![text("a"), text("b")],
///     }
/// });
/// ```
pub struct OrientationBuilder;

impl OrientationBuilder {
    /// A builder from `build`, given the orientation of the box.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<Msg: 'static, W: Widget<Msg> + 'static>(
        build: impl Fn(Orientation) -> W + 'static,
    ) -> LayoutBuilder<Msg> {
        LayoutBuilder::new(move |size: Size| build(Orientation::from_size(size.width, size.height)))
            .filling()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Keyed, Runtime, Theme};
    use frus_core::{Color, Primitive};

    /// **The orientation is the box's, not the window's**: the same builder in a wide box and
    /// a tall one, in one window, builds each way.
    #[test]
    fn the_orientation_is_the_boxs() {
        let (red, blue) = (Color::rgb(1.0, 0.0, 0.0), Color::rgb(0.0, 0.0, 1.0));
        let pick = move || {
            OrientationBuilder::new(move |orientation| {
                let color = if orientation == Orientation::Landscape {
                    red
                } else {
                    blue
                };
                Container::<()>::new().width(10.0).height(10.0).color(color)
            })
        };
        let colour_in = |width: f32, height: f32| {
            let root = Container::<()>::new()
                .width(width)
                .height(height)
                .child(pick());
            let ui = build_ui(
                &root,
                Size::new(400.0, 400.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene().primitives().iter().find_map(|p| match p {
                Primitive::Rect { color, .. } if *color == red || *color == blue => Some(*color),
                _ => None,
            })
        };
        assert_eq!(colour_in(300.0, 100.0), Some(red), "a wide box");
        assert_eq!(
            colour_in(100.0, 300.0),
            Some(blue),
            "a tall box, in the same window"
        );
    }

    /// **Two listeners at one place both hear**, the inner one first, and a wrapper forwards
    /// them.
    #[test]
    fn listeners_at_one_place_both_hear() {
        let inner = Listener::new(Container::<u8>::new(), |_| Some(1));
        let outer = Listener::new(inner, |_| Some(2));
        let wrapped = Keyed::new(5u64, outer);
        assert!(wrapped.pointer_listener());
        let event = ListenerEvent {
            kind: ListenerEventKind::Down,
            local: Point::new(0.0, 0.0),
            touch: true,
        };
        assert_eq!(wrapped.on_pointer_event(event), vec![1, 2]);
        assert!(!Widget::<u8>::pointer_listener(&Container::<u8>::new()));
    }

    /// **A listener is found under the pointer, the innermost first, also from a frame replayed
    /// from the paint cache.**
    #[test]
    fn a_listener_is_found_even_from_the_cache() {
        let tree = Listener::new(
            Container::<u8>::new()
                .repaint_boundary()
                .width(200.0)
                .height(200.0)
                .child(Listener::new(
                    Container::new().width(50.0).height(50.0),
                    |_| None,
                )),
            |_| None,
        );
        let (rt, theme) = (Runtime::default(), Theme::default());
        let size = Size::new(200.0, 200.0);
        let first = build_ui(&tree, size, &rt, &theme);
        let second = build_ui(&tree, size, &rt, &theme);
        assert_eq!(rt.paint_cache.borrow().last_frame_stats().0, 1, "replayed");
        for ui in [&first, &second] {
            let inside = ui.pointer_listeners_at(Point::new(20.0, 20.0));
            assert_eq!(inside.len(), 2);
            assert_eq!(inside[0].1.width, 50.0, "the inner one first");
            assert_eq!(ui.pointer_listeners_at(Point::new(150.0, 150.0)).len(), 1);
        }
    }
}

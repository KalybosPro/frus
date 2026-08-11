//! [`SafeArea`] — insets its child away from the parts of the screen the system has
//! taken: the status bar, the notch or camera cut-out, the gesture handle.
//!
//! ```ignore
//! SafeArea::new(screen)                       // every edge
//! SafeArea::new(list).edges(Edges::ALL.without_bottom())   // let it run under the handle
//! SafeArea::new(form).avoid_keyboard()        // and move up when the keyboard opens
//! ```
//!
//! The insets come from the ambient [`MediaQuery`], so nothing has to be threaded down
//! from the application. On desktop they are zero and this widget is a no-op — which is
//! the point: the same screen code is correct on both.

use frus_core::{Insets, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::media::{Edges, MediaQuery};
use crate::theme::Theme;
use crate::widget::Widget;

/// Pads its child by the system's occupied edges.
///
/// ## Nesting
///
/// A `SafeArea` **consumes** the padding it applies: anything built inside
/// [`SafeArea::build`] sees a [`MediaQuery`] whose consumed edges are already zero, so
/// a second `SafeArea` further down adds nothing and the notch is not avoided twice.
///
/// [`SafeArea::new`] cannot do that — its child is already built by the time the
/// widget exists — so it pads and says nothing about it. That is the right constructor
/// for the usual case of one `SafeArea` at the root of a screen, and the wrong one if
/// screens compose into each other; reach for `build` there.
pub struct SafeArea<Msg> {
    edges: Edges,
    minimum: Insets,
    keyboard: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
    /// The padding this widget resolved when it was constructed. Kept rather than
    /// recomputed in `style`, because `style` is also called from the layout cache,
    /// outside any [`MediaQuery::scope`] the shell installed.
    resolved: Insets,
}

impl<Msg> SafeArea<Msg> {
    /// Insets `child` away from every occupied edge.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let mut area = Self {
            edges: Edges::ALL,
            minimum: Insets::ZERO,
            keyboard: false,
            children: vec![Box::new(child)],
            resolved: Insets::ZERO,
        };
        area.resolved = area.resolve();
        area
    }

    /// Builds the child **inside** the safe area: `build` receives a [`MediaQuery`]
    /// whose consumed edges have already been zeroed, and the same value is installed
    /// as the ambient one for the whole of the call.
    ///
    /// Use this whenever a `SafeArea` may end up inside another one.
    ///
    /// ```ignore
    /// SafeArea::build(|mq| Column::new().child(Text::new(format!("{} px", mq.size.width))))
    /// ```
    pub fn build<W: Widget<Msg> + 'static>(build: impl FnOnce(MediaQuery) -> W) -> Self {
        Self::build_with(Edges::ALL, Insets::ZERO, false, build)
    }

    /// [`build`](Self::build) with the edges, the floor and the keyboard behaviour
    /// chosen up front — they have to be known **before** the child is built, since
    /// they decide what the child gets to see.
    pub fn build_with<W: Widget<Msg> + 'static>(
        edges: Edges,
        minimum: Insets,
        keyboard: bool,
        build: impl FnOnce(MediaQuery) -> W,
    ) -> Self {
        let mut area = Self {
            edges,
            minimum,
            keyboard,
            children: Vec::new(),
            resolved: Insets::ZERO,
        };
        area.resolved = area.resolve();

        let mut inner = MediaQuery::of().remove_padding(edges);
        if keyboard {
            inner = inner.remove_view_insets(edges);
        }
        let child = inner.scope(|| build(inner));
        area.children.push(Box::new(child));
        area
    }

    /// Which edges to inset. The rest are left to run to the screen's border — a list
    /// that should scroll under the gesture handle, say, keeps its bottom edge free.
    pub fn edges(mut self, edges: Edges) -> Self {
        self.edges = edges;
        self.resolved = self.resolve();
        self
    }

    /// A floor for the padding: each side ends up at least this far in, even where the
    /// system asks for nothing. The usual way to give a screen its margin and its
    /// notch avoidance in one widget.
    pub fn minimum(mut self, minimum: Insets) -> Self {
        self.minimum = minimum;
        self.resolved = self.resolve();
        self
    }

    /// Also avoids the **soft keyboard**, not just the permanent bars.
    ///
    /// Off by default, and deliberately: a screen whose content scrolls wants the
    /// keyboard handled by scrolling the focused field into view, not by shrinking the
    /// whole screen. Turn it on for a short, non-scrolling form.
    pub fn avoid_keyboard(mut self) -> Self {
        self.keyboard = true;
        self.resolved = self.resolve();
        self
    }

    /// The padding to apply: the occupied edges this widget was asked for, floored by
    /// `minimum`.
    fn resolve(&self) -> Insets {
        let mq = MediaQuery::of();
        let occupied = if self.keyboard { mq.safe() } else { mq.padding };
        let selected = self.edges.select(occupied);
        Insets::new(
            selected.top.max(self.minimum.top),
            selected.right.max(self.minimum.right),
            selected.bottom.max(self.minimum.bottom),
            selected.left.max(self.minimum.left),
        )
    }

    /// The padding this widget resolved — for tests and for a parent that needs to
    /// know how much was taken.
    pub fn padding(&self) -> Insets {
        self.resolved
    }
}

impl<Msg: Clone> Widget<Msg> for SafeArea<Msg> {
    fn style(&self) -> Style {
        Style {
            padding: self.resolved,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Pure layout: the safe area has no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Text};
    use frus_core::{Size, WindowInsets};

    fn phone() -> MediaQuery {
        MediaQuery::new(Size::new(360.0, 780.0)).with_insets(WindowInsets {
            padding: Insets::new(28.0, 0.0, 16.0, 0.0),
            view_insets: Insets::ZERO,
        })
    }

    #[test]
    fn it_pads_by_the_occupied_edges() {
        let area = phone().scope(|| SafeArea::<()>::new(Container::new()));
        assert_eq!(area.padding().top, 28.0);
        assert_eq!(area.padding().bottom, 16.0);
        assert_eq!(area.padding().left, 0.0);
    }

    #[test]
    fn on_a_bare_surface_it_is_a_no_op() {
        let area = SafeArea::<()>::new(Container::new());
        assert_eq!(area.padding(), Insets::ZERO);
    }

    #[test]
    fn an_unselected_edge_is_left_free() {
        let area = phone()
            .scope(|| SafeArea::<()>::new(Container::new()).edges(Edges::ALL.without_bottom()));
        assert_eq!(area.padding().top, 28.0);
        assert_eq!(
            area.padding().bottom,
            0.0,
            "the content should run under the gesture handle"
        );
    }

    #[test]
    fn the_minimum_is_a_floor_and_not_an_addition() {
        let area =
            phone().scope(|| SafeArea::<()>::new(Container::new()).minimum(Insets::uniform(20.0)));
        assert_eq!(area.padding().top, 28.0, "28 already clears 20");
        assert_eq!(area.padding().bottom, 20.0, "16 is raised to 20");
        assert_eq!(area.padding().left, 20.0, "nothing occupied, so the floor");
    }

    #[test]
    fn the_keyboard_is_avoided_only_when_asked() {
        let with_keyboard = phone().with_insets(WindowInsets {
            padding: Insets::new(28.0, 0.0, 16.0, 0.0),
            view_insets: Insets::new(0.0, 0.0, 320.0, 0.0),
        });
        let ignoring = with_keyboard.scope(|| SafeArea::<()>::new(Container::new()));
        assert_eq!(ignoring.padding().bottom, 16.0);

        let avoiding =
            with_keyboard.scope(|| SafeArea::<()>::new(Container::new()).avoid_keyboard());
        assert_eq!(avoiding.padding().bottom, 320.0);
    }

    #[test]
    fn a_nested_build_does_not_avoid_the_same_notch_twice() {
        let inner_padding = std::cell::Cell::new(Insets::ZERO);
        phone().scope(|| {
            SafeArea::<()>::build(|_| {
                let inner = SafeArea::<()>::new(Container::new());
                inner_padding.set(inner.padding());
                inner
            })
        });
        assert_eq!(
            inner_padding.get(),
            Insets::ZERO,
            "the outer SafeArea consumed the padding"
        );
    }

    #[test]
    fn build_hands_the_consumed_surface_to_the_closure() {
        let seen = std::cell::Cell::new(Insets::uniform(-1.0));
        phone().scope(|| {
            SafeArea::<()>::build(|mq| {
                seen.set(mq.padding);
                Text::new("x")
            })
        });
        assert_eq!(seen.get(), Insets::ZERO);
    }

    #[test]
    fn build_with_only_consumes_the_edges_it_insets() {
        let seen = std::cell::Cell::new(Insets::ZERO);
        phone().scope(|| {
            SafeArea::<()>::build_with(Edges::ALL.without_bottom(), Insets::ZERO, false, |mq| {
                seen.set(mq.padding);
                Text::new("x")
            })
        });
        assert_eq!(seen.get().top, 0.0);
        assert_eq!(
            seen.get().bottom,
            16.0,
            "an edge left free is still there for a descendant to use"
        );
    }

    #[test]
    fn the_padding_reaches_the_layout_style() {
        let area = phone().scope(|| SafeArea::<()>::new(Container::new()));
        assert_eq!(Widget::<()>::style(&area).padding.top, 28.0);
    }
}

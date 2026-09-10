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

use std::cell::Cell;
use std::rc::Rc;

use frus_core::{Insets, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::media::{Edges, MediaQuery};
use crate::mediascope::MediaScope;
use crate::theme::Theme;
use crate::widget::Widget;

/// Pads its child by the system's occupied edges.
///
/// ## Nesting
///
/// A `SafeArea` **consumes** the padding it applies: its subtree sees a [`MediaQuery`]
/// whose consumed edges are already zero, so a second `SafeArea` further down adds
/// nothing and the notch is not avoided twice — and a widget that clears the status bar
/// on its own, a [`NavigationBar`](crate::NavigationBar) or a
/// [`DrawerHeader`](crate::DrawerHeader), is not pushed down by it twice either.
///
/// That holds for both constructors. [`SafeArea::new`] used to pad and say nothing, on
/// the grounds that its child was already built; but the description is resolved **during
/// the walk**, not at construction, so the child is wrapped in a
/// [`MediaScope`](crate::MediaScope) that removes the edges — the reference's safe area
/// does the same with `MediaQuery.removePadding`. [`SafeArea::build`] still hands the
/// consumed description to its closure, for code that reads it while composing.
pub struct SafeArea<Msg> {
    edges: Edges,
    minimum: Insets,
    keyboard: bool,
    /// Pad the bottom by the intrusion that does **not** move; see
    /// [`SafeArea::maintain_bottom_view_padding`].
    maintain_bottom: bool,
    /// What the child's subtree is told has been consumed: the edges, and whether the
    /// keyboard's with them. Shared with the scope wrapping the child, because the
    /// builders below change it after the child was wrapped.
    consumed: Rc<Cell<(Edges, bool)>>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

/// Wraps `child` in the scope that tells its subtree what the safe area took.
fn consuming<Msg: 'static>(
    consumed: Rc<Cell<(Edges, bool)>>,
    child: impl Widget<Msg> + 'static,
) -> Box<dyn Widget<Msg>> {
    Box::new(MediaScope::tweak(
        move |mq: &mut MediaQuery| {
            let (edges, keyboard) = consumed.get();
            let mut inner = mq.remove_padding(edges);
            if keyboard {
                inner = inner.remove_view_insets(edges);
            }
            *mq = inner;
        },
        child,
    ))
}

impl<Msg: 'static> SafeArea<Msg> {
    /// Insets `child` away from every occupied edge.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let consumed = Rc::new(Cell::new((Edges::ALL, false)));
        Self {
            edges: Edges::ALL,
            minimum: Insets::ZERO,
            keyboard: false,
            maintain_bottom: false,
            children: vec![consuming(consumed.clone(), child)],
            consumed,
        }
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
        let consumed = Rc::new(Cell::new((edges, keyboard)));
        let mut area = Self {
            edges,
            minimum,
            keyboard,
            // Off here, and [`SafeArea::maintain_bottom_view_padding`] still works on
            // the result: it changes how much this widget pads, not what the child was
            // told had been consumed, and those are different questions.
            maintain_bottom: false,
            consumed: consumed.clone(),
            children: Vec::new(),
        };

        let mut inner = MediaQuery::of().remove_padding(edges);
        if keyboard {
            inner = inner.remove_view_insets(edges);
        }
        let child = inner.scope(|| build(inner));
        // What the closure was told while composing, the subtree is told again while it
        // is walked — the two have to agree, or a descendant that reads the surface late
        // would see the edges the closure was told were gone.
        area.children.push(consuming(consumed, child));
        area
    }

    /// Which edges to inset. The rest are left to run to the screen's border — a list
    /// that should scroll under the gesture handle, say, keeps its bottom edge free.
    ///
    /// Only those are consumed for the subtree: an edge left free is still there for a
    /// descendant to clear.
    pub fn edges(mut self, edges: Edges) -> Self {
        self.edges = edges;
        self.consumed.set((edges, self.keyboard));
        self
    }

    /// A floor for the padding: each side ends up at least this far in, even where the
    /// system asks for nothing. The usual way to give a screen its margin and its
    /// notch avoidance in one widget.
    pub fn minimum(mut self, minimum: Insets) -> Self {
        self.minimum = minimum;
        self
    }

    /// Keeps the bottom padding the keyboard would otherwise consume.
    ///
    /// The bottom padding goes to **zero** while the keyboard is up: the navigation bar
    /// it hides is not an edge anything needs to stay clear of, and that is the right
    /// answer nearly always. It is the wrong one for a screen holding a flexible child,
    /// which would then grow by exactly that bar's height the moment a field was tapped
    /// and shrink again when the keyboard closed — a whole layout twitching because
    /// somebody started typing.
    ///
    /// On, the bottom is padded by [`MediaQuery::view_padding`] instead, which does not
    /// move. The content sits behind the keyboard rather than above it, which is what it
    /// was already doing.
    pub fn maintain_bottom_view_padding(mut self) -> Self {
        self.maintain_bottom = true;
        self
    }

    /// Also avoids the **soft keyboard**, not just the permanent bars.
    ///
    /// Off by default, and deliberately: a screen whose content scrolls wants the
    /// keyboard handled by scrolling the focused field into view, not by shrinking the
    /// whole screen. Turn it on for a short, non-scrolling form.
    pub fn avoid_keyboard(mut self) -> Self {
        self.keyboard = true;
        self.consumed.set((self.edges, true));
        self
    }
}

/// The resolvers, in an impl of their own: the constructors above wrap the child in a
/// scope, which asks for `Msg: 'static`, and the `Widget` impl does not — a resolver
/// declared beside them could not be called from the layout.
impl<Msg> SafeArea<Msg> {
    /// The padding to apply: the occupied edges this widget was asked for, floored by
    /// `minimum`.
    ///
    /// Read **when it is asked for**, not when the widget was built. It used to be
    /// resolved once in the constructor with a comment saying `style` runs outside any
    /// surface the shell installed — which was true when it was written and stopped being
    /// true at milestone 408, when the shell began holding one surface across the build,
    /// the layout *and* the paint. Resolving here is what lets a
    /// [`MediaScope`](crate::MediaScope) above this widget reach it: a shell that has
    /// already dealt with the status bar can say so, and the safe area below believes it.
    fn resolve(&self) -> Insets {
        let mq = MediaQuery::of();
        let mut occupied = if self.keyboard { mq.safe() } else { mq.padding };
        if self.maintain_bottom {
            occupied.bottom = mq.view_padding.bottom;
        }
        let selected = self.edges.select(occupied);
        Insets::new(
            selected.top.max(self.minimum.top),
            selected.right.max(self.minimum.right),
            selected.bottom.max(self.minimum.bottom),
            selected.left.max(self.minimum.left),
        )
    }

    /// The padding this widget resolves **against the surface in force right now** — for
    /// tests and for a parent that needs to know how much was taken.
    pub fn padding(&self) -> Insets {
        self.resolve()
    }
}

impl<Msg: Clone> Widget<Msg> for SafeArea<Msg> {
    fn style(&self) -> Style {
        Style {
            padding: self.resolve(),
            // **It fills what it is given.** The reference's is a `Padding` under the
            // screen's own constraints, which are tight: it is the size of the box it
            // was handed, and its child is that box less the intrusions. A flex node
            // that grows nothing would instead hug its content, and a screen wrapped in
            // one would come out the width of its widest line — the failure milestone
            // 392 chased. Growing is what makes it the box.
            flex_grow: 1.0,
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
        MediaQuery::new(Size::new(360.0, 780.0))
            .with_insets(WindowInsets::bars(Insets::new(28.0, 0.0, 16.0, 0.0)))
    }

    /// The padding is asked for **under the surface**, because that is the question:
    /// milestone 417 made a safe area resolve when it is asked rather than when it was
    /// built, so that a `MediaScope` above it can change the answer. Asking outside any
    /// surface is asking about a screen that is not there.
    #[test]
    fn it_pads_by_the_occupied_edges() {
        phone().scope(|| {
            let area = SafeArea::<()>::new(Container::new());
            assert_eq!(area.padding().top, 28.0);
            assert_eq!(area.padding().bottom, 16.0);
            assert_eq!(area.padding().left, 0.0);
        });
    }

    #[test]
    fn on_a_bare_surface_it_is_a_no_op() {
        let area = SafeArea::<()>::new(Container::new());
        assert_eq!(area.padding(), Insets::ZERO);
    }

    #[test]
    fn an_unselected_edge_is_left_free() {
        phone().scope(|| {
            let area = SafeArea::<()>::new(Container::new()).edges(Edges::ALL.without_bottom());
            assert_eq!(area.padding().top, 28.0);
            assert_eq!(
                area.padding().bottom,
                0.0,
                "the content should run under the gesture handle"
            );
        });
    }

    #[test]
    fn the_minimum_is_a_floor_and_not_an_addition() {
        phone().scope(|| {
            let area = SafeArea::<()>::new(Container::new()).minimum(Insets::uniform(20.0));
            assert_eq!(area.padding().top, 28.0, "28 already clears 20");
            assert_eq!(area.padding().bottom, 20.0, "16 is raised to 20");
            assert_eq!(area.padding().left, 20.0, "nothing occupied, so the floor");
        });
    }

    fn with_keyboard() -> MediaQuery {
        phone().with_insets(WindowInsets::from_baseline(
            Insets::new(28.0, 0.0, 16.0, 0.0),
            Insets::new(28.0, 0.0, 320.0, 0.0),
        ))
    }

    #[test]
    fn the_keyboard_is_avoided_only_when_asked() {
        let with_keyboard = with_keyboard();
        // **Zero**, not sixteen. The navigation bar is under the keyboard, so there is
        // nothing left at the bottom to stay clear of, and padding by it as well would
        // leave a strip of nothing between the content and the keys.
        with_keyboard.scope(|| {
            let ignoring = SafeArea::<()>::new(Container::new());
            assert_eq!(ignoring.padding().bottom, 0.0);
            let avoiding = SafeArea::<()>::new(Container::new()).avoid_keyboard();
            assert_eq!(avoiding.padding().bottom, 320.0);
        });
    }

    /// The bottom padding going to zero is right for nearly every screen and wrong for
    /// one holding a flexible child, which would grow by the bar's height the moment a
    /// field was tapped and shrink again when the keyboard closed. Asked to, the safe
    /// area keeps the intrusion that does not move.
    #[test]
    fn the_bottom_can_be_kept_still_while_the_keyboard_comes_and_goes() {
        let bottom = |mq: MediaQuery| {
            mq.scope(|| {
                SafeArea::<()>::new(Container::new())
                    .maintain_bottom_view_padding()
                    .padding()
                    .bottom
            })
        };
        assert_eq!(bottom(phone()), 16.0);
        assert_eq!(
            bottom(with_keyboard()),
            16.0,
            "the same, keyboard or no keyboard"
        );
        // And it is the *view* padding it keeps, not the keyboard: the content still
        // sits behind the keys rather than being lifted above them.
        assert!(bottom(with_keyboard()) < 320.0);
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

    const MARK: frus_core::Color = frus_core::Color::rgb(0.9, 0.1, 0.5);

    /// The marked block's box in `root`, built and walked under the phone surface.
    fn marked_in(root: &dyn Widget<()>) -> Rect {
        let ui = phone().scope(|| {
            crate::build_ui(
                root,
                Size::new(360.0, 780.0),
                &crate::Runtime::default(),
                &Theme::default(),
            )
        });
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. } if *color == MARK => Some(*rect),
                _ => None,
            })
            .expect("the marked block is drawn")
    }

    /// **`SafeArea::new` consumes what it pads**, as `build` always did (milestone 504).
    ///
    /// It used to pad and say nothing, on the grounds that its child was built before it
    /// existed. But the surface is resolved during the walk, so a scope around the child
    /// can still tell the subtree — and a `SafeArea` that did not left a status bar for a
    /// navigation bar under it to clear a second time.
    #[test]
    fn a_nested_new_does_not_avoid_the_same_notch_twice() {
        let root = SafeArea::new(SafeArea::new(Container::new().height(10.0).color(MARK)));
        assert_eq!(marked_in(&root).y, 28.0, "the status bar once, not twice");
    }

    /// Only the edges it insets are consumed: one left free is still there for a
    /// descendant to clear.
    #[test]
    fn an_edge_left_free_is_left_for_a_descendant() {
        let root =
            SafeArea::new(SafeArea::new(Container::new().height(10.0).color(MARK))).edges(Edges {
                top: false,
                ..Edges::ALL
            });
        assert_eq!(
            marked_in(&root).y,
            28.0,
            "the inner one still found the status bar the outer one left free"
        );
    }

    #[test]
    fn the_padding_reaches_the_layout_style() {
        phone().scope(|| {
            let area = SafeArea::<()>::new(Container::new());
            assert_eq!(Widget::<()>::style(&area).padding.top, 28.0);
        });
    }
}

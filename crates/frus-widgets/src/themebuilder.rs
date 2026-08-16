//! [`ThemeBuilder`]: builds its subtree **from the ambient theme**.
//!
//! Milestone 309 gave the framework `caller ?? theme ?? framework`, and milestone 310
//! carried it into layout. Both work by handing a widget the theme when it is asked how
//! it looks or how big it is. That covers everything a theme decides *about a widget*,
//! and nothing a theme decides *about a composition*.
//!
//! An application bar is the example that forced this. Whether it centres its title
//! changes **which children exist and in what order** — a centred title is a spring, a
//! title and another spring; a flush one is a title after the leading. By the time
//! `paint` is called the row has already been assembled, so no amount of theme-at-paint
//! can answer it. And an `AppBar` is put together by a builder that never sees a theme in
//! the first place.
//!
//! ```ignore
//! ThemeBuilder::new(|theme| {
//!     AppBar::new("Inbox")
//!         .center_title(theme.widgets.app_bar.center_title.unwrap_or(platform()))
//!         .build()
//! })
//! ```
//!
//! ## Why this is not `LayoutBuilder`
//!
//! [`LayoutBuilder`](crate::LayoutBuilder) also builds late, and its documentation is
//! blunt about the price: the content is rebuilt every frame from a **box**, so it has no
//! retained state — no persistent focus, no deferred overlays. A field inside one would
//! lose the caret between frames.
//!
//! A theme is not a box. It is the same from frame to frame unless the application
//! changes it, and when it changes, the tree is rebuilt anyway. So this builds **once per
//! instance**, the subtree keeps its positional identity, and retained state survives —
//! which is the whole reason an application bar can live in one and keep its overflow
//! menu open.

use std::cell::{OnceCell, RefCell};

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// What a [`ThemeBuilder`] holds until the theme arrives: a composition that has not been
/// made yet.
type Deferred<Msg> = Box<dyn FnOnce(&Theme) -> Box<dyn Widget<Msg>>>;

/// Builds its child from the ambient theme (`theme → widget`).
///
/// Transparent otherwise: it has no box, no paint and no identity of its own, so the
/// child sits exactly where the builder does.
pub struct ThemeBuilder<Msg> {
    /// Taken and run the first time the layout pass reaches this node. `FnOnce`, not
    /// `Fn`: what it captures is usually a builder holding boxed widgets, which cannot be
    /// produced twice.
    build: RefCell<Option<Deferred<Msg>>>,
    /// The built child, as the one-element slice [`Widget::children`] has to hand back.
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: 'static> ThemeBuilder<Msg> {
    /// Defers a subtree until the ambient theme is known.
    pub fn new<W: Widget<Msg> + 'static>(build: impl FnOnce(&Theme) -> W + 'static) -> Self {
        Self {
            build: RefCell::new(Some(Box::new(move |theme| {
                Box::new(build(theme)) as Box<dyn Widget<Msg>>
            }))),
            built: OnceCell::new(),
        }
    }

    /// Defers an **already boxed** subtree, which is what a builder's `build()` returns.
    pub fn boxed(build: impl FnOnce(&Theme) -> Box<dyn Widget<Msg>> + 'static) -> Self {
        Self {
            build: RefCell::new(Some(Box::new(build))),
            built: OnceCell::new(),
        }
    }

    /// The child, building it if the layout pass has not already.
    ///
    /// Every traversal goes through here rather than reading the cell directly, so a
    /// caller that reaches this node without the layout pass having been down it first —
    /// a bare `natural_size`, a test — gets a built subtree rather than an empty one.
    fn child(&self, theme: &Theme) -> &[Box<dyn Widget<Msg>>] {
        if self.built.get().is_none() {
            let built = match self.build.borrow_mut().take() {
                Some(build) => vec![build(theme)],
                // Already taken and stored by another call: the `get` below finds it.
                None => Vec::new(),
            };
            let _ = self.built.set(built);
        }
        self.built.get().map(Vec::as_slice).unwrap_or(&[])
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ThemeBuilder<Msg> {
    fn build_themed(&self, theme: &Theme) {
        self.child(theme);
    }

    fn style(&self) -> Style {
        Style::default()
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        // The builder is **transparent**: it takes the child's box, so putting one around
        // a widget does not change where that widget lands.
        self.child(theme)
            .first()
            .map(|c| c.style_themed(theme))
            .unwrap_or_default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        // Unbuilt, this is empty — which is why `build_themed` runs on the way down in
        // the layout pass, before anything reads it. A traversal that arrives first
        // should call `build_themed`, not this.
        self.built.get().map(Vec::as_slice).unwrap_or(&[])
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "ThemeBuilder"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::Container;
    use crate::ui::build_ui;
    use crate::{Runtime, WidgetId};
    use frus_core::Color;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {}

    /// The point of the widget: a subtree that could not be decided until the theme was
    /// known. The same builder under two themes produces two different trees.
    #[test]
    fn the_subtree_is_built_from_the_ambient_theme() {
        let under = |theme: Theme| {
            // A subtree whose *shape* comes from the theme, which is the case no
            // paint-time or style-time hook can serve.
            let builder: ThemeBuilder<Msg> =
                ThemeBuilder::new(|t: &Theme| Container::new().width(t.radius).height(10.0));
            Widget::<Msg>::build_themed(&builder, &theme);
            let children = Widget::<Msg>::children(&builder);
            assert_eq!(children.len(), 1, "exactly one child, always");
            children[0].style().width
        };
        let sharp = Theme {
            radius: 0.0,
            ..Theme::default()
        };
        let round = Theme {
            radius: 24.0,
            ..Theme::default()
        };
        assert_eq!(under(sharp), frus_layout::Dimension::Length(0.0));
        assert_eq!(under(round), frus_layout::Dimension::Length(24.0));
    }

    /// A traversal that arrives before the layout pass must not see an empty node. Asking
    /// for the style builds the subtree, so a bare `natural_size` or a test is not a
    /// different answer from a real frame.
    #[test]
    fn asking_for_the_style_builds_it() {
        let builder: ThemeBuilder<Msg> =
            ThemeBuilder::new(|_: &Theme| Container::new().width(120.0).height(40.0));
        let style = Widget::<Msg>::style_themed(&builder, &Theme::default());
        assert_eq!(style.width, frus_layout::Dimension::Length(120.0));
        assert_eq!(
            Widget::<Msg>::children(&builder).len(),
            1,
            "and the child is there afterwards"
        );
    }

    /// Transparent: the builder has no box of its own, so the child lands exactly where
    /// it would have without it.
    #[test]
    fn the_builder_takes_no_room() {
        let bare: Container<Msg> = Container::new()
            .width(200.0)
            .height(50.0)
            .color(Color::rgb(1.0, 0.0, 0.0));
        let wrapped: ThemeBuilder<Msg> = ThemeBuilder::new(|_: &Theme| {
            Container::new()
                .width(200.0)
                .height(50.0)
                .color(Color::rgb(1.0, 0.0, 0.0))
        });
        let rect_of = |w: &dyn Widget<Msg>| {
            let runtime = Runtime::default();
            let ui = build_ui(
                w,
                frus_core::Size::new(400.0, 300.0),
                &runtime,
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                    _ => None,
                })
                .expect("the child painted")
        };
        assert_eq!(rect_of(&bare), rect_of(&wrapped));
    }

    /// Built **once**, not once per traversal: the closure runs a single time however
    /// many passes go over the node, which is what lets the subtree keep retained state.
    #[test]
    fn the_subtree_is_built_once() {
        use std::rc::Rc;
        let calls = Rc::new(std::cell::Cell::new(0));
        let seen = calls.clone();
        let builder: ThemeBuilder<Msg> = ThemeBuilder::new(move |_: &Theme| {
            seen.set(seen.get() + 1);
            Container::new().width(1.0)
        });
        let theme = Theme::default();
        for _ in 0..3 {
            Widget::<Msg>::build_themed(&builder, &theme);
            let _ = Widget::<Msg>::style_themed(&builder, &theme);
            let _ = Widget::<Msg>::children(&builder);
        }
        assert_eq!(calls.get(), 1);
    }

    /// The subtree keeps a **stable identity** across frames, which is what separates this
    /// from `LayoutBuilder`: the child's id is the builder's child id, and it does not
    /// move because the builder was rebuilt.
    #[test]
    fn the_child_keeps_its_identity() {
        let id_of = || {
            let builder: ThemeBuilder<Msg> =
                ThemeBuilder::new(|_: &Theme| Container::new().width(1.0));
            Widget::<Msg>::build_themed(&builder, &Theme::default());
            let child = &Widget::<Msg>::children(&builder)[0];
            crate::ui::child_id(WidgetId::from_u64(7), 0, child.as_ref())
        };
        assert_eq!(id_of(), id_of());
    }
}

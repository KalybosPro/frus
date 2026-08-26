//! [`MediaScope`]: a **surface for one subtree**, not for the frame.
//!
//! The shell installs one description per frame — the window's size, the intrusions the
//! platform reported, the reader's font setting — and every widget reads it with
//! [`MediaQuery::of`]. Until milestone 417 that description could only be narrowed **where
//! a widget is constructed**: [`SafeArea::build`](crate::SafeArea::build) takes a closure,
//! runs it under a narrowed surface, and keeps what it returns.
//!
//! That is the wrong end for a shell. A [`Scaffold`](crate::Scaffold) is handed slots that
//! are **already built**, so it could not tell its app bar's subtree "the status bar is not
//! yours to worry about" — it could only inset the bar from outside, which is one switch
//! where the reference has two: there the shell makes the slot tall enough and the bar
//! decides for itself whether to consume the status bar. The cost was not cosmetic. An
//! [`AppBar`](crate::AppBar) used **outside** a shell drew under the status bar, because
//! nothing insetted it and it would not inset itself.
//!
//! ```ignore
//! MediaScope::tweak(|mq| mq.padding.top = 0.0, app_bar)   // not yours to worry about
//! ```
//!
//! [`MediaScope::tweak`] is the form that gets used, for the same reason
//! [`Themed::tweak`](crate::Themed::tweak) is: it receives the description **inherited from
//! above** and changes part of it, so a scope can consume one edge without also inventing a
//! screen size. It has to be a closure rather than a value because the ambient surface is
//! not known when the tree is built — the walk resolves it on the way down.
//!
//! It is a **transparent wrapper**: it lays out, paints and answers exactly as its child
//! does (see [`crate::transparent`]). The only thing it adds is the surface its subtree
//! inherits — for **layout as much as paint**, and for the composition of anything deferred
//! below it, since [`Widget::build_themed`] runs under the swap.
//!
//! Nesting works the way it reads: each scope starts from what it inherits, so an inner one
//! removing the bottom padding keeps the outer one's removed top.

use crate::media::MediaQuery;
use crate::widget::Widget;

/// Where a [`MediaScope`]'s description comes from.
enum Source {
    /// A description entire, ignoring what it inherits.
    Data(MediaQuery),
    /// A change applied to the description inherited from above.
    Tweak(Box<dyn Fn(&mut MediaQuery)>),
}

/// Describes the surface for its child and everything under it.
pub struct MediaScope<Msg> {
    source: Source,
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg> MediaScope<Msg> {
    /// Replaces the description wholesale for `child`'s subtree.
    ///
    /// For the rare case where a subtree is being laid out against something that is not
    /// the window at all — a preview of a phone screen inside a desktop tool, a thumbnail
    /// of a page. Everything else wants [`tweak`](Self::tweak).
    pub fn new(media: MediaQuery, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            source: Source::Data(media),
            inner: Box::new(child),
        }
    }

    /// **Changes part** of the inherited description for `child`'s subtree, which is what a
    /// shell wants: this slot's top intrusion has already been dealt with, and everything
    /// else stays as the platform reported it.
    ///
    /// The closure runs during the walk, on the description inherited at that point —
    /// nested scopes compose, and a scope under a [`MediaScope::new`] starts from that one.
    pub fn tweak(
        change: impl Fn(&mut MediaQuery) + 'static,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self {
            source: Source::Tweak(Box::new(change)),
            inner: Box::new(child),
        }
    }
}

impl<Msg> MediaScope<Msg> {
    /// A description does reach sizes — a `SafeArea` below one pads by what it was told —
    /// but through the child's own `style_themed`, under the swap. This wrapper adds
    /// nothing on top of it.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(MediaScope {
    /// A surface says nothing about identity: a keyed widget in a scope keeps its key.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }

    /// Forwarded: a description is not a place.
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }

    /// Forwarded: a surface says nothing about the theme.
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }

    /// The one thing a `MediaScope` does not delegate: the surface it exists to impose.
    ///
    /// It still asks its child, because a transparent wrapper **is** its child: two nested
    /// scopes are one node in the tree, and the inner one would otherwise never be asked at
    /// all. Applied outer first, so the inner one sees what it inherits — which is what
    /// nesting means when it is written out.
    fn media_override(&self, inherited: MediaQuery) -> Option<MediaQuery> {
        let mine = match &self.source {
            Source::Data(media) => *media,
            Source::Tweak(change) => {
                let mut media = inherited;
                change(&mut media);
                media
            }
        };
        Some(self.inner.media_override(mine).unwrap_or(mine))
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flex::Flex;
    use crate::runtime::Runtime;
    use crate::theme::Theme;
    use crate::ui::build_ui;
    use frus_core::{Insets, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {}

    /// Reads `MediaQuery::of()` at the moment the walk builds it, which is what a deferred
    /// subtree — an `AppBar`, a `ThemeBuilder` — does.
    fn probe(seen: std::rc::Rc<std::cell::Cell<Insets>>) -> impl Widget<Msg> {
        crate::ThemeBuilder::new(move |_: &Theme| {
            seen.set(MediaQuery::of().padding);
            crate::container::Container::<Msg>::new()
                .width(10.0)
                .height(10.0)
        })
    }

    /// A phone-shaped surface: a status bar at the top, a gesture handle at the bottom.
    fn surface() -> MediaQuery {
        let padding = Insets::new(48.0, 0.0, 24.0, 0.0);
        MediaQuery::new(Size::new(200.0, 200.0)).with_insets(frus_core::WindowInsets {
            padding,
            view_insets: Insets::ZERO,
            view_padding: padding,
        })
    }

    #[test]
    fn a_scope_narrows_the_surface_its_subtree_is_built_under() {
        // The point of the milestone: the widget that knows what to consume is not the one
        // doing the constructing, so the swap has to happen during the **walk**.
        let seen = std::rc::Rc::new(std::cell::Cell::new(Insets::ZERO));
        let scoped = MediaScope::tweak(
            |mq: &mut MediaQuery| mq.padding.top = 0.0,
            probe(seen.clone()),
        );
        let root = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(scoped);
        surface().scope(|| {
            build_ui(
                &root,
                Size::new(200.0, 200.0),
                &Runtime::default(),
                &Theme::default(),
            )
        });
        assert_eq!(
            seen.get(),
            Insets::new(0.0, 0.0, 24.0, 0.0),
            "the top was consumed for the subtree, the bottom left alone"
        );
    }

    #[test]
    fn without_a_scope_a_subtree_sees_the_frames_own_surface() {
        let seen = std::rc::Rc::new(std::cell::Cell::new(Insets::ZERO));
        let root = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(probe(seen.clone()));
        surface().scope(|| {
            build_ui(
                &root,
                Size::new(200.0, 200.0),
                &Runtime::default(),
                &Theme::default(),
            )
        });
        assert_eq!(seen.get(), Insets::new(48.0, 0.0, 24.0, 0.0));
    }

    #[test]
    fn scopes_compose_from_the_inside_out() {
        // Each starts from what it inherits, so an inner one consuming the bottom keeps the
        // outer one's consumed top. Written out because the alternative — the inner one
        // starting from the frame's surface — reads identically at the call site and is
        // wrong in a way nothing would say out loud.
        let seen = std::rc::Rc::new(std::cell::Cell::new(Insets::ZERO));
        let inner = MediaScope::tweak(
            |mq: &mut MediaQuery| mq.padding.bottom = 0.0,
            probe(seen.clone()),
        );
        let outer = MediaScope::tweak(|mq: &mut MediaQuery| mq.padding.top = 0.0, inner);
        let root = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(outer);
        surface().scope(|| {
            build_ui(
                &root,
                Size::new(200.0, 200.0),
                &Runtime::default(),
                &Theme::default(),
            )
        });
        assert_eq!(seen.get(), Insets::ZERO);
    }

    #[test]
    fn a_scoped_surface_is_part_of_the_relayout_fingerprint() {
        // The cache keys on a fingerprint of the walk, and a scoped surface is part of that
        // walk. If it were not hashed, two subtrees given different descriptions would
        // share a fingerprint and the second would keep the first's geometry — the same
        // trap `theme_override` documents beside it.
        let runtime = Runtime::default();
        let height = |top: f32| {
            let scoped = MediaScope::tweak(
                move |mq: &mut MediaQuery| mq.padding.top = top,
                crate::safearea::SafeArea::<Msg>::new(
                    crate::container::Container::<Msg>::new()
                        .width(10.0)
                        .height(10.0)
                        .color(frus_core::Color::rgb8(255, 0, 128)),
                ),
            );
            let root = Flex::<Msg>::column()
                .width(200.0)
                .height(200.0)
                .child(scoped);
            surface().scope(|| {
                build_ui(&root, Size::new(200.0, 200.0), &runtime, &Theme::default())
                    .scene()
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        frus_core::Primitive::Rect { rect, .. } => Some(rect.y),
                        _ => None,
                    })
            })
        };
        let first = height(40.0);
        let second = height(0.0);
        assert_ne!(
            first, second,
            "a scoped surface must not be served from the cache of a different one"
        );
        assert_eq!(height(40.0), first, "and back again");
    }
}

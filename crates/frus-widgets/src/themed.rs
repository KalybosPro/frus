//! [`Themed`]: a **theme for one subtree**, not for the frame.
//!
//! Milestone 309 gave the theme per-widget defaults — an application can say *every card
//! is flat* once. It could still only say it once, for everything: there was one theme per
//! frame, so a dark panel on a light page had to be written out colour by colour at every
//! call site inside it, which is the framework failing at the exact point a theme was
//! supposed to help.
//!
//! ```ignore
//! Themed::new(Theme::dark(), sidebar())        // this subtree is dark
//!
//! Themed::tweak(|t| t.widgets.card.elevation = Some(0.0), settings_page())
//! ```
//!
//! [`Themed::tweak`] is the form that gets used: it receives the theme **inherited from
//! above** and changes part of it, so a panel can be flat without also deciding the
//! application's colours. It has to be a closure rather than a value because the ambient
//! theme is not known when the tree is built — the walk resolves it on the way down, which
//! is the same reason the reference reads its theme from the *context* and not from the
//! constructor.
//!
//! It is a **transparent wrapper**: it lays out, paints and answers exactly as its child
//! does (see [`crate::transparent`]). The only thing it adds is the theme its subtree
//! inherits — for **layout as much as paint**, so a themed subtree can hold thinner
//! dividers and not merely differently coloured ones.
//!
//! Nesting works the way it reads: each `Themed` starts from the theme it inherits, so an
//! inner one changing the ink colour keeps the outer one's dark scheme.

use crate::theme::Theme;
use crate::widget::Widget;

/// Where a [`Themed`]'s theme comes from.
enum Source {
    /// A theme entire, ignoring what it inherits. Boxed: a `Theme` dwarfs a closure, and
    /// this enum sits in every `Themed` in the tree.
    Data(Box<Theme>),
    /// A change applied to the theme inherited from above.
    Tweak(Box<dyn Fn(&mut Theme)>),
}

/// Applies a theme to its child and everything under it.
pub struct Themed<Msg> {
    source: Source,
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg> Themed<Msg> {
    /// Replaces the theme wholesale for `child`'s subtree — a page that is dark whatever
    /// the application around it is.
    pub fn new(theme: Theme, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            source: Source::Data(Box::new(theme)),
            inner: Box::new(child),
        }
    }

    /// **Changes part** of the inherited theme for `child`'s subtree, which is what an
    /// application usually wants: this section's cards are flat, or its ink is the brand
    /// colour, and everything else stays as the application decided.
    ///
    /// The closure runs during the walk, on the theme inherited at that point — nested
    /// tweaks compose, and a tweak under a [`Themed::new`] starts from that one.
    pub fn tweak(change: impl Fn(&mut Theme) + 'static, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            source: Source::Tweak(Box::new(change)),
            inner: Box::new(child),
        }
    }
}

impl<Msg> Themed<Msg> {
    /// A theme does reach sizes and spacing, but through the child's own
    /// `style_themed`; this wrapper adds nothing on top of it.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(Themed {
    /// A theme says nothing about identity: a keyed widget wrapped in a theme keeps its key.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }

    /// Forwarded: a theme is not a place.
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }

    /// The one thing a `Themed` does not delegate: the theme it exists to impose.
    ///
    /// It still asks its child, because a transparent wrapper **is** its child: two
    /// nested `Themed`s are one node in the tree, and the inner one would otherwise never
    /// be asked at all. Applied outer first, so the inner one sees what it inherits —
    /// which is what nesting means when it is written out.
    fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
        let mine = match &self.source {
            Source::Data(theme) => **theme,
            Source::Tweak(change) => {
                let mut theme = *inherited;
                change(&mut theme);
                theme
            }
        };
        Some(self.inner.theme_override(&mine).unwrap_or_else(|| Box::new(mine)))
    }

    /// Forwarded: a theme says nothing about the surface.
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CARD_MARGIN;
    use crate::divider::Divider;
    use crate::flex::Flex;
    use crate::runtime::Runtime;
    use crate::ui::build_ui;
    use frus_core::{Color, Primitive, Rect, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {}

    /// The crisp boxes a tree paints, in paint order (shadows excluded).
    fn boxes(tree: &dyn Widget<Msg>) -> Vec<(Rect, Color)> {
        build_ui(
            tree,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Rect {
                rect, color, blur, ..
            } if *blur == 0.0 => Some((*rect, *color)),
            _ => None,
        })
        .collect()
    }

    #[test]
    fn a_subtree_theme_reaches_paint() {
        let ink = Color::rgb8(255, 0, 128);
        let tree = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(Divider::new())
            .child(Themed::tweak(
                move |t| t.widgets.divider.color = Some(ink),
                Divider::new(),
            ));
        let painted = boxes(&tree);
        assert_eq!(painted.len(), 2, "two dividers");
        assert_ne!(painted[0].1, ink, "the first keeps the theme it inherited");
        assert_eq!(painted[1].1, ink, "the second takes the subtree's");
    }

    #[test]
    fn a_subtree_theme_reaches_layout() {
        // The half that a paint-only theme would miss. Two dividers stacked: the second
        // starts where the first's *box* ended, so a themed height moves it.
        let tree = |themed: bool| {
            let second: Box<dyn Widget<Msg>> = if themed {
                Box::new(Themed::tweak(
                    |t| t.widgets.divider.height = Some(40.0),
                    Divider::new(),
                ))
            } else {
                Box::new(Divider::new())
            };
            Flex::<Msg>::column()
                .width(200.0)
                .height(200.0)
                .child(second)
                .child(Divider::new())
        };
        let plain = boxes(&tree(false));
        let themed = boxes(&tree(true));
        assert!(
            themed[1].0.y > plain[1].0.y,
            "the themed divider's box is taller, so the next one is pushed down: \
             {:?} against {:?}",
            themed[1].0,
            plain[1].0
        );
    }

    #[test]
    fn the_theme_is_restored_on_the_way_out() {
        // A sibling *after* a themed subtree must be untouched — the walk's theme is a
        // stack, not a switch. This is the failure that would look like "the theme leaks
        // downward from wherever it was set".
        let ink = Color::rgb8(255, 0, 128);
        let tree = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(Themed::tweak(
                move |t| t.widgets.divider.color = Some(ink),
                Divider::new(),
            ))
            .child(Divider::new());
        let painted = boxes(&tree);
        assert_eq!(painted[0].1, ink);
        assert_ne!(painted[1].1, ink, "the sibling after it is not themed");
    }

    #[test]
    fn nesting_starts_from_what_it_inherits() {
        // `tweak` changes *part* of the theme it is handed, so an inner one keeps what
        // the outer one set. A `Themed` that started from the default instead would
        // silently undo its parent.
        let ink = Color::rgb8(255, 0, 128);
        let tree = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(Themed::tweak(
                move |t| t.widgets.divider.color = Some(ink),
                Themed::tweak(|t| t.widgets.divider.height = Some(40.0), Divider::new()),
            ));
        let painted = boxes(&tree);
        assert_eq!(painted[0].1, ink, "the outer theme's colour survives");
        assert_eq!(painted[0].0.y, 19.5, "and the inner theme's height applies");
    }

    #[test]
    fn a_whole_theme_replaces_rather_than_merges() {
        // `new` is the other half: it ignores what it inherits. An outer tweak must not
        // survive it.
        let ink = Color::rgb8(255, 0, 128);
        let tree = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(Themed::tweak(
                move |t| t.widgets.divider.color = Some(ink),
                Themed::new(Theme::default(), Divider::new()),
            ));
        assert_ne!(boxes(&tree)[0].1, ink);
    }

    #[test]
    fn an_overlay_keeps_the_theme_it_was_declared_under() {
        // An overlay is painted long after the walk has left the node that declared it.
        // Without the theme travelling with it, a dialog opened from inside a themed
        // section would come out in the application's theme instead of the section's —
        // and only when it opened, which is the worst place to find out.
        let ink = Color::rgb8(255, 0, 128);
        let panel = Flex::<Msg>::column()
            .width(100.0)
            .height(20.0)
            .child(Divider::new());
        let tree = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(Themed::tweak(
                move |t| t.widgets.divider.color = Some(ink),
                crate::portal::OverlayPortal::new(crate::Container::new().width(20.0).height(20.0))
                    .overlay(panel, crate::portal::Placement::Center),
            ));
        assert!(
            boxes(&tree).iter().any(|(_, color)| *color == ink),
            "the overlay's divider takes the theme of the subtree that declared it"
        );
    }

    #[test]
    fn it_is_transparent_to_layout() {
        // The wrapper must not become a box of its own: a themed card sits exactly where
        // the same card would, or every layout it is dropped into shifts.
        let plain = boxes(
            &Flex::<Msg>::column()
                .width(200.0)
                .height(200.0)
                .child(crate::card::Card::new()),
        );
        let wrapped = boxes(
            &Flex::<Msg>::column()
                .width(200.0)
                .height(200.0)
                .child(Themed::tweak(|_| {}, crate::card::Card::new())),
        );
        assert_eq!(plain[0].0.x, CARD_MARGIN);
        assert_eq!(plain[0].0, wrapped[0].0, "same box, wrapped or not");
    }
}

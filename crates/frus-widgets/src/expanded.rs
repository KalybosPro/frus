//! [`Expanded`]: the child of a row or a column that takes **the room the others left**.
//!
//! A row sizes its children by their content first, and only then shares out what is
//! spare. So a row of a checkbox, a long label and a delete button asks the label how
//! wide it would like to be, is told *558 pixels*, and the row is already over budget
//! before the button is measured. Flexbox then takes the deficit out of every child in
//! proportion — and the label, whose automatic minimum size is its own content, refuses,
//! so the whole of it lands on the button. That is how a delete button 40 px wide came to
//! be laid out at 13, then off the card entirely, and with it out of the hit registry:
//! the task could no longer be deleted (milestone 333).
//!
//! `Expanded` is the fix, and it is the reference's: the label is not a box that must be
//! squeezed, it is the one that takes **what is left**.
//!
//! ```ignore
//! row![
//!     Checkbox::new(done),
//!     Expanded::new(text(&task.title).ellipsis()),   // ← takes the rest
//!     IconButton::new(IconName::Close),              // ← keeps its 40 px
//! ]
//! ```
//!
//! Three things at once, none of which is useful without the other two:
//!
//! - **a basis of zero**, so the child no longer tells the row how wide it wants to be;
//! - **a grow factor**, so it then takes the spare room — all of it, or a share, if
//!   several children expand;
//! - **no automatic minimum**, which is the one everybody forgets. `flex: 1` on its own
//!   still floors an item at its content width, on the web as here; without lifting that
//!   floor the child grows but never yields, and the row overflows exactly as before.
//!
//! It changes only the box. Everything else — identity, hit testing, scrolling, the
//! structural questions — is the child's, forwarded by the transparent-wrapper macro.

use frus_layout::{Dimension, Style};

use crate::widget::Widget;

/// Wraps a child so it takes the spare room along its parent's main axis.
///
/// Three properties at once — a basis of zero, a grow factor, and no automatic minimum
/// — because a bare grow factor does nothing for the case this exists for: a flex item's
/// automatic minimum size is its own content, so `flex: 1` on a long label grows it and
/// still lets it refuse to be narrower than its text.
pub struct Expanded<Msg> {
    inner: Box<dyn Widget<Msg>>,
    flex: f32,
    /// A **loose** fit: the child may take less than its share. The reference calls this
    /// `Flexible`; `Expanded` is the tight one, and the difference is one line here.
    loose: bool,
}

impl<Msg> Expanded<Msg> {
    /// Wraps `inner`, which then takes all the spare room (a flex factor of 1).
    pub fn new(inner: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(inner),
            flex: 1.0,
            loose: false,
        }
    }

    /// A **loose** fit: the child takes *at most* its share, and less if that is all it
    /// wants — the reference's `Flexible`, where [`Expanded`] is `Flexible(fit: tight)`.
    ///
    /// The difference is the basis: tight starts the child at nothing and grows it into
    /// its whole share, loose starts it at its content and only lets it *give way* when
    /// the share is smaller than that.
    ///
    /// It had a caveat the reference does not until milestone 349: flexbox shares a
    /// deficit in proportion to basis, so a fixed sibling used to give up its share too
    /// unless it said so. Inflexible children are never squeezed now, here as there, and
    /// the deficit lands where it was aimed.
    pub fn loose(mut self) -> Self {
        self.loose = true;
        self
    }

    /// The share of the spare room, when several children expand: two children at
    /// `flex(2.0)` and `flex(1.0)` split it two to one.
    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }

    /// The one thing this wrapper changes: the flex item its child is.
    fn restyle(&self, base: Style) -> Style {
        Style {
            flex_grow: if self.loose { 0.0 } else { self.flex },
            flex_shrink: self.flex,
            flex_basis: if self.loose {
                Dimension::Auto
            } else {
                Dimension::Length(0.0)
            },
            // The floor that makes `flex: 1` alone a no-op. `Auto` here means *the
            // content*, so the child would grow to its share and then refuse to go below
            // its natural width — which is the whole of the bug this widget exists for.
            min_width: Dimension::Length(0.0),
            min_height: Dimension::Length(0.0),
            // `width` and `height` are **left alone**. A basis already overrules the size
            // on the main axis, and on the cross axis the child's own is the one that is
            // wanted: clearing both made an expanded `Text` — which reports its measured
            // size as its style rather than through a measure function — lose its height
            // and lay out 0 px tall in a centred row.
            ..base
        }
    }
}

crate::transparent::forward_transparent!(Expanded {
    /// Forwarded: expanding a widget says nothing about which widget it is, and the
    /// identity has to survive the wrapper or the child's retained state moves with its
    /// position in the list.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }

    /// Forwarded too: a box is not a palette.
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::WidgetId;
    use crate::runtime::Runtime;
    use crate::text::Text;
    use crate::theme::Theme;
    use crate::{Container, Flex};

    /// Lays a row out in `width` pixels and returns each direct child's box.
    fn boxes(row: &Flex<()>, width: f32) -> Vec<(f32, f32)> {
        let runtime = Runtime::default();
        let theme = Theme::light();
        let mut layout = frus_layout::Layout::new();
        let node = crate::ui::build_layout(row, WidgetId::ROOT, &runtime, &theme, &mut layout);
        layout.compute_filled(node, width, 100.0);
        let rects = layout.absolute_rects(node);
        // The row itself is first; its children follow, one level down and in order.
        rects
            .iter()
            .skip(1)
            .take(Widget::<()>::children(row).len())
            .map(|(r, _)| (r.width, r.height))
            .collect()
    }

    /// Lays a row out and returns each direct child's width.
    fn widths(row: Flex<()>, width: f32) -> Vec<f32> {
        boxes(&row, width).into_iter().map(|(w, _)| w).collect()
    }

    /// The cross axis is **not** the wrapper's business, and clearing it cost a
    /// milestone: a `Text` reports its measured size as its style rather than through a
    /// measure function, so an `Expanded` that blanked `height` too laid one out 0 px
    /// tall — invisible in a row aligned to its centre.
    #[test]
    fn it_leaves_the_child_s_cross_axis_alone() {
        let plain = Flex::row().child(Text::new("hello").size(18.0));
        let natural = boxes(&plain, 200.0)[0].1;
        assert!(natural > 0.0, "a text has a height to begin with");

        let row = Flex::row().child(Expanded::new(Text::new("hello").size(18.0)));
        let (width, height) = boxes(&row, 200.0)[0];
        assert_eq!(width, 200.0, "the main axis is the wrapper's");
        assert_eq!(height, natural, "the cross axis is the child's");
    }

    /// The device finding of milestone 333, in three lines: a long label and a 40 px
    /// button in a row too narrow for both.
    #[test]
    fn an_expanded_label_leaves_the_button_its_width() {
        let long = "a task label far longer than the row it has to live in";
        let row = Flex::row()
            .child(Expanded::new(Text::new(long).size(18.0).ellipsis()))
            .child(Container::<()>::new().width(40.0).height(40.0));
        let widths = widths(row, 200.0);
        assert_eq!(widths.len(), 2);
        assert_eq!(widths[1], 40.0, "the button keeps its width");
        assert_eq!(widths[0], 160.0, "the label takes exactly what is left");
    }

    /// Without the wrapper, the same row **overflows** — which is the reference's answer
    /// and, since milestone 349, this one's. The button keeps its 40 px, the label keeps
    /// its whole width, and the row wears a striped band saying by how much it did not
    /// fit. That is the improvement: the defect used to be silent, and it took three
    /// milestones and a device report to find the button crushed to 13 px.
    #[test]
    fn without_it_the_row_overflows_instead_of_crushing_the_button() {
        let long = "a task label far longer than the row it has to live in";
        let row = Flex::row()
            .child(Text::new(long).size(18.0))
            .child(Container::<()>::new().width(40.0).height(40.0));
        let widths = widths(row, 200.0);
        assert_eq!(widths[1], 40.0, "the button keeps its width");
        assert!(
            widths[0] > 160.0,
            "and the label keeps its own, so the row runs over: {widths:?}"
        );
    }

    /// Several expanding children share the room in proportion, and the fixed one is
    /// still untouched.
    #[test]
    fn several_expanded_children_split_what_is_left() {
        let row = Flex::row()
            .child(Expanded::new(Container::<()>::new()).flex(2.0))
            .child(Expanded::new(Container::<()>::new()).flex(1.0))
            .child(Container::<()>::new().width(40.0).height(40.0));
        let widths = widths(row, 340.0);
        assert_eq!(widths, vec![200.0, 100.0, 40.0]);
    }

    /// A fixed width inside an `Expanded` is the wrapper's, not the child's — otherwise
    /// the basis of zero is overruled and the widget silently does nothing.
    #[test]
    fn it_overrules_the_child_s_own_width() {
        let row = Flex::row()
            .child(Expanded::new(Container::<()>::new().width(100.0)))
            .child(Container::<()>::new().width(40.0).height(40.0));
        let widths = widths(row, 200.0);
        assert_eq!(widths[0], 160.0, "the expanded child fills, not 100");
    }

    /// The **loose** fit — the reference's `Flexible`. It never grows a child into room
    /// it did not ask for; it only lets one give way when there is not enough.
    #[test]
    fn a_loose_child_gives_way_but_does_not_fill() {
        // Room to spare: the tight one fills it, the loose one keeps its size.
        let tight = Flex::row()
            .child(Expanded::new(
                Container::<()>::new().width(40.0).height(20.0),
            ))
            .child(Container::<()>::new().width(40.0).height(20.0));
        assert_eq!(widths(tight, 200.0)[0], 160.0);

        let loose = Flex::row()
            .child(Expanded::new(Container::<()>::new().width(40.0).height(20.0)).loose())
            .child(Container::<()>::new().width(40.0).height(20.0));
        assert_eq!(widths(loose, 200.0)[0], 40.0, "at most its share, not more");

        // Not enough room: it gives way, and the whole deficit lands here because the
        // sibling is inflexible and inflexible children are not squeezed (milestone 349).
        // It used to take a `no_shrink()` on the sibling — flexbox shares a deficit in
        // proportion to basis, and a wrapper cannot speak for its siblings.
        let squeezed = Flex::row()
            .child(
                Expanded::new(
                    Text::new("a label far longer than this row")
                        .size(18.0)
                        .ellipsis(),
                )
                .loose(),
            )
            .child(Container::<()>::new().width(40.0).height(20.0));
        let widths = widths(squeezed, 200.0);
        assert_eq!(widths[1], 40.0, "the fixed sibling keeps its width");
        assert_eq!(widths[0], 160.0, "and the loose one takes what is left");
    }

    /// It is a wrapper, so the things a wrapper must not eat: the child's identity, and
    /// the structural answers the walk asks for before it looks at anything.
    #[test]
    fn it_stays_transparent() {
        let stack = crate::Stack::<()>::new()
            .width(100.0)
            .height(50.0)
            .layer(Container::new())
            .layer(Container::new());
        assert!(Widget::<()>::stack(&Expanded::new(stack)));

        let keyed = crate::Keyed::new(7u64, Container::<()>::new());
        let key = Widget::<()>::key(&keyed);
        assert_eq!(Widget::<()>::key(&Expanded::new(keyed)), key);
    }
}

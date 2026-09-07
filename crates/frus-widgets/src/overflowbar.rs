//! [`OverflowBar`]: a row of buttons that becomes a **column** when they stop fitting.
//!
//! A flex row does not wrap and its children do not shrink, so a row of buttons that runs
//! out of room does not squash and does not fold: it keeps its natural width and runs
//! straight past whatever is holding it. In a dialog that is not a rare case. It is what
//! a reader who has turned their system font size up gets, every time: the buttons grow,
//! the dialog does not, and "Delete permanently" leaves the screen.
//!
//! This is the widget for that. One line while the children fit on one line, a column
//! when they do not — never a squashed row, and never a second line with one button
//! stranded on it.
//!
//! ```ignore
//! OverflowBar::new()
//!     .spacing(8.0)
//!     .alignment(Justify::End)
//!     .overflow_alignment(Align::End)
//!     .child(button("Cancel", Msg::Close))
//!     .child(button("Delete permanently", Msg::Delete))
//! ```
//!
//! ## All of them, or none
//!
//! It is not [`Wrap`](crate::Wrap), and the difference is the point. A wrap fills each
//! line and moves what is left to the next, so three buttons become two and one — which
//! for a row of chips is exactly right and for a row of buttons reads as a mistake, the
//! last one hanging alone under the others. A bar folds **all** of them or none.
//!
//! ## How it knows
//!
//! The available width is not known when the tree is built, so the arrangement cannot be
//! either: it is chosen during layout, from the box actually offered, against the natural
//! width of the children measured under the theme in force — which is what makes the text
//! scale reach it. Two consequences, both of them the price of building late:
//!
//! - **The children are held by shared pointer** ([`std::rc::Rc`]), because the subtree is
//!   composed more than once per frame — once to measure, once to draw.
//! - **A deferred overlay declared inside one does not open.** Clicks, hovers, focus and
//!   the accessibility tree all work; a tooltip on a button in a bar does not, for the
//!   reason [`LayoutBuilder`](crate::LayoutBuilder) documents. Put it around the bar
//!   rather than inside it.

use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use frus_core::{Rect, Scene, Size};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::rowcolumn::VerticalDirection;
use crate::theme::Theme;
use crate::widget::Widget;

/// The arrangement, once the width it is laid out in is known — the same shape a
/// [`GridView`](crate::GridView) that builds its cells late holds.
type Composed<Msg> = Box<dyn Fn(Size) -> Box<dyn Widget<Msg>>>;

/// A row of children that folds into a **column** when they do not fit on one line.
///
/// A flex row does not wrap and its children do not shrink, so a row of buttons that has
/// run out of room keeps its natural width and is drawn straight past whatever is holding
/// it — which is what a reader who has turned their system font size up gets, every time.
/// A bar folds instead, and folds **all** of them rather than stranding the last one on a
/// second line the way a [`Wrap`](crate::Wrap) would.
///
/// ```ignore
/// OverflowBar::new()
///     .spacing(8.0)
///     .alignment(Justify::End)
///     .overflow_alignment(Align::End)
///     .child(button("Cancel", Msg::Close))
///     .child(button("Delete permanently", Msg::Delete))
/// ```
///
/// The arrangement is chosen **during layout**, from the box actually offered and the
/// natural width of the children measured under the theme in force — which is what lets
/// the text scale reach it. That has a price, the same one
/// [`LayoutBuilder`](crate::LayoutBuilder) pays: the subtree is composed more than once a
/// frame, so the children are held by shared pointer, and a **deferred overlay declared
/// inside a bar does not open**. Clicks, hover, focus and the accessibility tree all
/// work; a tooltip on a button in a bar does not, and belongs around the bar instead.
///
/// No child is ever wider than the bar: one that would be is capped, and ellipsises or
/// wraps inside its own box rather than escaping it.
pub struct OverflowBar<Msg> {
    children: Vec<Rc<dyn Widget<Msg>>>,
    spacing: f32,
    overflow_spacing: f32,
    alignment: Justify,
    overflow_alignment: Align,
    overflow_direction: VerticalDirection,
    /// Composed on the first ask and kept, because the hook hands back a borrow.
    composed: OnceCell<Composed<Msg>>,
}

impl<Msg: 'static> Default for OverflowBar<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: 'static> OverflowBar<Msg> {
    /// An empty bar: children packed at the start of the line, nothing between them.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: 0.0,
            overflow_spacing: 0.0,
            alignment: Justify::Start,
            overflow_alignment: Align::Start,
            overflow_direction: VerticalDirection::Down,
            composed: OnceCell::new(),
        }
    }

    /// Adds a child, after the ones already there.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Rc::new(child));
        self
    }

    /// Adds an **already boxed** child — what a builder holding `Vec<Box<dyn Widget>>`
    /// has to hand.
    pub fn child_boxed(mut self, child: Box<dyn Widget<Msg>>) -> Self {
        self.children.push(Rc::from(child));
        self
    }

    /// The space between the children **on one line**.
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    /// The space between them **once they have stacked**.
    ///
    /// A separate number rather than [`spacing`](Self::spacing) reused, because the two
    /// are not the same distance: buttons side by side sit closer than buttons above one
    /// another, where the gap is what keeps a mis-tap from being the wrong answer.
    #[must_use]
    pub fn overflow_spacing(mut self, spacing: f32) -> Self {
        self.overflow_spacing = spacing.max(0.0);
        self
    }

    /// How the children are distributed **along the line** while they fit.
    #[must_use]
    pub fn alignment(mut self, alignment: Justify) -> Self {
        self.alignment = alignment;
        self
    }

    /// Which edge the **column** lines up on once they have stacked.
    ///
    /// [`Align::Stretch`] is not one of the answers — a stacked column of buttons each
    /// stretched to the full width is a different design, and one a caller can build with
    /// a [`Flex`](crate::Flex) — so it is read as [`Align::Start`].
    #[must_use]
    pub fn overflow_alignment(mut self, alignment: Align) -> Self {
        self.overflow_alignment = match alignment {
            Align::Stretch | Align::Baseline => Align::Start,
            other => other,
        };
        self
    }

    /// Which way the column runs once they have stacked.
    ///
    /// It matters because the conventions disagree with each other: the confirming button
    /// goes **last** on a line and **first** in a column, so a bar that folded without
    /// being told would move the destructive answer under the reader's thumb.
    /// [`VerticalDirection::Up`] keeps it where it was.
    #[must_use]
    pub fn overflow_direction(mut self, direction: VerticalDirection) -> Self {
        self.overflow_direction = direction;
        self
    }
}

impl<Msg: 'static> Widget<Msg> for OverflowBar<Msg> {
    /// No box of its own: it is as wide as it is offered and as tall as what it arranged.
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// The arrangement is built from the box actually offered — see the module docs.
    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        if self.children.is_empty() {
            return None;
        }
        let children = self.children.clone();
        let (spacing, overflow_spacing) = (self.spacing, self.overflow_spacing);
        let (alignment, overflow_alignment) = (self.alignment, self.overflow_alignment);
        let overflow_direction = self.overflow_direction;
        Some(&**self.composed.get_or_init(|| {
            Box::new(move |size: Size| {
                Box::new(Bar {
                    children: children
                        .iter()
                        .map(|child| Box::new(Capped::new(child.clone())) as Box<dyn Widget<Msg>>)
                        .collect(),
                    offered: size.width,
                    spacing,
                    overflow_spacing,
                    alignment,
                    overflow_alignment,
                    overflow_direction,
                    stacked: Cell::new(None),
                }) as Box<dyn Widget<Msg>>
            })
        }))
    }

    fn debug_name(&self) -> &'static str {
        "OverflowBar"
    }
}

/// A child of a bar, held by shared pointer and **never wider than the bar**.
///
/// The cap is the reference's rule and the second half of the fix: folding two buttons
/// into a column does nothing for a single button that is wider than the surface on its
/// own, which is what the longest label becomes at a large enough font size. A ceiling of
/// a hundred per cent leaves anything that fits exactly as it was — a percentage of an
/// indefinite width is no ceiling at all, so an intrinsic measurement still comes back
/// with the child's own width — and gives the ones that do not something to wrap or
/// ellipsise inside.
struct Capped<Msg> {
    inner: Rc<dyn Widget<Msg>>,
}

impl<Msg> Capped<Msg> {
    fn new(inner: Rc<dyn Widget<Msg>>) -> Self {
        Self { inner }
    }

    fn restyle(&self, base: Style) -> Style {
        Style {
            max_width: Dimension::Percent(1.0),
            ..base
        }
    }
}

crate::transparent::forward_transparent!(Capped {
    /// Forwarded, all of them: a ceiling on the width is not an identity, not a place,
    /// not a theme and not a surface.
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
});

/// One arrangement of a bar: the children, and the width they were offered.
///
/// It exists because the two halves of the answer arrive at different moments. The
/// **width** comes from the layout, through the closure above; the **theme** comes from
/// [`Widget::style_themed`], and there is no getting at it before then — which matters,
/// because a reader's font size lives in it and changing the font size is the everyday
/// way to make a row of buttons stop fitting.
struct Bar<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    offered: f32,
    spacing: f32,
    overflow_spacing: f32,
    alignment: Justify,
    overflow_alignment: Align,
    overflow_direction: VerticalDirection,
    /// The answer, once someone has asked for it. `style_themed` is called more than once
    /// on the same arrangement and the measurement is a layout pass per child.
    stacked: Cell<Option<bool>>,
}

impl<Msg: 'static> Bar<Msg> {
    /// Whether the children have to stack: their natural widths and the gaps between
    /// them, against the width offered.
    ///
    /// **An offer of nothing is an intrinsic question** — *how wide would you like to
    /// be?* — and the honest answer to that is the row, which is the width a parent
    /// deciding how much to offer needs to hear. Answering "a column" would let a bar
    /// talk a dialog into being narrow and then complain that it was.
    fn stacked(&self, theme: &Theme) -> bool {
        if self.offered <= 0.0 || self.children.len() < 2 {
            return false;
        }
        if let Some(known) = self.stacked.get() {
            return known;
        }
        let runtime = crate::runtime::Runtime::default();
        let mut needed = self.spacing * (self.children.len() - 1) as f32;
        for (index, child) in self.children.iter().enumerate() {
            needed += crate::ui::natural_size(
                child.as_ref(),
                crate::interaction::WidgetId::ROOT.child(index),
                &runtime,
                theme,
            )
            .width;
        }
        // Half a pixel of slack: a row that fits to within a rounding error fits, and
        // folding a dialog's buttons over one is a change nobody can see the reason for.
        let stacked = needed > self.offered + 0.5;
        self.stacked.set(Some(stacked));
        stacked
    }

    fn arrangement(&self, stacked: bool) -> Style {
        if stacked {
            return Style {
                flex_direction: match self.overflow_direction {
                    VerticalDirection::Down => FlexDirection::Column,
                    VerticalDirection::Up => FlexDirection::ColumnReverse,
                },
                align: self.overflow_alignment,
                gap: self.overflow_spacing,
                width: Dimension::Percent(1.0),
                ..Default::default()
            };
        }
        Style {
            flex_direction: FlexDirection::Row,
            justify: self.alignment,
            gap: self.spacing,
            ..Default::default()
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Bar<Msg> {
    /// Without a theme there is no measuring to be done — no font size, no button
    /// padding — so this is the row, the same answer an intrinsic question gets.
    fn style(&self) -> Style {
        self.arrangement(false)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.arrangement(self.stacked(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "OverflowBar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, MediaQuery, Runtime};
    use frus_core::{Color, Primitive};

    type Msg = ();

    /// A fixed box, so a bar's arithmetic is the test's arithmetic.
    fn block(width: f32) -> Container<Msg> {
        Container::new()
            .width(width)
            .height(20.0)
            .color(Color::WHITE)
    }

    /// The boxes a bar drew, in paint order.
    fn boxes(bar: OverflowBar<Msg>, width: f32) -> Vec<Rect> {
        let ui = build_ui(
            &crate::Flex::<Msg>::column().width(width).child(bar),
            Size::new(width, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.a > 0.5 => Some(*rect),
                _ => None,
            })
            .collect()
    }

    /// **While they fit, it is a row** — and nothing about it is different from the row
    /// it replaces, which is what lets a dialog take it without moving.
    #[test]
    fn children_that_fit_stay_on_one_line() {
        let bar = OverflowBar::new()
            .spacing(8.0)
            .child(block(60.0))
            .child(block(60.0));
        let drawn = boxes(bar, 300.0);
        assert_eq!(drawn.len(), 2);
        assert_eq!(drawn[0].y, drawn[1].y, "one line: {drawn:?}");
        assert_eq!(drawn[1].x, drawn[0].x + 60.0 + 8.0, "spaced by `spacing`");
    }

    /// **When they do not, they stack** — all of them, rather than the last one alone on
    /// a second line, which is what a wrap would have done.
    #[test]
    fn children_that_do_not_fit_stack() {
        let bar = OverflowBar::new()
            .spacing(8.0)
            .overflow_spacing(4.0)
            .child(block(120.0))
            .child(block(120.0));
        let drawn = boxes(bar, 200.0);
        assert_eq!(drawn.len(), 2);
        assert_eq!(drawn[0].x, drawn[1].x, "one column: {drawn:?}");
        assert_eq!(
            drawn[1].y,
            drawn[0].y + 20.0 + 4.0,
            "spaced by `overflow_spacing`, not by `spacing`"
        );
    }

    /// **Nothing leaves the box.** The bug this widget exists for was not a row that
    /// looked wrong; it was buttons drawn outside the surface holding them.
    #[test]
    fn nothing_is_drawn_outside_the_width_it_was_given() {
        let bar = OverflowBar::new()
            .spacing(8.0)
            .child(block(120.0))
            .child(block(150.0));
        for rect in boxes(bar, 200.0) {
            assert!(rect.x >= 0.0, "starts inside: {rect:?}");
            assert!(rect.x + rect.width <= 200.0 + 0.5, "ends inside: {rect:?}");
        }
    }

    /// **The stacked column lines up where it was told**, which is the whole of
    /// `overflow_alignment`: a dialog wants its answers on the trailing edge, not
    /// scattered under the question.
    #[test]
    fn a_stacked_column_takes_its_own_alignment() {
        let ends = OverflowBar::new()
            .overflow_alignment(Align::End)
            .child(block(120.0))
            .child(block(150.0));
        let drawn = boxes(ends, 200.0);
        assert_eq!(drawn[0].x + drawn[0].width, 200.0, "flush right: {drawn:?}");
        assert_eq!(drawn[1].x + drawn[1].width, 200.0);
    }

    /// **`Up` puts the first child at the bottom**, which is how the confirming button
    /// stays where a reader expects it: last on a line, first in a column.
    #[test]
    fn the_column_can_run_the_other_way() {
        let bar = |direction| {
            OverflowBar::new()
                .overflow_direction(direction)
                .child(block(120.0))
                .child(block(150.0))
        };
        let down = boxes(bar(VerticalDirection::Down), 200.0);
        let up = boxes(bar(VerticalDirection::Up), 200.0);
        // The wide one is second in both; it is at the bottom in one and the top in the
        // other. Paint order is the order the children were given, either way.
        assert!(down[1].y > down[0].y, "first at the top: {down:?}");
        assert!(up[1].y < up[0].y, "first at the bottom: {up:?}");
    }

    /// **A reader's font size is what makes a row stop fitting**, so it is the case the
    /// widget has to answer: the same two buttons, the same box, and only the ambient
    /// text scale different.
    #[test]
    fn a_larger_text_scale_is_what_folds_it() {
        let bar = || {
            OverflowBar::<Msg>::new()
                .spacing(8.0)
                .child(crate::dsl::button("Cancel", ()))
                .child(crate::dsl::button("Delete permanently", ()))
        };
        let ordinary = MediaQuery::new(Size::new(340.0, 400.0)).scope(|| boxes(bar(), 340.0));
        assert_eq!(
            ordinary[0].y, ordinary[1].y,
            "at the ordinary size they fit: {ordinary:?}"
        );
        let large = MediaQuery::new(Size::new(340.0, 400.0))
            .with_text_scaler(2.0)
            .scope(|| boxes(bar(), 340.0));
        assert_eq!(large[0].x, large[1].x, "turned up, they stack: {large:?}");
    }

    /// **Asked how wide it would like to be, a bar answers as a row.** A parent that
    /// sizes itself to its content — a dialog — has to hear the width the children
    /// actually want, or it would offer the narrow box it was told about and then be told
    /// the children do not fit in it.
    #[test]
    fn its_natural_width_is_the_row_s() {
        let bar = OverflowBar::new()
            .spacing(8.0)
            .child(block(120.0))
            .child(block(150.0));
        // A column that hugs its content, in a window far wider than the row.
        let ui = build_ui(
            &crate::Flex::<Msg>::column().child(bar),
            Size::new(1000.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let drawn: Vec<Rect> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.a > 0.5 => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(drawn[0].y, drawn[1].y, "one line: {drawn:?}");
    }

    /// An empty bar is nothing at all, and one child never has anything to fold.
    #[test]
    fn an_empty_bar_builds_nothing() {
        let empty = OverflowBar::<Msg>::new();
        assert!(Widget::<Msg>::layout_builder(&empty).is_none());
        let one = OverflowBar::<Msg>::new().child(block(400.0));
        let drawn = boxes(one, 200.0);
        assert_eq!(drawn.len(), 1, "still drawn: {drawn:?}");
    }
}

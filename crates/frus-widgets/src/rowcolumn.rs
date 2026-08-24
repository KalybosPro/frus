//! [`Row`] and [`Column`]: the two containers almost every screen is built out of.
//!
//! [`crate::Flex`] has laid children out in a line since the beginning, and it is a
//! flexbox container: it shrink-wraps its line, it stretches its children across it, and
//! its `start` is the left whatever the reading direction. Those are flexbox's answers,
//! and they are not the reference's. A row there **fills the line it is given**, aligns
//! its children on their **centres**, and reads `start` as the start *of the reading
//! direction*.
//!
//! Those three differences are not decoration. A row of buttons that shrink-wraps
//! silently ignores a `SpaceBetween` — there is no space to put between anything. A row
//! that stretches its children makes every icon beside a two-line label as tall as the
//! label. And a `start` that means "left" puts the Arabic label on the wrong side.
//!
//! So `Row` and `Column` are not aliases for `Flex::row()` and `Flex::column()`. They
//! are the same machinery with the reference's defaults and the reference's two ordering
//! knobs — which way the main axis runs, and which way the cross axis runs — and those
//! are what the rest of this module is about.

use frus_core::{Rect, Scene, TextDirection};
use frus_layout::{Align, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// How much room a [`Row`] or [`Column`] takes **along its own main axis**.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MainAxisSize {
    /// All of it. A row fills the width it is offered, a column the height — which is
    /// what makes `SpaceBetween`, `SpaceAround` and `End` mean anything at all, and it
    /// is the default here as it is in the reference.
    #[default]
    Max,
    /// Only as much as the children need: the line shrink-wraps. This is what a row
    /// inside a button or a chip wants, where the box is measured *from* its contents.
    Min,
}

/// Which way the **vertical** axis runs: a column's main axis, a row's cross axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VerticalDirection {
    /// Top to bottom — the ordinary reading order, and the default.
    #[default]
    Down,
    /// Bottom to top. A column laid out this way puts its first child at the bottom and
    /// grows upwards, which is the shape of a chat transcript or a stack of toasts.
    Up,
}

/// A **horizontal** run of children.
///
/// ```ignore
/// Row::new()
///     .main_axis_alignment(Justify::SpaceBetween)
///     .cross_axis_alignment(Align::Center)
///     .spacing(8.0)
///     .child(Text::new("Total"))
///     .child(Text::new("42.00"))
/// ```
///
/// The defaults are the reference's, and they differ from [`crate::Flex`]'s on three
/// counts: the row **fills** the width it is offered ([`MainAxisSize::Max`]), it aligns
/// its children on their **centres** rather than stretching them, and its main axis runs
/// with the **reading direction**.
///
/// To give one child the space the others do not need, wrap it in [`crate::Expanded`];
/// to let it take that space without being forced to fill it,
/// [`Expanded::loose`](crate::Expanded::loose). To put the children on one **text
/// baseline** rather than on their centres, ask for [`Align::Baseline`].
pub struct Row<Msg> {
    common: Common<Msg>,
}

/// A **vertical** run of children.
///
/// ```ignore
/// Column::new()
///     .cross_axis_alignment(Align::Start)
///     .main_axis_size(MainAxisSize::Min)
///     .spacing(4.0)
///     .child(Text::new("Title"))
///     .child(Text::new("Subtitle"))
/// ```
///
/// Everything [`Row`] says applies, with the axes swapped: the column fills the height
/// it is offered, centres its children horizontally, and runs top to bottom.
///
/// A column does not scroll. More children than fit is an overflow, reported as one; if
/// the content can outgrow the screen, that is what [`crate::ListView`] is for.
pub struct Column<Msg> {
    common: Common<Msg>,
}

/// Everything the two share. Splitting it out is what lets the two types differ only in
/// **which axis is which**, which is the only thing that actually differs.
struct Common<Msg> {
    main: Justify,
    cross: Align,
    size: MainAxisSize,
    spacing: f32,
    text_direction: Option<TextDirection>,
    vertical_direction: VerticalDirection,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Default for Common<Msg> {
    fn default() -> Self {
        Self {
            main: Justify::Start,
            // The reference's default, and not flexbox's: stretching is the wrong answer
            // for a line of mixed content, where it makes an icon as tall as the
            // paragraph beside it.
            cross: Align::Center,
            size: MainAxisSize::Max,
            spacing: 0.0,
            text_direction: None,
            vertical_direction: VerticalDirection::Down,
            children: Vec::new(),
        }
    }
}

/// Writes the settings both types share, given which axis each one governs.
macro_rules! shared_api {
    ($t:ident) => {
        impl<Msg> $t<Msg> {
            /// A run with the reference's defaults and no children yet.
            pub fn new() -> Self {
                Self {
                    common: Common::default(),
                }
            }

            /// How the children are distributed **along** the main axis.
            pub fn main_axis_alignment(mut self, alignment: Justify) -> Self {
                self.common.main = alignment;
                self
            }

            /// How the children are aligned **across** the main axis.
            pub fn cross_axis_alignment(mut self, alignment: Align) -> Self {
                self.common.cross = alignment;
                self
            }

            /// Whether the run fills its main axis or shrink-wraps its children.
            pub fn main_axis_size(mut self, size: MainAxisSize) -> Self {
                self.common.size = size;
                self
            }

            /// Shrink-wraps the main axis: [`MainAxisSize::Min`], named.
            pub fn shrink_wrap(self) -> Self {
                self.main_axis_size(MainAxisSize::Min)
            }

            /// Space between adjacent children, in logical pixels. It is not added
            /// before the first or after the last.
            pub fn spacing(mut self, spacing: f32) -> Self {
                self.common.spacing = spacing;
                self
            }

            /// The **reading direction** this run should use, overriding the ambient one.
            ///
            /// For a [`Row`] it decides which end the first child goes to; for a
            /// [`Column`] it decides which side `Align::Start` means. Left unset — the
            /// usual case — the run follows the theme, and an application flips as a
            /// whole rather than a container at a time.
            pub fn text_direction(mut self, direction: TextDirection) -> Self {
                self.common.text_direction = Some(direction);
                self
            }

            /// Which way the **vertical** axis runs: a [`Column`]'s order, a [`Row`]'s
            /// idea of which edge `Align::Start` means.
            pub fn vertical_direction(mut self, direction: VerticalDirection) -> Self {
                self.common.vertical_direction = direction;
                self
            }

            /// Adds a child.
            pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
                self.common.children.push(Box::new(child));
                self
            }

            /// Adds an **already boxed** child, for children built in a loop.
            pub fn child_boxed(mut self, child: Box<dyn Widget<Msg>>) -> Self {
                self.common.children.push(child);
                self
            }

            /// Adds every child of an iterator.
            pub fn children(
                mut self,
                children: impl IntoIterator<Item = Box<dyn Widget<Msg>>>,
            ) -> Self {
                self.common.children.extend(children);
                self
            }
        }

        impl<Msg> Default for $t<Msg> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

shared_api!(Row);
shared_api!(Column);

impl<Msg> Common<Msg> {
    /// Whether this run reads **against** the ambient direction, which is the only thing
    /// a per-container reading direction can mean here.
    ///
    /// The frame is mirrored as a whole for a right-to-left theme, so a run that agrees
    /// with the ambient direction needs no reversal of its own — and one that disagrees
    /// needs exactly one, whichever direction the two of them happen to be.
    fn reads_backwards(&self, theme: &Theme) -> bool {
        matches!(self.text_direction, Some(d) if d != theme.direction)
    }

    /// The cross alignment, with `Start` and `End` swapped when the cross axis runs
    /// backwards. Everything else is symmetric and unaffected.
    fn cross_flipped(&self, flipped: bool) -> Align {
        match (self.cross, flipped) {
            (Align::Start, true) => Align::End,
            (Align::End, true) => Align::Start,
            (align, _) => align,
        }
    }

    fn style(&self, direction: FlexDirection, cross_flipped: bool) -> Style {
        Style {
            flex_direction: direction,
            justify: self.main,
            align: self.cross_flipped(cross_flipped),
            gap: self.spacing,
            ..Default::default()
        }
    }

    /// The axis to fill, or `None` when the run shrink-wraps. The direction returned is
    /// the **unreversed** one: filling is about which axis, not which way along it.
    fn fill(&self, axis: FlexDirection) -> Option<FlexDirection> {
        match self.size {
            MainAxisSize::Max => Some(axis),
            MainAxisSize::Min => None,
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Row<Msg> {
    fn style(&self) -> Style {
        self.common.style(FlexDirection::Row, false)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let direction = if self.common.reads_backwards(theme) {
            FlexDirection::RowReverse
        } else {
            FlexDirection::Row
        };
        // A row's cross axis is the vertical one, so it is `vertical_direction` that
        // decides whether `Start` means the top or the bottom.
        let flipped = self.common.vertical_direction == VerticalDirection::Up;
        self.common.style(direction, flipped)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.common.children
    }

    fn main_axis_fill(&self, _theme: &Theme) -> Option<FlexDirection> {
        self.common.fill(FlexDirection::Row)
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A run of children paints nothing of its own; wrap it if it needs a background.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "Row"
    }
}

impl<Msg: Clone> Widget<Msg> for Column<Msg> {
    fn style(&self) -> Style {
        self.common.style(FlexDirection::Column, false)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let direction = if self.common.vertical_direction == VerticalDirection::Up {
            FlexDirection::ColumnReverse
        } else {
            FlexDirection::Column
        };
        // A column's cross axis is the horizontal one, so it is the reading direction
        // that decides whether `Start` means the left or the right.
        let flipped = self.common.reads_backwards(theme);
        self.common.style(direction, flipped)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.common.children
    }

    fn main_axis_fill(&self, _theme: &Theme) -> Option<FlexDirection> {
        self.common.fill(FlexDirection::Column)
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "Column"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Expanded};
    use frus_core::{Color, Primitive, Size};

    /// The painted rectangles, in walk order — the only honest way to ask where a
    /// layout put things.
    fn boxes(root: &impl Widget<()>, size: Size, theme: &Theme) -> Vec<Rect> {
        let rt = crate::runtime::Runtime::default();
        let ui = crate::ui::build_ui(root, size, &rt, theme);
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    fn tile(w: f32, h: f32) -> Container<()> {
        Container::new().width(w).height(h).color(Color::WHITE)
    }

    /// The difference that started it: a row **fills its line**, so a distribution has
    /// something to distribute. `Flex::row()` shrink-wraps and `SpaceBetween` does
    /// nothing at all.
    #[test]
    fn a_row_fills_the_line_it_is_given() {
        let theme = Theme::dark();
        let root = Container::new().width(300.0).height(100.0).child(
            Row::<()>::new()
                .main_axis_alignment(Justify::SpaceBetween)
                .child(tile(20.0, 20.0))
                .child(tile(20.0, 20.0)),
        );
        let rects = boxes(&root, Size::new(400.0, 200.0), &theme);
        // The outer box paints nothing of its own, so the two tiles are all there is.
        let (first, last) = (rects[0], rects[1]);
        assert!(first.x < 1.0, "at the left edge: {}", first.x);
        assert!(
            (last.x + last.width - 300.0).abs() < 1.0,
            "and at the right: {}",
            last.x + last.width
        );
    }

    /// And asked to shrink-wrap, it does: the two tiles end up side by side with
    /// nothing between them, however wide the box around them is.
    #[test]
    fn shrink_wrapped_it_does_not() {
        let theme = Theme::dark();
        let root = Container::new().width(300.0).height(100.0).child(
            Row::<()>::new()
                .shrink_wrap()
                .main_axis_alignment(Justify::SpaceBetween)
                .child(tile(20.0, 20.0))
                .child(tile(20.0, 20.0)),
        );
        let rects = boxes(&root, Size::new(400.0, 200.0), &theme);
        assert!((rects[1].x - 20.0).abs() < 1.0, "abutting: {}", rects[1].x);
    }

    /// The second difference: children are **centred** across the line rather than
    /// stretched to fill it, so a short thing beside a tall one stays short.
    #[test]
    fn children_are_centred_across_the_line_not_stretched() {
        let theme = Theme::dark();
        let root = Row::<()>::new()
            .child(tile(20.0, 60.0))
            .child(tile(20.0, 20.0));
        let rects = boxes(&root, Size::new(300.0, 100.0), &theme);
        assert!((rects[1].height - 20.0).abs() < 0.5, "still short");
        // Centred inside the 60 the tall one sets: (60 - 20) / 2.
        assert!((rects[1].y - 20.0).abs() < 1.0, "centred: {}", rects[1].y);
    }

    /// The third: `Justify::Start` is the start **of the reading direction**. Setting
    /// the row's own direction against the ambient one reverses it.
    #[test]
    fn a_reading_direction_of_its_own_reverses_the_row() {
        let theme = Theme::dark();
        let root = Row::<()>::new()
            .text_direction(TextDirection::Rtl)
            .child(tile(20.0, 20.0))
            .child(tile(40.0, 20.0));
        let rects = boxes(&root, Size::new(300.0, 100.0), &theme);
        // The first child is now at the right-hand end.
        assert!(
            (rects[0].x + rects[0].width - 300.0).abs() < 1.0,
            "first child at the right: {}",
            rects[0].x
        );
        assert!(
            (rects[1].x + rects[1].width - 280.0).abs() < 1.0,
            "and the second beside it: {}",
            rects[1].x
        );
    }

    /// Agreeing with the ambient direction is not a reversal: the frame is mirrored as
    /// a whole, and a second reversal would undo it.
    #[test]
    fn agreeing_with_the_theme_changes_nothing() {
        let plain = Row::<()>::new()
            .child(tile(20.0, 20.0))
            .child(tile(40.0, 20.0));
        let spelled = Row::<()>::new()
            .text_direction(TextDirection::Ltr)
            .child(tile(20.0, 20.0))
            .child(tile(40.0, 20.0));
        let theme = Theme::dark();
        let size = Size::new(300.0, 100.0);
        assert_eq!(boxes(&plain, size, &theme), boxes(&spelled, size, &theme));
    }

    /// A column that runs upwards puts its first child at the bottom.
    #[test]
    fn a_column_can_grow_upwards() {
        let theme = Theme::dark();
        let root = Column::<()>::new()
            .vertical_direction(VerticalDirection::Up)
            .child(tile(20.0, 20.0))
            .child(tile(20.0, 40.0));
        let rects = boxes(&root, Size::new(100.0, 300.0), &theme);
        assert!(
            (rects[0].y + rects[0].height - 300.0).abs() < 1.0,
            "first child at the bottom: {}",
            rects[0].y
        );
    }

    /// `Align::Start` on a row follows the vertical direction, which is what makes it
    /// mean *the start of the cross axis* and not *the top*.
    #[test]
    fn an_upward_row_starts_at_the_bottom() {
        let theme = Theme::dark();
        let row = |direction: VerticalDirection| {
            Row::<()>::new()
                .cross_axis_alignment(Align::Start)
                .vertical_direction(direction)
                .child(tile(20.0, 60.0))
                .child(tile(20.0, 20.0))
        };
        let down = boxes(
            &row(VerticalDirection::Down),
            Size::new(300.0, 100.0),
            &theme,
        );
        let up = boxes(&row(VerticalDirection::Up), Size::new(300.0, 100.0), &theme);
        assert!((down[1].y - 0.0).abs() < 1.0, "top: {}", down[1].y);
        assert!((up[1].y - 40.0).abs() < 1.0, "bottom: {}", up[1].y);
    }

    /// Filling is a question about the **parent**, not about the row: down a column the
    /// row stretches across, and the width it ends up with is the column's.
    #[test]
    fn a_row_in_a_column_fills_the_column() {
        let theme = Theme::dark();
        let root = Container::new().width(300.0).height(200.0).child(
            Column::<()>::new().child(
                Row::<()>::new()
                    .main_axis_alignment(Justify::End)
                    .child(tile(20.0, 20.0)),
            ),
        );
        let rects = boxes(&root, Size::new(400.0, 300.0), &theme);
        assert!(
            (rects[0].x + rects[0].width - 300.0).abs() < 1.0,
            "pushed to the right edge of the column: {}",
            rects[0].x
        );
    }

    /// `Expanded` still works inside one, which is the other half of how a line is
    /// divided up.
    #[test]
    fn expanded_takes_what_is_left() {
        let theme = Theme::dark();
        let root = Container::new().width(300.0).height(100.0).child(
            Row::<()>::new()
                .child(tile(100.0, 20.0))
                .child(Expanded::new(tile(10.0, 20.0))),
        );
        let rects = boxes(&root, Size::new(400.0, 200.0), &theme);
        assert!(
            (rects[1].width - 200.0).abs() < 1.0,
            "the rest of the line: {}",
            rects[1].width
        );
    }

    /// A column of columns is the ordinary case, and the fill has to reach through: the
    /// inner one is stretched by the outer, not shrink-wrapped inside it.
    #[test]
    fn spacing_goes_between_children_and_not_around_them() {
        let theme = Theme::dark();
        let root = Column::<()>::new()
            .spacing(10.0)
            .child(tile(20.0, 20.0))
            .child(tile(20.0, 20.0));
        let rects = boxes(&root, Size::new(100.0, 300.0), &theme);
        assert!((rects[0].y - 0.0).abs() < 0.5, "no leading gap");
        assert!((rects[1].y - 30.0).abs() < 0.5, "one gap: {}", rects[1].y);
    }

    /// `SpaceEvenly` is the distribution `SpaceAround` is mistaken for: the gap at each
    /// end is the same as the gaps between.
    #[test]
    fn space_evenly_makes_every_gap_the_same() {
        let theme = Theme::dark();
        let root = Container::new().width(300.0).height(100.0).child(
            Row::<()>::new()
                .main_axis_alignment(Justify::SpaceEvenly)
                .child(tile(20.0, 20.0))
                .child(tile(20.0, 20.0)),
        );
        let rects = boxes(&root, Size::new(400.0, 200.0), &theme);
        // Two 20px tiles in 300: three gaps of (300 - 40) / 3.
        let gap = 260.0 / 3.0;
        assert!((rects[0].x - gap).abs() < 1.0, "first gap: {}", rects[0].x);
        assert!(
            (rects[1].x - (2.0 * gap + 20.0)).abs() < 1.0,
            "second gap: {}",
            rects[1].x
        );
    }
}

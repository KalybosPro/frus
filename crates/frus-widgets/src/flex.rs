//! [`Flex`]: a container that lays its children out in a row or a column, with
//! distribution (justify) and alignment (align).

use frus_core::{Insets, Rect, Scene};
use frus_layout::{Align, AlignContent, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A flex container, row or column. It paints no decoration of its own.
pub struct Flex<Msg> {
    direction: FlexDirection,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    flex_shrink: f32,
    justify: Justify,
    align: Align,
    padding: Insets,
    gap: f32,
    /// Spacing **between the lines** of a wrapping container; `None` = `gap`.
    run_gap: Option<f32>,
    /// How those lines are distributed across the container.
    align_content: AlignContent,
    wrap: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Flex<Msg> {
    /// A container laying its children out horizontally.
    pub fn row() -> Self {
        Self::with_direction(FlexDirection::Row)
    }

    /// A container laying its children out vertically.
    pub fn column() -> Self {
        Self::with_direction(FlexDirection::Column)
    }

    fn with_direction(direction: FlexDirection) -> Self {
        Self {
            direction,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            justify: Justify::Start,
            align: Align::Stretch,
            padding: Insets::ZERO,
            gap: 0.0,
            run_gap: None,
            align_content: AlignContent::default(),
            wrap: false,
            children: Vec::new(),
        }
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Sets the height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Sets the width as a **fraction of the space the parent offers** — `1.0` fills it.
    ///
    /// The answer for a row that must occupy its parent rather than hug its children,
    /// when the number is the layout's to know and not the caller's. A row that hugs
    /// leaves a spring inside it nothing to share, so a centred child is not centred;
    /// naming a width the caller only guessed at is the other way to get that wrong.
    pub fn width_fraction(mut self, fraction: f32) -> Self {
        self.width = Dimension::Percent(fraction);
        self
    }

    /// The flex grow factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// How much of a row's deficit this box absorbs. The default is `0.0` — the
    /// reference's rule, where an inflexible child is never squeezed and a row that does
    /// not fit overflows and says so.
    ///
    /// `shrink(1.0)` asks for flexbox's behaviour instead: give way rather than let the
    /// row run over. It is the right answer for a box whose size is a preference rather
    /// than a requirement, and the wrong one for fixed chrome — an icon button at the end
    /// of a row should keep its width however long the label beside it grows.
    pub fn shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = shrink;
        self
    }

    /// This box never shrinks — the default said out loud, kept because a layout that
    /// depends on it reads better for saying so. See [`Self::shrink`].
    pub fn no_shrink(self) -> Self {
        self.shrink(0.0)
    }

    /// Lays the children out from the **far end** of the axis: a column from the bottom
    /// up, a row from the end of the line back. Their order is unchanged — the first
    /// child is simply placed last.
    ///
    /// It is what a transcript wants, where the newest line is at the bottom and the
    /// column grows upwards from there, and it composes with a scroll region that starts
    /// at its end.
    pub fn reverse(mut self) -> Self {
        self.direction = match self.direction {
            FlexDirection::Row => FlexDirection::RowReverse,
            FlexDirection::RowReverse => FlexDirection::Row,
            FlexDirection::Column => FlexDirection::ColumnReverse,
            FlexDirection::ColumnReverse => FlexDirection::Column,
        };
        self
    }

    /// How the children are distributed along the main axis.
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// How the children are aligned on the cross axis.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Uniform padding, in logical pixels.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::uniform(padding);
        self
    }

    /// Padding per side: top, right, bottom, left.
    pub fn padding_each(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Insets::new(top, right, bottom, left);
        self
    }

    /// Spacing between children, in logical pixels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Spacing **between the lines** of a wrapping container, where it differs from
    /// [`gap`](Self::gap) — the reference's `runSpacing` to `gap`'s `spacing`.
    ///
    /// One number for both is the wrong shape often enough to be worth two: a wrap of
    /// chips usually wants them close side by side and further apart line to line,
    /// because the eye reads a line as a unit and needs the break to see where it ends.
    ///
    /// Untold, the lines are spaced by `gap`, so nothing that says nothing changes.
    /// Silent on a container that does not wrap, which has only one line.
    pub fn run_gap(mut self, gap: f32) -> Self {
        self.run_gap = Some(gap);
        self
    }

    /// How the **lines** of a wrapping container are distributed across it — the
    /// reference's `runAlignment`.
    ///
    /// Not [`align`](Self::align), which places each child within its line. This places
    /// the lines themselves, and it has an answer only when there is cross-axis room to
    /// spare: three lines in a box exactly three lines tall sit where they sit.
    pub fn align_lines(mut self, align: AlignContent) -> Self {
        self.align_content = align;
        self
    }

    /// Turns **wrapping** on: children that overflow the main axis move to a new
    /// line, a responsive reflow. See also [`Wrap`] as a named entry point.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Adds a child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Adds an **already boxed** child — useful when children are built dynamically,
    /// in a `Vec<Box<dyn Widget>>`, and already erased to the dynamic type.
    pub fn child_boxed(mut self, child: Box<dyn Widget<Msg>>) -> Self {
        self.children.push(child);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Flex<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_direction: self.direction,
            justify: self.justify,
            align: self.align,
            padding: self.padding,
            margin: Insets::ZERO,
            aspect_ratio: None,
            gap: self.gap,
            // `run_gap` is spacing **between lines**, and which CSS axis that is
            // depends on which way the container runs: the lines of a wrapping *row*
            // stack downwards, the lines of a wrapping *column* stack sideways. Writing
            // it straight into `row_gap` would silently do the wrong thing to half the
            // wraps in the framework — and, worse, the right thing to the other half,
            // so it would look correct wherever anyone happened to check.
            row_gap: match self.direction {
                FlexDirection::Row | FlexDirection::RowReverse => self.run_gap,
                _ => None,
            },
            column_gap: match self.direction {
                FlexDirection::Column | FlexDirection::ColumnReverse => self.run_gap,
                _ => None,
            },
            align_content: self.align_content,
            flex_wrap: self.wrap,
            grid_columns: None,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A flex container is transparent: no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A named entry point for the **body of a scrollable**: the reference's `ListBody`.
///
/// Children one after another along an axis, each at its natural extent on that axis and
/// stretched across the other, starting at the beginning and never squashed. Its box grows
/// to hold them all, which is why it belongs inside something that scrolls — a body taller
/// than its parent with nowhere to go is an overflow, and says so.
///
/// It returns a [`Flex`], because in this framework that is already what a list body is:
/// the flex defaults here are `flex_shrink: 0`, `align: Stretch` and `justify: Start` — a
/// column that squashes nothing, stretches everything across, and packs from the top. What
/// the name adds is the **statement**: this column is the content of a viewport, not a
/// layout with opinions, and nothing in it is meant to flex. Every one of `Flex`'s
/// settings is still there, `gap` and `padding` included.
///
/// ```ignore
/// Scroll::new().child(
///     ListBody::vertical()
///         .gap(8.0)
///         .child(header())
///         .child(body()),
/// )
/// ```
///
/// [`ListBody::vertical`] is the reference's default. Its `reverse` is
/// [`Flex::reverse`], which lays the children out from the far end — the shape a chat
/// transcript wants, together with a scroll region that starts at the bottom.
pub struct ListBody;

impl ListBody {
    /// Children stacked downwards — the usual one.
    pub fn vertical<Msg>() -> Flex<Msg> {
        Flex::with_direction(FlexDirection::Column)
    }

    /// Children laid out across, in the reading direction.
    pub fn horizontal<Msg>() -> Flex<Msg> {
        Flex::with_direction(FlexDirection::Row)
    }
}

/// A named entry point for a **wrapping** row (flex-wrap).
///
/// `Wrap::new().gap(8.0).child(a).child(b)…` — the children flow over several lines
/// according to the available width, with no breakpoint. It returns a [`Flex`], a row
/// with `wrap` turned on, so all its settings remain available.
pub struct Wrap;

impl Wrap {
    /// A row with wrapping turned on.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<Msg>() -> Flex<Msg> {
        Flex::row().wrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::Color;

    /// Four fixed boxes that wrap two per line in a 210-wide window.
    fn wrapped(flex: Flex<()>) -> Vec<Rect> {
        let boxes = (0..4).fold(flex, |f, _| {
            f.child(
                Container::<()>::new()
                    .width(100.0)
                    .height(20.0)
                    .color(Color::WHITE),
            )
        });
        let ui = build_ui(
            &boxes,
            Size::new(210.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { rect, .. } if rect.width == 100.0 => Some(*rect),
                _ => None,
            })
            .collect()
    }

    /// The **lines** of a wrap can be spaced apart from the items on them — the
    /// reference\'s `runSpacing` beside its `spacing`. This is measured from the laid-out
    /// rectangles rather than read back off the style, because a field set and never
    /// reaching the layout engine is exactly the failure worth catching.
    #[test]
    fn a_wraps_lines_take_their_own_spacing() {
        let together = wrapped(Wrap::new::<()>().gap(4.0));
        assert_eq!(together.len(), 4, "two lines of two");
        // Line two starts one box height plus the gap below line one.
        assert_eq!(together[2].y - together[0].y, 24.0);

        let apart = wrapped(Wrap::new::<()>().gap(4.0).run_gap(30.0));
        assert_eq!(apart[2].y - apart[0].y, 50.0, "the lines, not the items");
        // The items on a line keep the plain gap.
        assert_eq!(apart[1].x - apart[0].x, 104.0);
    }

    /// **Which axis the runs stack on depends on which way the container runs.** The
    /// lines of a wrapping *row* stack downwards; the lines of a wrapping *column*
    /// stack sideways. Writing `run_gap` straight into `row_gap` would be right for
    /// half the wraps in the framework and silently wrong for the other half.
    #[test]
    fn the_run_gap_follows_the_direction() {
        let row = Widget::<()>::style(&Flex::<()>::row().wrap().run_gap(30.0));
        assert_eq!(row.row_gap, Some(30.0), "a row\'s lines stack downwards");
        assert_eq!(row.column_gap, None);

        let column = Widget::<()>::style(&Flex::<()>::column().wrap().run_gap(30.0));
        assert_eq!(column.column_gap, Some(30.0), "a column\'s stack sideways");
        assert_eq!(column.row_gap, None);
    }

    /// The lines can be packed rather than stretched. Flexbox stretches them by
    /// default, which is what we keep — so a wrap that says nothing is unchanged.
    #[test]
    fn a_wraps_lines_can_be_packed() {
        // The height is the point: a wrap sizes to its content, and lines with no
        // cross-axis room to spare sit where they sit whatever they are told. This is
        // what the first draft of the test got wrong — both alignments came out at
        // y = 0 and the feature looked broken when the container was.
        let tall = || Wrap::new::<()>().gap(4.0).height(300.0);
        let stretched = wrapped(tall());
        let packed = wrapped(tall().align_lines(AlignContent::End));
        // Stretched, the two lines share the 300; packed at the far edge they sit at
        // the bottom of it.
        assert_eq!(stretched[0].y, 0.0);
        assert!(
            packed[0].y > 200.0,
            "packed at the end, got {}",
            packed[0].y
        );
        // And they keep their own height rather than filling the room between them.
        assert_eq!(packed[2].y - packed[0].y, 24.0);
        assert!(
            stretched[2].y - stretched[0].y > 24.0,
            "stretched, the lines spread: {}",
            stretched[2].y - stretched[0].y
        );
    }

    /// A container that says nothing is untouched: no run gap, lines stretched, the
    /// same rectangles.
    #[test]
    fn a_wrap_that_says_nothing_is_what_it_was() {
        let style = Widget::<()>::style(&Wrap::new::<()>().gap(8.0));
        assert_eq!(style.row_gap, None);
        assert_eq!(style.column_gap, None);
        assert_eq!(style.align_content, AlignContent::Stretch);
    }

    #[test]
    fn wrap_sets_flex_wrap_in_style() {
        let plain = Flex::<()>::row();
        assert!(!Widget::<()>::style(&plain).flex_wrap);

        let wrapped = Wrap::new::<()>().child(Flex::<()>::row());
        assert!(Widget::<()>::style(&wrapped).flex_wrap);
    }

    /// The boxes of `list`, laid out in a 100 × 100 window, in paint order.
    fn laid_out(list: Flex<()>) -> Vec<Rect> {
        let ui = build_ui(
            &list,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .collect()
    }

    /// A row of `height` with no width of its own, red.
    fn row_of(height: f32) -> Container<()> {
        Container::<()>::new()
            .height(height)
            .color(Color::rgb(1.0, 0.0, 0.0))
    }

    /// The two halves of what a list body promises: each child at its own extent along
    /// the axis, and the whole width across it.
    #[test]
    fn a_list_body_gives_each_child_its_own_extent_and_the_full_width() {
        let boxes = laid_out(
            ListBody::vertical()
                .width(100.0)
                .child(row_of(20.0))
                .child(row_of(30.0)),
        );
        assert_eq!(boxes.len(), 2);
        assert_eq!((boxes[0].y, boxes[0].height), (0.0, 20.0));
        assert_eq!((boxes[1].y, boxes[1].height), (20.0, 30.0));
        assert!(
            boxes.iter().all(|r| (r.width - 100.0).abs() < 0.5),
            "stretched across: {boxes:?}"
        );
    }

    /// And the one that matters inside a viewport: children that do not fit are **not**
    /// squashed to make them. A body taller than its box overflows it, which is what a
    /// scroll region is for — and what the debug band says when there is no scroll region.
    #[test]
    fn a_list_body_never_squashes_a_child_that_does_not_fit() {
        let boxes = laid_out(
            ListBody::vertical()
                .width(100.0)
                .height(50.0)
                .child(row_of(40.0))
                .child(row_of(40.0)),
        );
        assert_eq!(boxes[0].height, 40.0);
        assert_eq!(boxes[1].height, 40.0, "the second one absorbed the deficit");
        assert_eq!(boxes[1].y, 40.0);
    }

    /// Reversed, the first child is placed last: a transcript grows from the bottom.
    #[test]
    fn a_reversed_column_lays_its_children_out_from_the_far_end() {
        let boxes = laid_out(
            ListBody::vertical()
                .reverse()
                .width(100.0)
                .height(100.0)
                .child(row_of(20.0))
                .child(row_of(30.0)),
        );
        assert_eq!(boxes[0].y, 80.0, "the first child sits at the bottom");
        assert_eq!(boxes[1].y, 50.0, "the second one above it");
    }

    /// Reversing twice is the direction it started with — the builder is a flip, not a
    /// flag, so `ListBody::horizontal().reverse()` is a row from the end.
    #[test]
    fn reversing_a_row_twice_is_the_row_it_was() {
        let there = Widget::<()>::style(&ListBody::horizontal::<()>().reverse()).flex_direction;
        let back = Widget::<()>::style(&ListBody::horizontal::<()>().reverse().reverse());
        assert_eq!(there, FlexDirection::RowReverse);
        assert_eq!(back.flex_direction, FlexDirection::Row);
    }
}

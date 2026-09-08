//! The **constraint boxes**: widgets whose whole job is to change the size their
//! child is allowed, or made, to be.
//!
//! - [`SizedBox`] — a fixed box, or one that fills, or one that hugs.
//! - [`ConstrainedBox`] — floors and ceilings on either axis.
//! - [`IntrinsicWidth`] / [`IntrinsicHeight`] — a box the size its content would
//!   *like* to be, rather than the size the space on offer suggests.
//! - [`OverflowBox`] — a child laid out to constraints of its own, which it may
//!   exceed, painted over whatever is around it.
//!
//! The first three are ordinary layout nodes and cost nothing beyond what taffy
//! already does. [`OverflowBox`] is not: its child is laid out **separately**, which
//! is what lets the child be bigger than the box that holds it.

use frus_core::{Alignment, Rect, Scene, Size};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A box of a given size — or one that fills what it is given, or one that takes
/// only what its child needs.
///
/// `SizedBox::empty()` with a size is also the plainest way to put a gap somewhere
/// that has no gap of its own; with no child it draws nothing at all.
pub struct SizedBox<Msg> {
    width: Dimension,
    height: Dimension,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> SizedBox<Msg> {
    /// An empty box, `Auto` on both axes until a size is set.
    pub fn empty() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            children: Vec::new(),
        }
    }

    /// A box holding `child`, its size still to be chosen.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let mut boxed = Self::empty();
        boxed.children.push(Box::new(child));
        boxed
    }

    /// A `side`×`side` box holding `child`.
    pub fn square(side: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self::new(child).width(side).height(side)
    }

    /// A box that **fills** the space on offer, on both axes, and hands it to
    /// `child`.
    pub fn expand(child: impl Widget<Msg> + 'static) -> Self {
        Self::new(child)
            .width_dimension(Dimension::Percent(1.0))
            .height_dimension(Dimension::Percent(1.0))
    }

    /// A box that takes **only what its child needs**, on both axes — the useful
    /// opposite of [`SizedBox::expand`] inside a container that would otherwise
    /// stretch it.
    pub fn shrink(child: impl Widget<Msg> + 'static) -> Self {
        Self::new(child)
    }

    /// Sets the width, in logical pixels.
    pub fn width(self, width: f32) -> Self {
        self.width_dimension(Dimension::Length(width))
    }

    /// Sets the height, in logical pixels.
    pub fn height(self, height: f32) -> Self {
        self.height_dimension(Dimension::Length(height))
    }

    /// Sets the width as a fraction of the parent's (`1.0` = all of it).
    pub fn width_fraction(self, fraction: f32) -> Self {
        self.width_dimension(Dimension::Percent(fraction))
    }

    /// Sets the height as a fraction of the parent's.
    pub fn height_fraction(self, fraction: f32) -> Self {
        self.height_dimension(Dimension::Percent(fraction))
    }

    fn width_dimension(mut self, width: Dimension) -> Self {
        self.width = width;
        self
    }

    fn height_dimension(mut self, height: Dimension) -> Self {
        self.height = height;
        self
    }

    /// Sets the child, replacing any already there.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for SizedBox<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure layout widget: it draws nothing of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Floors and ceilings on either axis, for a child that would otherwise be free to
/// take any size.
///
/// A floor and a ceiling that are equal make the box **tight**: whatever the child
/// or the space on offer says, that is the size. Set nothing and the box is
/// transparent — it is the constraints that make it worth having, not the box.
pub struct ConstrainedBox<Msg> {
    min_width: Dimension,
    max_width: Dimension,
    min_height: Dimension,
    max_height: Dimension,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ConstrainedBox<Msg> {
    /// An unconstrained box holding `child`; the constraints are set from here.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            min_width: Dimension::Auto,
            max_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_height: Dimension::Auto,
            children: vec![Box::new(child)],
        }
    }

    /// The same, for a child that is **already boxed** — what a slot holds once the
    /// caller's type has been erased.
    pub fn new_boxed(child: Box<dyn Widget<Msg>>) -> Self {
        Self {
            min_width: Dimension::Auto,
            max_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_height: Dimension::Auto,
            children: vec![child],
        }
    }

    /// A box whose floor **is** its ceiling on both axes: exactly `width`×`height`.
    pub fn tight(width: f32, height: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self::new(child)
            .min_width(width)
            .max_width(width)
            .min_height(height)
            .max_height(height)
    }

    /// A box with ceilings only: the child may be anything up to `width`×`height`.
    pub fn loose(width: f32, height: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self::new(child).max_width(width).max_height(height)
    }

    /// The width the box never goes below.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Dimension::Length(width);
        self
    }

    /// The width the box never goes above.
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Dimension::Length(width);
        self
    }

    /// The height the box never goes below.
    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = Dimension::Length(height);
        self
    }

    /// The height the box never goes above.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Dimension::Length(height);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ConstrainedBox<Msg> {
    fn style(&self) -> Style {
        Style {
            min_width: self.min_width,
            max_width: self.max_width,
            min_height: self.min_height,
            max_height: self.max_height,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Which axis a box takes from its content's **preferred** size rather than from
/// the space on offer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IntrinsicAxis {
    Width,
    Height,
}

/// A box sized to what its content would **like** to be.
///
/// The usual case: a column of buttons that should all be as wide as the widest
/// label, and no wider. Stretching gives them the whole column; hugging gives each
/// its own width; only an intrinsic width gives them all the widest one.
///
/// It is not free. The content is measured **once more**, unconstrained, before it
/// is laid out for real — which is why it is a widget you reach for deliberately and
/// not a property every box carries. Nested one inside another, the cost multiplies.
pub struct Intrinsic<Msg> {
    axis: IntrinsicAxis,
    step: Option<f32>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

/// A box as wide as its content would like to be. See [`Intrinsic`].
pub type IntrinsicWidth<Msg> = Intrinsic<Msg>;
/// A box as tall as its content would like to be. See [`Intrinsic`].
pub type IntrinsicHeight<Msg> = Intrinsic<Msg>;

impl<Msg> Intrinsic<Msg> {
    /// A box as **wide** as `child` would like to be.
    pub fn width(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            axis: IntrinsicAxis::Width,
            step: None,
            children: vec![Box::new(child)],
        }
    }

    /// A box as **tall** as `child` would like to be.
    pub fn height(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            axis: IntrinsicAxis::Height,
            step: None,
            children: vec![Box::new(child)],
        }
    }

    /// Rounds the measured size **up** to a multiple of `step`.
    ///
    /// For a box whose width should change in steps rather than continuously — a
    /// label that grows a character at a time makes everything beside it jitter.
    pub fn step(mut self, step: f32) -> Self {
        self.step = (step > 0.0).then_some(step);
        self
    }

    /// The axis this box takes from its content.
    pub fn axis(&self) -> IntrinsicAxis {
        self.axis
    }

    /// Applies the step rounding to a measured extent.
    pub fn quantise(&self, extent: f32) -> f32 {
        match self.step {
            Some(step) => (extent / step).ceil() * step,
            None => extent,
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Intrinsic<Msg> {
    fn style(&self) -> Style {
        // The measured extent is written into this style by `build_layout`, which is
        // the only place with a runtime to measure against.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn intrinsic(&self) -> Option<(IntrinsicAxis, Option<f32>)> {
        Some((self.axis, self.step))
    }
}

/// The constraints an [`OverflowBox`] hands its child, and where the result sits.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Overflow {
    /// Width given to the child; `None` = the box's own width.
    pub width: Option<f32>,
    /// Height given to the child; `None` = the box's own height.
    pub height: Option<f32>,
    /// Lay the child out **unconstrained**, at the size it asks for. Overrides the
    /// two above.
    pub unconstrained: bool,
    /// Where the child sits inside the box, and therefore which way it spills.
    pub alignment: Alignment,
}

/// A child laid out to constraints of **its own**, free to be bigger than the box
/// that holds it, and painted over whatever is around it.
///
/// The box itself takes the size it would have taken with no child at all, so
/// nothing around it moves. What changes is the child: it is laid out separately —
/// at a size given here, or at the size it asks for
/// ([`OverflowBox::unconstrained`]) — then anchored in the box and allowed to spill
/// past its edges.
///
/// This is the escape hatch for a background that should bleed past its slot, or a
/// decoration wider than the row it belongs to. It does **not** clip: put a
/// [`crate::ClipRRect`] above it if the spill should stop somewhere.
pub struct OverflowBox<Msg> {
    overflow: Overflow,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> OverflowBox<Msg> {
    /// A box that lays `child` out to the sizes given by the builders below,
    /// centred; unset axes keep the box's own size.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            overflow: Overflow {
                width: None,
                height: None,
                unconstrained: false,
                alignment: Alignment::CENTER,
            },
            children: vec![Box::new(child)],
        }
    }

    /// A box that lets `child` be **whatever size it asks for**, as if nothing were
    /// constraining it, and centres the result.
    pub fn unconstrained(child: impl Widget<Msg> + 'static) -> Self {
        let mut boxed = Self::new(child);
        boxed.overflow.unconstrained = true;
        boxed
    }

    /// The width to lay the child out at.
    pub fn width(mut self, width: f32) -> Self {
        self.overflow.width = Some(width);
        self
    }

    /// The height to lay the child out at.
    pub fn height(mut self, height: f32) -> Self {
        self.overflow.height = Some(height);
        self
    }

    /// Where the child sits in the box — and so which edges it spills past.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.overflow.alignment = alignment;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for OverflowBox<Msg> {
    fn style(&self) -> Style {
        // The box takes **the largest size it is allowed**, because its child is laid
        // out separately and so contributes nothing to its size. Left to hug, it would
        // collapse to nothing on the main axis, and the child would be anchored to a
        // box of zero width — spilling equally in both directions from a point.
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overflow_box(&self) -> Option<Overflow> {
        Some(self.overflow)
    }
}

impl Overflow {
    /// The size to lay the child out at, given the box's own, and whether the
    /// measurement should be unconstrained.
    pub(crate) fn child_size(&self, own: Size) -> Size {
        Size::new(
            self.width.unwrap_or(own.width),
            self.height.unwrap_or(own.height),
        )
    }

    /// Where the child's top-left corner goes, given both sizes.
    pub(crate) fn origin(&self, own: Rect, child: Size) -> (f32, f32) {
        (
            own.x + (own.width - child.width) * self.alignment.fraction_x(),
            own.y + (own.height - child.height) * self.alignment.fraction_y(),
        )
    }
}

/// A box of a **stated size** whose child is laid out at another one, and spills.
///
/// The size given here is what the parent sees and what the neighbours make room for.
/// The child is laid out separately — at the size it asks for, or at one stated with
/// [`SizedOverflowBox::child_size`] — anchored in the box by its alignment, and allowed
/// past every edge.
///
/// It is [`OverflowBox`] with the hole given a size: an overflow box fills whatever it is
/// offered, because nothing else could anchor a child that contributes no size, and this
/// is the form for a slot narrower than the space around it — a 24 px gap in a toolbar
/// showing a 40 px control, a caption strip under an image that reaches past both sides.
///
/// It does **not** clip: put a [`crate::ClipRRect`] above it if the spill should stop.
pub struct SizedOverflowBox<Msg> {
    width: f32,
    height: f32,
    overflow: Overflow,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> SizedOverflowBox<Msg> {
    /// A `width`×`height` box holding `child`, which is laid out at **its own** natural
    /// size and centred.
    pub fn new(width: f32, height: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            width,
            height,
            overflow: Overflow {
                width: None,
                height: None,
                unconstrained: true,
                alignment: Alignment::CENTER,
            },
            children: vec![Box::new(child)],
        }
    }

    /// Lays the child out at a size of its own rather than at the one it asks for.
    pub fn child_size(mut self, width: f32, height: f32) -> Self {
        self.overflow.unconstrained = false;
        self.overflow.width = Some(width);
        self.overflow.height = Some(height);
        self
    }

    /// Where the child sits in the box — and so which edges it spills past.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.overflow.alignment = alignment;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for SizedOverflowBox<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overflow_box(&self) -> Option<Overflow> {
        Some(self.overflow)
    }

    fn debug_name(&self) -> &'static str {
        "SizedOverflowBox"
    }
}

/// What a [`ConstraintsTransformBox`] does to **one axis** of the space on offer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisConstraint {
    /// As it came: the child is given the room the parent offered.
    AsGiven,
    /// Taken away: the child is asked how big it would like to be, and may come back
    /// bigger than the room there was.
    Unbounded,
    /// A number of its own, whatever was offered.
    Fixed(f32),
}

/// What a [`ConstraintsTransformBox`] gives its child, per axis, and what it does about a
/// child that comes back too big.
///
/// It is a small vocabulary rather than an arbitrary function of the space on offer, and
/// that is a decision worth stating. The layout and the paint walk are two passes over the
/// same tree, and only the first is told what the parent offered: by the time the walk lays
/// the child out, the box has been sized to the child and the offer is not recoverable from
/// it. A function of the offer could not be run again there — and a function run again on
/// the wrong input answers plausibly, which is worse than not answering at all. Each of
/// these three **can** be run again: a fixed number is itself, an unbounded axis is a
/// question with no input, and an axis given as it came produced a child of exactly the
/// box's own size.
///
/// The reference's transform is a closure, and every transform the reference itself ships
/// is one of these three.
#[derive(Clone, Copy, Debug)]
pub struct ConstraintsTransform {
    /// What the child is given across.
    pub width: AxisConstraint,
    /// And down.
    pub height: AxisConstraint,
    /// Where a child smaller than the box sits in it.
    pub alignment: Alignment,
    /// Whether a child that came out **bigger** than the box is reported as an overflow —
    /// the yellow and black band, in debug builds. On by default: a box that transforms its
    /// constraints is asking a question about what fits, and a silent answer is no answer.
    pub report: bool,
}

impl ConstraintsTransform {
    /// Both axes as they came, which is the transform that does nothing.
    pub fn new() -> Self {
        Self {
            width: AxisConstraint::AsGiven,
            height: AxisConstraint::AsGiven,
            alignment: Alignment::CENTER,
            report: true,
        }
    }

    /// What the child is given across.
    pub fn width(mut self, width: AxisConstraint) -> Self {
        self.width = width;
        self
    }

    /// What the child is given down.
    pub fn height(mut self, height: AxisConstraint) -> Self {
        self.height = height;
        self
    }

    /// Where a child smaller than the box sits in it.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Whether a child bigger than the box wears a band. Off is what a deliberate spill
    /// wants.
    pub fn report(mut self, report: bool) -> Self {
        self.report = report;
        self
    }
}

impl Default for ConstraintsTransform {
    fn default() -> Self {
        Self::new()
    }
}

impl AxisConstraint {
    /// `(extent, free)` for the **measurement**, given what the layout offered on this
    /// axis — `None` being no limit at all.
    pub(crate) fn offered(self, offer: Option<f32>) -> (f32, bool) {
        match self {
            AxisConstraint::Unbounded => (0.0, true),
            AxisConstraint::Fixed(extent) => (extent.max(0.0), false),
            AxisConstraint::AsGiven => match offer {
                Some(extent) => (extent.max(0.0), false),
                None => (0.0, true),
            },
        }
    }

    /// `(extent, free)` for the **walk**, given the box the measurement produced.
    ///
    /// It answers the same thing the measurement did, from what is left of it: a fixed
    /// number is itself, an unbounded axis is the same question either way, and an axis
    /// given as it came produced a child of the box's own extent — the box being that child
    /// held to what was offered.
    pub(crate) fn at(self, own: f32) -> (f32, bool) {
        match self {
            AxisConstraint::Unbounded => (0.0, true),
            AxisConstraint::Fixed(extent) => (extent.max(0.0), false),
            AxisConstraint::AsGiven => (own.max(0.0), false),
        }
    }
}

/// A box that **changes the space on offer** before its child is laid out in it, and is
/// then as big as what came back — never bigger than what it was itself offered.
///
/// This is the general form of the boxes above, and the one they are special cases of: what
/// the child is given, per axis, said in [`AxisConstraint`]. Taking a ceiling away lets the
/// child be its natural size ([`UnconstrainedBox`]); a fixed number lays it out in a room
/// of that size whatever the parent offered.
///
/// ```ignore
/// // "However tall you need to be" — and a band across the box if that was too tall.
/// ConstraintsTransformBox::new(paragraph).height(AxisConstraint::Unbounded)
/// ```
///
/// A child that comes out bigger than the box **overflows it**: painted past the edges, and
/// reported with the debug band. That is the point of the widget — it asks what the space
/// on offer is doing to the content, and answers on the screen. Turn the band off with
/// [`ConstraintsTransformBox::report`] when the spill is deliberate.
pub struct ConstraintsTransformBox<Msg> {
    transform: ConstraintsTransform,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ConstraintsTransformBox<Msg> {
    /// A box holding `child`, with both axes still as they came.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            transform: ConstraintsTransform::new(),
            children: vec![Box::new(child)],
        }
    }

    /// What the child is given across.
    pub fn width(mut self, width: AxisConstraint) -> Self {
        self.transform.width = width;
        self
    }

    /// What the child is given down.
    pub fn height(mut self, height: AxisConstraint) -> Self {
        self.transform.height = height;
        self
    }

    /// Where a child smaller than the box sits in it.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.transform.alignment = alignment;
        self
    }

    /// Whether a child bigger than the box wears a band.
    pub fn report(mut self, report: bool) -> Self {
        self.transform.report = report;
        self
    }

    /// A box built from a transform prepared elsewhere.
    pub fn with(child: impl Widget<Msg> + 'static, transform: ConstraintsTransform) -> Self {
        Self {
            transform,
            children: vec![Box::new(child)],
        }
    }
}

impl<Msg: Clone> Widget<Msg> for ConstraintsTransformBox<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn constraints_transform(&self) -> Option<ConstraintsTransform> {
        Some(self.transform)
    }

    fn debug_name(&self) -> &'static str {
        "ConstraintsTransformBox"
    }
}

/// Lets its child be its **natural size** even where the parent offers less — and says so
/// when that does not fit.
///
/// The child is laid out as if nothing were constraining it, and the box is as big as the
/// child, up to the space it was itself offered. Where the child fits, nothing is
/// unusual — the box is simply the child's size, hugging it in a place that would have
/// stretched it. Where it does not, the child is painted past the edges and a band says by
/// how much, in debug builds.
///
/// ```ignore
/// // A row of chips that must not be squashed: too many, and the overflow is visible.
/// UnconstrainedBox::new(Flex::row().child(chip_a).child(chip_b))
/// ```
///
/// That visible failure is the point, and the difference between this and [`OverflowBox`],
/// which spills on purpose and quietly. It is [`ConstraintsTransformBox`] with the
/// constraint taken away, and nothing more.
///
/// [`UnconstrainedBox::axis`] frees **one** axis and leaves the other as it was, which is
/// usually what is wanted: a paragraph that may be as tall as it likes is still as wide as
/// its column.
pub struct UnconstrainedBox;

impl UnconstrainedBox {
    /// Frees both axes.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<Msg: Clone + 'static>(
        child: impl Widget<Msg> + 'static,
    ) -> ConstraintsTransformBox<Msg> {
        ConstraintsTransformBox::new(child)
            .width(AxisConstraint::Unbounded)
            .height(AxisConstraint::Unbounded)
    }

    /// Frees the given axis, and leaves the other one as the parent offered it.
    pub fn axis<Msg: Clone + 'static>(
        child: impl Widget<Msg> + 'static,
        axis: crate::scroll::Axis,
    ) -> ConstraintsTransformBox<Msg> {
        let freed = |free: bool| match free {
            true => AxisConstraint::Unbounded,
            false => AxisConstraint::AsGiven,
        };
        ConstraintsTransformBox::new(child)
            .width(freed(axis.free_x()))
            .height(freed(axis.free_y()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Flex, InspectorNode, Runtime, Text};
    use frus_core::{Color, Primitive};

    const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// A child that fills whatever box it is put in, painted red so the tests below
    /// can read that box off the scene. `Align::Stretch` gives it the cross axis;
    /// `flex(1)` gives it the main one.
    fn filler() -> Container<()> {
        Container::new().flex(1.0).color(RED)
    }

    /// The first red rectangle painted.
    fn red_box<W: Widget<()> + 'static>(root: W, available: Size) -> Rect {
        let runtime = Runtime::default();
        let ui = build_ui(&root, available, &runtime, &Theme::dark());
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 && color.g < 0.5 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the red box")
    }

    #[test]
    fn a_ceiling_stops_a_box_that_would_fill() {
        // Stretched by its column, the box would be 400 wide; capped, it is 120.
        let root = Flex::<()>::column()
            .width(400.0)
            .child(ConstrainedBox::new(filler().height(40.0)).max_width(120.0));
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!((rect.width - 120.0).abs() < 0.5, "capped: {rect:?}");
    }

    #[test]
    fn a_floor_stops_a_box_that_would_hug() {
        // In a row the box hugs its content, which asks for nothing; the floor is
        // the only thing giving it a width.
        let root = Flex::<()>::row()
            .width(400.0)
            .height(60.0)
            .child(ConstrainedBox::new(filler()).min_width(90.0));
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!((rect.width - 90.0).abs() < 0.5, "floored: {rect:?}");
    }

    #[test]
    fn a_tight_box_is_that_size_whatever_is_inside_it() {
        let root = Flex::<()>::column()
            .width(400.0)
            .child(ConstrainedBox::tight(75.0, 33.0, filler()));
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!(
            (rect.width - 75.0).abs() < 0.5 && (rect.height - 33.0).abs() < 0.5,
            "tight: {rect:?}"
        );
    }

    #[test]
    fn a_sized_box_that_expands_takes_everything_on_offer() {
        let root = Flex::<()>::column()
            .width(400.0)
            .height(200.0)
            .child(SizedBox::expand(filler()));
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!(
            (rect.width - 400.0).abs() < 0.5 && (rect.height - 200.0).abs() < 0.5,
            "expanded: {rect:?}"
        );
    }

    #[test]
    fn a_sized_box_is_the_size_it_was_given() {
        let root = Flex::<()>::column()
            .width(400.0)
            .child(SizedBox::square(64.0, filler()));
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!(
            (rect.width - 64.0).abs() < 0.5 && (rect.height - 64.0).abs() < 0.5,
            "square: {rect:?}"
        );
    }

    #[test]
    fn an_intrinsic_width_is_what_the_content_wants_not_what_the_column_offers() {
        // Stretched by the 400 px column, the inner block would be 400 wide. Taken
        // intrinsically, it is as wide as the line inside it — and much less.
        let root = Flex::<()>::column().width(400.0).child(Intrinsic::width(
            Flex::<()>::column()
                .child(Text::new("a short line"))
                .child(Container::new().height(10.0).color(RED)),
        ));
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!(
            rect.width > 1.0 && rect.width < 380.0,
            "the content's width, not the column's: {rect:?}"
        );
    }

    #[test]
    fn an_intrinsic_width_rounds_up_to_its_step() {
        let step = Intrinsic::<()>::width(Container::new()).step(20.0);
        assert_eq!(step.quantise(1.0), 20.0);
        assert_eq!(step.quantise(20.0), 20.0);
        assert_eq!(step.quantise(21.0), 40.0);
        // With no step, the measurement passes through untouched.
        let plain = Intrinsic::<()>::width(Container::new());
        assert_eq!(plain.quantise(21.0), 21.0);
    }

    #[test]
    fn an_overflowing_child_is_bigger_than_its_box_and_centred_on_it() {
        // A 40 px slot holding a 100 px child: it hangs 30 px off either side.
        let root = Flex::<()>::column().width(400.0).child(
            SizedBox::empty()
                .width(40.0)
                .height(40.0)
                .child(OverflowBox::new(filler()).width(100.0).height(100.0)),
        );
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!(
            (rect.width - 100.0).abs() < 0.5 && (rect.height - 100.0).abs() < 0.5,
            "the child keeps its own size: {rect:?}"
        );
        assert!(
            rect.x < -29.0 && rect.x > -31.0,
            "centred, spilling: {rect:?}"
        );
    }

    #[test]
    fn an_overflow_box_can_anchor_the_spill_to_one_corner() {
        let root = Flex::<()>::column().width(400.0).child(
            SizedBox::empty().width(40.0).height(40.0).child(
                OverflowBox::new(filler())
                    .width(100.0)
                    .height(100.0)
                    .alignment(Alignment::TOP_LEFT),
            ),
        );
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!(
            rect.x.abs() < 0.5 && rect.y.abs() < 0.5,
            "anchored top-left, spilling right and down: {rect:?}"
        );
    }

    #[test]
    fn the_box_around_an_overflowing_child_does_not_grow() {
        // The neighbour below sits at 40, not at 100: the box did not grow, which is
        // the whole difference between an overflow and a big child.
        let root = Flex::<()>::column()
            .width(400.0)
            .child(
                SizedBox::empty().width(40.0).height(40.0).child(
                    OverflowBox::new(Container::new().flex(1.0))
                        .width(100.0)
                        .height(100.0),
                ),
            )
            .child(Container::new().width(10.0).height(10.0).color(RED));
        let rect = red_box(root, Size::new(400.0, 300.0));
        assert!((rect.y - 40.0).abs() < 0.5, "the neighbour moved: {rect:?}");
    }

    #[test]
    fn an_unconstrained_child_is_the_size_it_asks_for() {
        // The slot is 20 px; the child asks for 90×30 and gets it.
        let root = Flex::<()>::column().width(400.0).child(
            SizedBox::empty()
                .width(20.0)
                .height(20.0)
                .child(OverflowBox::unconstrained(
                    Container::new().width(90.0).height(30.0).color(RED),
                )),
        );
        let rect = red_box(root, Size::new(400.0, 200.0));
        assert!(
            (rect.width - 90.0).abs() < 0.5 && (rect.height - 30.0).abs() < 0.5,
            "unconstrained: {rect:?}"
        );
    }

    /// The scene and the inspector's boxes, for a tree laid out in `available`.
    fn inspected(root: &dyn Widget<()>, available: Size) -> (crate::Ui<()>, Vec<InspectorNode>) {
        crate::build_ui_inspected(root, available, &Runtime::default(), &Theme::dark())
    }

    /// The box the walk gave the widget named `name` — the box itself, which is the thing
    /// these widgets are about and the one thing the scene does not show.
    fn box_named(nodes: &[InspectorNode], name: &str) -> Rect {
        nodes
            .iter()
            .find(|n| n.name == name)
            .unwrap_or_else(|| panic!("no {name} in {}", crate::dump_tree(nodes)))
            .rect
    }

    /// A red box of a stated size — laid out at whatever it is given, and visible.
    fn red(width: f32, height: f32) -> Container<()> {
        Container::new().width(width).height(height).color(RED)
    }

    /// A column that does **not** stretch its children, so a box that hugs can be seen
    /// hugging.
    fn narrow_column(width: f32) -> Flex<()> {
        Flex::<()>::column()
            .width(width)
            .align(frus_layout::Align::Start)
    }

    #[test]
    fn a_child_that_fits_gives_the_unconstrained_box_its_own_size() {
        let root = narrow_column(400.0).child(UnconstrainedBox::new(red(90.0, 30.0)));
        let (ui, nodes) = inspected(&root, Size::new(400.0, 200.0));
        let own = box_named(&nodes, "ConstraintsTransformBox");
        assert!(
            (own.width - 90.0).abs() < 0.5 && (own.height - 30.0).abs() < 0.5,
            "the box is the child's size: {own:?}"
        );
        assert!(ui.overflows().is_empty(), "nothing ran past anything");
    }

    /// The other half, and the reason the widget is worth having: the child keeps the
    /// size it asked for, the box keeps the size it was offered, and the difference is
    /// **reported** rather than quietly absorbed.
    #[test]
    fn an_unconstrained_child_that_does_not_fit_overflows_and_says_so() {
        let root = narrow_column(100.0).child(UnconstrainedBox::new(red(300.0, 20.0)));
        let (ui, nodes) = inspected(&root, Size::new(100.0, 200.0));
        let child = red_box_in(&ui);
        assert!(
            (child.width - 300.0).abs() < 0.5,
            "the child kept its size: {child:?}"
        );
        let own = box_named(&nodes, "ConstraintsTransformBox");
        assert!(
            (own.width - 100.0).abs() < 0.5,
            "the box kept the room it was offered: {own:?}"
        );
        // Centred, it runs past **both** sides, and both are reported: a band on one of
        // them would say the child was a hundred too wide rather than two.
        let spills = ui.overflows();
        let mut sides: Vec<_> = spills.iter().map(|o| (o.side, o.amount)).collect();
        sides.sort_by_key(|(side, _)| format!("{side:?}"));
        assert_eq!(sides.len(), 2, "two edges: {spills:?}");
        assert_eq!(sides[0].0, frus_layout::Side::Left);
        assert_eq!(sides[1].0, frus_layout::Side::Right);
        assert!(
            sides.iter().all(|(_, amount)| (amount - 100.0).abs() < 0.5),
            "a hundred either side: {spills:?}"
        );
    }

    /// One axis freed, the other left as it was: the child is as tall as it likes and no
    /// wider than the column, which is what a paragraph wants.
    #[test]
    fn freeing_one_axis_leaves_the_other_one_alone() {
        let root = narrow_column(100.0).child(UnconstrainedBox::axis(
            Container::new().height(200.0).color(RED),
            crate::scroll::Axis::Vertical,
        ));
        let (ui, _) = inspected(&root, Size::new(100.0, 80.0));
        let child = red_box_in(&ui);
        assert!(
            (child.width - 100.0).abs() < 0.5,
            "held to the column: {child:?}"
        );
        assert!(
            (child.height - 200.0).abs() < 0.5,
            "and as tall as it asked: {child:?}"
        );
    }

    /// Unbounded is only one of the three things an axis can say: this one gives the
    /// child a hundred pixels of room in a column twice that wide, and the box comes back
    /// the size of what it asked for.
    #[test]
    fn a_transform_can_give_the_child_a_room_of_its_own() {
        let root = narrow_column(200.0).child(
            ConstraintsTransformBox::new(Container::new().height(20.0).color(RED))
                .width(AxisConstraint::Fixed(100.0)),
        );
        let (ui, nodes) = inspected(&root, Size::new(200.0, 200.0));
        let child = red_box_in(&ui);
        assert!(
            (child.width - 100.0).abs() < 0.5,
            "the child was laid out in the room it was given: {child:?}"
        );
        assert!((box_named(&nodes, "ConstraintsTransformBox").width - 100.0).abs() < 0.5);
        assert!(ui.overflows().is_empty(), "half of it fits in all of it");
    }

    #[test]
    fn a_sized_overflow_box_reports_one_size_and_lays_its_child_out_at_another() {
        let root = Flex::<()>::column()
            .width(400.0)
            .child(SizedOverflowBox::new(40.0, 40.0, red(100.0, 100.0)));
        let (ui, nodes) = inspected(&root, Size::new(400.0, 300.0));
        let own = box_named(&nodes, "SizedOverflowBox");
        assert!(
            (own.width - 40.0).abs() < 0.5 && (own.height - 40.0).abs() < 0.5,
            "the box is the size it stated: {own:?}"
        );
        let child = red_box_in(&ui);
        assert!(
            (child.width - 100.0).abs() < 0.5,
            "the child is not: {child:?}"
        );
        assert!(
            (child.x + 30.0).abs() < 0.5 && (child.y + 30.0).abs() < 0.5,
            "centred on the box it hangs out of: {child:?}"
        );
    }

    /// The first red rectangle in a finished scene.
    fn red_box_in(ui: &crate::Ui<()>) -> Rect {
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 && color.g < 0.5 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the red box")
    }
}

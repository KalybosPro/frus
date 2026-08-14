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
/// [`crate::ClipRect`] above it if the spill should stop somewhere.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Flex, Runtime, Text};
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
}

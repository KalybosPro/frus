//! [`Stack`]: overlays its children in the **same box**, z-layering them. The last layer
//! sits on top. A badge on an avatar, a caption over a picture, a button floating over
//! content.
//!
//! Two things decide where a layer lands, as in the reference (`rendering/stack.dart`):
//!
//! - a layer wrapped in [`crate::Positioned`] is pinned against the stack's own edges,
//!   and what is pinned decides its size as well as its place;
//! - every other layer is sized by the stack's [`StackFit`] and placed by its
//!   [`Stack::alignment`].
//!
//! **The stack is as big as its largest unpinned layer** (`stack.dart:625`), held to the
//! room it is given; with none, or under [`StackFit::Expand`], it takes all the room it is
//! given. Pinned layers never size it. A layer that goes past its edges is clipped there,
//! unless [`Stack::clip_behavior`] says [`Clip::None`].

use frus_core::{AlignmentDirectional, AlignmentGeometry, Rect, Scene, TextDirection};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::{FillAxes, Widget};

/// How a stack sizes the layers that are not [`crate::Positioned`] — the reference's
/// `StackFit` (`stack.dart:301`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackFit {
    /// **Allowed** up to the stack's room: each layer is as big as it wants to be, at most
    /// the stack. The default, as in the reference.
    #[default]
    Loose,
    /// **Forced** to the room the stack is given: every unpinned layer is exactly that big,
    /// whatever size it has of its own, and so is the stack.
    Expand,
    /// **Handed** the room the stack is given: a layer with no size of its own fills it,
    /// and one with a size keeps it — the stack's own constraints, passed through, for a
    /// stack that is allowed its room rather than forced into it.
    Passthrough,
}

/// What happens to a layer that goes past its stack's edges — the reference's `Clip`, as a
/// stack uses it (`stack.dart:476`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Clip {
    /// Nothing is cut: a badge may hang off the corner of the avatar it sits on.
    None,
    /// Cut at the stack's edges. The default.
    #[default]
    HardEdge,
    /// Cut at the stack's edges, smoothed. The edges of a stack are straight, so this cuts
    /// as [`Clip::HardEdge`] does.
    AntiAlias,
    /// Cut at the stack's edges, smoothed, in a layer of its own. As [`Clip::AntiAlias`]
    /// here.
    AntiAliasWithSaveLayer,
}

impl Clip {
    /// Whether anything is cut.
    pub fn clips(self) -> bool {
        self != Clip::None
    }
}

/// Whether a stack with these layers takes all the room it is given: under
/// [`StackFit::Expand`], or with no layer that is not pinned (`stack.dart:657`).
fn takes_the_room<Msg>(fit: StackFit, layers: &[Box<dyn Widget<Msg>>]) -> FillAxes {
    if fit == StackFit::Expand || layers.iter().all(|layer| layer.positioned().is_some()) {
        FillAxes::BOTH
    } else {
        FillAxes::NONE
    }
}

/// A container of overlaid layers.
pub struct Stack<Msg = crate::callback::Callback> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    fit: StackFit,
    alignment: AlignmentGeometry,
    clip: Clip,
    direction: Option<TextDirection>,
    layers: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Stack<Msg> {
    /// Creates an empty stack.
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            fit: StackFit::Loose,
            // The reference's default anchor, and it follows the reading direction: the
            // start corner is the left in a left-to-right script and the right in a
            // right-to-left one.
            alignment: AlignmentGeometry::Directional(AlignmentDirectional::TOP_START),
            clip: Clip::HardEdge,
            direction: None,
            layers: Vec::new(),
        }
    }

    /// How the layers that are not [`crate::Positioned`] are sized. Unset,
    /// [`StackFit::Loose`].
    pub fn fit(mut self, fit: StackFit) -> Self {
        self.fit = fit;
        self
    }

    /// Where a layer smaller than the stack sits in it: the layers that are not
    /// [`crate::Positioned`], and the axes of one that pinned neither edge. Unset, the top
    /// start corner.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// **What happens to a layer that goes past the stack's edges.** Unset,
    /// [`Clip::HardEdge`]: it is cut there. [`Clip::None`] lets it hang over.
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip = clip;
        self
    }

    /// **The reading direction** the alignment and the start and end pins are resolved
    /// in. Unset, the theme's.
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.direction = Some(direction);
        self
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

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Adds a layer, on top of the previous ones.
    pub fn layer(mut self, layer: impl Widget<Msg> + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }
}

impl<Msg> Default for Stack<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for Stack<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.layers
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn stack(&self) -> bool {
        true
    }

    fn stack_fit(&self) -> StackFit {
        self.fit
    }

    fn stack_measured(&self) -> bool {
        true
    }

    fn stack_clips(&self) -> bool {
        self.clip.clips()
    }

    fn stack_direction(&self) -> Option<TextDirection> {
        self.direction
    }

    /// All the room it is given under [`StackFit::Expand`] or with every layer pinned.
    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        takes_the_room(self.fit, &self.layers)
    }

    fn alignment_geometry(&self) -> Option<AlignmentGeometry> {
        Some(self.alignment)
    }
}

/// A stack that lays **every** child out and paints one of them.
///
/// The difference between this and [`crate::Offstage`] is the whole reason it exists.
/// `Offstage` takes a branch out of the tree — no box, no paint, and nothing retained:
/// right for a branch that is genuinely gone, wrong for the three pages of a tabbed
/// screen. There, every switch measures a subtree that was laid out a moment ago, and
/// the page that comes back comes back blank: its scroll offset, its caret and its
/// focus were the runtime's record of a widget that stopped existing.
///
/// An `IndexedStack` keeps them all. Each child is laid out, in the same box, and one is
/// painted; the others are **silent** — not drawn, not clickable, not read out — but
/// still measured, and still holding whatever the runtime holds for them. So switching
/// keeps their sizes, and keeps where they had been scrolled to.
///
/// ```ignore
/// IndexedStack::new(self.tab)
///     .child(inbox_page())
///     .child(sent_page())
///     .child(drafts_page())
/// ```
///
/// The price is exactly that: every page laid out on every frame, however many are
/// shown. It is what a handful of tabs wants, and what a list of ten thousand rows does
/// not — an index past the end simply paints nothing.
pub struct IndexedStack<Msg = crate::callback::Callback> {
    index: usize,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    fit: StackFit,
    alignment: AlignmentGeometry,
    clip: Clip,
    direction: Option<TextDirection>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> IndexedStack<Msg> {
    /// A stack showing the child at `index`, with none yet added.
    pub fn new(index: usize) -> Self {
        Self {
            index,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            fit: StackFit::Loose,
            alignment: AlignmentGeometry::Directional(frus_core::AlignmentDirectional::TOP_START),
            clip: Clip::HardEdge,
            direction: None,
            children: Vec::new(),
        }
    }

    /// Which child is shown. The others stay laid out.
    pub fn index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    /// Adds a child, after the previous ones.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// How the children are sized. Unset, [`StackFit::Loose`].
    pub fn fit(mut self, fit: StackFit) -> Self {
        self.fit = fit;
        self
    }

    /// Where a child smaller than the stack sits in it.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// What happens to a child that goes past the stack's edges. Unset, it is cut there.
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip = clip;
        self
    }

    /// The reading direction the alignment is resolved in. Unset, the theme's.
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.direction = Some(direction);
        self
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

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for IndexedStack<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
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

    fn stack(&self) -> bool {
        true
    }

    fn stack_fit(&self) -> StackFit {
        self.fit
    }

    /// As big as its largest child, shown or not (`stack.dart:768` lays every child out
    /// and sizes as a stack does): switching pages does not resize it.
    fn stack_measured(&self) -> bool {
        true
    }

    fn stack_clips(&self) -> bool {
        self.clip.clips()
    }

    fn stack_direction(&self) -> Option<TextDirection> {
        self.direction
    }

    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        takes_the_room(self.fit, &self.children)
    }

    fn stack_visible(&self) -> Option<usize> {
        Some(self.index)
    }

    fn alignment_geometry(&self) -> Option<AlignmentGeometry> {
        Some(self.alignment)
    }

    fn debug_name(&self) -> &'static str {
        "IndexedStack"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    #[test]
    fn layers_overlap_in_same_box() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let stack = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .layer(Container::<()>::new().width(100.0).height(100.0).color(red))
            .layer(Container::<()>::new().width(40.0).height(40.0).color(blue));
        let ui = build_ui(
            &stack,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );

        let rect_of = |c: Color| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, rect, .. } if *color == c => Some(*rect),
                    _ => None,
                })
                .expect("the layer is present")
        };
        // Both layers share the same origin, being overlaid.
        assert_eq!(rect_of(red).origin(), rect_of(blue).origin());
    }

    /// Every rectangle of a colour, in a 100×100 stack.
    fn rects(stack: Stack<()>, of: Color) -> Vec<Rect> {
        let ui = build_ui(
            &stack,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { color, rect, .. } if *color == of => Some(*rect),
                _ => None,
            })
            .collect()
    }

    const BADGE: Color = Color::rgb(0.0, 0.0, 1.0);

    fn badge() -> Container<()> {
        Container::<()>::new().width(20.0).height(20.0).color(BADGE)
    }

    /// Two edges pinned: the badge sits that far from each of them.
    #[test]
    fn a_pinned_layer_sits_against_the_edges_it_names() {
        let stack = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .layer(crate::Positioned::new(badge()).top(8.0).right(12.0));
        let r = rects(stack, BADGE)[0];
        assert_eq!(r.y, 8.0, "8 below the top: {r:?}");
        assert_eq!(r.x, 100.0 - 12.0 - 20.0, "12 in from the right: {r:?}");
        assert_eq!((r.width, r.height), (20.0, 20.0), "its own size");
    }

    /// Both edges of an axis pinned: they decide the extent, and the layer's own is
    /// overruled — which is what makes a bar across the bottom of a stack one line.
    #[test]
    fn opposite_pins_decide_the_extent() {
        let stack = Stack::<()>::new().width(100.0).height(100.0).layer(
            crate::Positioned::new(badge())
                .left(10.0)
                .right(30.0)
                .bottom(0.0),
        );
        let r = rects(stack, BADGE)[0];
        assert_eq!(r.x, 10.0);
        assert_eq!(r.width, 60.0, "100 - 10 - 30: {r:?}");
        assert_eq!(r.y, 80.0, "its own height, off the bottom: {r:?}");
    }

    /// `inset` is all four at once: the layer fills the stack, held off every edge.
    #[test]
    fn an_inset_layer_fills_what_is_left() {
        let stack = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .layer(crate::Positioned::new(badge()).inset(5.0));
        let r = rects(stack, BADGE)[0];
        assert_eq!((r.x, r.y, r.width, r.height), (5.0, 5.0, 90.0, 90.0));
    }

    /// An axis nobody pinned is placed by the stack's alignment, at the layer's own size.
    #[test]
    fn an_unpinned_axis_follows_the_alignment() {
        let stack = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .alignment(frus_core::Alignment::CENTER)
            .layer(crate::Positioned::new(badge()).bottom(0.0));
        let r = rects(stack, BADGE)[0];
        assert_eq!(r.x, 40.0, "centred across: {r:?}");
        assert_eq!(r.y, 80.0, "and pinned down: {r:?}");
    }

    /// **The reference's three fits** (`stack.dart:634`): loose, the default, asks a layer
    /// and lets the alignment place it; expand forces every layer to the stack's box,
    /// whatever size it has of its own; passthrough hands it the box, which fills a layer
    /// with no size and leaves a sized one alone.
    #[test]
    fn the_three_fits_are_the_reference_s() {
        let unsized_layer = || {
            Container::<()>::new()
                .color(BADGE)
                .child(crate::Text::new("x"))
        };
        let sized_layer = || Container::<()>::new().width(40.0).height(40.0).color(BADGE);
        let in_box = |fit: StackFit, layer: Container<()>| {
            let stack = Stack::<()>::new()
                .width(100.0)
                .height(100.0)
                .fit(fit)
                .alignment(frus_core::Alignment::BOTTOM_RIGHT)
                .layer(layer);
            rects(stack, BADGE)[0]
        };

        let r = in_box(StackFit::Loose, unsized_layer());
        assert!(r.width < 60.0, "asked, and it hugged its text: {r:?}");
        assert_eq!(r.x + r.width, 100.0, "against the right edge: {r:?}");
        assert_eq!(r.y + r.height, 100.0, "and the bottom: {r:?}");
        assert_eq!(
            Stack::<()>::new().stack_fit(),
            StackFit::Loose,
            "the default, as in the reference"
        );

        let r = in_box(StackFit::Expand, sized_layer());
        assert_eq!((r.width, r.height), (100.0, 100.0), "forced: {r:?}");
        let r = in_box(StackFit::Expand, unsized_layer());
        assert_eq!((r.width, r.height), (100.0, 100.0), "forced: {r:?}");

        let r = in_box(StackFit::Passthrough, unsized_layer());
        assert_eq!((r.width, r.height), (100.0, 100.0), "handed the box: {r:?}");
        let r = in_box(StackFit::Passthrough, sized_layer());
        assert_eq!(
            (r.width, r.height),
            (40.0, 40.0),
            "and a size of its own kept: {r:?}"
        );
        let r = in_box(StackFit::Loose, sized_layer());
        assert_eq!((r.width, r.height), (40.0, 40.0));
        // A layer that asks for the room takes it, loose: an empty container is as big as
        // the stack, as a childless box is in the reference.
        let r = in_box(StackFit::Loose, Container::<()>::new().color(BADGE));
        assert_eq!((r.width, r.height), (100.0, 100.0), "it asked: {r:?}");
    }

    /// Every rectangle painted in `colour`, in a column of a stack and a 10 px square under
    /// it, in a 200 px square.
    fn in_a_column(stack: Stack<()>, colour: Color) -> (Vec<Rect>, Rect) {
        let under = Color::rgb(0.1, 0.7, 0.3);
        let col = crate::Flex::<()>::column()
            .child(stack)
            .child(Container::new().width(10.0).height(10.0).color(under));
        let ui = build_ui(
            &col,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        let painted = |c: Color| -> Vec<Rect> {
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if *color == c => Some(*rect),
                    _ => None,
                })
                .collect()
        };
        let next = painted(under)[0];
        (painted(colour), next)
    }

    /// **A stack is as big as its largest unpinned layer** (`stack.dart:625`), and a
    /// pinned layer does not count: the next child of the column starts under the biggest
    /// one, not at the top.
    #[test]
    fn a_stack_is_as_big_as_its_largest_unpinned_layer() {
        // The tallest is not the last: the stack takes each axis's largest.
        let stack = Stack::<()>::new()
            .layer(Container::new().width(20.0).height(50.0).color(BADGE))
            .layer(Container::new().width(40.0).height(30.0).color(BADGE))
            .layer(
                crate::Positioned::new(Container::new().width(90.0).height(90.0).color(BADGE))
                    .top(0.0)
                    .left(0.0),
            );
        let (_, next) = in_a_column(stack, BADGE);
        assert_eq!(
            next.y, 50.0,
            "under the tallest unpinned layer, not the pinned one"
        );
    }

    /// **With every layer pinned, or under `Expand`, it takes the room it is given**
    /// (`stack.dart:657`, `:636`).
    #[test]
    fn with_only_pinned_layers_it_takes_the_room() {
        let pinned = Stack::<()>::new().height(60.0).layer(
            crate::Positioned::new(Container::new().width(10.0).height(10.0).color(BADGE))
                .top(0.0)
                .left(0.0),
        );
        assert_eq!(
            Widget::<()>::fill_axes(&pinned, &crate::Theme::default()),
            FillAxes::BOTH
        );
        let expanded = Stack::<()>::new()
            .fit(StackFit::Expand)
            .layer(Container::new().width(10.0).height(10.0).color(BADGE));
        assert_eq!(
            Widget::<()>::fill_axes(&expanded, &crate::Theme::default()),
            FillAxes::BOTH
        );
        let loose = Stack::<()>::new().layer(Container::new().width(10.0).height(10.0));
        assert_eq!(
            Widget::<()>::fill_axes(&loose, &crate::Theme::default()),
            FillAxes::NONE,
            "a layer that is not pinned sizes it"
        );
        // Laid out: a stack of pinned layers inside a sized box is that box.
        let boxed = Container::<()>::new().width(120.0).height(70.0).child(
            Stack::new().layer(
                crate::Positioned::new(Container::new().color(BADGE))
                    .right(0.0)
                    .bottom(0.0)
                    .width(10.0)
                    .height(10.0),
            ),
        );
        let ui = build_ui(
            &boxed,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        let badge = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if *color == BADGE => Some(*rect),
                _ => None,
            })
            .expect("the badge");
        assert_eq!((badge.x, badge.y), (110.0, 60.0), "in the box's corner");
    }

    /// **`Clip::None` lets a layer hang over the edges**; the default cuts it there
    /// (`stack.dart:718`).
    #[test]
    fn a_layer_hangs_over_only_when_told() {
        let hanging = |clip: Clip| {
            Stack::<()>::new()
                .width(40.0)
                .height(40.0)
                .clip_behavior(clip)
                .layer(
                    crate::Positioned::new(Container::new().width(20.0).height(20.0).color(BADGE))
                        .top(-10.0)
                        .right(-10.0),
                )
        };
        assert!(Widget::<()>::stack_clips(&hanging(Clip::HardEdge)));
        assert!(!Widget::<()>::stack_clips(&hanging(Clip::None)));
        assert!(Clip::AntiAlias.clips() && Clip::AntiAliasWithSaveLayer.clips());
        assert_eq!(Clip::default(), Clip::HardEdge);
        let r = rects(hanging(Clip::None), BADGE)[0];
        assert_eq!(
            (r.x, r.y),
            (30.0, -10.0),
            "past the top right corner: {r:?}"
        );
        // What is cut: at the stack's edges by default, not at all with `Clip::None`.
        let clip_of = |clip: Clip| {
            let stack = hanging(clip);
            let ui = build_ui(
                &stack,
                Size::new(100.0, 100.0),
                &Runtime::default(),
                &crate::Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, clip, .. } if *color == BADGE => Some(*clip),
                    _ => None,
                })
                .expect("the badge")
        };
        assert_eq!(clip_of(Clip::HardEdge), Rect::new(0.0, 0.0, 40.0, 40.0));
        assert!(
            clip_of(Clip::None).contains(frus_core::Point::new(45.0, 5.0)),
            "nothing cut past the edge"
        );
    }

    /// **A layer hanging over the edge is drawn there but not touched there**: the stack
    /// hit-tests inside its own box only, as the reference's does (`box.dart`'s `hitTest`
    /// asks the box first).
    #[test]
    fn a_hanging_layer_is_only_touched_inside_the_stack() {
        #[derive(Clone, Debug, PartialEq)]
        struct Tap;
        let page = Container::<Tap>::new().padding(50.0).child(
            Stack::<Tap>::new()
                .width(40.0)
                .height(40.0)
                .clip_behavior(Clip::None)
                .layer(
                    crate::Positioned::new(Container::new().width(20.0).height(20.0).on_click(Tap))
                        .top(-10.0)
                        .right(-10.0),
                ),
        );
        let ui = build_ui(
            &page,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        let tap = |x: f32, y: f32| {
            ui.hit(frus_core::Point::new(x, y))
                .and_then(|id| ui.msg_for(id))
        };
        assert_eq!(tap(85.0, 55.0), Some(Tap), "inside the stack");
        assert_eq!(
            tap(95.0, 45.0),
            None,
            "outside it, where only the drawing is"
        );
    }

    /// **The stack's own reading direction** places a start-anchored layer.
    #[test]
    fn the_stack_s_own_direction_is_read() {
        let stack = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .text_direction(TextDirection::Rtl)
            .layer(Container::new().width(20.0).height(20.0).color(BADGE));
        let r = rects(stack, BADGE)[0];
        assert_eq!(r.x, 80.0, "the start is the right: {r:?}");
    }

    /// **`Positioned::fill`, `from_rect` and `from_relative_rect`** are the reference's
    /// three shorthands.
    #[test]
    fn the_positioned_shorthands() {
        let at = |layer: crate::Positioned<()>| {
            rects(
                Stack::<()>::new().width(100.0).height(100.0).layer(layer),
                BADGE,
            )[0]
        };
        let square = || Container::<()>::new().color(BADGE);
        assert_eq!(
            at(crate::Positioned::fill(square())),
            Rect::new(0.0, 0.0, 100.0, 100.0)
        );
        assert_eq!(
            at(crate::Positioned::from_rect(
                square(),
                Rect::new(10.0, 20.0, 30.0, 40.0)
            )),
            Rect::new(10.0, 20.0, 30.0, 40.0)
        );
        assert_eq!(
            at(crate::Positioned::from_relative_rect(
                square(),
                frus_core::Insets::new(5.0, 10.0, 15.0, 20.0)
            )),
            Rect::new(20.0, 5.0, 70.0, 80.0)
        );
    }

    /// **An indexed stack is as big as its largest child, shown or not**, so switching
    /// pages does not resize it.
    #[test]
    fn an_indexed_stack_is_as_big_as_its_largest_child() {
        let pages = |index: usize| {
            IndexedStack::<()>::new(index)
                .child(Container::new().width(30.0).height(20.0).color(BADGE))
                .child(Container::new().width(30.0).height(60.0).color(BADGE))
        };
        for index in [0, 1] {
            let col = crate::Flex::<()>::column().child(pages(index)).child(
                Container::new()
                    .width(10.0)
                    .height(10.0)
                    .color(Color::rgb(0.1, 0.7, 0.3)),
            );
            let ui = build_ui(
                &col,
                Size::new(200.0, 200.0),
                &Runtime::default(),
                &crate::Theme::default(),
            );
            let next = ui
                .scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if *color == Color::rgb(0.1, 0.7, 0.3) => {
                        Some(*rect)
                    }
                    _ => None,
                })
                .expect("the square under it");
            assert_eq!(next.y, 60.0, "page {index}: the tallest page's height");
        }
    }

    /// A wrapper must not eat the pins: `Keyed(Positioned(…))` keeps its place, which is
    /// what the transparent-wrapper macro's third stated hook is for.
    #[test]
    fn a_wrapped_pin_still_pins() {
        let stack = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .layer(crate::Keyed::new(
                3u64,
                crate::Positioned::new(badge()).top(8.0).left(8.0),
            ));
        let r = rects(stack, BADGE)[0];
        assert_eq!((r.x, r.y), (8.0, 8.0));
    }

    /// A page in `colour`, `side` on a side, answering `msg` when clicked.
    fn page(colour: Color, side: f32, msg: i32) -> Container<i32> {
        Container::<i32>::new()
            .width(side)
            .height(side)
            .color(colour)
            .on_click(msg)
    }

    const PAGE_ONE: Color = Color::rgb(1.0, 0.0, 0.0);
    const PAGE_TWO: Color = Color::rgb(0.0, 0.0, 1.0);

    /// Two pages of different sizes in an indexed stack, one of them shown.
    fn pages(shown: usize) -> IndexedStack<i32> {
        IndexedStack::<i32>::new(shown)
            .width(100.0)
            .height(100.0)
            .child(page(PAGE_ONE, 40.0, 1))
            .child(page(PAGE_TWO, 80.0, 2))
    }

    fn built(root: &dyn Widget<i32>) -> (crate::Ui<i32>, Vec<crate::InspectorNode>) {
        crate::build_ui_inspected(
            root,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        )
    }

    /// Every box of `side`×`side` the walk gave out, whether or not it was painted.
    fn boxes(nodes: &[crate::InspectorNode], side: f32) -> usize {
        nodes
            .iter()
            .filter(|n| (n.rect.width - side).abs() < 0.5 && (n.rect.height - side).abs() < 0.5)
            .count()
    }

    fn painted(ui: &crate::Ui<i32>, of: Color) -> bool {
        ui.scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == of))
    }

    #[test]
    fn an_indexed_stack_lays_every_page_out_and_paints_one() {
        let (ui, nodes) = built(&pages(0));
        assert!(painted(&ui, PAGE_ONE), "the shown page is drawn");
        assert!(!painted(&ui, PAGE_TWO), "the others are not");
        assert_eq!(
            boxes(&nodes, 80.0),
            1,
            "the hidden page still has its box: {}",
            crate::dump_tree(&nodes)
        );
    }

    /// The promise of the whole widget: a page's box does not depend on which page is
    /// shown, so switching costs no re-measurement and moves nothing.
    #[test]
    fn a_hidden_page_keeps_the_size_it_had() {
        let (_, showing_first) = built(&pages(0));
        let (_, showing_second) = built(&pages(1));
        for side in [40.0, 80.0] {
            assert_eq!(
                boxes(&showing_first, side),
                boxes(&showing_second, side),
                "the {side} px page changed box when the other one was shown"
            );
        }
    }

    /// And the contrast that is the reason for it: [`crate::Offstage`] takes the page out
    /// of the tree, so there is no box to keep.
    #[test]
    fn an_offstage_page_has_no_box_at_all() {
        let stack = Stack::<i32>::new()
            .width(100.0)
            .height(100.0)
            .fit(StackFit::Loose)
            .layer(page(PAGE_ONE, 40.0, 1))
            .layer(crate::Offstage::new(page(PAGE_TWO, 80.0, 2)));
        let (_, nodes) = built(&stack);
        assert_eq!(boxes(&nodes, 80.0), 0, "{}", crate::dump_tree(&nodes));
    }

    #[test]
    fn a_hidden_page_takes_no_input() {
        // (60, 60) is inside the second page and outside the first.
        let point = frus_core::Point::new(60.0, 60.0);
        let showing_first = built(&pages(0)).0;
        assert!(
            showing_first
                .hit(point)
                .and_then(|id| showing_first.msg_for(id))
                != Some(2),
            "the hidden page answered a click"
        );
        let showing_second = built(&pages(1)).0;
        assert_eq!(
            showing_second
                .hit(point)
                .and_then(|id| showing_second.msg_for(id)),
            Some(2),
            "and the shown one does"
        );
    }
}

//! [`Stack`]: overlays its children in the **same box**, z-layering them. The last layer
//! sits on top. It underpins the internal overlays: a scrim over content, a badge in a
//! corner, a floating button.
//!
//! Two things decide where a layer lands:
//!
//! - a layer wrapped in [`crate::Positioned`] is pinned against the stack's own edges,
//!   and what is pinned decides its size as well as its place;
//! - every other layer is sized by the stack's [`StackFit`] and placed by its
//!   [`Stack::alignment`].
//!
//! **The default fit is [`StackFit::Expand`], and the reference's is loose.** That is a
//! deliberate difference, not an oversight. In the reference a loosely-constrained child
//! with no size of its own still fills, because a childless box there takes the biggest
//! size it is allowed; under this framework's layout engine it would hug and come out at
//! nothing — invisibly, since a stack draws no box of its own. A scrim, a barrier and
//! every internal overlay here are exactly that widget. `fit(StackFit::Loose)` asks for
//! the other behaviour, and is what a badge or a caption wants.

use frus_core::{AlignmentDirectional, AlignmentGeometry, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// How a stack sizes the layers that are not [`crate::Positioned`] — the reference's
/// `StackFit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackFit {
    /// **Given** the stack's box: every layer is as big as the stack. The default here,
    /// for the reason the module documentation gives.
    #[default]
    Expand,
    /// **Asked** what size it would like, and then placed by the stack's alignment. The
    /// reference's default, and what a badge, a caption or a floating button wants.
    Loose,
}

/// A container of overlaid layers.
pub struct Stack<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    fit: StackFit,
    alignment: AlignmentGeometry,
    layers: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Stack<Msg> {
    /// Creates an empty stack.
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            fit: StackFit::Expand,
            // The reference's default anchor, and it follows the reading direction: the
            // start corner is the left in a left-to-right script and the right in a
            // right-to-left one.
            alignment: AlignmentGeometry::Directional(AlignmentDirectional::TOP_START),
            layers: Vec::new(),
        }
    }

    /// How the layers that are not [`crate::Positioned`] are sized.
    pub fn fit(mut self, fit: StackFit) -> Self {
        self.fit = fit;
        self
    }

    /// Where a layer smaller than the stack sits in it.
    ///
    /// It reaches the layers the stack had to place itself: those under
    /// [`StackFit::Loose`], and the axes of a [`crate::Positioned`] that pinned neither
    /// edge. Under [`StackFit::Expand`] an unpinned layer is already the size of the
    /// stack and there is nothing left to align.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
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

    fn stack_loose(&self) -> bool {
        self.fit == StackFit::Loose
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
pub struct IndexedStack<Msg> {
    index: usize,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    fit: StackFit,
    alignment: AlignmentGeometry,
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
            // The reference's sizing for an indexed stack is loose, and here the argument
            // that makes `Stack`'s default `Expand` does not apply: these children are
            // pages, each with a size of its own, not the scrims and barriers that would
            // collapse to nothing if merely asked.
            fit: StackFit::Loose,
            alignment: AlignmentGeometry::Directional(frus_core::AlignmentDirectional::TOP_START),
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

    /// How the children are sized — [`StackFit::Loose`] here, unlike [`Stack`].
    pub fn fit(mut self, fit: StackFit) -> Self {
        self.fit = fit;
        self
    }

    /// Where a child smaller than the stack sits in it.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
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

    fn stack_loose(&self) -> bool {
        self.fit == StackFit::Loose
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

    /// The default fit hands every layer the box; the loose fit asks it, and then the
    /// alignment says where it goes.
    #[test]
    fn a_loose_layer_keeps_its_size_and_is_aligned() {
        let unsized_layer = || {
            Container::<()>::new()
                .color(BADGE)
                .child(crate::Text::new("x"))
        };

        let expanded = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .layer(unsized_layer());
        let r = rects(expanded, BADGE)[0];
        assert_eq!((r.width, r.height), (100.0, 100.0), "given the box: {r:?}");

        let loose = Stack::<()>::new()
            .width(100.0)
            .height(100.0)
            .fit(StackFit::Loose)
            .alignment(frus_core::Alignment::BOTTOM_RIGHT)
            .layer(unsized_layer());
        let r = rects(loose, BADGE)[0];
        assert!(r.width < 60.0, "asked, and it hugged its text: {r:?}");
        assert_eq!(r.x + r.width, 100.0, "against the right edge: {r:?}");
        assert_eq!(r.y + r.height, 100.0, "and the bottom: {r:?}");
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

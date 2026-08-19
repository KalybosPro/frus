//! Building the layout tree and computing absolute rectangles.

use std::collections::HashMap;

use frus_core::{Rect, Size};
use taffy::{AvailableSpace, TaffyTree, TraversePartialTree};

use crate::style::Style;

/// An opaque identifier for a layout node.
pub type NodeId = taffy::NodeId;

/// A **constrained measurement** for a leaf whose size depends on the space
/// offered — wrapping text, custom-painted content. It receives the maximum
/// width and height (`None` = unconstrained) and returns the content's size. Taffy
/// calls it during layout, including for **intrinsic** sizes (min-content gives a
/// width of `Some(0.0)`, max-content gives `None`).
pub type MeasureFn = Box<dyn Fn(Option<f32>, Option<f32>) -> Size>;

/// The edge a box's content ran past.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Past the left edge.
    Left,
    /// Past the right edge.
    Right,
    /// Past the top edge.
    Top,
    /// Past the bottom edge.
    Bottom,
}

/// A box whose children do not fit inside it, and by how much.
///
/// The reference reports this to the console and paints a striped band across the
/// offending edge. Here a child simply draws outside its parent and nothing says so,
/// which is how a task row's delete button came to be laid out past the window — drawn
/// nowhere, hittable nowhere, and undiagnosed across three milestones (327, 333, 334).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overflowing {
    /// The **parent's** absolute box: the one that turned out to be too small.
    pub rect: Rect,
    /// The edge the content ran past.
    pub side: Side,
    /// By how many logical pixels.
    pub amount: f32,
}

/// A layout tree. `T` is user data attached to the nodes — a colour, say — handed
/// back with each computed rectangle.
pub struct Layout<T> {
    tree: TaffyTree<T>,
    /// Constrained measurements, one per "measured" leaf.
    measures: HashMap<NodeId, MeasureFn>,
}

impl<T> Layout<T> {
    /// Creates an empty tree.
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            measures: HashMap::new(),
        }
    }

    /// Adds a leaf — a node with no children — carrying associated data.
    pub fn leaf(&mut self, style: Style, data: T) -> NodeId {
        self.tree
            .new_leaf_with_context(style.to_taffy(), data)
            .expect("creating a layout leaf")
    }

    /// Adds a **measured** leaf: its size comes from `measure`, given the space
    /// offered, rather than from a dimension fixed in the style.
    pub fn measured_leaf(&mut self, style: Style, data: T, measure: MeasureFn) -> NodeId {
        let node = self.leaf(style, data);
        self.measures.insert(node, measure);
        node
    }

    /// Adds `dy` to a node's **top margin**, after it has been built.
    ///
    /// Baseline alignment needs it: a row can only know how far to push a child down
    /// once it has measured every child, and by then the children are nodes rather
    /// than styles. Nothing else should reach for this — a style is otherwise settled
    /// when the node is made.
    pub fn add_margin_top(&mut self, node: NodeId, dy: f32) {
        if dy <= 0.0 {
            return;
        }
        let mut style = self.tree.style(node).expect("a node we made").clone();
        let base = match style.margin.top {
            taffy::LengthPercentageAuto::Length(v) => v,
            // A percentage or `auto` margin is resolved against the parent, which is
            // not a number we have here; the shift is added as if it were zero, which
            // is the only reading that cannot make the row worse than it was.
            _ => 0.0,
        };
        style.margin.top = taffy::LengthPercentageAuto::Length(base + dy);
        self.tree.set_style(node, style).expect("setting a style");
    }

    /// Adds a container — a node with children — with no associated data.
    pub fn container(&mut self, style: Style, children: &[NodeId]) -> NodeId {
        self.tree
            .new_with_children(style.to_taffy(), children)
            .expect("creating a layout container")
    }

    /// Computes the layout from `root`, within the given available space.
    pub fn compute(&mut self, root: NodeId, available: Size) {
        self.compute_in(
            root,
            taffy::Size {
                width: AvailableSpace::Definite(available.width),
                height: AvailableSpace::Definite(available.height),
            },
        );
    }

    /// Computes the layout of content that is **handed a box**: both axes are
    /// definite, and an unset (`Auto`) root dimension takes the box's size instead
    /// of hugging the content.
    ///
    /// The distinction from [`Layout::compute`] is who decides the size. A widget
    /// laid out in the ordinary way is asked how big it wants to be; a page of a
    /// paged view is *told*, and a page that hugged its content would leave the rest
    /// of the panel empty and unclickable.
    pub fn compute_filled(&mut self, root: NodeId, width: f32, height: f32) {
        if let Ok(current) = self.tree.style(root) {
            let mut style = current.clone();
            let mut changed = false;
            if matches!(style.size.width, taffy::Dimension::Auto) {
                style.size.width = taffy::Dimension::Length(width);
                changed = true;
            }
            if matches!(style.size.height, taffy::Dimension::Auto) {
                style.size.height = taffy::Dimension::Length(height);
                changed = true;
            }
            if changed {
                let _ = self.tree.set_style(root, style);
            }
        }
        self.compute(root, Size::new(width, height));
    }

    /// Computes the layout of scrollable content: each axis is either constrained
    /// to the viewport or **free**, in which case the content takes its natural
    /// size, according to `free_x` and `free_y`.
    pub fn compute_scroll(
        &mut self,
        root: NodeId,
        width: f32,
        height: f32,
        free_x: bool,
        free_y: bool,
    ) {
        // **Fill the constrained axis**: the content of a **single-axis** scrollable takes
        // the viewport's size on the **cross** (constrained) axis, instead of hugging its
        // content. Without this a `flex(1)` or `Percent` child on that axis would collapse
        // for want of a definite basis, and the app would have to insert a "filler"
        // container. We only touch the axis when it is **constrained while the other is
        // free** — true single-axis scrolling, neither definite layout (both constrained)
        // nor 2D scrolling (both free) — and **only** when the root dimension is `Auto`,
        // since an explicit size choice is respected.
        let fill_w = !free_x && free_y;
        let fill_h = !free_y && free_x;
        if fill_w || fill_h {
            if let Ok(current) = self.tree.style(root) {
                let mut style = current.clone();
                let mut changed = false;
                if fill_w && matches!(style.size.width, taffy::Dimension::Auto) {
                    style.size.width = taffy::Dimension::Length(width);
                    changed = true;
                }
                if fill_h && matches!(style.size.height, taffy::Dimension::Auto) {
                    style.size.height = taffy::Dimension::Length(height);
                    changed = true;
                }
                if changed {
                    let _ = self.tree.set_style(root, style);
                }
            }
        }

        let axis = |free: bool, size: f32| {
            if free {
                AvailableSpace::MaxContent
            } else {
                AvailableSpace::Definite(size)
            }
        };
        self.compute_in(
            root,
            taffy::Size {
                width: axis(free_x, width),
                height: axis(free_y, height),
            },
        );
    }

    /// The shared computation: taffy, with measured leaves' **constrained
    /// measurements** routed to their closures.
    fn compute_in(&mut self, root: NodeId, available: taffy::Size<AvailableSpace>) {
        let measures = &self.measures;
        self.tree
            .compute_layout_with_measure(
                root,
                available,
                |known: taffy::Size<Option<f32>>,
                 space: taffy::Size<AvailableSpace>,
                 node,
                 _ctx,
                 _style| {
                    let Some(measure) = measures.get(&node) else {
                        return taffy::Size::ZERO;
                    };
                    // The constraint resolved per axis: the known dimension if taffy
                    // has already settled it, otherwise from the space offered
                    // (min-content = as narrow as possible, 0; max-content = free).
                    let bound = |known: Option<f32>, space: AvailableSpace| {
                        known.or(match space {
                            AvailableSpace::Definite(v) => Some(v),
                            AvailableSpace::MinContent => Some(0.0),
                            AvailableSpace::MaxContent => None,
                        })
                    };
                    let size = measure(
                        bound(known.width, space.width),
                        bound(known.height, space.height),
                    );
                    taffy::Size {
                        width: size.width,
                        height: size.height,
                    }
                },
            )
            .expect("computing the layout");
    }

    /// Walks the tree in prefix order and returns, for each node, its rectangle in
    /// **absolute** coordinates along with any associated data.
    ///
    /// taffy expresses positions relative to the parent; here we accumulate the
    /// offsets to get absolute coordinates that can be rendered directly.
    pub fn absolute_rects(&self, root: NodeId) -> Vec<(Rect, Option<&T>)> {
        let mut out = Vec::new();
        self.collect(root, 0.0, 0.0, &mut out);
        out
    }

    /// Every box in the tree whose children spill out of it, with the edge and the
    /// amount — the measurement behind [`Overflowing`].
    ///
    /// Only within **one** taffy tree, which is exactly the useful scope: a scrollable, a
    /// stack, a page view, a fitter and an overflow box are all laid out as leaves here
    /// and their content computed separately, so content larger than its viewport — the
    /// one overflow that is deliberate — never reaches this walk.
    pub fn overflows(&self, root: NodeId) -> Vec<Overflowing> {
        let mut out = Vec::new();
        self.collect_overflow(root, 0.0, 0.0, &mut out);
        out
    }

    fn collect_overflow(
        &self,
        node: NodeId,
        offset_x: f32,
        offset_y: f32,
        out: &mut Vec<Overflowing>,
    ) {
        let layout = self.tree.layout(node).expect("the node's layout");
        let x = offset_x + layout.location.x;
        let y = offset_y + layout.location.y;

        let child_count = self.tree.child_count(node);
        if child_count > 0 {
            // The content box, in the node's own coordinates. Taffy places children
            // relative to the border box, so these bounds are directly comparable.
            let left = layout.border.left + layout.padding.left;
            let top = layout.border.top + layout.padding.top;
            let right = layout.size.width - layout.border.right - layout.padding.right;
            let bottom = layout.size.height - layout.border.bottom - layout.padding.bottom;

            let (mut over_l, mut over_t, mut over_r, mut over_b) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for i in 0..child_count {
                let child = self.tree.child_at_index(node, i).expect("the node's child");
                let c = self.tree.layout(child).expect("the child's layout");
                over_l = over_l.max(left - c.location.x);
                over_t = over_t.max(top - c.location.y);
                over_r = over_r.max(c.location.x + c.size.width - right);
                over_b = over_b.max(c.location.y + c.size.height - bottom);
            }

            // Sub-pixel slack. Rounding a fractional text width against a fractional box
            // produces overflows of a few hundredths that no one can see and nobody
            // should be told about; the reference has the same tolerance for the same
            // reason.
            const TOLERANCE: f32 = 0.5;
            let rect = Rect::new(x, y, layout.size.width, layout.size.height);
            for (amount, side) in [
                (over_l, Side::Left),
                (over_t, Side::Top),
                (over_r, Side::Right),
                (over_b, Side::Bottom),
            ] {
                if amount > TOLERANCE {
                    out.push(Overflowing { rect, side, amount });
                }
            }
        }

        for i in 0..child_count {
            let child = self.tree.child_at_index(node, i).expect("the node's child");
            self.collect_overflow(child, x, y, out);
        }
    }

    fn collect<'a>(
        &'a self,
        node: NodeId,
        offset_x: f32,
        offset_y: f32,
        out: &mut Vec<(Rect, Option<&'a T>)>,
    ) {
        let layout = self.tree.layout(node).expect("the node's layout");
        let x = offset_x + layout.location.x;
        let y = offset_y + layout.location.y;

        let rect = Rect::new(x, y, layout.size.width, layout.size.height);
        out.push((rect, self.tree.get_node_context(node)));

        let child_count = self.tree.child_count(node);
        for i in 0..child_count {
            let child = self.tree.child_at_index(node, i).expect("the node's child");
            self.collect(child, x, y, out);
        }
    }
}

impl<T> Default for Layout<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    /// A row that does not fit says so, on the side it ran past and by how much.
    #[test]
    fn a_row_too_small_for_its_children_reports_it() {
        let mut layout: Layout<()> = Layout::new();
        let a = layout.leaf(
            Style {
                width: Dimension::Length(80.0),
                flex_shrink: 0.0,
                ..Default::default()
            },
            (),
        );
        let b = layout.leaf(
            Style {
                width: Dimension::Length(80.0),
                flex_shrink: 0.0,
                ..Default::default()
            },
            (),
        );
        let row = layout.container(
            Style {
                width: Dimension::Length(100.0),
                height: Dimension::Length(20.0),
                ..Default::default()
            },
            &[a, b],
        );
        layout.compute(row, Size::new(200.0, 200.0));
        let over = layout.overflows(row);
        assert_eq!(over.len(), 1, "one edge, once: {over:?}");
        assert_eq!(over[0].side, Side::Right);
        assert_eq!(over[0].amount, 60.0);
        assert_eq!(over[0].rect.width, 100.0, "the box named is the parent's");
    }

    /// Padding is part of the box the children have to fit inside.
    #[test]
    fn padding_counts_as_room_taken() {
        let mut layout: Layout<()> = Layout::new();
        let child = layout.leaf(
            Style {
                width: Dimension::Length(100.0),
                flex_shrink: 0.0,
                ..Default::default()
            },
            (),
        );
        let boxed = layout.container(
            Style {
                width: Dimension::Length(100.0),
                height: Dimension::Length(20.0),
                padding: frus_core::Insets::uniform(10.0),
                ..Default::default()
            },
            &[child],
        );
        layout.compute(boxed, Size::new(200.0, 200.0));
        let over = layout.overflows(boxed);
        assert_eq!(over.len(), 1);
        assert_eq!(over[0].amount, 20.0, "10 px of padding on either side");
    }

    /// And a row that fits says nothing — including when the arithmetic lands a hair off,
    /// which fractional text widths do constantly.
    #[test]
    fn a_row_that_fits_is_silent() {
        let mut layout: Layout<()> = Layout::new();
        let a = layout.leaf(
            Style {
                width: Dimension::Length(50.2),
                flex_shrink: 0.0,
                ..Default::default()
            },
            (),
        );
        let b = layout.leaf(
            Style {
                width: Dimension::Length(50.1),
                flex_shrink: 0.0,
                ..Default::default()
            },
            (),
        );
        let row = layout.container(
            Style {
                width: Dimension::Length(100.0),
                height: Dimension::Length(20.0),
                ..Default::default()
            },
            &[a, b],
        );
        layout.compute(row, Size::new(200.0, 200.0));
        assert!(layout.overflows(row).is_empty(), "0.3 px is nobody's bug");
    }
    use super::*;
    use crate::{Align, Dimension, FlexDirection, Justify, Style};
    use frus_core::Insets;

    #[test]
    fn flex_row_computes_absolute_positions() {
        let mut layout: Layout<()> = Layout::new();

        // Child A: fixed width 120. Child B: flex_grow 1, filling the rest.
        let a = layout.leaf(
            Style {
                width: Dimension::Length(120.0),
                ..Default::default()
            },
            (),
        );
        let b = layout.leaf(
            Style {
                flex_grow: 1.0,
                ..Default::default()
            },
            (),
        );
        let root = layout.container(
            Style {
                width: Dimension::Length(400.0),
                height: Dimension::Length(100.0),
                flex_direction: FlexDirection::Row,
                padding: Insets::uniform(10.0),
                gap: 8.0,
                ..Default::default()
            },
            &[a, b],
        );

        layout.compute(root, Size::new(400.0, 100.0));
        let rects = layout.absolute_rects(root);

        // Prefix walk: [root, a, b].
        let (root_rect, _) = rects[0];
        let (a_rect, _) = rects[1];
        let (b_rect, _) = rects[2];

        assert_eq!(root_rect, Rect::new(0.0, 0.0, 400.0, 100.0));

        // The content area: 10 of padding on every side.
        // A: x = 10, width 120, stretched in height (align stretch) => 80, y = 10.
        assert_eq!(a_rect, Rect::new(10.0, 10.0, 120.0, 80.0));

        // B: after A plus the gap => x = 10 + 120 + 8 = 138.
        // Width = content(380) - A(120) - gap(8) = 252.
        assert_eq!(b_rect, Rect::new(138.0, 10.0, 252.0, 80.0));
    }

    #[test]
    fn flex_wrap_moves_overflowing_child_to_next_line() {
        let mut layout: Layout<()> = Layout::new();

        // Three 80px children in a 200px container: 80+80 fit on the first line, and
        // the third (240 > 200) wraps to the next.
        let kids: Vec<_> = (0..3)
            .map(|_| {
                layout.leaf(
                    Style {
                        width: Dimension::Length(80.0),
                        height: Dimension::Length(40.0),
                        ..Default::default()
                    },
                    (),
                )
            })
            .collect();
        let root = layout.container(
            Style {
                width: Dimension::Length(200.0),
                height: Dimension::Length(200.0),
                flex_direction: FlexDirection::Row,
                align: Align::Start,
                flex_wrap: true,
                ..Default::default()
            },
            &kids,
        );

        layout.compute(root, Size::new(200.0, 200.0));
        let rects = layout.absolute_rects(root);
        // Prefix walk: [root, k0, k1, k2].
        let (k0, _) = rects[1];
        let (k1, _) = rects[2];
        let (k2, _) = rects[3];

        // k0 and k1 on the first line; k2 comes back to the left, lower down.
        assert_eq!(k0.y, k1.y);
        assert_eq!(k2.x, k0.x);
        assert!(k2.y >= k0.y + 40.0, "k2.y = {}, expected >= 40", k2.y);
    }

    #[test]
    fn measured_leaf_wraps_to_the_offered_width() {
        // Simulates a 250px text that wraps: given a width W, it occupies min(W, 250)
        // across and grows in height when it wraps.
        let measure: crate::MeasureFn = Box::new(|w, _| {
            let w = w.unwrap_or(250.0).min(250.0);
            let lines = (250.0 / w).ceil();
            Size::new(w, lines * 20.0)
        });
        let mut layout: Layout<()> = Layout::new();
        let text = layout.measured_leaf(Style::default(), (), measure);
        let root = layout.container(
            Style {
                width: Dimension::Length(100.0),
                flex_direction: FlexDirection::Column,
                align: Align::Start,
                ..Default::default()
            },
            &[text],
        );
        layout.compute(root, Size::new(100.0, 600.0));
        let rects = layout.absolute_rects(root);
        let (text_rect, _) = rects[1];
        assert!(
            text_rect.width <= 100.0,
            "wrapped to the offered width: {text_rect:?}"
        );
        assert!(
            text_rect.height >= 60.0,
            "3 lines expected (250/100 → 3 × 20): {text_rect:?}"
        );
    }

    /// The same measured leaf, but **centred** on the cross axis instead of
    /// stretched. Centring means the item is sized to fit rather than filled, and the
    /// height must still be the one that goes with the width it ends up at.
    #[test]
    fn a_centred_measured_leaf_reports_the_height_of_the_width_it_got() {
        let measure: crate::MeasureFn = Box::new(|w, _| {
            let w = w.unwrap_or(250.0).min(250.0);
            let lines = (250.0 / w).ceil();
            Size::new(w, lines * 20.0)
        });
        let mut layout: Layout<()> = Layout::new();
        let text = layout.measured_leaf(Style::default(), (), measure);
        let root = layout.container(
            Style {
                width: Dimension::Length(100.0),
                flex_direction: FlexDirection::Column,
                align: Align::Center,
                ..Default::default()
            },
            &[text],
        );
        layout.compute(root, Size::new(100.0, 600.0));
        let rects = layout.absolute_rects(root);
        let (text_rect, _) = rects[1];
        assert!(
            text_rect.width <= 100.0,
            "wrapped to the offered width: {text_rect:?}"
        );
        assert!(
            text_rect.height >= 60.0,
            "3 lines expected (250/100 -> 3 x 20): {text_rect:?}"
        );
    }

    #[test]
    fn justify_center_centers_child() {
        let mut layout: Layout<()> = Layout::new();
        let child = layout.leaf(
            Style {
                width: Dimension::Length(100.0),
                height: Dimension::Length(40.0),
                align: Align::Start,
                ..Default::default()
            },
            (),
        );
        let root = layout.container(
            Style {
                width: Dimension::Length(400.0),
                height: Dimension::Length(100.0),
                flex_direction: FlexDirection::Row,
                justify: Justify::Center,
                ..Default::default()
            },
            &[child],
        );
        layout.compute(root, Size::new(400.0, 100.0));
        let rects = layout.absolute_rects(root);

        // A child centred on the main axis: x = (400 - 100) / 2 = 150.
        assert_eq!(rects[1].0, Rect::new(150.0, 0.0, 100.0, 40.0));
    }
}

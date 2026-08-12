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

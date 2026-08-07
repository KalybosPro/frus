//! [`Tree`]: a **controlled** hierarchical tree. The application holds the structure
//! and the expanded nodes, and passes only the **visible rows**, flat (with their
//! depth). The widget does nothing but render.
//!
//! Two distinct gestures, as in a file explorer: clicking the **chevron** expands or
//! collapses the node (`on_toggle`), clicking **anywhere else** on the row **selects**
//! it (`on_select`, leaves included). Vertical **guide lines** make the indentation
//! levels visible, and the selected row is highlighted.

use std::rc::Rc;

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const ROW_H: f32 = 32.0;
const INDENT: f32 = 20.0;
const SIZE: f32 = 16.0;

/// One tree row: indented, a chevron if the node has children, **guide lines** back to its
/// ancestors, and a **selection** background. Clicking the chevron expands or collapses
/// (`toggle`), the rest selects (`select`) — told apart by
/// [`positional_click`](Widget::positional_click).
struct Row<Msg> {
    depth: usize,
    label: String,
    expandable: bool,
    expanded: bool,
    selected: bool,
    /// Toggle message (collapsible nodes only).
    toggle: Option<Msg>,
    /// Selection message (if the tree is selectable).
    select: Option<Msg>,
}

impl<Msg: Clone> Row<Msg> {
    /// The chevron's local start x, within the row's box.
    fn chevron_start(&self) -> f32 {
        self.depth as f32 * INDENT
    }
}

/// Margin right of the label (the hover/selection background overruns the text a little).
const ROW_PAD_R: f32 = 12.0;

impl<Msg: Clone> Widget<Msg> for Row<Msg> {
    fn style(&self) -> Style {
        // **Intrinsic** width = indentation + chevron + label (+ margin): without it the row
        // would have no width at all (no child to measure) and the selection background
        // would be invisible. The `Tree` (a column, align Stretch) then stretches every
        // row out to the widest one.
        let text_w = frus_text::measure(&self.label, SIZE).width;
        let width = self.chevron_start() + INDENT + text_w + ROW_PAD_R;
        Style {
            width: Dimension::Length(width),
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let clickable = self.toggle.is_some() || self.select.is_some();
        // Background: selection (a primary tint) wins, otherwise the hover state.
        if self.selected {
            let bg = theme.surface.lerp(theme.primary, 0.16);
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        } else if clickable && status.hover_progress > 0.0 {
            let bg = theme.state_layer(theme.surface, theme.on_surface, &status);
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }

        // Vertical guide lines: one per ancestor level, centred in its indentation step.
        for d in 0..self.depth {
            let gx = bounds.x + d as f32 * INDENT + INDENT * 0.5;
            scene.draw_rect(
                Rect::new(gx, bounds.y, 1.0, bounds.height),
                theme.border.fade(o * 0.6),
                0.0,
                0.0,
                Color::TRANSPARENT,
            );
        }

        let x = bounds.x + self.chevron_start();
        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        if self.expandable {
            scene.text(
                Point::new(x, ty),
                if self.expanded { "▾" } else { "▸" }.to_string(),
                SIZE,
                theme.muted.fade(o),
            );
        }
        scene.text(
            Point::new(x + INDENT, ty),
            self.label.clone(),
            SIZE,
            theme.on_surface.fade(o),
        );
    }

    fn positional_click(
        &self,
        local_x: f32,
        _local_y: f32,
        _width: f32,
        _height: f32,
    ) -> Option<Msg> {
        // The chevron expands or collapses; the rest of the row selects, or toggles by default.
        let start = self.chevron_start();
        if self.expandable && local_x >= start && local_x < start + INDENT {
            return self.toggle.clone();
        }
        self.select.clone().or_else(|| self.toggle.clone())
    }

    fn on_click(&self) -> Option<Msg> {
        // Keyboard fallback (Enter/Space): the main action is selection, otherwise toggling.
        self.select.clone().or_else(|| self.toggle.clone())
    }

    fn focusable(&self) -> bool {
        self.toggle.is_some() || self.select.is_some()
    }
}

/// A hierarchical tree (visible rows, flattened).
pub struct Tree<Msg> {
    on_toggle: Box<dyn Fn(u64) -> Msg>,
    on_select: Option<Rc<dyn Fn(u64) -> Msg>>,
    selected: Option<u64>,
    nodes: Vec<(u64, usize, String, bool, bool)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Tree<Msg> {
    /// Creates a tree; `on_toggle(id)` is emitted when a collapsible node's **chevron** is clicked.
    pub fn new(on_toggle: impl Fn(u64) -> Msg + 'static) -> Self {
        Self {
            on_toggle: Box::new(on_toggle),
            on_select: None,
            selected: None,
            nodes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Makes the nodes **selectable**: `on_select(id)` when the row's body is clicked (outside
    /// the chevron), **leaves included**. Without it, clicking the row expands or collapses.
    pub fn on_select(mut self, on_select: impl Fn(u64) -> Msg + 'static) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self.rebuild();
        self
    }

    /// Marks the **selected** node (highlighted). `None` = none.
    pub fn selected(mut self, id: Option<u64>) -> Self {
        self.selected = id;
        self.rebuild();
        self
    }

    /// Adds a visible row: `id`, `depth` (indentation), `label`, whether the node has
    /// children (`expandable`) and whether it is expanded (`expanded`).
    pub fn node(
        mut self,
        id: u64,
        depth: usize,
        label: impl Into<String>,
        expandable: bool,
        expanded: bool,
    ) -> Self {
        self.nodes
            .push((id, depth, label.into(), expandable, expanded));
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        self.children = self
            .nodes
            .iter()
            .map(|(id, depth, label, expandable, expanded)| {
                Box::new(Row {
                    depth: *depth,
                    label: label.clone(),
                    expandable: *expandable,
                    expanded: *expanded,
                    selected: self.selected == Some(*id),
                    toggle: expandable.then(|| (self.on_toggle)(*id)),
                    select: self.on_select.as_ref().map(|f| f(*id)),
                }) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for Tree<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            gap: 1.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle(u64),
        Select(u64),
    }

    #[test]
    fn expandable_nodes_toggle_leaves_do_not() {
        // Without `on_select`, the row behaves as before: the folder toggles, the leaf does not.
        let tree = Tree::new(Msg::Toggle)
            .node(1, 0, "Folder", true, true)
            .node(2, 1, "file.txt", false, false);
        let rows = Widget::<Msg>::children(&tree);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].on_click(), Some(Msg::Toggle(1))); // a collapsible folder
        assert_eq!(rows[1].on_click(), None); // a leaf
    }

    #[test]
    fn chevron_toggles_body_selects() {
        // With `on_select`, the chevron expands/collapses and the row body selects — leaves included.
        let tree = Tree::new(Msg::Toggle)
            .on_select(Msg::Select)
            .selected(Some(2))
            .node(1, 0, "Folder", true, true)
            .node(2, 1, "file.txt", false, false);
        let rows = Widget::<Msg>::children(&tree);
        // A collapsible node (depth 0): chevron in [0, INDENT) → toggle; beyond that → selection.
        assert_eq!(
            rows[0].positional_click(INDENT * 0.5, 0.0, 200.0, ROW_H),
            Some(Msg::Toggle(1))
        );
        assert_eq!(
            rows[0].positional_click(INDENT * 3.0, 0.0, 200.0, ROW_H),
            Some(Msg::Select(1))
        );
        // A leaf (depth 1): no chevron zone → everything selects (even under the indentation step).
        assert_eq!(
            rows[1].positional_click(INDENT * 0.5, 0.0, 200.0, ROW_H),
            Some(Msg::Select(2))
        );
        // Keyboard (on_click): the main action is selection.
        assert_eq!(rows[0].on_click(), Some(Msg::Select(1)));
    }
}

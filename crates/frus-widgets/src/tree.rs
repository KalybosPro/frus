//! [`Tree`] : un arbre hiérarchique **contrôlé**. L'application tient la structure
//! et les nœuds dépliés, et ne passe que les **lignes visibles**, à plat (avec
//! leur profondeur). Le widget se contente de rendre.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const ROW_H: f32 = 32.0;
const INDENT: f32 = 20.0;
const SIZE: f32 = 16.0;

/// Une ligne d'arbre : indentée, avec chevron si le nœud a des enfants.
struct Row<Msg> {
    depth: usize,
    label: String,
    expandable: bool,
    expanded: bool,
    /// Message de bascule (nœuds pliables seulement).
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Row<Msg> {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        if self.message.is_some() && status.hover_progress > 0.0 {
            let bg = theme.state_layer(theme.surface, theme.on_surface, &status);
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, frus_core::Color::TRANSPARENT);
        }
        let x = bounds.x + self.depth as f32 * INDENT;
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

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        self.message.is_some()
    }
}

/// Un arbre hiérarchique (lignes visibles à plat).
pub struct Tree<Msg> {
    on_toggle: Box<dyn Fn(u64) -> Msg>,
    nodes: Vec<(u64, usize, String, bool, bool)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Tree<Msg> {
    /// Crée un arbre ; `on_toggle(id)` est émis au clic sur un nœud pliable.
    pub fn new(on_toggle: impl Fn(u64) -> Msg + 'static) -> Self {
        Self {
            on_toggle: Box::new(on_toggle),
            nodes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Ajoute une ligne visible : `id`, `depth` (indentation), `label`, si le nœud
    /// a des enfants (`expandable`) et s'il est déplié (`expanded`).
    pub fn node(
        mut self,
        id: u64,
        depth: usize,
        label: impl Into<String>,
        expandable: bool,
        expanded: bool,
    ) -> Self {
        self.nodes.push((id, depth, label.into(), expandable, expanded));
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
                    message: expandable.then(|| (self.on_toggle)(*id)),
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
    }

    #[test]
    fn expandable_nodes_toggle_leaves_do_not() {
        let tree = Tree::new(Msg::Toggle)
            .node(1, 0, "Dossier", true, true)
            .node(2, 1, "fichier.txt", false, false);
        let rows = Widget::<Msg>::children(&tree);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].on_click(), Some(Msg::Toggle(1))); // dossier pliable
        assert_eq!(rows[1].on_click(), None); // feuille
    }
}

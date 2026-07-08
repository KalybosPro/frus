//! Construction de l'arbre de mise en page et calcul des rectangles absolus.

use frus_core::{Rect, Size};
use taffy::{AvailableSpace, TaffyTree, TraversePartialTree};

use crate::style::Style;

/// Identifiant opaque d'un nœud de mise en page.
pub type NodeId = taffy::NodeId;

/// Un arbre de mise en page. `T` est une donnée utilisateur attachée aux nœuds
/// (par exemple une couleur), restituée avec chaque rectangle calculé.
pub struct Layout<T> {
    tree: TaffyTree<T>,
}

impl<T> Layout<T> {
    /// Crée un arbre vide.
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
        }
    }

    /// Ajoute une feuille (nœud sans enfant) avec une donnée associée.
    pub fn leaf(&mut self, style: Style, data: T) -> NodeId {
        self.tree
            .new_leaf_with_context(style.to_taffy(), data)
            .expect("création d'une feuille de layout")
    }

    /// Ajoute un conteneur (nœud avec enfants), sans donnée associée.
    pub fn container(&mut self, style: Style, children: &[NodeId]) -> NodeId {
        self.tree
            .new_with_children(style.to_taffy(), children)
            .expect("création d'un conteneur de layout")
    }

    /// Calcule la mise en page à partir de `root`, dans l'espace disponible donné.
    pub fn compute(&mut self, root: NodeId, available: Size) {
        self.tree
            .compute_layout(
                root,
                taffy::Size {
                    width: AvailableSpace::Definite(available.width),
                    height: AvailableSpace::Definite(available.height),
                },
            )
            .expect("calcul de la mise en page");
    }

    /// Parcourt l'arbre (préfixe) et renvoie, pour chaque nœud, son rectangle en
    /// coordonnées **absolues** ainsi que sa donnée associée éventuelle.
    ///
    /// taffy exprime les positions relativement au parent ; on accumule ici les
    /// offsets pour obtenir des coordonnées absolues directement rendables.
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
        let layout = self.tree.layout(node).expect("layout du nœud");
        let x = offset_x + layout.location.x;
        let y = offset_y + layout.location.y;

        let rect = Rect::new(x, y, layout.size.width, layout.size.height);
        out.push((rect, self.tree.get_node_context(node)));

        let child_count = self.tree.child_count(node);
        for i in 0..child_count {
            let child = self.tree.child_at_index(node, i).expect("enfant du nœud");
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
    use crate::{Dimension, FlexDirection, Style};

    #[test]
    fn flex_row_computes_absolute_positions() {
        let mut layout: Layout<()> = Layout::new();

        // Enfant A : largeur fixe 120. Enfant B : flex_grow 1 (remplit le reste).
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
                padding: 10.0,
                gap: 8.0,
                ..Default::default()
            },
            &[a, b],
        );

        layout.compute(root, Size::new(400.0, 100.0));
        let rects = layout.absolute_rects(root);

        // Parcours préfixe : [root, a, b].
        let (root_rect, _) = rects[0];
        let (a_rect, _) = rects[1];
        let (b_rect, _) = rects[2];

        assert_eq!(root_rect, Rect::new(0.0, 0.0, 400.0, 100.0));

        // Zone de contenu : padding 10 de chaque côté.
        // A : x = 10, largeur 120, étirée en hauteur (align stretch) => 80, y = 10.
        assert_eq!(a_rect, Rect::new(10.0, 10.0, 120.0, 80.0));

        // B : après A + gap => x = 10 + 120 + 8 = 138.
        // Largeur = contenu(380) - A(120) - gap(8) = 252.
        assert_eq!(b_rect, Rect::new(138.0, 10.0, 252.0, 80.0));
    }
}

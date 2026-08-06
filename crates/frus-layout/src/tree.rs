//! Construction de l'arbre de mise en page et calcul des rectangles absolus.

use std::collections::HashMap;

use frus_core::{Rect, Size};
use taffy::{AvailableSpace, TaffyTree, TraversePartialTree};

use crate::style::Style;

/// Identifiant opaque d'un nœud de mise en page.
pub type NodeId = taffy::NodeId;

/// Une **mesure sous contraintes** pour une feuille dont la taille dépend de
/// l'espace offert (texte qui se replie, contenu peint sur mesure). Reçoit les
/// largeur/hauteur maximales (`None` = non contraint) et renvoie la taille du
/// contenu. Appelée par taffy pendant le calcul — y compris pour les tailles
/// **intrinsèques** (min-content → largeur `Some(0.0)`, max-content → `None`).
pub type MeasureFn = Box<dyn Fn(Option<f32>, Option<f32>) -> Size>;

/// Un arbre de mise en page. `T` est une donnée utilisateur attachée aux nœuds
/// (par exemple une couleur), restituée avec chaque rectangle calculé.
pub struct Layout<T> {
    tree: TaffyTree<T>,
    /// Mesures sous contraintes, par feuille « mesurée ».
    measures: HashMap<NodeId, MeasureFn>,
}

impl<T> Layout<T> {
    /// Crée un arbre vide.
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            measures: HashMap::new(),
        }
    }

    /// Ajoute une feuille (nœud sans enfant) avec une donnée associée.
    pub fn leaf(&mut self, style: Style, data: T) -> NodeId {
        self.tree
            .new_leaf_with_context(style.to_taffy(), data)
            .expect("création d'une feuille de layout")
    }

    /// Ajoute une feuille **mesurée** : sa taille est calculée par `measure`
    /// selon l'espace offert (au lieu d'une dimension fixée dans le style).
    pub fn measured_leaf(&mut self, style: Style, data: T, measure: MeasureFn) -> NodeId {
        let node = self.leaf(style, data);
        self.measures.insert(node, measure);
        node
    }

    /// Ajoute un conteneur (nœud avec enfants), sans donnée associée.
    pub fn container(&mut self, style: Style, children: &[NodeId]) -> NodeId {
        self.tree
            .new_with_children(style.to_taffy(), children)
            .expect("création d'un conteneur de layout")
    }

    /// Calcule la mise en page à partir de `root`, dans l'espace disponible donné.
    pub fn compute(&mut self, root: NodeId, available: Size) {
        self.compute_in(
            root,
            taffy::Size {
                width: AvailableSpace::Definite(available.width),
                height: AvailableSpace::Definite(available.height),
            },
        );
    }

    /// Calcule la mise en page d'un contenu scrollable : chaque axe est soit
    /// contraint au viewport, soit **libre** (le contenu prend sa taille
    /// naturelle) selon `free_x` / `free_y`.
    pub fn compute_scroll(
        &mut self,
        root: NodeId,
        width: f32,
        height: f32,
        free_x: bool,
        free_y: bool,
    ) {
        // **Remplir l'axe contraint** (façon `ListView` de Flutter) : le contenu d'un défilable
        // **à un seul axe** prend la taille du viewport sur l'axe **transverse** (contraint), au
        // lieu de se caler sur son contenu — sans quoi un enfant `flex(1)`/`Percent` sur cet axe
        // s'effondrerait faute d'une base définie, et l'app devrait insérer un conteneur
        // « remplisseur ». On ne touche l'axe que s'il est **contraint alors que l'autre est
        // libre** (vrai défilement mono-axe : ni la mise en page définie — deux axes contraints —,
        // ni le défilement 2D — deux axes libres), et **seulement** si la dimension racine est
        // `Auto` (un choix explicite de taille est respecté).
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

    /// Le calcul commun : taffy, avec les **mesures sous contraintes** des
    /// feuilles mesurées routées vers leur closure.
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
                    // Contrainte résolue par axe : dimension connue si taffy l'a
                    // déjà tranchée, sinon selon l'espace offert (min-content =
                    // le plus étroit possible → 0 ; max-content = non contraint).
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
    use crate::{Align, Dimension, FlexDirection, Justify, Style};
    use frus_core::Insets;

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
                padding: Insets::uniform(10.0),
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

    #[test]
    fn flex_wrap_moves_overflowing_child_to_next_line() {
        let mut layout: Layout<()> = Layout::new();

        // 3 enfants de 80px dans un conteneur de 200px : 80+80 tiennent sur la
        // 1re ligne, le 3e (240 > 200) passe à la ligne suivante.
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
        // Parcours préfixe : [root, k0, k1, k2].
        let (k0, _) = rects[1];
        let (k1, _) = rects[2];
        let (k2, _) = rects[3];

        // k0 et k1 sur la 1re ligne ; k2 revient à gauche, plus bas.
        assert_eq!(k0.y, k1.y);
        assert_eq!(k2.x, k0.x);
        assert!(k2.y >= k0.y + 40.0, "k2.y = {} attendu >= 40", k2.y);
    }

    #[test]
    fn measured_leaf_wraps_to_the_offered_width() {
        // Simule un texte de 250 px qui se replie : à largeur W offerte, il
        // occupe min(W, 250) de large et grandit en hauteur s'il se replie.
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
            "replié à la largeur offerte : {text_rect:?}"
        );
        assert!(
            text_rect.height >= 60.0,
            "3 lignes attendues (250/100 → 3 × 20) : {text_rect:?}"
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

        // Enfant centré sur l'axe principal : x = (400 - 100) / 2 = 150.
        assert_eq!(rects[1].0, Rect::new(150.0, 0.0, 100.0, 40.0));
    }
}

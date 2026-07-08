//! Le pilote : construit une [`Ui`] (scène + carte de hit-test) à partir d'un
//! arbre de widgets, et permet de retrouver le message sous un point.

use frus_core::{Point, Rect, Scene, Size};
use frus_layout::{Layout, NodeId};

use crate::widget::Widget;

/// Résultat de la construction d'une interface pour une frame donnée :
/// la [`Scene`] à dessiner et la carte des zones cliquables.
pub struct Ui<Msg> {
    scene: Scene,
    /// Zones cliquables, en ordre préfixe (parents avant enfants).
    hits: Vec<(Rect, Msg)>,
}

impl<Msg: Clone> Ui<Msg> {
    /// La scène à envoyer au renderer.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Message du widget cliquable le plus **au-dessus** contenant `point`.
    ///
    /// Comme les enfants sont peints après (donc au-dessus de) leurs parents, on
    /// prend la dernière zone correspondante en ordre préfixe.
    pub fn hit(&self, point: Point) -> Option<Msg> {
        self.hits
            .iter()
            .rev()
            .find(|(rect, _)| rect_contains(*rect, point))
            .map(|(_, msg)| msg.clone())
    }
}

fn rect_contains(rect: Rect, point: Point) -> bool {
    point.x >= rect.x
        && point.x < rect.x + rect.width
        && point.y >= rect.y
        && point.y < rect.y + rect.height
}

/// Construit récursivement l'arbre de layout à partir de l'arbre de widgets.
fn build_layout<Msg>(widget: &dyn Widget<Msg>, layout: &mut Layout<()>) -> NodeId {
    let children = widget.children();
    if children.is_empty() {
        layout.leaf(widget.style(), ())
    } else {
        let child_ids: Vec<NodeId> = children
            .iter()
            .map(|child| build_layout(child.as_ref(), layout))
            .collect();
        layout.container(widget.style(), &child_ids)
    }
}

/// Aplati l'arbre de widgets en ordre préfixe (parent avant enfants), pour
/// s'aligner avec l'ordre de [`Layout::absolute_rects`].
fn flatten<'a, Msg>(widget: &'a dyn Widget<Msg>, out: &mut Vec<&'a dyn Widget<Msg>>) {
    out.push(widget);
    for child in widget.children() {
        flatten(child.as_ref(), out);
    }
}

/// Traduit un arbre de widgets en [`Ui`] pour un espace disponible donné.
///
/// Étapes : construction du layout, calcul flexbox, appariement de chaque widget
/// avec son rectangle absolu (même ordre préfixe), puis peinture et collecte des
/// zones cliquables.
pub fn build_ui<Msg: Clone>(root: &dyn Widget<Msg>, available: Size) -> Ui<Msg> {
    let mut layout: Layout<()> = Layout::new();
    let root_id = build_layout(root, &mut layout);
    layout.compute(root_id, available);

    let rects = layout.absolute_rects(root_id);
    let mut widgets = Vec::new();
    flatten(root, &mut widgets);

    debug_assert_eq!(
        widgets.len(),
        rects.len(),
        "l'arbre de widgets et l'arbre de layout doivent avoir la même taille"
    );

    let mut scene = Scene::new();
    let mut hits = Vec::new();
    for (widget, (rect, _)) in widgets.iter().zip(rects.iter()) {
        widget.paint(*rect, &mut scene);
        if let Some(msg) = widget.on_click() {
            hits.push((*rect, msg));
        }
    }

    Ui { scene, hits }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex};
    use frus_core::{Color, Point, Primitive, Rect, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        A,
        B,
    }

    #[test]
    fn build_ui_paints_and_maps_clickable_zones() {
        let tree = Flex::row()
            .width(400.0)
            .height(100.0)
            .padding(10.0)
            .gap(8.0)
            .child(
                Container::new()
                    .width(120.0)
                    .color(Color::rgb(1.0, 0.0, 0.0))
                    .on_click(Msg::A),
            )
            .child(
                Container::new()
                    .flex(1.0)
                    .color(Color::rgb(0.0, 0.0, 1.0))
                    .on_click(Msg::B),
            );

        let ui = build_ui(&tree, Size::new(400.0, 100.0));

        // Deux rectangles peints, aux positions flex attendues (cf. Jalon 2).
        let prims = ui.scene().primitives();
        assert_eq!(prims.len(), 2);
        assert_eq!(
            prims[0],
            Primitive::Rect {
                rect: Rect::new(10.0, 10.0, 120.0, 80.0),
                color: Color::rgb(1.0, 0.0, 0.0),
            }
        );

        // Hit-test : A occupe x∈[10,130), B x∈[138,390).
        assert_eq!(ui.hit(Point::new(50.0, 50.0)), Some(Msg::A));
        assert_eq!(ui.hit(Point::new(300.0, 50.0)), Some(Msg::B));
        // Le conteneur Flex n'est pas cliquable → zone de padding = aucun message.
        assert_eq!(ui.hit(Point::new(3.0, 3.0)), None);
    }

    #[test]
    fn hit_returns_topmost_widget_on_overlap() {
        // Un conteneur cliquable (A) avec un enfant cliquable (B) par-dessus.
        let tree = Container::new()
            .width(200.0)
            .height(200.0)
            .color(Color::rgb(1.0, 0.0, 0.0))
            .on_click(Msg::A)
            .child(
                Container::new()
                    .width(100.0)
                    .height(100.0)
                    .color(Color::rgb(0.0, 0.0, 1.0))
                    .on_click(Msg::B),
            );

        let ui = build_ui(&tree, Size::new(200.0, 200.0));

        // Sur l'enfant (0,0,100,100) → B (au-dessus). Ailleurs → A.
        assert_eq!(ui.hit(Point::new(50.0, 50.0)), Some(Msg::B));
        assert_eq!(ui.hit(Point::new(150.0, 150.0)), Some(Msg::A));
    }
}

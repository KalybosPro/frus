//! Le trait [`Widget`] et le pilote [`build_scene`].

use frus_core::{Rect, Scene, Size};
use frus_layout::{Layout, NodeId, Style};

/// Un widget : un élément d'interface composable.
///
/// Un widget fournit son style de mise en page, ses enfants, et sait se peindre
/// une fois ses bornes (rectangle absolu) connues.
pub trait Widget {
    /// Style de mise en page (transmis à `frus-layout`).
    fn style(&self) -> Style;

    /// Enfants du widget (éventuellement vide).
    fn children(&self) -> &[Box<dyn Widget>];

    /// Peint la décoration propre du widget dans `scene`, aux bornes `bounds`.
    ///
    /// Les enfants sont peints séparément par le pilote ; un widget ne peint que
    /// lui-même.
    fn paint(&self, bounds: Rect, scene: &mut Scene);
}

/// Construit récursivement l'arbre de layout à partir de l'arbre de widgets.
fn build_layout(widget: &dyn Widget, layout: &mut Layout<()>) -> NodeId {
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
fn flatten<'a>(widget: &'a dyn Widget, out: &mut Vec<&'a dyn Widget>) {
    out.push(widget);
    for child in widget.children() {
        flatten(child.as_ref(), out);
    }
}

/// Traduit un arbre de widgets en [`Scene`] pour un espace disponible donné.
///
/// Étapes : construction de l'arbre de layout, calcul flexbox, appariement de
/// chaque widget avec son rectangle absolu (même ordre préfixe), puis peinture.
pub fn build_scene(root: &dyn Widget, available: Size) -> Scene {
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
    for (widget, (rect, _)) in widgets.iter().zip(rects.iter()) {
        widget.paint(*rect, &mut scene);
    }
    scene
}

#[cfg(test)]
mod tests {
    use crate::{Container, Flex};
    use frus_core::{Color, Primitive, Rect, Size};

    #[test]
    fn row_of_two_containers_produces_absolute_primitives() {
        let ui = Flex::row()
            .width(400.0)
            .height(100.0)
            .padding(10.0)
            .gap(8.0)
            .child(Container::new().width(120.0).color(Color::rgb(1.0, 0.0, 0.0)))
            .child(Container::new().flex(1.0).color(Color::rgb(0.0, 0.0, 1.0)));

        let scene = super::build_scene(&ui, Size::new(400.0, 100.0));
        let primitives = scene.primitives();

        // Le Flex ne peint rien ; seuls les deux Container produisent une primitive.
        assert_eq!(primitives.len(), 2);
        assert_eq!(
            primitives[0],
            Primitive::Rect {
                rect: Rect::new(10.0, 10.0, 120.0, 80.0),
                color: Color::rgb(1.0, 0.0, 0.0),
            }
        );
        assert_eq!(
            primitives[1],
            Primitive::Rect {
                rect: Rect::new(138.0, 10.0, 252.0, 80.0),
                color: Color::rgb(0.0, 0.0, 1.0),
            }
        );
    }
}

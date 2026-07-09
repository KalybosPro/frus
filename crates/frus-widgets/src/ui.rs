//! Le pilote : construit une [`Ui`] (scène + carte de hit-test avec identités) à
//! partir d'un arbre de widgets et de l'état d'interaction courant.

use frus_core::{Point, Rect, Scene, Size};
use frus_layout::{Layout, NodeId};

use crate::interaction::{InputState, WidgetId};
use crate::widget::Widget;

/// Une zone cliquable : identité, bornes et message associé.
struct Hit<Msg> {
    id: WidgetId,
    rect: Rect,
    msg: Msg,
}

/// Résultat de la construction d'une interface pour une frame donnée :
/// la [`Scene`] à dessiner et la carte des zones cliquables.
pub struct Ui<Msg> {
    scene: Scene,
    hits: Vec<Hit<Msg>>,
}

impl<Msg: Clone> Ui<Msg> {
    /// La scène à envoyer au renderer.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Identité du widget cliquable le plus **au-dessus** contenant `point`.
    pub fn hit(&self, point: Point) -> Option<WidgetId> {
        self.hits
            .iter()
            .rev()
            .find(|hit| rect_contains(hit.rect, point))
            .map(|hit| hit.id)
    }

    /// Message associé à un widget cliquable donné.
    pub fn msg_for(&self, id: WidgetId) -> Option<Msg> {
        self.hits
            .iter()
            .find(|hit| hit.id == id)
            .map(|hit| hit.msg.clone())
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

/// Aplati l'arbre en ordre préfixe, en calculant l'identité positionnelle de
/// chaque widget. Le même ordre que [`Layout::absolute_rects`].
fn flatten<'a, Msg>(
    widget: &'a dyn Widget<Msg>,
    id: WidgetId,
    out: &mut Vec<(&'a dyn Widget<Msg>, WidgetId)>,
) {
    out.push((widget, id));
    for (index, child) in widget.children().iter().enumerate() {
        flatten(child.as_ref(), id.child(index), out);
    }
}

/// Traduit un arbre de widgets en [`Ui`], en tenant compte de l'état
/// d'interaction (pour le survol/pression) et en calculant les identités.
pub fn build_ui<Msg: Clone>(
    root: &dyn Widget<Msg>,
    available: Size,
    input: &InputState,
) -> Ui<Msg> {
    let mut layout: Layout<()> = Layout::new();
    let root_node = build_layout(root, &mut layout);
    layout.compute(root_node, available);

    let rects = layout.absolute_rects(root_node);
    let mut widgets = Vec::new();
    flatten(root, WidgetId::ROOT, &mut widgets);

    debug_assert_eq!(
        widgets.len(),
        rects.len(),
        "l'arbre de widgets et l'arbre de layout doivent avoir la même taille"
    );

    let mut scene = Scene::new();
    let mut hits = Vec::new();
    for ((widget, id), (rect, _)) in widgets.iter().zip(rects.iter()) {
        let status = input.status_for(*id);
        widget.paint(*rect, status, &mut scene);
        if let Some(msg) = widget.on_click() {
            hits.push(Hit {
                id: *id,
                rect: *rect,
                msg,
            });
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

    fn sample() -> Flex<Msg> {
        Flex::row()
            .width(400.0)
            .height(100.0)
            .padding(10.0)
            .gap(8.0)
            .child(
                Container::new()
                    .width(120.0)
                    .color(Color::rgb(1.0, 0.0, 0.0))
                    .hover_color(Color::rgb(0.0, 1.0, 0.0))
                    .on_click(Msg::A),
            )
            .child(
                Container::new()
                    .flex(1.0)
                    .color(Color::rgb(0.0, 0.0, 1.0))
                    .on_click(Msg::B),
            )
    }

    #[test]
    fn hit_and_msg_for_route_correctly() {
        let ui = build_ui(&sample(), Size::new(400.0, 100.0), &InputState::default());

        let id_a = ui.hit(Point::new(50.0, 50.0)).expect("A sous le point");
        let id_b = ui.hit(Point::new(300.0, 50.0)).expect("B sous le point");
        assert_ne!(id_a, id_b);
        assert_eq!(ui.msg_for(id_a), Some(Msg::A));
        assert_eq!(ui.msg_for(id_b), Some(Msg::B));
        assert_eq!(ui.hit(Point::new(3.0, 3.0)), None);
    }

    #[test]
    fn hover_changes_painted_color() {
        // Sans survol : le premier rectangle est rouge.
        let base = build_ui(&sample(), Size::new(400.0, 100.0), &InputState::default());
        assert_eq!(
            base.scene().primitives()[0],
            Primitive::Rect {
                rect: Rect::new(10.0, 10.0, 120.0, 80.0),
                color: Color::rgb(1.0, 0.0, 0.0),
                radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        );

        // On récupère l'id de A, puis on le marque survolé : il devient vert.
        let id_a = base.hit(Point::new(50.0, 50.0)).unwrap();
        let hovered = InputState {
            hovered: Some(id_a),
            pressed: None,
        };
        let ui = build_ui(&sample(), Size::new(400.0, 100.0), &hovered);
        assert_eq!(
            ui.scene().primitives()[0],
            Primitive::Rect {
                rect: Rect::new(10.0, 10.0, 120.0, 80.0),
                color: Color::rgb(0.0, 1.0, 0.0),
                radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        );
    }
}

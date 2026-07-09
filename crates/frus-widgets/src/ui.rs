//! Le pilote : parcourt l'arbre en portant un contexte (translation + découpe),
//! produit la [`Scene`] et les cartes de hit-test (clic, focus, scroll), et
//! permet de retrouver un widget par identité pour lui router clavier/édition.

use frus_core::{Point, Rect, Scene, Size};
use frus_layout::{Layout, NodeId};

use crate::interaction::WidgetId;
use crate::runtime::Runtime;
use crate::widget::Widget;

struct Hit<Msg> {
    id: WidgetId,
    rect: Rect,
    msg: Msg,
}

/// Résultat de la construction d'une interface pour une frame donnée.
pub struct Ui<Msg> {
    scene: Scene,
    hits: Vec<Hit<Msg>>,
    focusables: Vec<(WidgetId, Rect)>,
    scrollables: Vec<(WidgetId, Rect, f32)>,
}

impl<Msg: Clone> Ui<Msg> {
    /// La scène à envoyer au renderer.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Identité du widget cliquable le plus au-dessus contenant `point`.
    pub fn hit(&self, point: Point) -> Option<WidgetId> {
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.rect.contains(point))
            .map(|hit| hit.id)
    }

    /// Message associé à un widget cliquable donné.
    pub fn msg_for(&self, id: WidgetId) -> Option<Msg> {
        self.hits
            .iter()
            .find(|hit| hit.id == id)
            .map(|hit| hit.msg.clone())
    }

    /// Widget focalisable le plus au-dessus contenant `point` : (id, ses bornes).
    pub fn focus_hit(&self, point: Point) -> Option<(WidgetId, Rect)> {
        self.focusables
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(point))
            .map(|(id, rect)| (*id, *rect))
    }

    /// Zone défilable la plus au-dessus contenant `point` : (id, offset max).
    pub fn scroll_hit(&self, point: Point) -> Option<(WidgetId, f32)> {
        self.scrollables
            .iter()
            .rev()
            .find(|(_, rect, _)| rect.contains(point))
            .map(|(id, _, max)| (*id, *max))
    }
}

/// Construit l'arbre de layout principal (un défilable est une **feuille**).
fn build_layout<Msg>(widget: &dyn Widget<Msg>, layout: &mut Layout<()>) -> NodeId {
    if widget.scroll_content().is_some() {
        return layout.leaf(widget.style(), ());
    }
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

struct Builder<'a, Msg> {
    scene: Scene,
    hits: Vec<Hit<Msg>>,
    focusables: Vec<(WidgetId, Rect)>,
    scrollables: Vec<(WidgetId, Rect, f32)>,
    runtime: &'a Runtime,
}

impl<Msg: Clone> Builder<'_, Msg> {
    fn walk(
        &mut self,
        widget: &dyn Widget<Msg>,
        id: WidgetId,
        translation: (f32, f32),
        clip: Rect,
        rects: &[Rect],
        index: &mut usize,
    ) {
        let rect = rects[*index];
        *index += 1;
        let draw_rect = rect.translate(translation.0, translation.1);

        // Statut : interaction pointeur + focus, plus curseur/sélection éventuels.
        let mut status = self.runtime.input.status_for(id);
        if status.focused {
            if let Some(edit) = self.runtime.edits.get(&id) {
                status.cursor = Some(edit.cursor);
                status.selection = edit.selection_range();
            }
        }

        self.scene.set_clip(clip);
        widget.paint(draw_rect, status, &mut self.scene);

        let visible = draw_rect.intersect(clip);
        if visible.width > 0.0 && visible.height > 0.0 {
            if let Some(msg) = widget.on_click() {
                self.hits.push(Hit {
                    id,
                    rect: visible,
                    msg,
                });
            }
            if widget.focusable() {
                self.focusables.push((id, visible));
            }
        }

        if let Some(content) = widget.scroll_content() {
            let viewport = draw_rect;
            let content_clip = clip.intersect(viewport);
            let offset = self.runtime.scroll.get(&id).copied().unwrap_or(0.0);

            let mut layout: Layout<()> = Layout::new();
            let content_root = build_layout(content, &mut layout);
            layout.compute_unbounded_height(content_root, viewport.width);
            let content_rects: Vec<Rect> = layout
                .absolute_rects(content_root)
                .into_iter()
                .map(|(rect, _)| rect)
                .collect();

            let content_height = content_rects.first().map(|r| r.height).unwrap_or(0.0);
            let max_offset = (content_height - viewport.height).max(0.0);
            self.scrollables.push((id, viewport, max_offset));

            let content_translation = (viewport.x, viewport.y - offset);
            let mut content_index = 0;
            self.walk(
                content,
                id.child(0),
                content_translation,
                content_clip,
                &content_rects,
                &mut content_index,
            );
        } else {
            for (child_index, child) in widget.children().iter().enumerate() {
                self.walk(
                    child.as_ref(),
                    id.child(child_index),
                    translation,
                    clip,
                    rects,
                    index,
                );
            }
        }
    }
}

/// Traduit un arbre de widgets en [`Ui`] pour une taille et un état runtime donnés.
pub fn build_ui<Msg: Clone>(root: &dyn Widget<Msg>, available: Size, runtime: &Runtime) -> Ui<Msg> {
    let mut layout: Layout<()> = Layout::new();
    let root_node = build_layout(root, &mut layout);
    layout.compute(root_node, available);
    let rects: Vec<Rect> = layout
        .absolute_rects(root_node)
        .into_iter()
        .map(|(rect, _)| rect)
        .collect();

    let mut builder = Builder {
        scene: Scene::new(),
        hits: Vec::new(),
        focusables: Vec::new(),
        scrollables: Vec::new(),
        runtime,
    };
    let mut index = 0;
    builder.walk(root, WidgetId::ROOT, (0.0, 0.0), Rect::UNBOUNDED, &rects, &mut index);

    Ui {
        scene: builder.scene,
        hits: builder.hits,
        focusables: builder.focusables,
        scrollables: builder.scrollables,
    }
}

/// Retrouve le widget d'identité `target` dans l'arbre (identités positionnelles).
pub fn find_widget<Msg>(
    root: &dyn Widget<Msg>,
    target: WidgetId,
) -> Option<&dyn Widget<Msg>> {
    fn walk<Msg>(
        widget: &dyn Widget<Msg>,
        id: WidgetId,
        target: WidgetId,
    ) -> Option<&dyn Widget<Msg>> {
        if id == target {
            return Some(widget);
        }
        for (index, child) in widget.children().iter().enumerate() {
            if let Some(found) = walk(child.as_ref(), id.child(index), target) {
                return Some(found);
            }
        }
        None
    }
    walk(root, WidgetId::ROOT, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Edit;
    use crate::{Container, Flex, Key, Scroll, TextInput};
    use frus_core::{Color, Point, Primitive, Rect, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        A,
        B,
        Edited(String),
    }

    fn clickable_sample() -> Flex<Msg> {
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
        let rt = Runtime::default();
        let ui = build_ui(&clickable_sample(), Size::new(400.0, 100.0), &rt);
        let id_a = ui.hit(Point::new(50.0, 50.0)).expect("A");
        let id_b = ui.hit(Point::new(300.0, 50.0)).expect("B");
        assert_ne!(id_a, id_b);
        assert_eq!(ui.msg_for(id_a), Some(Msg::A));
        assert_eq!(ui.msg_for(id_b), Some(Msg::B));
        assert_eq!(ui.hit(Point::new(3.0, 3.0)), None);
    }

    #[test]
    fn hover_changes_painted_color() {
        let rt = Runtime::default();
        let base = build_ui(&clickable_sample(), Size::new(400.0, 100.0), &rt);
        let id_a = base.hit(Point::new(50.0, 50.0)).unwrap();

        let mut rt = Runtime::default();
        rt.input.hovered = Some(id_a);
        let ui = build_ui(&clickable_sample(), Size::new(400.0, 100.0), &rt);
        if let Primitive::Rect { color, .. } = ui.scene().primitives()[0] {
            assert_eq!(color, Color::rgb(0.0, 1.0, 0.0));
        } else {
            panic!("attendu un rectangle");
        }
    }

    #[test]
    fn find_widget_and_edit_types() {
        let tree = Flex::column()
            .width(300.0)
            .height(80.0)
            .child(TextInput::new("hi").width(200.0).on_input(Msg::Edited));
        let rt = Runtime::default();
        let ui = build_ui(&tree, Size::new(300.0, 80.0), &rt);
        let (id, _rect) = ui.focus_hit(Point::new(10.0, 10.0)).expect("champ");

        let widget = find_widget(&tree, id).expect("widget trouvé");
        let mut edit = Edit { cursor: 2, anchor: None };
        assert_eq!(
            widget.on_edit(&mut edit, &Key::Text("!".to_string())),
            Some(Msg::Edited("hi!".to_string()))
        );
    }

    #[test]
    fn scroll_translates_and_clips_content() {
        let content = Flex::<Msg>::column()
            .gap(0.0)
            .child(Container::new().height(60.0).color(Color::rgb(1.0, 0.0, 0.0)))
            .child(Container::new().height(60.0).color(Color::rgb(0.0, 1.0, 0.0)))
            .child(Container::new().height(60.0).color(Color::rgb(0.0, 0.0, 1.0)));
        let tree = Scroll::new().width(200.0).height(100.0).child(content);

        let rt = Runtime::default();
        let ui = build_ui(&tree, Size::new(200.0, 100.0), &rt);
        let (sid, _viewport, max) = ui.scrollables[0];
        assert_eq!(max, 80.0);
        assert_eq!(ui.first_rect().0.y, 0.0);
        assert_eq!(ui.first_rect().1, Rect::new(0.0, 0.0, 200.0, 100.0));

        let mut rt = Runtime::default();
        rt.scroll.insert(sid, 50.0);
        let ui2 = build_ui(&tree, Size::new(200.0, 100.0), &rt);
        assert_eq!(ui2.first_rect().0.y, -50.0);
    }

    impl<Msg: Clone> Ui<Msg> {
        fn first_rect(&self) -> (Rect, Rect) {
            for primitive in self.scene.primitives() {
                if let Primitive::Rect { rect, clip, .. } = primitive {
                    return (*rect, *clip);
                }
            }
            panic!("aucun rectangle");
        }
    }
}

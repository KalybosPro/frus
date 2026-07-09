//! Le pilote : parcourt l'arbre de widgets en portant un contexte
//! (translation + découpe), produit la [`Scene`] et les cartes de hit-test
//! (clic, focus, scroll), et route les touches vers le widget focalisé.

use std::collections::HashMap;

use frus_core::{Point, Rect, Scene, Size};
use frus_layout::{Layout, NodeId};

use crate::interaction::{InputState, Key, WidgetId};
use crate::widget::Widget;

/// Offsets de défilement retenus au runtime, par widget défilable.
pub type ScrollState = HashMap<WidgetId, f32>;

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
    /// (id, viewport, offset max) pour chaque zone défilable.
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

    /// Identité du widget focalisable le plus au-dessus contenant `point`.
    pub fn focus_hit(&self, point: Point) -> Option<WidgetId> {
        self.focusables
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(point))
            .map(|(id, _)| *id)
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

/// Construit l'arbre de layout principal. Un widget défilable est une **feuille**
/// (son contenu est mis en page séparément par le pilote).
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

/// État mutable accumulé pendant le parcours.
struct Builder<'a, Msg> {
    scene: Scene,
    hits: Vec<Hit<Msg>>,
    focusables: Vec<(WidgetId, Rect)>,
    scrollables: Vec<(WidgetId, Rect, f32)>,
    input: &'a InputState,
    scroll: &'a ScrollState,
}

impl<Msg: Clone> Builder<'_, Msg> {
    /// Parcourt un widget et ses descendants. `translation` est le décalage à
    /// ajouter aux rectangles (issus de `rects`) pour obtenir les coordonnées
    /// écran ; `clip` est la découpe courante.
    #[allow(clippy::too_many_arguments)]
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

        self.scene.set_clip(clip);
        widget.paint(draw_rect, self.input.status_for(id), &mut self.scene);

        // Zone visible (dans la découpe) : sert au hit-test.
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
            let offset = self.scroll.get(&id).copied().unwrap_or(0.0);

            // Mise en page du contenu à hauteur libre.
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

            // Le contenu est placé à l'origine du viewport, décalé du scroll.
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

/// Traduit un arbre de widgets en [`Ui`] pour une taille et un état donnés.
pub fn build_ui<Msg: Clone>(
    root: &dyn Widget<Msg>,
    available: Size,
    input: &InputState,
    scroll: &ScrollState,
) -> Ui<Msg> {
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
        input,
        scroll,
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

/// Route une touche vers le widget d'identité `focused` et renvoie le message
/// éventuel. Parcourt l'arbre en recalculant les identités positionnelles.
pub fn dispatch_key<Msg>(root: &dyn Widget<Msg>, focused: WidgetId, key: &Key) -> Option<Msg> {
    fn walk<Msg>(
        widget: &dyn Widget<Msg>,
        id: WidgetId,
        target: WidgetId,
        key: &Key,
    ) -> Option<Msg> {
        if id == target {
            if let Some(msg) = widget.on_key(key) {
                return Some(msg);
            }
        }
        for (index, child) in widget.children().iter().enumerate() {
            if let Some(msg) = walk(child.as_ref(), id.child(index), target, key) {
                return Some(msg);
            }
        }
        None
    }
    walk(root, WidgetId::ROOT, focused, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex, Scroll, TextInput};
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
        let ui = build_ui(
            &clickable_sample(),
            Size::new(400.0, 100.0),
            &InputState::default(),
            &ScrollState::new(),
        );
        let id_a = ui.hit(Point::new(50.0, 50.0)).expect("A");
        let id_b = ui.hit(Point::new(300.0, 50.0)).expect("B");
        assert_ne!(id_a, id_b);
        assert_eq!(ui.msg_for(id_a), Some(Msg::A));
        assert_eq!(ui.msg_for(id_b), Some(Msg::B));
        assert_eq!(ui.hit(Point::new(3.0, 3.0)), None);
    }

    #[test]
    fn hover_changes_painted_color() {
        let base = build_ui(
            &clickable_sample(),
            Size::new(400.0, 100.0),
            &InputState::default(),
            &ScrollState::new(),
        );
        assert_eq!(
            base.scene().primitives()[0],
            Primitive::Rect {
                rect: Rect::new(10.0, 10.0, 120.0, 80.0),
                color: Color::rgb(1.0, 0.0, 0.0),
                radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                clip: Rect::UNBOUNDED,
            }
        );

        let id_a = base.hit(Point::new(50.0, 50.0)).unwrap();
        let hovered = InputState {
            hovered: Some(id_a),
            ..Default::default()
        };
        let ui = build_ui(&clickable_sample(), Size::new(400.0, 100.0), &hovered, &ScrollState::new());
        if let Primitive::Rect { color, .. } = ui.scene().primitives()[0] {
            assert_eq!(color, Color::rgb(0.0, 1.0, 0.0));
        } else {
            panic!("attendu un rectangle");
        }
    }

    #[test]
    fn focus_hit_and_dispatch_types() {
        let tree = Flex::column()
            .width(300.0)
            .height(80.0)
            .child(TextInput::new("hi").width(200.0).on_input(Msg::Edited));
        let ui = build_ui(&tree, Size::new(300.0, 80.0), &InputState::default(), &ScrollState::new());
        let id = ui.focus_hit(Point::new(10.0, 10.0)).expect("champ");
        assert_eq!(
            dispatch_key(&tree, id, &Key::Text("!".to_string())),
            Some(Msg::Edited("hi!".to_string()))
        );
    }

    #[test]
    fn scroll_translates_and_clips_content() {
        // Viewport 100 de haut ; contenu = 3 cartes de 60 → hauteur 180.
        let content = Flex::<Msg>::column()
            .gap(0.0)
            .child(Container::new().height(60.0).color(Color::rgb(1.0, 0.0, 0.0)))
            .child(Container::new().height(60.0).color(Color::rgb(0.0, 1.0, 0.0)))
            .child(Container::new().height(60.0).color(Color::rgb(0.0, 0.0, 1.0)));
        let tree = Scroll::new().width(200.0).height(100.0).child(content);

        // Sans offset : première carte à y = 0, clip = viewport.
        let ui = build_ui(&tree, Size::new(200.0, 100.0), &InputState::default(), &ScrollState::new());
        let (_id, _viewport, max) = ui.scrollables_first();
        assert_eq!(max, 80.0); // 180 - 100
        let first = ui.first_rect_primitive();
        assert_eq!(first.0.y, 0.0);
        assert_eq!(first.1, Rect::new(0.0, 0.0, 200.0, 100.0)); // clip = viewport

        // Avec offset 50 : la première carte remonte à y = -50.
        let sid = ui.scroll_hit(Point::new(10.0, 10.0)).unwrap().0;
        let mut scroll = ScrollState::new();
        scroll.insert(sid, 50.0);
        let ui2 = build_ui(&tree, Size::new(200.0, 100.0), &InputState::default(), &scroll);
        assert_eq!(ui2.first_rect_primitive().0.y, -50.0);
    }

    // Accès de test aux internes.
    impl<Msg: Clone> Ui<Msg> {
        fn scrollables_first(&self) -> (WidgetId, Rect, f32) {
            self.scrollables[0]
        }
        fn first_rect_primitive(&self) -> (Rect, Rect) {
            for primitive in self.scene.primitives() {
                if let Primitive::Rect { rect, clip, .. } = primitive {
                    return (*rect, *clip);
                }
            }
            panic!("aucun rectangle");
        }
    }
}

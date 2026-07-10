//! Le pilote : parcourt l'arbre en portant un contexte (translation + découpe),
//! produit la [`Scene`] et les cartes de hit-test (clic, focus, scroll), et
//! permet de retrouver un widget par identité pour lui router clavier/édition.

use frus_core::{Color, Point, Rect, Scene, Size};
use frus_layout::{Layout, NodeId};

use crate::interaction::WidgetId;
use crate::runtime::Runtime;
use crate::widget::Widget;

/// Épaisseur d'une barre de défilement, en pixels.
const BAR_SIZE: f32 = 10.0;
/// Longueur minimale d'une poignée.
const MIN_THUMB: f32 = 28.0;
const TRACK_COLOR: Color = Color::rgba(1.0, 1.0, 1.0, 0.06);
const THUMB_COLOR: Color = Color::rgba(1.0, 1.0, 1.0, 0.28);

/// Une poignée de barre de défilement (pour le hit-test au drag).
#[derive(Copy, Clone, Debug)]
pub struct Scrollbar {
    pub id: WidgetId,
    pub vertical: bool,
    pub thumb: Rect,
    /// Début et longueur de la piste, le long de l'axe.
    pub track_start: f32,
    pub track_len: f32,
    pub thumb_len: f32,
    /// Offset maximal correspondant.
    pub max: f32,
}

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
    /// (id, viewport, offset max x, offset max y)
    scrollables: Vec<(WidgetId, Rect, f32, f32)>,
    scrollbars: Vec<Scrollbar>,
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

    /// Zone défilable la plus au-dessus contenant `point` : (id, max x, max y).
    pub fn scroll_hit(&self, point: Point) -> Option<(WidgetId, f32, f32)> {
        self.scrollables
            .iter()
            .rev()
            .find(|(_, rect, _, _)| rect.contains(point))
            .map(|(id, _, max_x, max_y)| (*id, *max_x, *max_y))
    }

    /// Poignée de barre de défilement sous `point` (pour démarrer un glissement).
    pub fn scrollbar_at(&self, point: Point) -> Option<Scrollbar> {
        self.scrollbars
            .iter()
            .rev()
            .find(|bar| bar.thumb.contains(point))
            .copied()
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
    scrollables: Vec<(WidgetId, Rect, f32, f32)>,
    scrollbars: Vec<Scrollbar>,
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

        // Statut : interaction pointeur + focus + progression d'animation, plus
        // curseur/sélection éventuels.
        let mut status = self.runtime.input.status_for(id);
        status.hover_progress = self.runtime.hover_progress(id);
        status.focus_progress = self.runtime.focus_progress(id);
        status.opacity = self.runtime.opacity(id);
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
            let axis = widget.scroll_axis();
            let viewport = draw_rect;
            let content_clip = clip.intersect(viewport);
            let (offset_x, offset_y) = self.runtime.scroll.get(&id).copied().unwrap_or((0.0, 0.0));

            let mut layout: Layout<()> = Layout::new();
            let content_root = build_layout(content, &mut layout);
            layout.compute_scroll(
                content_root,
                viewport.width,
                viewport.height,
                axis.free_x(),
                axis.free_y(),
            );
            let content_rects: Vec<Rect> = layout
                .absolute_rects(content_root)
                .into_iter()
                .map(|(rect, _)| rect)
                .collect();

            let content_size = content_rects.first().copied().unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
            let max_x = (content_size.width - viewport.width).max(0.0);
            let max_y = (content_size.height - viewport.height).max(0.0);
            self.scrollables.push((id, viewport, max_x, max_y));

            let content_translation = (viewport.x - offset_x, viewport.y - offset_y);
            let mut content_index = 0;
            self.walk(
                content,
                id.child(0),
                content_translation,
                content_clip,
                &content_rects,
                &mut content_index,
            );

            // Barres de défilement, par-dessus le contenu (non découpées par lui).
            self.scene.set_clip(clip);
            if max_y > 0.0 {
                self.add_scrollbar(id, viewport, true, offset_y, max_y);
            }
            if max_x > 0.0 {
                self.add_scrollbar(id, viewport, false, offset_x, max_x);
            }
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

impl<Msg: Clone> Builder<'_, Msg> {
    /// Dessine une barre de défilement (piste + poignée) et l'enregistre pour
    /// le hit-test au glissement.
    fn add_scrollbar(&mut self, id: WidgetId, viewport: Rect, vertical: bool, offset: f32, max: f32) {
        let (track_start, track_len, content_len) = if vertical {
            (viewport.y, viewport.height, viewport.height + max)
        } else {
            (viewport.x, viewport.width, viewport.width + max)
        };
        let thumb_len = (track_len * track_len / content_len)
            .max(MIN_THUMB)
            .min(track_len);
        let travel = track_len - thumb_len;
        let thumb_pos = track_start + if max > 0.0 { offset / max * travel } else { 0.0 };

        let (track, thumb) = if vertical {
            let x = viewport.x + viewport.width - BAR_SIZE;
            (
                Rect::new(x, viewport.y, BAR_SIZE, viewport.height),
                Rect::new(x + 1.0, thumb_pos, BAR_SIZE - 2.0, thumb_len),
            )
        } else {
            let y = viewport.y + viewport.height - BAR_SIZE;
            (
                Rect::new(viewport.x, y, viewport.width, BAR_SIZE),
                Rect::new(thumb_pos, y + 1.0, thumb_len, BAR_SIZE - 2.0),
            )
        };

        self.scene
            .draw_rect(track, TRACK_COLOR, BAR_SIZE * 0.5, 0.0, Color::TRANSPARENT);
        self.scene
            .draw_rect(thumb, THUMB_COLOR, (BAR_SIZE - 2.0) * 0.5, 0.0, Color::TRANSPARENT);
        self.scrollbars.push(Scrollbar {
            id,
            vertical,
            thumb,
            track_start,
            track_len,
            thumb_len,
            max,
        });
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
        scrollbars: Vec::new(),
        runtime,
    };
    let mut index = 0;
    builder.walk(root, WidgetId::ROOT, (0.0, 0.0), Rect::UNBOUNDED, &rects, &mut index);

    Ui {
        scene: builder.scene,
        hits: builder.hits,
        focusables: builder.focusables,
        scrollables: builder.scrollables,
        scrollbars: builder.scrollbars,
    }
}

/// Collecte les identités de tous les widgets de l'arbre (ordre préfixe),
/// selon le même schéma positionnel que [`build_ui`]. Sert à détecter les
/// montages/démontages entre deux frames.
pub fn collect_ids<Msg>(root: &dyn Widget<Msg>) -> Vec<WidgetId> {
    fn walk<Msg>(widget: &dyn Widget<Msg>, id: WidgetId, out: &mut Vec<WidgetId>) {
        out.push(id);
        for (index, child) in widget.children().iter().enumerate() {
            walk(child.as_ref(), id.child(index), out);
        }
    }
    let mut out = Vec::new();
    walk(root, WidgetId::ROOT, &mut out);
    out
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
    fn hover_progress_interpolates_color() {
        let rt = Runtime::default();
        let base = build_ui(&clickable_sample(), Size::new(400.0, 100.0), &rt);
        let id_a = base.hit(Point::new(50.0, 50.0)).unwrap();

        // Sans progression : couleur de base (rouge).
        if let Primitive::Rect { color, .. } = base.scene().primitives()[0] {
            assert_eq!(color, Color::rgb(1.0, 0.0, 0.0));
        } else {
            panic!("attendu un rectangle");
        }

        // Progression pleine : couleur de survol (vert).
        let mut rt = Runtime::default();
        rt.input.hovered = Some(id_a);
        rt.anims.insert(id_a, crate::Anim { hover: 1.0, ..Default::default() });
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
        let (sid, _viewport, max_x, max_y) = ui.scrollables[0];
        assert_eq!(max_y, 80.0); // 180 - 100
        assert_eq!(max_x, 0.0);
        assert_eq!(ui.first_rect().0.y, 0.0);
        assert_eq!(ui.first_rect().1, Rect::new(0.0, 0.0, 200.0, 100.0));

        let mut rt = Runtime::default();
        rt.scroll.insert(sid, (0.0, 50.0));
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

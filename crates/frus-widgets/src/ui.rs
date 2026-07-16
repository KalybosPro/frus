//! Le pilote : parcourt l'arbre en portant un contexte (translation + découpe),
//! produit la [`Scene`] et les cartes de hit-test (clic, focus, scroll), et
//! permet de retrouver un widget par identité pour lui router clavier/édition.

use frus_core::{Color, Point, Rect, Scene, Size};
use frus_layout::{Layout, NodeId};

use crate::interaction::WidgetId;
use crate::portal::Placement;
use crate::relayout::Constraints;
use crate::runtime::Runtime;
use crate::theme::Theme;
use crate::widget::Widget;

/// Facteur de parallaxe de l'écran arrière lors d'une transition (0 = fixe,
/// 1 = suit à l'identique). Donne la profondeur d'une navigation native.
const NAV_PARALLAX: f32 = 0.3;

/// Épaisseur d'une barre de défilement, en pixels.
const BAR_SIZE: f32 = 10.0;
/// Longueur minimale d'une poignée.
const MIN_THUMB: f32 = 28.0;

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

/// Direction de navigation du focus aux flèches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
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
    /// Cibles d'appui long (id, bornes visibles, message).
    long_presses: Vec<Hit<Msg>>,
    focusables: Vec<(WidgetId, Rect)>,
    /// (id, viewport, offset max x, offset max y)
    scrollables: Vec<(WidgetId, Rect, f32, f32)>,
    scrollbars: Vec<Scrollbar>,
    draggables: Vec<(WidgetId, Rect)>,
    wants_animation: bool,
}

impl<Msg: Clone> Ui<Msg> {
    /// La scène à envoyer au renderer.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// `true` si un widget s'anime en continu (le framework doit redessiner).
    pub fn wants_animation(&self) -> bool {
        self.wants_animation
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

    /// Message d'**appui long** de la cible la plus au-dessus contenant `point`.
    pub fn long_press_at(&self, point: Point) -> Option<Msg> {
        self.long_presses
            .iter()
            .rev()
            .find(|hit| hit.rect.contains(point))
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

    /// Focusable le plus proche dans la **direction** donnée (navigation aux
    /// flèches, politique géométrique) : parmi les focusables dont le centre est
    /// du bon côté, minimise la distance sur l'axe principal avec une pénalité
    /// sur l'écart transversal. `None` si rien dans cette direction.
    pub fn focus_directional(
        &self,
        current: WidgetId,
        direction: FocusDirection,
    ) -> Option<WidgetId> {
        let from = self
            .focusables
            .iter()
            .find(|(id, _)| *id == current)
            .map(|(_, rect)| rect)?;
        let center = |r: &Rect| (r.x + r.width * 0.5, r.y + r.height * 0.5);
        let (fx, fy) = center(from);

        let mut best: Option<(WidgetId, f32)> = None;
        for (id, rect) in &self.focusables {
            if *id == current {
                continue;
            }
            let (cx, cy) = center(rect);
            // (avance dans la direction, écart transversal)
            let (ahead, cross) = match direction {
                FocusDirection::Right => (cx - fx, (cy - fy).abs()),
                FocusDirection::Left => (fx - cx, (cy - fy).abs()),
                FocusDirection::Down => (cy - fy, (cx - fx).abs()),
                FocusDirection::Up => (fy - cy, (cx - fx).abs()),
            };
            // Dans un **cône** autour de la direction (pas un simple demi-plan) :
            // un candidat quasi aligné transversalement mais à peine « devant »
            // (largeurs légèrement différentes) n'est pas une cible directionnelle.
            if ahead <= 0.5 || cross > ahead * 3.0 {
                continue;
            }
            let score = ahead + cross * 3.0;
            if best.map(|(_, s)| score < s).unwrap_or(true) {
                best = Some((*id, score));
            }
        }
        best.map(|(id, _)| id)
    }

    /// Focusable suivant/précédent dans l'ordre d'arbre (avec bouclage), pour la
    /// navigation Tab. Sans focus courant, renvoie le premier (ou dernier).
    pub fn focus_next(&self, current: Option<WidgetId>, forward: bool) -> Option<WidgetId> {
        if self.focusables.is_empty() {
            return None;
        }
        let n = self.focusables.len();
        match current.and_then(|c| self.focusables.iter().position(|(id, _)| *id == c)) {
            Some(i) => {
                let j = if forward { (i + 1) % n } else { (i + n - 1) % n };
                Some(self.focusables[j].0)
            }
            None => Some(self.focusables[if forward { 0 } else { n - 1 }].0),
        }
    }

    /// Zone défilable la plus au-dessus contenant `point` : (id, max x, max y).
    pub fn scroll_hit(&self, point: Point) -> Option<(WidgetId, f32, f32)> {
        self.scrollables
            .iter()
            .rev()
            .find(|(_, rect, _, _)| rect.contains(point))
            .map(|(id, _, max_x, max_y)| (*id, *max_x, *max_y))
    }

    /// Bornes de défilement `(id, max_x, max_y)` de chaque zone défilable, pour
    /// piloter l'inertie côté framework.
    pub fn scrollable_maxes(&self) -> Vec<(WidgetId, f32, f32)> {
        self.scrollables
            .iter()
            .map(|(id, _, max_x, max_y)| (*id, *max_x, *max_y))
            .collect()
    }

    /// Poignée de barre de défilement sous `point` (pour démarrer un glissement).
    pub fn scrollbar_at(&self, point: Point) -> Option<Scrollbar> {
        self.scrollbars
            .iter()
            .rev()
            .find(|bar| bar.thumb.contains(point))
            .copied()
    }

    /// Widget glissable le plus au-dessus sous `point` : (id, ses bornes).
    pub fn draggable_at(&self, point: Point) -> Option<(WidgetId, Rect)> {
        self.draggables
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(point))
            .map(|(id, rect)| (*id, *rect))
    }
}

/// Identité du `index`-ième enfant : **par clé** si l'enfant en déclare une
/// (stable quel que soit sa position), sinon **positionnelle**. Doit être utilisée
/// partout où l'on dérive une identité d'enfant (rendu, collecte, recherche,
/// animations) pour rester cohérent.
pub(crate) fn child_id<Msg>(parent: WidgetId, index: usize, child: &dyn Widget<Msg>) -> WidgetId {
    match child.key() {
        Some(key) => parent.keyed(key),
        None => parent.child(index),
    }
}

/// Construit l'arbre de layout principal (un défilable est une **feuille**).
pub(crate) fn build_layout<Msg>(widget: &dyn Widget<Msg>, layout: &mut Layout<()>) -> NodeId {
    // Défilables, navigateurs, listes virtualisées et piles : contenu mis en page
    // à part (couches / écrans / éléments indépendants).
    if widget.scroll_content().is_some()
        || widget.navigator().is_some()
        || widget.virtual_list().is_some()
        || widget.layout_builder().is_some()
        || widget.stack()
    {
        return layout.leaf(widget.style(), ());
    }
    // Un portail ne met en page que son ancre (enfant 0) ; l'overlay est différé.
    if widget.overlay().is_some() {
        let anchor = build_layout(widget.children()[0].as_ref(), layout);
        return layout.container(widget.style(), &[anchor]);
    }
    let children = widget.children();
    if children.is_empty() {
        // Feuille à mesure sous contraintes (paragraphe qui se replie…) : taffy
        // interroge la closure pendant le calcul.
        if let Some(measure) = widget.measure() {
            return layout.measured_leaf(widget.style(), (), measure);
        }
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
    long_presses: Vec<Hit<Msg>>,
    focusables: Vec<(WidgetId, Rect)>,
    scrollables: Vec<(WidgetId, Rect, f32, f32)>,
    scrollbars: Vec<Scrollbar>,
    draggables: Vec<(WidgetId, Rect)>,
    /// Overlays différés : (contenu, id, bornes de l'ancre, placement, fermeture,
    /// progression `0..=1`). La progression anime l'apparition (tiroir qui glisse,
    /// voile qui se fond) ; elle vaut `1.0` pour les overlays non animés.
    overlays: Vec<(&'a dyn Widget<Msg>, WidgetId, Rect, Placement, Option<Msg>, f32)>,
    /// Un widget demande une animation continue (pilotée par le temps).
    wants_animation: bool,
    available: Size,
    runtime: &'a Runtime,
    theme: &'a Theme,
}

impl<'a, Msg: Clone> Builder<'a, Msg> {
    /// Rectangles d'une racine de layout, via le cache de relayout retenu dans le
    /// runtime (recalcule via taffy seulement si style/structure/contraintes ont
    /// changé). Emprunt mutable bref : la `Vec` renvoyée est possédée.
    fn cached_rects(&self, key: WidgetId, root: &dyn Widget<Msg>, c: Constraints) -> Vec<Rect> {
        self.runtime.layout_cache.borrow_mut().rects(key, root, c)
    }

    fn walk(
        &mut self,
        widget: &'a dyn Widget<Msg>,
        id: WidgetId,
        translation: (f32, f32),
        clip: Rect,
        rects: &[Rect],
        index: &mut usize,
    ) {
        let rect = rects[*index];
        *index += 1;
        let draw_rect = rect.translate(translation.0, translation.1);

        let status = self.full_status(id);
        if widget.continuous() {
            self.wants_animation = true;
        }

        self.scene.set_clip(clip);
        self.scene.set_owner(id.as_u64());
        widget.paint(draw_rect, status, self.theme, &mut self.scene);
        // Un widget a pu resserrer la découpe (ex. TextInput) : on la restaure.
        self.scene.set_clip(clip);
        self.draw_focus_ring(draw_rect, &status, widget);

        let visible = draw_rect.intersect(clip);
        if visible.width > 0.0 && visible.height > 0.0 {
            if let Some(msg) = widget.on_click() {
                self.hits.push(Hit {
                    id,
                    rect: visible,
                    msg,
                });
            }
            if let Some(msg) = widget.on_long_press() {
                self.long_presses.push(Hit { id, rect: visible, msg });
            }
            if widget.focusable() {
                self.focusables.push((id, visible));
            }
            if widget.draggable() {
                self.draggables.push((id, visible));
            }
        }

        if let Some((progress, forward)) = widget.navigator() {
            let bounds = draw_rect;
            let children = widget.children();
            let w = bounds.width;
            if children.len() >= 2 {
                // Transition : deux écrans décalés. L'écran « arrière » (offset
                // négatif) se déplace moins (parallaxe) → sensation de profondeur.
                let dir = if forward { 1.0 } else { -1.0 };
                let raw = [-progress * w * dir, (1.0 - progress) * w * dir];
                let off = [
                    if raw[0] < 0.0 { raw[0] * NAV_PARALLAX } else { raw[0] },
                    if raw[1] < 0.0 { raw[1] * NAV_PARALLAX } else { raw[1] },
                ];
                // Ordre de profondeur : le plus décalé à gauche (arrière) d'abord.
                let (back, front) = if off[0] <= off[1] { (0, 1) } else { (1, 0) };
                self.render_screen(children[back].as_ref(), child_id(id, back, children[back].as_ref()), bounds, off[back], clip);
                // Assombrit l'écran arrière proportionnellement à son recouvrement.
                let coverage = (off[back].abs() / (w * NAV_PARALLAX)).min(1.0);
                if coverage > 0.0 {
                    let scrim = Rect::new(bounds.x + off[back], bounds.y, bounds.width, bounds.height);
                    self.scene.set_owner(0);
                    self.scene.set_clip(clip);
                    self.scene
                        .fill_rect(scrim, self.theme.scheme.scrim.with_alpha(0.22 * coverage));
                }
                self.render_screen(children[front].as_ref(), child_id(id, front, children[front].as_ref()), bounds, off[front], clip);
            } else if let Some(screen) = children.first() {
                self.render_screen(screen.as_ref(), child_id(id, 0, screen.as_ref()), bounds, 0.0, clip);
            }
        } else if let Some(content) = widget.scroll_content() {
            let axis = widget.scroll_axis();
            let viewport = draw_rect;
            let content_clip = clip.intersect(viewport);
            let (offset_x, offset_y) = self.runtime.scroll.get(&id).copied().unwrap_or((0.0, 0.0));

            let content_rects = self.cached_rects(
                child_id(id, 0, content),
                content,
                Constraints::scroll(viewport.width, viewport.height, axis.free_x(), axis.free_y()),
            );

            let content_size = content_rects.first().copied().unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
            let max_x = (content_size.width - viewport.width).max(0.0);
            let max_y = (content_size.height - viewport.height).max(0.0);
            self.scrollables.push((id, viewport, max_x, max_y));

            let content_translation = (viewport.x - offset_x, viewport.y - offset_y);
            let mut content_index = 0;
            self.walk(
                content,
                child_id(id, 0, content),
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
        } else if let Some(vlist) = widget.virtual_list() {
            // Liste virtualisée : ne construire/poser/peindre que la fenêtre visible.
            let viewport = draw_rect;
            let content_clip = clip.intersect(viewport);
            let (_, offset_y) = self.runtime.scroll.get(&id).copied().unwrap_or((0.0, 0.0));
            let content_h = vlist.count as f32 * vlist.item_height;
            let max_y = (content_h - viewport.height).max(0.0);
            self.scrollables.push((id, viewport, 0.0, max_y));

            if vlist.item_height > 0.0 && vlist.count > 0 {
                let first = (offset_y / vlist.item_height).floor().max(0.0) as usize;
                let last = (((offset_y + viewport.height) / vlist.item_height).ceil() as usize)
                    .min(vlist.count);
                for i in first..last {
                    let item = (vlist.build)(i);
                    let top = viewport.y + i as f32 * vlist.item_height - offset_y;

                    let item_rects = self.cached_rects(
                        id.child(i),
                        item.as_ref(),
                        Constraints::definite(Size::new(viewport.width, vlist.item_height)),
                    );

                    let mut item_index = 0;
                    self.render_item(
                        item.as_ref(),
                        id.child(i),
                        (viewport.x, top),
                        content_clip,
                        &item_rects,
                        &mut item_index,
                    );
                }
            }

            self.scene.set_clip(clip);
            if max_y > 0.0 {
                self.add_scrollbar(id, viewport, true, offset_y, max_y);
            }
        } else if let Some(build) = widget.layout_builder() {
            // Construit le contenu à partir de la boîte réelle, puis le met en page
            // et le rend à l'intérieur (comme un élément de liste : sans état retenu).
            let bounds = draw_rect;
            let content_clip = clip.intersect(bounds);
            let child = build(Size::new(bounds.width, bounds.height));

            let child_rects = self.cached_rects(
                id.child(0),
                child.as_ref(),
                Constraints::definite(Size::new(bounds.width, bounds.height)),
            );

            let mut child_index = 0;
            self.render_item(
                child.as_ref(),
                id.child(0),
                (bounds.x, bounds.y),
                content_clip,
                &child_rects,
                &mut child_index,
            );
        } else if widget.stack() {
            // Pile : chaque couche remplit la boîte, rendue dans l'ordre.
            let bounds = draw_rect;
            let layer_clip = clip.intersect(bounds);
            for (i, layer) in widget.children().iter().enumerate() {
                let layer_rects = self.cached_rects(
                    child_id(id, i, layer.as_ref()),
                    layer.as_ref(),
                    Constraints::definite(Size::new(bounds.width, bounds.height)),
                );
                let mut layer_index = 0;
                self.walk(
                    layer.as_ref(),
                    child_id(id, i, layer.as_ref()),
                    (bounds.x, bounds.y),
                    layer_clip,
                    &layer_rects,
                    &mut layer_index,
                );
            }
        } else if let Some((content, placement)) = widget.overlay() {
            // Ancre (enfant 0) rendue inline ; overlay (enfant 1) différé.
            self.walk(
                widget.children()[0].as_ref(),
                child_id(id, 0, widget.children()[0].as_ref()),
                translation,
                clip,
                rects,
                index,
            );
            // Progression d'apparition : pour un overlay animé (tiroir), la valeur
            // interpolée par le runtime ; sinon `1.0` (affiché d'emblée). Sans
            // valeur enregistrée (rendu isolé), on adopte la cible immédiatement.
            let target = widget.anim_target().unwrap_or(1.0);
            let progress = self.runtime.value_or(id, target);
            // Un tooltip ne s'affiche que si l'ancre est survolée ; un overlay animé
            // disparaît une fois sa progression retombée à zéro.
            let visible = match placement {
                Placement::Tooltip => self.runtime.input.hovered == Some(id.child(0)),
                _ => true,
            };
            if visible && progress > 0.001 {
                self.overlays.push((
                    content,
                    child_id(id, 1, content),
                    draw_rect,
                    placement,
                    widget.overlay_dismiss(),
                    progress,
                ));
            }
        } else {
            for (child_index, child) in widget.children().iter().enumerate() {
                self.walk(
                    child.as_ref(),
                    child_id(id, child_index, child.as_ref()),
                    translation,
                    clip,
                    rects,
                    index,
                );
            }
        }
    }

    /// Statut complet d'un widget : interaction pointeur + focus + progressions
    /// d'animation + curseur/sélection éventuels.
    fn full_status(&self, id: WidgetId) -> crate::interaction::Status {
        let mut status = self.runtime.input.status_for(id);
        status.hover_progress = self.runtime.hover_progress(id);
        status.focus_progress = self.runtime.focus_progress(id);
        status.opacity = self.runtime.opacity(id);
        status.value = self.runtime.value(id);
        status.time = self.runtime.time;
        if status.focused {
            if let Some(edit) = self.runtime.edits.get(&id) {
                status.cursor = Some(edit.cursor);
                status.selection = edit.selection_range();
            }
        }
        status
    }

    /// Anneau de focus générique (widgets qui ne gèrent pas le leur).
    fn draw_focus_ring(
        &mut self,
        draw_rect: Rect,
        status: &crate::interaction::Status,
        widget: &dyn Widget<Msg>,
    ) {
        // L'anneau générique n'apparaît que si la dernière interaction était
        // **clavier** (`focus_visible`) — un clic ne fait pas flasher d'anneau.
        if status.focused
            && self.runtime.focus_visible
            && widget.focusable()
            && !widget.draws_own_focus()
        {
            let ring = Rect::new(
                draw_rect.x - 2.0,
                draw_rect.y - 2.0,
                draw_rect.width + 4.0,
                draw_rect.height + 4.0,
            );
            let alpha = 0.4 + 0.6 * status.focus_progress.clamp(0.0, 1.0);
            self.scene.draw_rect(
                ring,
                Color::TRANSPARENT,
                self.theme.radius + 2.0,
                2.0,
                self.theme.focus.fade(alpha),
            );
        }
    }

    /// Rend un **élément de liste virtualisée** : construit à la volée, il ne peut
    /// pas différer d'overlay (d'où un rendu propre, sans les branches spéciales).
    fn render_item(
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

        let status = self.full_status(id);
        if widget.continuous() {
            self.wants_animation = true;
        }
        self.scene.set_clip(clip);
        self.scene.set_owner(id.as_u64());
        widget.paint(draw_rect, status, self.theme, &mut self.scene);
        self.scene.set_clip(clip);
        self.draw_focus_ring(draw_rect, &status, widget);

        let visible = draw_rect.intersect(clip);
        if visible.width > 0.0 && visible.height > 0.0 {
            if let Some(msg) = widget.on_click() {
                self.hits.push(Hit { id, rect: visible, msg });
            }
            if let Some(msg) = widget.on_long_press() {
                self.long_presses.push(Hit { id, rect: visible, msg });
            }
            if widget.focusable() {
                self.focusables.push((id, visible));
            }
            if widget.draggable() {
                self.draggables.push((id, visible));
            }
        }

        for (child_index, child) in widget.children().iter().enumerate() {
            self.render_item(
                child.as_ref(),
                child_id(id, child_index, child.as_ref()),
                translation,
                clip,
                rects,
                index,
            );
        }
    }

    /// Met en page un écran plein-fenêtre et le rend décalé de `off_x`.
    fn render_screen(
        &mut self,
        screen: &'a dyn Widget<Msg>,
        id: WidgetId,
        bounds: Rect,
        off_x: f32,
        clip: Rect,
    ) {
        let rects = self.cached_rects(
            id,
            screen,
            Constraints::definite(Size::new(bounds.width, bounds.height)),
        );
        let screen_clip = clip.intersect(bounds);
        let mut index = 0;
        self.walk(
            screen,
            id,
            (bounds.x + off_x, bounds.y),
            screen_clip,
            &rects,
            &mut index,
        );
    }

    /// Traite les overlays différés : sous-layout, positionnement et rendu
    /// **au-dessus** de tout (leurs zones cliquables priment). Peut engendrer
    /// d'autres overlays (portails imbriqués).
    fn process_overlays(&mut self) {
        let window = Rect::new(0.0, 0.0, self.available.width, self.available.height);
        while let Some((content, oid, anchor, placement, dismiss, progress)) = self.overlays.pop() {
            // Les tiroirs glissent selon une **courbe en ressort** (arrivée douce),
            // pas linéairement ; les autres overlays gardent leur progression brute.
            let progress = if matches!(
                placement,
                Placement::Left | Placement::Right | Placement::Bottom
            ) {
                crate::runtime::spring_ease(progress)
            } else {
                progress
            };
            // Taille naturelle du contenu. Un tiroir (`Left`) est contraint en
            // hauteur à la fenêtre (son panneau `Percent(1.0)` se déploie),
            // largeur libre ; les autres overlays prennent leur taille naturelle.
            let (free_x, free_y) = match placement {
                Placement::Left | Placement::Right => (true, false),
                // La feuille est pleine-largeur (contrainte à la fenêtre), hauteur
                // naturelle : son panneau `Percent(1.0)` en largeur se déploie.
                Placement::Bottom => (false, true),
                _ => (true, true),
            };
            let rects = self.cached_rects(
                oid,
                content,
                Constraints::scroll(self.available.width, self.available.height, free_x, free_y),
            );
            let size = rects.first().copied().unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));

            let mut pos = match placement {
                Placement::Below => (anchor.x, anchor.y + anchor.height + 4.0),
                Placement::Center => (
                    (self.available.width - size.width) * 0.5,
                    (self.available.height - size.height) * 0.5,
                ),
                Placement::Tooltip => (anchor.x, anchor.y - size.height - 6.0),
                // Le tiroir glisse depuis la gauche : décalé de `(1-progress)·largeur`.
                Placement::Left => (-(1.0 - progress) * size.width, 0.0),
                // Idem depuis la droite : le bord droit reste collé à la fenêtre.
                Placement::Right => (self.available.width - progress * size.width, 0.0),
                // La feuille glisse depuis le bas : le bord bas reste collé à la
                // fenêtre, décalée de `(1-progress)·hauteur` vers le bas.
                Placement::Bottom => (0.0, self.available.height - progress * size.height),
            };

            // Auto-flip : si un overlay ancré déborde d'un bord, on le bascule /
            // le recale à l'intérieur de la fenêtre.
            if matches!(placement, Placement::Below | Placement::Tooltip) {
                // Débordement vertical → basculer de l'autre côté de l'ancre.
                if placement == Placement::Below
                    && pos.1 + size.height > self.available.height
                    && anchor.y - size.height - 4.0 >= 0.0
                {
                    pos.1 = anchor.y - size.height - 4.0;
                } else if placement == Placement::Tooltip
                    && pos.1 < 0.0
                    && anchor.y + anchor.height + size.height + 6.0 <= self.available.height
                {
                    pos.1 = anchor.y + anchor.height + 6.0;
                }
                // Débordement horizontal → recaler dans la fenêtre.
                if pos.0 + size.width > self.available.width {
                    pos.0 = (self.available.width - size.width).max(0.0);
                }
                if pos.0 < 0.0 {
                    pos.0 = 0.0;
                }
            }

            if matches!(
                placement,
                Placement::Center | Placement::Left | Placement::Right | Placement::Bottom
            ) {
                // Voile derrière la modale / le tiroir (rôle `scrim`), modulé par
                // la progression (fondu synchronisé avec le glissement).
                self.scene.set_owner(0);
                self.scene.set_clip(window);
                self.scene
                    .fill_rect(window, self.theme.scheme.scrim.with_alpha(0.5 * progress));
            }

            // Fermeture au clic **hors** du contenu (modale, menu…) : un hit plein
            // écran ajouté **avant** le contenu, donc battu par lui au recouvrement.
            if let Some(msg) = dismiss {
                self.hits.push(Hit { id: oid, rect: window, msg });
            }

            let mut index = 0;
            self.walk(content, oid, pos, window, &rects, &mut index);
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

        let track_color = self.theme.muted.fade(0.18);
        let thumb_color = self.theme.muted.fade(0.55);
        self.scene
            .draw_rect(track, track_color, BAR_SIZE * 0.5, 0.0, Color::TRANSPARENT);
        self.scene
            .draw_rect(thumb, thumb_color, (BAR_SIZE - 2.0) * 0.5, 0.0, Color::TRANSPARENT);
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

/// Traduit un arbre de widgets en [`Ui`] pour une taille, un état runtime et un
/// thème donnés.
pub fn build_ui<'a, Msg: Clone>(
    root: &'a dyn Widget<Msg>,
    available: Size,
    runtime: &'a Runtime,
    theme: &'a Theme,
) -> Ui<Msg> {
    let rects = runtime
        .layout_cache
        .borrow_mut()
        .rects(WidgetId::ROOT, root, Constraints::definite(available));

    let mut builder = Builder {
        scene: Scene::new(),
        hits: Vec::new(),
        long_presses: Vec::new(),
        focusables: Vec::new(),
        scrollables: Vec::new(),
        scrollbars: Vec::new(),
        draggables: Vec::new(),
        overlays: Vec::new(),
        wants_animation: false,
        available,
        runtime,
        theme,
    };
    let mut index = 0;
    builder.walk(root, WidgetId::ROOT, (0.0, 0.0), Rect::UNBOUNDED, &rects, &mut index);

    // Overlays (menus flottants, modales, tooltips) par-dessus tout le reste.
    builder.process_overlays();

    // Fin de frame : oublie les racines de layout des widgets disparus et fige
    // les compteurs de diagnostic du cache.
    runtime.layout_cache.borrow_mut().end_frame();

    // Rejoue les sous-arbres sortants, en fondu, par-dessus la scène courante.
    builder.scene.set_clip(Rect::UNBOUNDED);
    for (_, (primitives, opacity)) in &runtime.leaving {
        for primitive in primitives {
            builder.scene.push_faded(primitive, *opacity);
        }
    }

    Ui {
        scene: builder.scene,
        hits: builder.hits,
        long_presses: builder.long_presses,
        focusables: builder.focusables,
        scrollables: builder.scrollables,
        scrollbars: builder.scrollbars,
        draggables: builder.draggables,
        wants_animation: builder.wants_animation,
    }
}

/// Collecte les identités de tous les widgets de l'arbre (ordre préfixe),
/// selon le même schéma positionnel que [`build_ui`]. Sert à détecter les
/// montages/démontages entre deux frames.
pub fn collect_ids<Msg>(root: &dyn Widget<Msg>) -> Vec<WidgetId> {
    fn walk<Msg>(widget: &dyn Widget<Msg>, id: WidgetId, out: &mut Vec<WidgetId>) {
        out.push(id);
        for (index, child) in widget.children().iter().enumerate() {
            walk(child.as_ref(), child_id(id, index, child.as_ref()), out);
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
            if let Some(found) = walk(child.as_ref(), child_id(id, index, child.as_ref()), target) {
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
    use crate::{Button, Container, Flex, Key, Keyed, Placement, Portal, Scroll, TextInput};
    use frus_core::{Color, Point, Primitive, Rect, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        A,
        B,
        C,
        D,
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
    fn wrapped_text_wraps_in_layout_and_invalidates_the_cache() {
        let tree = |text: &str| {
            crate::Flex::column()
                .width(120.0)
                .child(crate::Text::new(text).wrap())
                .child(Container::new().height(10.0).on_click(Msg::A))
        };
        // Position du suiveur cliquable : premier y touché en balayant.
        let follower_y = |ui: &Ui<Msg>| {
            (0..600)
                .map(|y| y as f32)
                .find(|&y| ui.hit(Point::new(60.0, y)).is_some())
                .expect("suiveur cliquable")
        };

        let rt = Runtime::default();
        let long = "un paragraphe assez long pour se replier sur plusieurs lignes";
        let ui = build_ui(&tree(long), Size::new(120.0, 600.0), &rt, &Theme::default());

        // Le rendu du paragraphe porte sa largeur de repli (≤ colonne).
        let max_w = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Text { max_width, .. } => *max_width,
                _ => None,
            })
            .expect("paragraphe replié");
        assert!(max_w <= 120.5, "repli à la largeur de la colonne : {max_w}");

        // Le texte replié occupe plusieurs lignes : le suiveur est repoussé.
        let y_long = follower_y(&ui);
        assert!(y_long > 30.0, "suiveur repoussé par le repli : {y_long}");

        // MÊME structure/styles, contenu différent, MÊME runtime (cache chaud) :
        // la clé de mesure doit invalider le cache — sinon vieux rectangles.
        let ui2 = build_ui(&tree("court"), Size::new(120.0, 600.0), &rt, &Theme::default());
        let y_short = follower_y(&ui2);
        assert!(
            y_short < y_long,
            "contenu plus court → suiveur plus haut (cache invalidé) : {y_short} vs {y_long}"
        );
    }

    #[test]
    fn relayout_cache_reuses_the_root_layout_across_frames() {
        let rt = Runtime::default();
        let size = Size::new(400.0, 100.0);
        // Frame 1 : rien en cache → recalcul (au moins la racine).
        let _ = build_ui(&clickable_sample(), size, &rt, &Theme::default());
        let (hits1, misses1) = rt.layout_cache.borrow().last_frame_stats();
        assert_eq!(hits1, 0, "1re frame : aucune réutilisation");
        assert!(misses1 >= 1, "1re frame : au moins un calcul");

        // Frame 2 : même arbre, mêmes contraintes → la racine est réutilisée.
        let _ = build_ui(&clickable_sample(), size, &rt, &Theme::default());
        let (hits2, misses2) = rt.layout_cache.borrow().last_frame_stats();
        assert_eq!(hits2, 1, "2e frame : racine réutilisée");
        assert_eq!(misses2, 0, "2e frame : aucun recalcul");

        // Frame 3 : fenêtre redimensionnée → contraintes changées → recalcul.
        let _ = build_ui(&clickable_sample(), Size::new(500.0, 100.0), &rt, &Theme::default());
        let (hits3, misses3) = rt.layout_cache.borrow().last_frame_stats();
        assert_eq!((hits3, misses3), (0, 1), "redimensionnement → recalcul");
    }

    #[test]
    fn long_press_targets_are_collected_topmost_first() {
        // Un conteneur à appui long contenant un enfant à appui long : le point
        // dans l'enfant renvoie le message de l'enfant (le plus au-dessus).
        let tree: Container<Msg> = Container::new()
            .width(200.0)
            .height(100.0)
            .on_long_press(Msg::A)
            .child(
                Container::new().width(50.0).height(50.0).on_long_press(Msg::B),
            );
        let ui = build_ui(&tree, Size::new(200.0, 100.0), &Runtime::default(), &Theme::default());
        assert_eq!(ui.long_press_at(Point::new(25.0, 25.0)), Some(Msg::B));
        assert_eq!(ui.long_press_at(Point::new(150.0, 80.0)), Some(Msg::A));
        assert_eq!(ui.long_press_at(Point::new(500.0, 500.0)), None);
    }

    #[test]
    fn hit_and_msg_for_route_correctly() {
        let rt = Runtime::default();
        let ui = build_ui(&clickable_sample(), Size::new(400.0, 100.0), &rt, &Theme::default());
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
        let base = build_ui(&clickable_sample(), Size::new(400.0, 100.0), &rt, &Theme::default());
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
        let ui = build_ui(&clickable_sample(), Size::new(400.0, 100.0), &rt, &Theme::default());
        if let Primitive::Rect { color, .. } = ui.scene().primitives()[0] {
            assert_eq!(color, Color::rgb(0.0, 1.0, 0.0));
        } else {
            panic!("attendu un rectangle");
        }
    }

    #[test]
    fn only_text_inputs_place_a_cursor() {
        // Invariant du correctif de clic (J39) : un bouton focusable ne renvoie PAS
        // de curseur (`cursor_at` = None), donc le shell ne démarre pas de sélection
        // texte dessus et ne capture pas le clic. Seuls les champs texte en posent un.
        let button = Button::<Msg>::new("x").on_press(Msg::A);
        assert_eq!(Widget::<Msg>::cursor_at(&button, 10.0, 200.0, 0), None);
        let input = TextInput::<Msg>::new("hi").on_input(Msg::Edited);
        assert!(Widget::<Msg>::cursor_at(&input, 10.0, 200.0, 0).is_some());
    }

    #[test]
    fn tab_cycles_focusables_in_order() {
        let tree = Flex::<Msg>::column()
            .child(Button::new("un").on_press(Msg::A))
            .child(Button::new("deux").on_press(Msg::B));
        let ui = build_ui(&tree, Size::new(200.0, 200.0), &Runtime::default(), &Theme::default());
        assert_eq!(ui.focusables.len(), 2, "les deux boutons sont focusables");
        let first = ui.focusables[0].0;
        let second = ui.focusables[1].0;

        // Sans focus : Tab → premier, Shift+Tab → dernier.
        assert_eq!(ui.focus_next(None, true), Some(first));
        assert_eq!(ui.focus_next(None, false), Some(second));
        // Bouclage.
        assert_eq!(ui.focus_next(Some(first), true), Some(second));
        assert_eq!(ui.focus_next(Some(second), true), Some(first));
        assert_eq!(ui.focus_next(Some(first), false), Some(second));
    }

    #[test]
    fn focus_ring_for_button_not_for_textinput() {
        let theme = Theme::default();
        let ring = theme.focus.fade(0.4); // focus_progress = 0 → alpha 0.4
        let count_ring = |tree: &dyn Widget<Msg>, keyboard: bool| -> usize {
            let mut rt = Runtime::default();
            let probe = build_ui(tree, Size::new(200.0, 200.0), &rt, &theme);
            rt.input.focused = probe.focusables.first().map(|(id, _)| *id);
            rt.focus_visible = keyboard;
            let ui = build_ui(tree, Size::new(200.0, 200.0), &rt, &theme);
            ui.scene()
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { border_color, .. } if *border_color == ring))
                .count()
        };
        let with_button = Flex::<Msg>::column().child(Button::new("x").on_press(Msg::A));
        assert!(
            count_ring(&with_button, true) >= 1,
            "un bouton focalisé au clavier a un anneau générique"
        );
        // Focus obtenu au pointeur : pas d'anneau (FocusHighlightMode).
        assert_eq!(
            count_ring(&with_button, false),
            0,
            "un clic ne fait pas flasher d'anneau"
        );

        let with_input = Flex::<Msg>::column().child(TextInput::new("hi").on_input(Msg::Edited));
        assert_eq!(count_ring(&with_input, true), 0, "le champ gère son propre focus");
    }

    #[test]
    fn arrow_focus_navigates_geometrically() {
        // Grille 2×2 de boutons ; on identifie chaque cible par son message.
        let grid: Flex<Msg> = Flex::column()
            .child(
                Flex::row()
                    .child(Button::new("a").on_press(Msg::A))
                    .child(Button::new("b").on_press(Msg::B)),
            )
            .child(
                Flex::row()
                    .child(Button::new("c").on_press(Msg::C))
                    .child(Button::new("d").on_press(Msg::D)),
            );
        let ui = build_ui(&grid, Size::new(300.0, 200.0), &Runtime::default(), &Theme::default());
        let top_left = ui.focus_next(None, true).expect("premier focusable");
        assert_eq!(ui.msg_for(top_left), Some(Msg::A));

        // Droite : a → b ; bas : a → c ; et rien à gauche de a.
        let right = ui.focus_directional(top_left, FocusDirection::Right).expect("droite");
        assert_eq!(ui.msg_for(right), Some(Msg::B));
        let down = ui.focus_directional(top_left, FocusDirection::Down).expect("bas");
        assert_eq!(ui.msg_for(down), Some(Msg::C));
        assert_eq!(ui.focus_directional(top_left, FocusDirection::Left), None);
        // Diagonale contrôlée : depuis b, bas → d (aligné), pas c.
        let down_right = ui.focus_directional(right, FocusDirection::Down).expect("bas depuis b");
        assert_eq!(ui.msg_for(down_right), Some(Msg::D));
    }

    #[test]
    fn keyed_identity_survives_middle_removal() {
        let colored = |c: Color| Container::<Msg>::new().width(50.0).height(20.0).color(c);
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);

        let owner_of = |ui: &Ui<Msg>, c: Color| -> u64 {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, owner, .. } if *color == c => Some(*owner),
                    _ => None,
                })
                .expect("primitive présente")
        };

        // Liste [rouge(clé 1), vert(clé 2), bleu(clé 3)].
        let full = Flex::<Msg>::column()
            .child(Keyed::new(1u64, colored(red)))
            .child(Keyed::new(2u64, colored(green)))
            .child(Keyed::new(3u64, colored(blue)));
        let ui_full = build_ui(&full, Size::new(200.0, 200.0), &Runtime::default(), &Theme::default());

        // Liste [rouge(1), bleu(3)] : le vert du milieu est retiré → bleu passe de l'indice 2 à 1.
        let removed = Flex::<Msg>::column()
            .child(Keyed::new(1u64, colored(red)))
            .child(Keyed::new(3u64, colored(blue)));
        let ui_removed =
            build_ui(&removed, Size::new(200.0, 200.0), &Runtime::default(), &Theme::default());

        // L'identité (owner) du bleu (clé 3) est INCHANGÉE malgré le décalage de position.
        assert_eq!(owner_of(&ui_full, blue), owner_of(&ui_removed, blue));

        // Sans clé, l'identité positionnelle du 2e enfant CHANGE (indice 2 vs 1).
        let unkeyed_full = Flex::<Msg>::column()
            .child(colored(red))
            .child(colored(green))
            .child(colored(blue));
        let unkeyed_removed = Flex::<Msg>::column().child(colored(red)).child(colored(blue));
        let u1 = build_ui(&unkeyed_full, Size::new(200.0, 200.0), &Runtime::default(), &Theme::default());
        let u2 = build_ui(&unkeyed_removed, Size::new(200.0, 200.0), &Runtime::default(), &Theme::default());
        assert_ne!(owner_of(&u1, blue), owner_of(&u2, blue));
    }

    #[test]
    fn center_overlay_scrim_click_dismisses() {
        // Une modale Center avec `.dismiss` : cliquer le voile (hors contenu)
        // renvoie le message de fermeture ; cliquer le contenu ne le renvoie pas.
        let modal = Container::<Msg>::new().width(100.0).height(60.0).color(Color::WHITE);
        let portal: Portal<Msg> = Portal::new(Container::<Msg>::new().width(20.0).height(20.0))
            .overlay(modal, Placement::Center)
            .dismiss(Msg::A);
        let ui = build_ui(&portal, Size::new(400.0, 300.0), &Runtime::default(), &Theme::default());

        // Coin supérieur gauche : sur le voile → ferme.
        let corner = ui.hit(Point::new(5.0, 5.0)).expect("voile cliquable");
        assert_eq!(ui.msg_for(corner), Some(Msg::A));
    }

    #[test]
    fn find_widget_and_edit_types() {
        let tree = Flex::column()
            .width(300.0)
            .height(80.0)
            .child(TextInput::new("hi").width(200.0).on_input(Msg::Edited));
        let rt = Runtime::default();
        let ui = build_ui(&tree, Size::new(300.0, 80.0), &rt, &Theme::default());
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
        let ui = build_ui(&tree, Size::new(200.0, 100.0), &rt, &Theme::default());
        let (sid, _viewport, max_x, max_y) = ui.scrollables[0];
        assert_eq!(max_y, 80.0); // 180 - 100
        assert_eq!(max_x, 0.0);
        assert_eq!(ui.first_rect().0.y, 0.0);
        assert_eq!(ui.first_rect().1, Rect::new(0.0, 0.0, 200.0, 100.0));

        let mut rt = Runtime::default();
        rt.scroll.insert(sid, (0.0, 50.0));
        let ui2 = build_ui(&tree, Size::new(200.0, 100.0), &rt, &Theme::default());
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

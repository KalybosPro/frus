//! [`List`] : une liste **virtualisée** — seuls les éléments **visibles** sont
//! construits, mis en page et peints. Indispensable pour de grandes listes
//! (milliers de lignes) : coût par frame ∝ éléments visibles, pas au total.
//!
//! Contrat (v1) : hauteur d'élément **fixe**, défilement **vertical**. Les
//! éléments sont construits à la demande par une closure `index → widget` ; ils
//! n'ont donc **pas d'état retenu** ni de focus clavier (on ne peut pas retenir
//! l'état d'un élément qui n'existe pas hors écran) — parfait pour l'affichage
//! (journaux, tableaux, longues listes), cliquables/survolables quand visibles.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Description d'une liste virtualisée, exposée au pilote de rendu.
pub struct VirtualList<'a, Msg> {
    /// Nombre total d'éléments.
    pub count: usize,
    /// Hauteur (logique) d'un élément.
    pub item_height: f32,
    /// Fabrique un élément par index.
    pub build: &'a dyn Fn(usize) -> Box<dyn Widget<Msg>>,
}

/// Une liste virtualisée à hauteur d'élément fixe.
pub struct List<Msg> {
    count: usize,
    item_height: f32,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    build: Box<dyn Fn(usize) -> Box<dyn Widget<Msg>>>,
}

impl<Msg> List<Msg> {
    /// Crée une liste de `count` éléments de hauteur `item_height`, chaque élément
    /// étant construit à la demande par `build(index)`.
    pub fn new<W: Widget<Msg> + 'static>(
        count: usize,
        item_height: f32,
        build: impl Fn(usize) -> W + 'static,
    ) -> Self {
        Self {
            count,
            item_height,
            width: Dimension::Auto,
            height: Dimension::Length(200.0),
            flex_grow: 0.0,
            build: Box::new(move |index| Box::new(build(index)) as Box<dyn Widget<Msg>>),
        }
    }

    /// Fixe la largeur du viewport, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la hauteur du viewport, en pixels logiques.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Facteur d'expansion flex sur l'axe principal du parent.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }
}

impl<Msg> Widget<Msg> for List<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn virtual_list(&self) -> Option<VirtualList<'_, Msg>> {
        Some(VirtualList {
            count: self.count,
            item_height: self.item_height,
            build: &*self.build,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    #[test]
    fn only_visible_items_are_built() {
        use std::cell::Cell;
        use std::rc::Rc;

        // Compte combien d'éléments la liste construit.
        let built = Rc::new(Cell::new(0usize));
        let counter = built.clone();
        let list = List::<()>::new(5000, 40.0, move |_i| {
            counter.set(counter.get() + 1);
            Container::<()>::new().width(180.0).height(40.0).color(Color::rgb(1.0, 0.0, 0.0))
        })
        .width(200.0)
        .height(200.0);

        let _ui = build_ui(&list, Size::new(200.0, 200.0), &Runtime::default(), &Theme::default());
        // Viewport 200 / item 40 = 5 visibles (+ éventuellement 1 de marge) — jamais 5000.
        assert!(built.get() <= 8, "seuls les éléments visibles sont construits : {}", built.get());
        assert!(built.get() >= 5, "au moins la fenêtre visible : {}", built.get());
    }

    #[test]
    fn scroll_max_covers_full_content() {
        let list = List::<()>::new(100, 40.0, |_i| Container::<()>::new().height(40.0))
            .width(200.0)
            .height(200.0);
        let ui = build_ui(&list, Size::new(200.0, 200.0), &Runtime::default(), &Theme::default());
        // Contenu = 100×40 = 4000 ; viewport 200 → max défilement vertical 3800.
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1);
        assert_eq!(maxes[0].2, 3800.0);
    }

    #[test]
    fn builds_a_scene() {
        let list = List::<()>::new(50, 30.0, |i| {
            Container::<()>::new().height(30.0).color(if i % 2 == 0 {
                Color::rgb(0.2, 0.2, 0.2)
            } else {
                Color::rgb(0.3, 0.3, 0.3)
            })
        })
        .width(200.0)
        .height(120.0);
        let ui = build_ui(&list, Size::new(200.0, 120.0), &Runtime::default(), &Theme::default());
        let rects = ui
            .scene()
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { .. }))
            .count();
        assert!(rects > 0);
    }
}

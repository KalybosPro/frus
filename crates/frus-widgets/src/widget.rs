//! Le trait [`Widget`], générique sur le type de message émis à l'interaction.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Key, Status};
use crate::runtime::Edit;

/// Un widget : un élément d'interface composable.
///
/// `Msg` est le type de message applicatif émis lors d'une interaction (modèle
/// à messages, façon Elm/iced).
pub trait Widget<Msg> {
    /// Style de mise en page (transmis à `frus-layout`).
    fn style(&self) -> Style;

    /// Enfants du widget (éventuellement vide).
    fn children(&self) -> &[Box<dyn Widget<Msg>>];

    /// Peint la décoration propre du widget, selon son statut (survol / focus /
    /// curseur / sélection).
    fn paint(&self, bounds: Rect, status: Status, scene: &mut Scene);

    /// Message à émettre au clic (`None` = non cliquable).
    fn on_click(&self) -> Option<Msg>;

    /// Applique une touche au widget focalisé : mute l'état d'édition
    /// (curseur/sélection) et renvoie un message si la **valeur** change.
    fn on_edit(&self, _edit: &mut Edit, _key: &Key) -> Option<Msg> {
        None
    }

    /// Index de curseur correspondant à une position horizontale locale (px
    /// depuis le bord gauche du widget) — pour placer le curseur au clic.
    fn cursor_at(&self, _local_x: f32) -> Option<usize> {
        None
    }

    /// Texte actuellement sélectionné (pour le copier/couper).
    fn selected_text(&self, _edit: &Edit) -> Option<String> {
        None
    }

    /// Si `true`, le widget peut recevoir le focus clavier (au clic).
    fn focusable(&self) -> bool {
        false
    }

    /// Si le widget est un conteneur défilable, renvoie son contenu.
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        None
    }
}

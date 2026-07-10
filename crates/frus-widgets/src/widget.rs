//! Le trait [`Widget`], générique sur le type de message émis à l'interaction.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Key, Status};
use crate::portal::Placement;
use crate::runtime::Edit;
use crate::theme::Theme;

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
    /// curseur / sélection) et le thème courant.
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene);

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

    /// Plage `(début, fin)` du mot autour de l'index donné (pour le double-clic).
    fn word_at(&self, _index: usize) -> Option<(usize, usize)> {
        None
    }

    /// Si `true`, le widget peut recevoir le focus clavier (au clic).
    fn focusable(&self) -> bool {
        false
    }

    /// Si `true`, le widget répond au glissement du pointeur (curseurs, poignées).
    fn draggable(&self) -> bool {
        false
    }

    /// Message produit lors d'un glissement, `fraction` étant la position
    /// horizontale relative (`0.0..=1.0`) dans les bornes du widget.
    fn on_drag(&self, _fraction: f32) -> Option<Msg> {
        None
    }

    /// Si le widget est un conteneur défilable, renvoie son contenu.
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        None
    }

    /// Axe(s) de défilement (pour un conteneur défilable).
    fn scroll_axis(&self) -> crate::scroll::Axis {
        crate::scroll::Axis::Vertical
    }

    /// Si le widget est un portail, renvoie son contenu flottant et son placement.
    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        None
    }

    /// Message émis au clic sur le voile d'une modale (fermeture), le cas échéant.
    fn overlay_dismiss(&self) -> Option<Msg> {
        None
    }

    /// Cible de la valeur animée propre au widget (p. ex. `1.0` interrupteur on,
    /// `0.0` off). Le runtime fait tendre la valeur retenue vers cette cible et la
    /// restitue via `Status::value`. `None` = pas de valeur animée.
    fn anim_target(&self) -> Option<f32> {
        None
    }

    /// Si le widget est un navigateur d'écrans, renvoie `(progression, push?)`.
    /// Ses enfants (`[écran]` ou `[sortant, entrant]`) sont rendus plein-fenêtre
    /// avec une transition glissée.
    fn navigator(&self) -> Option<(f32, bool)> {
        None
    }
}

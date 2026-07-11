//! Le trait [`Widget`], générique sur le type de message émis à l'interaction.

use frus_core::{Rect, Scene, Size};
use frus_layout::Style;

use crate::interaction::{Key, Status};
use crate::portal::Placement;
use crate::runtime::Edit;
use crate::scroll::Axis;
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

    /// Clé d'identité **stable** (indépendante de la position parmi les frères).
    /// `None` = identité positionnelle. Voir [`crate::Keyed`].
    fn key(&self) -> Option<u64> {
        None
    }

    /// Applique une touche au widget focalisé : mute l'état d'édition
    /// (curseur/sélection) et renvoie un message si la **valeur** change.
    fn on_edit(&self, _edit: &mut Edit, _key: &Key) -> Option<Msg> {
        None
    }

    /// Index de curseur correspondant à une position horizontale locale (px
    /// depuis le bord gauche du widget) — pour placer le curseur au clic.
    ///
    /// `width` = largeur du champ, `scroll_cursor` = curseur d'où recalculer le
    /// **défilement horizontal** courant (le même que le rendu), pour que le clic
    /// tombe juste même quand le texte est défilé. `None` = pas un champ texte.
    fn cursor_at(&self, _local_x: f32, _width: f32, _scroll_cursor: usize) -> Option<usize> {
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

    /// Si `true`, le widget peut recevoir le focus clavier (clic ou Tab).
    fn focusable(&self) -> bool {
        false
    }

    /// Si `true`, le widget dessine **lui-même** son indicateur de focus (le
    /// pilote ne trace alors pas l'anneau générique). Ex. `TextInput`.
    fn draws_own_focus(&self) -> bool {
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

    /// Si `true`, le widget est une **pile** : ses enfants sont des couches
    /// superposées (même boîte), rendues dans l'ordre (dernière au-dessus).
    fn stack(&self) -> bool {
        false
    }

    /// Si `true`, le widget s'anime **en continu** (piloté par le temps, pas par
    /// une cible) : le framework continue de redessiner. Ex. `Spinner`.
    fn continuous(&self) -> bool {
        false
    }

    /// Si le widget est un conteneur défilable, renvoie son contenu.
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        None
    }

    /// Si le widget est une **liste virtualisée**, renvoie sa description (nombre
    /// d'éléments, hauteur, fabrique). Seuls les éléments visibles sont construits.
    fn virtual_list(&self) -> Option<crate::list::VirtualList<'_, Msg>> {
        None
    }

    /// Si le widget construit son contenu **à partir de sa boîte réelle** (façon
    /// Flutter `LayoutBuilder`), renvoie la fabrique `taille → widget`. Le contenu
    /// est construit à la volée : pas d'état retenu ni d'overlay (comme un élément
    /// de liste virtualisée).
    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
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

/// Permet de composer un widget **déjà boxé** là où un `impl Widget` est attendu
/// (p. ex. `Flex::child`). Délègue tout au widget contenu.
impl<Msg> Widget<Msg> for Box<dyn Widget<Msg>> {
    fn style(&self) -> Style {
        (**self).style()
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        (**self).children()
    }
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        (**self).paint(bounds, status, theme, scene)
    }
    fn on_click(&self) -> Option<Msg> {
        (**self).on_click()
    }
    fn key(&self) -> Option<u64> {
        (**self).key()
    }
    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        (**self).on_edit(edit, key)
    }
    fn cursor_at(&self, local_x: f32, width: f32, scroll_cursor: usize) -> Option<usize> {
        (**self).cursor_at(local_x, width, scroll_cursor)
    }
    fn selected_text(&self, edit: &Edit) -> Option<String> {
        (**self).selected_text(edit)
    }
    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        (**self).word_at(index)
    }
    fn focusable(&self) -> bool {
        (**self).focusable()
    }
    fn draws_own_focus(&self) -> bool {
        (**self).draws_own_focus()
    }
    fn draggable(&self) -> bool {
        (**self).draggable()
    }
    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        (**self).on_drag(fraction)
    }
    fn stack(&self) -> bool {
        (**self).stack()
    }
    fn continuous(&self) -> bool {
        (**self).continuous()
    }
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        (**self).scroll_content()
    }
    fn virtual_list(&self) -> Option<crate::list::VirtualList<'_, Msg>> {
        (**self).virtual_list()
    }
    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        (**self).layout_builder()
    }
    fn scroll_axis(&self) -> Axis {
        (**self).scroll_axis()
    }
    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        (**self).overlay()
    }
    fn overlay_dismiss(&self) -> Option<Msg> {
        (**self).overlay_dismiss()
    }
    fn anim_target(&self) -> Option<f32> {
        (**self).anim_target()
    }
    fn navigator(&self) -> Option<(f32, bool)> {
        (**self).navigator()
    }
}

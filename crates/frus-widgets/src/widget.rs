//! Le trait [`Widget`], générique sur le type de message émis à l'interaction.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Key, Status};

/// Un widget : un élément d'interface composable.
///
/// `Msg` est le type de message applicatif émis lors d'une interaction (modèle
/// à messages, façon Elm/iced).
pub trait Widget<Msg> {
    /// Style de mise en page (transmis à `frus-layout`).
    fn style(&self) -> Style;

    /// Enfants du widget (éventuellement vide).
    fn children(&self) -> &[Box<dyn Widget<Msg>>];

    /// Peint la décoration propre du widget, aux bornes `bounds`, selon son
    /// statut (survol/pression/focus).
    fn paint(&self, bounds: Rect, status: Status, scene: &mut Scene);

    /// Message à émettre au clic (`None` = non cliquable).
    fn on_click(&self) -> Option<Msg>;

    /// Message à émettre pour une touche, si le widget a le focus.
    fn on_key(&self, _key: &Key) -> Option<Msg> {
        None
    }

    /// Si `true`, le widget peut recevoir le focus clavier (au clic).
    fn focusable(&self) -> bool {
        false
    }

    /// Si le widget est un conteneur défilable, renvoie son contenu (mis en page
    /// séparément, à hauteur libre, puis découpé et translaté par le pilote).
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        None
    }
}

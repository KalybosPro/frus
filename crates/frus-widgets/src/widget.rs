//! Le trait [`Widget`], générique sur le type de message émis à l'interaction.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Interaction;

/// Un widget : un élément d'interface composable.
///
/// `Msg` est le type de message applicatif émis lors d'une interaction (modèle
/// à messages, façon Elm/iced).
pub trait Widget<Msg> {
    /// Style de mise en page (transmis à `frus-layout`).
    fn style(&self) -> Style;

    /// Enfants du widget (éventuellement vide).
    fn children(&self) -> &[Box<dyn Widget<Msg>>];

    /// Peint la décoration propre du widget, aux bornes `bounds`, en tenant
    /// compte de son statut d'interaction (survol/pression).
    fn paint(&self, bounds: Rect, status: Interaction, scene: &mut Scene);

    /// Message à émettre lorsque ce widget est cliqué (`None` = non cliquable).
    fn on_click(&self) -> Option<Msg>;
}

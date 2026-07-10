//! [`NavBar`] : une barre de navigation persistante — titre centré, bouton
//! retour optionnel à gauche. Placée en tête d'un écran, elle **glisse et fond
//! avec lui** pendant les transitions du [`crate::Navigator`].

use frus_core::{Insets, Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::button::{Button, Variant};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Hauteur de la barre, en pixels logiques.
const HEIGHT: f32 = 56.0;
/// Marge gauche : au-delà de la zone de geste retour, pour que le bouton reste
/// cliquable sans déclencher le swipe.
const PAD_LEFT: f32 = 28.0;
/// Taille du titre.
const TITLE_SIZE: f32 = 20.0;

/// Une barre de navigation : titre + bouton retour optionnel.
pub struct NavBar<Msg> {
    title: String,
    /// `[]` (racine) ou `[bouton retour]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavBar<Msg> {
    /// Crée une barre racine (titre seul, sans retour).
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
        }
    }

    /// Ajoute un bouton retour émettant `message`.
    pub fn on_back(mut self, message: Msg) -> Self {
        self.children = vec![Box::new(
            Button::new("←").variant(Variant::Secondary).size(16.0).on_press(message),
        )];
        self
    }
}

impl<Msg: Clone> Widget<Msg> for NavBar<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            height: Dimension::Length(HEIGHT),
            flex_direction: FlexDirection::Row,
            justify: Justify::Start,
            align: Align::Center,
            padding: Insets::new(6.0, 16.0, 6.0, PAD_LEFT),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Fond + fin séparateur bas.
        scene.fill_rect(bounds, theme.background.fade(o));
        scene.fill_rect(
            Rect::new(bounds.x, bounds.y + bounds.height - 1.0, bounds.width, 1.0),
            theme.border.fade(o),
        );

        // Titre centré horizontalement dans la barre.
        let measured = frus_text::measure(&self.title, TITLE_SIZE);
        let tx = bounds.x + (bounds.width - measured.width) * 0.5;
        let ty = bounds.y + (bounds.height - measured.height) * 0.5;
        scene.text(
            Point::new(tx, ty),
            self.title.clone(),
            TITLE_SIZE,
            theme.on_surface.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Back,
    }

    #[test]
    fn root_bar_has_no_back_button() {
        let bar: NavBar<Msg> = NavBar::new("Accueil");
        assert!(Widget::children(&bar).is_empty());
    }

    #[test]
    fn back_button_emits_message() {
        let bar: NavBar<Msg> = NavBar::new("Réglages").on_back(Msg::Back);
        let ui = build_ui(&bar, Size::new(400.0, 56.0), &Runtime::default(), &Theme::default());
        // Le bouton retour est à gauche ; un clic y renvoie le message de retour.
        let id = ui.hit(Point::new(40.0, 28.0)).expect("bouton retour");
        assert_eq!(ui.msg_for(id), Some(Msg::Back));
    }

    #[test]
    fn bar_paints_title_and_divider() {
        let bar: NavBar<Msg> = NavBar::new("Titre");
        let ui = build_ui(&bar, Size::new(400.0, 56.0), &Runtime::default(), &Theme::default());
        let has_text = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Titre"));
        assert!(has_text, "le titre est peint");
    }
}

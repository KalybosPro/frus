//! [`NavBar`] : une barre de navigation persistante — titre centré, bouton
//! retour optionnel à gauche. Placée en tête d'un écran, elle **glisse et fond
//! avec lui** pendant les transitions du [`crate::Navigator`].

use frus_core::{FontWeight, Insets, Point, Rect, Scene, TextStyle};
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
    /// Style du titre (défaut : 20 px, graisse medium, couleur du thème).
    title_style: TextStyle,
    /// Hauteur de la barre (défaut : [`HEIGHT`]).
    height: f32,
    /// `[]` (racine) ou `[bouton retour]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavBar<Msg> {
    /// Crée une barre racine (titre seul, sans retour).
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            title_style: TextStyle::new(TITLE_SIZE).weight(FontWeight::Medium),
            height: HEIGHT,
            children: Vec::new(),
        }
    }

    /// Surcharge le style du titre (taille/graisse/italique/couleur).
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = style;
        self
    }

    /// Surcharge la hauteur de la barre (défaut : 56 px).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
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
            height: Dimension::Length(self.height),
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

        // Titre centré horizontalement dans la barre, selon `title_style`
        // (défaut : graisse medium — un titre de barre est un « title », pas un
        // corps ; la couleur du style est héritée du thème si absente).
        let style = self.title_style;
        let measured =
            frus_text::measure_styled(&self.title, style.size, style.weight, style.italic);
        let tx = bounds.x + (bounds.width - measured.width) * 0.5;
        let ty = bounds.y + (bounds.height - measured.height) * 0.5;
        scene.text_styled(
            Point::new(tx, ty),
            self.title.clone(),
            &style,
            style.color.unwrap_or(theme.on_surface).fade(o),
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
    fn title_style_and_height_are_customizable() {
        // Style de titre et hauteur surchargés (défauts : medium 20, 56 px).
        let bar: NavBar<Msg> = NavBar::new("Titre")
            .title_style(TextStyle::new(24.0).weight(FontWeight::Bold).italic())
            .height(72.0);
        match Widget::style(&bar).height {
            Dimension::Length(h) => assert_eq!(h, 72.0),
            _ => panic!("hauteur imposée attendue"),
        }
        let ui = build_ui(&bar, Size::new(400.0, 72.0), &Runtime::default(), &Theme::default());
        let styled = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                Primitive::Text { text, size, weight, italic, .. }
                    if text == "Titre" && *size == 24.0 && *weight == FontWeight::Bold && *italic
            )
        });
        assert!(styled, "le titre doit porter le style surchargé");
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

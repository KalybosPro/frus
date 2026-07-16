//! [`Text`] : un widget affichant une ligne de texte.

use frus_core::{Color, FontWeight, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un widget de texte sur une ligne.
///
/// Sa taille de mise en page est calculée par mesure **stylée** (`frus-text`,
/// graisse/italique compris) ; il pousse une primitive de texte dans la scène lors
/// de la peinture. La couleur du [`TextStyle`] est héritée du thème si absente.
pub struct Text {
    content: String,
    style: TextStyle,
    /// Paragraphe : revient à la ligne à la largeur offerte par le parent.
    wrap: bool,
}

impl Text {
    /// Crée un texte (taille 16 px, graisse normale, couleur du thème par défaut).
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: TextStyle::new(16.0),
            wrap: false,
        }
    }

    /// Crée un texte à partir d'un [`TextStyle`] complet — typiquement un cran de
    /// l'échelle du thème (`Text::styled("Titre", theme.text.title_large)`).
    pub fn styled(content: impl Into<String>, style: TextStyle) -> Self {
        Self {
            content: content.into(),
            style,
            wrap: false,
        }
    }

    /// Fait du texte un **paragraphe** : il revient à la ligne à la largeur
    /// offerte par le parent (mesure sous contraintes via taffy) au lieu de
    /// s'étendre sur une seule ligne.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Fixe la taille de police, en pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.style.size = size;
        self
    }

    /// Fixe la graisse.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.style.weight = weight;
        self
    }

    /// Passe en italique.
    pub fn italic(mut self) -> Self {
        self.style.italic = true;
        self
    }

    /// Fixe la couleur du texte (sinon celle du thème).
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }
}

impl<Msg> Widget<Msg> for Text {
    fn style(&self) -> Style {
        // Paragraphe : dimensions libres, la taille vient de `measure()`.
        if self.wrap {
            return Style::default();
        }
        let measured = frus_text::measure_styled(
            &self.content,
            self.style.size,
            self.style.weight,
            self.style.italic,
        );
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(measured.height.ceil()),
            ..Default::default()
        }
    }

    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        if !self.wrap {
            return None;
        }
        let content = self.content.clone();
        let style = self.style;
        Some(Box::new(move |max_width, _| {
            frus_text::measure_wrapped(&content, style.size, style.weight, style.italic, max_width)
        }))
    }

    fn measure_key(&self) -> Option<u64> {
        if !self.wrap {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.content.hash(&mut hasher);
        self.style.size.to_bits().hash(&mut hasher);
        self.style.weight.to_u16().hash(&mut hasher);
        self.style.italic.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self
            .style
            .color
            .unwrap_or(theme.on_surface)
            .fade(status.opacity);
        if self.wrap {
            // Le rendu se replie à la largeur donnée par la mise en page.
            scene.text_wrapped(
                Point::new(bounds.x, bounds.y),
                self.content.clone(),
                &self.style,
                color,
                bounds.width,
            );
        } else {
            scene.text_styled(
                Point::new(bounds.x, bounds.y),
                self.content.clone(),
                &self.style,
                color,
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[test]
    fn text_paints_a_text_primitive() {
        let text = Text::new("Salut").size(20.0).color(Color::rgb(1.0, 0.0, 0.0));
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(5.0, 6.0, 100.0, 24.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );

        assert_eq!(scene.primitives().len(), 1);
        assert_eq!(
            scene.primitives()[0],
            Primitive::Text {
                position: Point::new(5.0, 6.0),
                text: "Salut".to_string(),
                size: 20.0,
                color: Color::rgb(1.0, 0.0, 0.0),
                weight: FontWeight::Regular,
                italic: false,
                max_width: None,
                clip: Rect::UNBOUNDED,
                owner: 0,
            }
        );
    }

    #[test]
    fn wrapped_text_measures_to_the_offered_width() {
        let long = "un paragraphe assez long pour se replier sur plusieurs lignes";
        let text = Text::new(long).wrap();
        // La mesure sous contraintes se replie : plus haut à 120 px qu'en libre.
        let measure = Widget::<()>::measure(&text).expect("closure de mesure");
        let free = measure(None, None);
        let narrow = measure(Some(120.0), None);
        assert!(narrow.width <= 120.0);
        assert!(narrow.height > free.height, "replié → plus haut");
        // Et la clé de mesure change avec le contenu (correction du cache).
        let other = Text::new("court").wrap();
        assert_ne!(
            Widget::<()>::measure_key(&text),
            Widget::<()>::measure_key(&other)
        );
        // Un texte sans repli n'expose ni mesure ni clé.
        let plain = Text::new(long);
        assert!(Widget::<()>::measure(&plain).is_none());
        assert!(Widget::<()>::measure_key(&plain).is_none());
    }

    #[test]
    fn styled_text_carries_weight_and_italic() {
        let theme = Theme::default();
        // Un cran de l'échelle du thème (title_medium = 16 px, graisse medium).
        let text = Text::styled("Titre", theme.text.title_medium).italic();
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(0.0, 0.0, 100.0, 24.0),
            Status::default(),
            &theme,
            &mut scene,
        );
        match &scene.primitives()[0] {
            Primitive::Text { size, weight, italic, color, .. } => {
                assert_eq!(*size, 16.0);
                assert_eq!(*weight, FontWeight::Medium);
                assert!(*italic);
                // Couleur héritée du thème (le style n'en précisait pas).
                assert_eq!(*color, theme.on_surface);
            }
            _ => panic!("attendu du texte"),
        }
    }

    #[test]
    fn bold_text_lays_out_wider() {
        let regular: Style = Widget::<()>::style(&Text::new("Largeur"));
        let bold: Style = Widget::<()>::style(&Text::new("Largeur").weight(FontWeight::Bold));
        let w = |s: &Style| match s.width {
            Dimension::Length(v) => v,
            _ => panic!("largeur mesurée attendue"),
        };
        assert!(w(&bold) > w(&regular), "le gras doit être plus large");
    }
}

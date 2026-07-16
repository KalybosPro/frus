//! [`TextStyle`] : les attributs typographiques d'un texte (taille, graisse,
//! italique, couleur), indépendants du widget et du thème.
//!
//! Un `TextStyle` est une valeur pure `Copy`. Sa `color` est optionnelle — `None`
//! signifie « hérite » (le widget résout vers la couleur du thème au paint). Les
//! échelles typographiques nommées (façon `TextTheme` Material) se composent à
//! partir de ce type.

use crate::Color;

/// Graisse de police (sous-ensemble utile, mappé sur les poids CSS/OpenType).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontWeight {
    /// 400 — normal.
    #[default]
    Regular,
    /// 500.
    Medium,
    /// 600.
    SemiBold,
    /// 700 — gras.
    Bold,
}

impl FontWeight {
    /// Poids numérique OpenType (400/500/600/700).
    pub fn to_u16(self) -> u16 {
        match self {
            FontWeight::Regular => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
        }
    }
}

/// Les attributs typographiques d'un texte sur une ligne.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// Taille de police, en pixels logiques.
    pub size: f32,
    /// Graisse.
    pub weight: FontWeight,
    /// Italique.
    pub italic: bool,
    /// Couleur explicite ; `None` = héritée (résolue par le widget au paint).
    pub color: Option<Color>,
}

impl TextStyle {
    /// Un style de taille `size`, graisse normale, couleur héritée.
    pub const fn new(size: f32) -> Self {
        Self {
            size,
            weight: FontWeight::Regular,
            italic: false,
            color: None,
        }
    }

    /// Fixe la graisse.
    pub const fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Passe en italique.
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Fixe la taille.
    pub const fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Fixe la couleur.
    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// **Fusionne** `over` par-dessus `self` : les attributs typographiques de
    /// `over` l'emportent, et sa couleur **hérite** de `self` si elle est absente
    /// (`None`). C'est la cascade (style de span > style par défaut > thème).
    pub fn merge(self, over: TextStyle) -> TextStyle {
        TextStyle {
            size: over.size,
            weight: over.weight,
            italic: over.italic,
            color: over.color.or(self.color),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_map_to_opentype() {
        assert_eq!(FontWeight::Regular.to_u16(), 400);
        assert_eq!(FontWeight::Bold.to_u16(), 700);
    }

    #[test]
    fn builders_compose() {
        let s = TextStyle::new(20.0).weight(FontWeight::Bold).italic();
        assert_eq!(s.size, 20.0);
        assert_eq!(s.weight, FontWeight::Bold);
        assert!(s.italic);
        assert_eq!(s.color, None);
    }

    #[test]
    fn merge_overrides_type_but_inherits_missing_colour() {
        let base = TextStyle::new(16.0).color(Color::WHITE);
        // `over` change la taille/graisse mais ne précise pas de couleur.
        let over = TextStyle::new(24.0).weight(FontWeight::Bold);
        let merged = base.merge(over);
        assert_eq!(merged.size, 24.0);
        assert_eq!(merged.weight, FontWeight::Bold);
        assert_eq!(merged.color, Some(Color::WHITE), "couleur héritée");

        // Si `over` précise une couleur, elle gagne.
        let over2 = TextStyle::new(24.0).color(Color::BLACK);
        assert_eq!(base.merge(over2).color, Some(Color::BLACK));
    }
}

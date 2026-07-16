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

/// Surcharges **partielles** d'un style (chaque champ absent hérite du parent).
/// Interne : on la compose via les builders de [`TextSpan`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Overrides {
    size: Option<f32>,
    weight: Option<FontWeight>,
    italic: Option<bool>,
    color: Option<Color>,
}

impl Overrides {
    /// Applique les surcharges sur un style **résolu** hérité.
    fn apply(self, base: TextStyle) -> TextStyle {
        TextStyle {
            size: self.size.unwrap_or(base.size),
            weight: self.weight.unwrap_or(base.weight),
            italic: self.italic.unwrap_or(base.italic),
            color: self.color.or(base.color),
        }
    }
}

/// Un **arbre de texte riche** : chaque nœud porte un fragment de texte, des
/// surcharges de style *partielles* (ce qui n'est pas précisé **hérite** du
/// parent — un enfant `.bold()` garde la taille de son parent), et des enfants.
///
/// ```
/// # use frus_core::{TextSpan, TextStyle};
/// let span = TextSpan::new("Hello ")
///     .child(TextSpan::new("bold").bold())
///     .child(TextSpan::new(" world"));
/// let runs = span.flatten(TextStyle::new(20.0));
/// assert_eq!(runs.len(), 3);
/// assert_eq!(runs[1].1.size, 20.0); // l'enfant gras hérite de la taille
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSpan {
    text: String,
    overrides: Overrides,
    children: Vec<TextSpan>,
}

impl TextSpan {
    /// Un fragment de texte sans surcharge (hérite tout du parent).
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            overrides: Overrides::default(),
            children: Vec::new(),
        }
    }

    /// Passe ce sous-arbre en gras.
    pub fn bold(mut self) -> Self {
        self.overrides.weight = Some(FontWeight::Bold);
        self
    }

    /// Fixe la graisse de ce sous-arbre.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.overrides.weight = Some(weight);
        self
    }

    /// Passe ce sous-arbre en italique.
    pub fn italic(mut self) -> Self {
        self.overrides.italic = Some(true);
        self
    }

    /// Fixe la taille de ce sous-arbre.
    pub fn size(mut self, size: f32) -> Self {
        self.overrides.size = Some(size);
        self
    }

    /// Fixe la couleur de ce sous-arbre.
    pub fn color(mut self, color: Color) -> Self {
        self.overrides.color = Some(color);
        self
    }

    /// Applique un [`TextStyle`] complet comme surcharge (sa couleur ne surcharge
    /// que si elle est précisée).
    pub fn style(mut self, style: TextStyle) -> Self {
        self.overrides = Overrides {
            size: Some(style.size),
            weight: Some(style.weight),
            italic: Some(style.italic),
            color: style.color,
        };
        self
    }

    /// Ajoute un enfant (rendu après le texte propre de ce nœud).
    pub fn child(mut self, child: TextSpan) -> Self {
        self.children.push(child);
        self
    }

    /// Aplati l'arbre en **runs résolus** `(texte, style)`, dans l'ordre de
    /// lecture, en cascadant les surcharges depuis `base` (le style par défaut du
    /// paragraphe). Les nœuds sans texte propre ne produisent pas de run vide.
    pub fn flatten(&self, base: TextStyle) -> Vec<(String, TextStyle)> {
        let mut runs = Vec::new();
        self.collect(base, &mut runs);
        runs
    }

    fn collect(&self, inherited: TextStyle, out: &mut Vec<(String, TextStyle)>) {
        let effective = self.overrides.apply(inherited);
        if !self.text.is_empty() {
            out.push((self.text.clone(), effective));
        }
        for child in &self.children {
            child.collect(effective, out);
        }
    }
}

/// Un run de texte **prêt à rendre** : fragment + attributs typographiques et
/// couleur **résolus** (plus aucune héritage à trancher).
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub size: f32,
    pub weight: FontWeight,
    pub italic: bool,
    pub color: Color,
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

    #[test]
    fn span_children_inherit_unspecified_attributes() {
        // « Hello **bold** _red italic_ » : le gras hérite taille/couleur, l'italique
        // rouge hérite taille/graisse.
        let span = TextSpan::new("Hello ")
            .child(TextSpan::new("bold").bold())
            .child(TextSpan::new("red italic").italic().color(Color::rgb(1.0, 0.0, 0.0)));
        let base = TextStyle::new(20.0).color(Color::WHITE);
        let runs = span.flatten(base);

        assert_eq!(runs.len(), 3);
        let (t0, s0) = &runs[0];
        assert_eq!((t0.as_str(), s0.size, s0.weight), ("Hello ", 20.0, FontWeight::Regular));
        let (t1, s1) = &runs[1];
        assert_eq!((t1.as_str(), s1.weight), ("bold", FontWeight::Bold));
        assert_eq!(s1.size, 20.0, "le gras hérite la taille");
        assert_eq!(s1.color, Some(Color::WHITE), "le gras hérite la couleur");
        let (t2, s2) = &runs[2];
        assert_eq!(t2, "red italic");
        assert!(s2.italic);
        assert_eq!(s2.weight, FontWeight::Regular, "l'italique hérite la graisse");
        assert_eq!(s2.color, Some(Color::rgb(1.0, 0.0, 0.0)));
    }

    #[test]
    fn nested_spans_cascade_depth_first() {
        // Un sous-arbre gras dont un petit-enfant repasse la taille à 12.
        let span = TextSpan::new("a")
            .child(TextSpan::new("b").bold().child(TextSpan::new("c").size(12.0)));
        let runs = span.flatten(TextStyle::new(16.0));
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[2].0, "c");
        assert_eq!(runs[2].1.size, 12.0);
        assert_eq!(runs[2].1.weight, FontWeight::Bold, "hérite le gras du parent");
    }

    #[test]
    fn empty_nodes_produce_no_run() {
        // Un nœud « groupe » sans texte propre sert seulement à styler ses enfants.
        let span = TextSpan::new("").bold().child(TextSpan::new("x"));
        let runs = span.flatten(TextStyle::new(16.0));
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].1.weight, FontWeight::Bold);
    }
}

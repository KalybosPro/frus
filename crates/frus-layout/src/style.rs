//! Style de mise en page — une API frus mince, traduite vers taffy en interne.

/// Dimension d'un axe (largeur ou hauteur).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dimension {
    /// Taille déterminée par le contenu / la mise en page.
    Auto,
    /// Taille fixe, en pixels logiques.
    Length(f32),
    /// Pourcentage de la taille du parent (`0.0..=1.0`).
    Percent(f32),
}

impl Dimension {
    fn to_taffy(self) -> taffy::Dimension {
        match self {
            Dimension::Auto => taffy::Dimension::Auto,
            Dimension::Length(v) => taffy::Dimension::Length(v),
            Dimension::Percent(p) => taffy::Dimension::Percent(p),
        }
    }
}

/// Direction de l'axe principal d'un conteneur flex.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlexDirection {
    /// Enfants disposés horizontalement.
    Row,
    /// Enfants disposés verticalement.
    Column,
}

impl FlexDirection {
    fn to_taffy(self) -> taffy::FlexDirection {
        match self {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
        }
    }
}

/// Style d'un nœud de mise en page.
///
/// Sous-ensemble volontairement minimal des propriétés flexbox ; il sera enrichi
/// au fil des jalons (alignements, marges par côté, bordures, etc.).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// Largeur.
    pub width: Dimension,
    /// Hauteur.
    pub height: Dimension,
    /// Facteur d'expansion sur l'axe principal (flexbox).
    pub flex_grow: f32,
    /// Direction de l'axe principal (pour un conteneur).
    pub flex_direction: FlexDirection,
    /// Marge intérieure uniforme, en pixels logiques.
    pub padding: f32,
    /// Espacement entre enfants, en pixels logiques.
    pub gap: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            flex_direction: FlexDirection::Row,
            padding: 0.0,
            gap: 0.0,
        }
    }
}

impl Style {
    pub(crate) fn to_taffy(self) -> taffy::Style {
        taffy::Style {
            size: taffy::Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            flex_grow: self.flex_grow,
            flex_direction: self.flex_direction.to_taffy(),
            padding: taffy::Rect {
                left: taffy::LengthPercentage::Length(self.padding),
                right: taffy::LengthPercentage::Length(self.padding),
                top: taffy::LengthPercentage::Length(self.padding),
                bottom: taffy::LengthPercentage::Length(self.padding),
            },
            gap: taffy::Size {
                width: taffy::LengthPercentage::Length(self.gap),
                height: taffy::LengthPercentage::Length(self.gap),
            },
            ..Default::default()
        }
    }
}

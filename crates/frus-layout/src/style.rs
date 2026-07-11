//! Style de mise en page — une API frus mince, traduite vers taffy en interne.

use frus_core::Insets;

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

/// Répartition des enfants sur l'axe **principal**.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

impl Justify {
    fn to_taffy(self) -> taffy::JustifyContent {
        match self {
            Justify::Start => taffy::JustifyContent::FlexStart,
            Justify::Center => taffy::JustifyContent::Center,
            Justify::End => taffy::JustifyContent::FlexEnd,
            Justify::SpaceBetween => taffy::JustifyContent::SpaceBetween,
            Justify::SpaceAround => taffy::JustifyContent::SpaceAround,
        }
    }
}

/// Alignement des enfants sur l'axe **croisé**.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    Start,
    Center,
    End,
    /// Les enfants s'étirent pour remplir l'axe croisé (défaut).
    Stretch,
}

impl Align {
    fn to_taffy(self) -> taffy::AlignItems {
        match self {
            Align::Start => taffy::AlignItems::FlexStart,
            Align::Center => taffy::AlignItems::Center,
            Align::End => taffy::AlignItems::FlexEnd,
            Align::Stretch => taffy::AlignItems::Stretch,
        }
    }
}

/// Style d'un nœud de mise en page.
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
    /// Répartition sur l'axe principal.
    pub justify: Justify,
    /// Alignement sur l'axe croisé.
    pub align: Align,
    /// Marge intérieure, par côté, en pixels logiques.
    pub padding: Insets,
    /// Espacement entre enfants, en pixels logiques.
    pub gap: f32,
    /// Si `Some(n)`, le conteneur est une **grille** de `n` colonnes égales
    /// (les enfants s'y placent automatiquement, ligne par ligne). `None` = flex.
    pub grid_columns: Option<usize>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            flex_direction: FlexDirection::Row,
            justify: Justify::Start,
            align: Align::Stretch,
            padding: Insets::ZERO,
            gap: 0.0,
            grid_columns: None,
        }
    }
}

impl Style {
    pub(crate) fn to_taffy(self) -> taffy::Style {
        let mut style = taffy::Style {
            size: taffy::Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            flex_grow: self.flex_grow,
            flex_direction: self.flex_direction.to_taffy(),
            justify_content: Some(self.justify.to_taffy()),
            align_items: Some(self.align.to_taffy()),
            padding: taffy::Rect {
                left: taffy::LengthPercentage::Length(self.padding.left),
                right: taffy::LengthPercentage::Length(self.padding.right),
                top: taffy::LengthPercentage::Length(self.padding.top),
                bottom: taffy::LengthPercentage::Length(self.padding.bottom),
            },
            gap: taffy::Size {
                width: taffy::LengthPercentage::Length(self.gap),
                height: taffy::LengthPercentage::Length(self.gap),
            },
            ..Default::default()
        };

        // Grille : `n` colonnes égales (1fr chacune) ; les enfants se placent
        // automatiquement, ligne par ligne (auto-flow), lignes dimensionnées au contenu.
        if let Some(columns) = self.grid_columns {
            use taffy::style_helpers::fr;
            style.display = taffy::Display::Grid;
            style.grid_template_columns = (0..columns).map(|_| fr(1.0)).collect();
        }

        style
    }
}

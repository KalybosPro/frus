//! Classes de taille (breakpoints) — le socle de la responsivité.
//!
//! Une `SizeClass` catégorise une largeur (en pixels **logiques**) en trois
//! paliers, façon Material 3. L'application (et les widgets responsives) s'en
//! servent pour adapter la disposition sans coder de seuils à la main.

/// Palier de largeur d'affichage.
///
/// Seuils (px logiques) : `Compact` < 600, `Medium` 600–840, `Expanded` ≥ 840.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeClass {
    /// Téléphone / fenêtre étroite (< 600).
    Compact,
    /// Tablette portrait / fenêtre moyenne (600–840).
    Medium,
    /// Bureau / fenêtre large (≥ 840).
    Expanded,
}

impl SizeClass {
    /// Seuil bas (inclus) du palier `Medium`, en px logiques.
    pub const MEDIUM: f32 = 600.0;
    /// Seuil bas (inclus) du palier `Expanded`, en px logiques.
    pub const EXPANDED: f32 = 840.0;

    /// Classe correspondant à une largeur (px logiques).
    pub fn from_width(width: f32) -> SizeClass {
        if width >= Self::EXPANDED {
            SizeClass::Expanded
        } else if width >= Self::MEDIUM {
            SizeClass::Medium
        } else {
            SizeClass::Compact
        }
    }

    /// Classe correspondant à une **hauteur** (px logiques), mêmes seuils.
    ///
    /// Utile pour l'axe vertical : une fenêtre courte (< 600) est `Compact` en
    /// hauteur, ce qui permet de masquer des libellés, réduire des marges, etc.
    pub fn from_height(height: f32) -> SizeClass {
        Self::from_width(height)
    }

    /// Rang ordinal (0 = Compact … 2 = Expanded), utile pour comparer les paliers.
    pub fn rank(self) -> u8 {
        match self {
            SizeClass::Compact => 0,
            SizeClass::Medium => 1,
            SizeClass::Expanded => 2,
        }
    }
}

/// Orientation d'affichage, déduite du rapport largeur / hauteur.
///
/// Un autre **axe** de responsivité que la classe de taille : une même largeur
/// peut être portrait (téléphone tenu droit) ou paysage (téléphone couché), ce
/// qui appelle parfois une disposition différente.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// Plus haut que large (ou carré) : `height >= width`.
    Portrait,
    /// Plus large que haut : `width > height`.
    Landscape,
}

impl Orientation {
    /// Orientation d'une fenêtre de dimensions données (px logiques).
    pub fn from_size(width: f32, height: f32) -> Orientation {
        if width > height {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        }
    }

    /// `true` si portrait (plus haut que large).
    pub fn is_portrait(self) -> bool {
        self == Orientation::Portrait
    }

    /// `true` si paysage (plus large que haut).
    pub fn is_landscape(self) -> bool {
        self == Orientation::Landscape
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds() {
        assert_eq!(SizeClass::from_width(0.0), SizeClass::Compact);
        assert_eq!(SizeClass::from_width(599.0), SizeClass::Compact);
        assert_eq!(SizeClass::from_width(600.0), SizeClass::Medium);
        assert_eq!(SizeClass::from_width(839.0), SizeClass::Medium);
        assert_eq!(SizeClass::from_width(840.0), SizeClass::Expanded);
        assert_eq!(SizeClass::from_width(1920.0), SizeClass::Expanded);
    }

    #[test]
    fn ordering_by_rank() {
        assert!(SizeClass::Compact < SizeClass::Medium);
        assert!(SizeClass::Medium < SizeClass::Expanded);
        assert_eq!(SizeClass::Expanded.rank(), 2);
    }

    #[test]
    fn height_uses_same_thresholds() {
        assert_eq!(SizeClass::from_height(500.0), SizeClass::Compact);
        assert_eq!(SizeClass::from_height(700.0), SizeClass::Medium);
        assert_eq!(SizeClass::from_height(900.0), SizeClass::Expanded);
    }

    #[test]
    fn orientation_from_size() {
        assert_eq!(Orientation::from_size(400.0, 800.0), Orientation::Portrait);
        assert_eq!(Orientation::from_size(800.0, 400.0), Orientation::Landscape);
        // Carré → portrait (convention `height >= width`).
        assert_eq!(Orientation::from_size(500.0, 500.0), Orientation::Portrait);
        assert!(Orientation::from_size(400.0, 800.0).is_portrait());
        assert!(Orientation::from_size(800.0, 400.0).is_landscape());
    }
}

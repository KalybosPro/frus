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

    /// Rang ordinal (0 = Compact … 2 = Expanded), utile pour comparer les paliers.
    pub fn rank(self) -> u8 {
        match self {
            SizeClass::Compact => 0,
            SizeClass::Medium => 1,
            SizeClass::Expanded => 2,
        }
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
}

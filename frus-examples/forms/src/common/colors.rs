//! The application's own colours, by role. Everything else comes from the theme.

use frus::Color;

use crate::features::signup::controllers::validation::Strength;

/// The password meter's colour for each strength: red to green.
pub fn strength(strength: Strength) -> Color {
    match strength {
        Strength::Weak => Color::hex("#DC2626"),
        Strength::Fair => Color::hex("#EA580C"),
        Strength::Good => Color::hex("#CA8A04"),
        Strength::Strong => Color::hex("#16A34A"),
    }
}

/// The tick on the summary page.
pub fn success() -> Color {
    Color::hex("#16A34A")
}

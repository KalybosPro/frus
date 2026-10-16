//! The shop's own colours, by role. Everything else comes from the theme
//! (`cx.theme()`), so the app follows light and dark mode.

use frus::Color;

/// Prices stand out from the text around them.
pub(crate) fn price() -> Color {
    Color::hex("#15803D")
}

/// The stars of a rating.
pub(crate) fn rating() -> Color {
    Color::hex("#F59E0B")
}

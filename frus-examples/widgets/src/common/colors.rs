//! The gallery's own colours. Demos use the theme's (`cx.theme()`), so that what they
//! show is what a widget looks like out of the box.

use frus::Color;

/// The background of the "Code" tab.
pub fn code_background(dark: bool) -> Color {
    if dark {
        Color::hex("#1E1E24")
    } else {
        Color::hex("#F4F4F5")
    }
}

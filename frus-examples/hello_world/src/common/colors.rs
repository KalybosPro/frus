//! The application's palette, in one place: screens ask for a colour by its role.

use frus::Color;

pub(crate) fn primary_color() -> Color {
    Color::hex("#4F46E5")
}

pub(crate) fn text_muted() -> Color {
    Color::hex("#71717B")
}

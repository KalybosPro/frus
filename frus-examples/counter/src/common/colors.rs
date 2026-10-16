//! The application's palette, in one place: screens ask for a colour by its role.

use frus::Color;

pub(crate) fn positive() -> Color {
    Color::hex("#16A34A")
}

pub(crate) fn negative() -> Color {
    Color::hex("#DC2626")
}

pub(crate) fn text_muted() -> Color {
    Color::hex("#71717B")
}

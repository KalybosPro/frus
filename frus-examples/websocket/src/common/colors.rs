//! The application's palette, in one place: screens ask for a colour by its role.
//!
//! The status colours are strong enough to carry white text in a light and a dark theme
//! alike; everything else comes from `cx.theme()`.

use frus::Color;

/// Connected: all is well.
pub fn online() -> Color {
    Color::hex("#16A34A")
}

/// Connecting: wait and see.
pub fn pending() -> Color {
    Color::hex("#D97706")
}

/// Failed: something to fix, or a retry on its way.
pub fn failed() -> Color {
    Color::hex("#DC2626")
}

/// Closed: nothing is happening, by choice.
pub fn offline() -> Color {
    Color::hex("#71717B")
}

/// Text that sits on one of the status colours.
pub fn on_status() -> Color {
    Color::hex("#FFFFFF")
}

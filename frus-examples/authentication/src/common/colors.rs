//! The application's palette, in one place: screens ask for a colour by its role.
//!
//! Most colours come from the theme (`cx.theme().primary`, `.muted`, `.error`…), which the
//! application builds from one seed colour in `lib.rs`. Only what the theme has no role
//! for is named here.

use frus::Color;

/// The seed the theme is made from: every role colour derives from it.
pub fn brand() -> Color {
    Color::hex("#4F46E5")
}

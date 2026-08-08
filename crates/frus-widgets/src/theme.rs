//! The theme: *design tokens* (colors, radius, spacing) injected into rendering.
//!
//! The theme is handed to [`crate::build_ui`] and passed on to `Widget::paint`;
//! widgets use it for their default values (text color, text fields, scrollbars…),
//! without preventing an explicit override.

use frus_core::{Color, FontWeight, TextDirection, TextStyle};

use crate::interaction::{Interaction, Status};

/// The **named** typographic scale (Material 3's 15 steps). Widgets pick a step
/// (`theme.text.title_medium`), never a hardcoded size — changing the scale
/// retypesets the whole app. The colors stay inherited (`None` → resolved against
/// the theme at paint time).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTheme {
    pub display_large: TextStyle,
    pub display_medium: TextStyle,
    pub display_small: TextStyle,
    pub headline_large: TextStyle,
    pub headline_medium: TextStyle,
    pub headline_small: TextStyle,
    pub title_large: TextStyle,
    pub title_medium: TextStyle,
    pub title_small: TextStyle,
    pub body_large: TextStyle,
    pub body_medium: TextStyle,
    pub body_small: TextStyle,
    pub label_large: TextStyle,
    pub label_medium: TextStyle,
    pub label_small: TextStyle,
}

impl Default for TextTheme {
    /// The reference Material 3 scale (sizes in logical pixels; the title and label
    /// steps carry a medium weight, as the spec has it).
    fn default() -> Self {
        let medium = |size: f32| TextStyle::new(size).weight(FontWeight::Medium);
        Self {
            display_large: TextStyle::new(57.0),
            display_medium: TextStyle::new(45.0),
            display_small: TextStyle::new(36.0),
            headline_large: TextStyle::new(32.0),
            headline_medium: TextStyle::new(28.0),
            headline_small: TextStyle::new(24.0),
            title_large: TextStyle::new(22.0),
            title_medium: medium(16.0),
            title_small: medium(14.0),
            body_large: TextStyle::new(16.0),
            body_medium: TextStyle::new(14.0),
            body_small: TextStyle::new(12.0),
            label_large: medium(14.0),
            label_medium: medium(12.0),
            label_small: medium(11.0),
        }
    }
}

/// The **color roles** (Material 3) — the **source of truth** for the theme's
/// colors. Widgets reference roles, never literal colors: changing scheme recolors
/// the whole app and guarantees the contrast of the `X`/`on_X` pairs. Written by
/// hand for light and dark; `from_seed` (HCT) comes after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorScheme {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub secondary: Color,
    pub on_secondary: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub background: Color,
    pub surface: Color,
    pub on_surface: Color,
    /// A discreet tonal surface (zone backgrounds, tracks).
    pub surface_variant: Color,
    /// Secondary content on surfaces (the historical `muted`).
    pub on_surface_variant: Color,
    /// An **elevated** surface (floating panels, menus).
    pub surface_container: Color,
    /// A surface higher still (menus above dialogs…).
    pub surface_container_high: Color,
    /// An inverted surface (toasts and snackbars that stand out from the background).
    pub inverse_surface: Color,
    pub on_inverse_surface: Color,
    /// Outlines at rest.
    pub outline: Color,
    /// Discreet outlines (thin separators).
    pub outline_variant: Color,
    pub error: Color,
    pub on_error: Color,
    /// The scrim for modals and drawers (the alpha is applied at the point of use).
    pub scrim: Color,
    /// The color of drop shadows (the alpha is applied at the point of use).
    pub shadow: Color,
}

impl ColorScheme {
    /// The dark scheme.
    pub fn dark() -> Self {
        Self {
            primary: Color::rgb8(96, 200, 130),
            on_primary: Color::rgb8(16, 28, 20),
            primary_container: Color::rgb8(30, 64, 44),
            on_primary_container: Color::rgb8(178, 240, 200),
            secondary: Color::rgb8(150, 170, 200),
            on_secondary: Color::rgb8(20, 26, 36),
            secondary_container: Color::rgb8(44, 52, 68),
            on_secondary_container: Color::rgb8(205, 220, 240),
            background: Color::rgb8(18, 20, 24),
            surface: Color::rgb8(30, 33, 40),
            on_surface: Color::rgb8(230, 232, 236),
            surface_variant: Color::rgb8(38, 42, 52),
            on_surface_variant: Color::rgb8(150, 156, 168),
            surface_container: Color::rgb8(36, 40, 48),
            surface_container_high: Color::rgb8(44, 48, 58),
            inverse_surface: Color::rgb8(226, 228, 234),
            on_inverse_surface: Color::rgb8(28, 32, 38),
            outline: Color::rgb8(70, 76, 88),
            outline_variant: Color::rgb8(48, 52, 62),
            error: Color::rgb8(224, 108, 108),
            on_error: Color::rgb8(38, 12, 12),
            scrim: Color::BLACK,
            shadow: Color::BLACK,
        }
    }

    /// The light scheme.
    pub fn light() -> Self {
        Self {
            primary: Color::rgb8(46, 160, 96),
            on_primary: Color::rgb8(255, 255, 255),
            primary_container: Color::rgb8(200, 238, 214),
            on_primary_container: Color::rgb8(10, 64, 36),
            secondary: Color::rgb8(90, 110, 150),
            on_secondary: Color::rgb8(255, 255, 255),
            secondary_container: Color::rgb8(220, 228, 244),
            on_secondary_container: Color::rgb8(30, 42, 66),
            background: Color::rgb8(245, 246, 248),
            surface: Color::rgb8(255, 255, 255),
            on_surface: Color::rgb8(28, 32, 38),
            surface_variant: Color::rgb8(238, 240, 244),
            on_surface_variant: Color::rgb8(110, 116, 126),
            surface_container: Color::rgb8(244, 245, 248),
            surface_container_high: Color::rgb8(238, 240, 244),
            inverse_surface: Color::rgb8(45, 50, 58),
            on_inverse_surface: Color::rgb8(240, 242, 246),
            outline: Color::rgb8(206, 210, 218),
            outline_variant: Color::rgb8(226, 230, 236),
            error: Color::rgb8(200, 64, 64),
            on_error: Color::rgb8(255, 255, 255),
            scrim: Color::BLACK,
            shadow: Color::BLACK,
        }
    }

    /// Generates a complete scheme from **a seed color** (Material 3 "dynamic
    /// color", through [HCT](frus_core::Hct)). The seed's hue feeds five tonal
    /// palettes (primary, secondary, tertiary — not exposed for now — and the
    /// neutrals); each role is a precise **tone** of its palette, which guarantees
    /// the contrast of the `X`/`on_X` pairs.
    ///
    /// Deliberate departures from the M3 spec: `surface` sits slightly apart from
    /// `background` (our cards lay a surface over the background, tones 12/6 in
    /// dark, 100/98 in light) — the 2023 spec conflates them.
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        use frus_core::{Hct, TonalPalette};

        let hct = Hct::from_color(seed);
        // M3 chromas: the primary keeps the seed's chroma (with a floor of 48), and
        // the other palettes are muted variations on the hue.
        let primary = TonalPalette::new(hct.hue, hct.chroma.max(48.0));
        let secondary = TonalPalette::new(hct.hue, 16.0);
        let neutral = TonalPalette::new(hct.hue, 4.0);
        let neutral_variant = TonalPalette::new(hct.hue, 8.0);
        let error = TonalPalette::new(25.0, 84.0);

        let p = |tone: f64| primary.tone(tone);
        let s = |tone: f64| secondary.tone(tone);
        let n = |tone: f64| neutral.tone(tone);
        let nv = |tone: f64| neutral_variant.tone(tone);
        let e = |tone: f64| error.tone(tone);

        if dark {
            Self {
                primary: p(80.0),
                on_primary: p(20.0),
                primary_container: p(30.0),
                on_primary_container: p(90.0),
                secondary: s(80.0),
                on_secondary: s(20.0),
                secondary_container: s(30.0),
                on_secondary_container: s(90.0),
                background: n(6.0),
                surface: n(12.0),
                on_surface: n(90.0),
                surface_variant: nv(20.0),
                on_surface_variant: nv(80.0),
                surface_container: n(17.0),
                surface_container_high: n(22.0),
                inverse_surface: n(90.0),
                on_inverse_surface: n(20.0),
                outline: nv(60.0),
                outline_variant: nv(30.0),
                error: e(80.0),
                on_error: e(20.0),
                scrim: n(0.0),
                shadow: n(0.0),
            }
        } else {
            Self {
                primary: p(40.0),
                on_primary: p(100.0),
                primary_container: p(90.0),
                on_primary_container: p(10.0),
                secondary: s(40.0),
                on_secondary: s(100.0),
                secondary_container: s(90.0),
                on_secondary_container: s(10.0),
                background: n(98.0),
                surface: n(100.0),
                on_surface: n(10.0),
                surface_variant: nv(94.0),
                on_surface_variant: nv(30.0),
                surface_container: n(96.0),
                surface_container_high: n(94.0),
                inverse_surface: n(20.0),
                on_inverse_surface: n(95.0),
                outline: nv(50.0),
                outline_variant: nv(80.0),
                error: e(40.0),
                on_error: e(100.0),
                scrim: n(0.0),
                shadow: n(0.0),
            }
        }
    }

    /// Interpolates role by role toward `other` (the light/dark switch fade).
    pub fn lerp(&self, other: &ColorScheme, t: f32) -> ColorScheme {
        let c = |a: Color, b: Color| a.lerp(b, t);
        ColorScheme {
            primary: c(self.primary, other.primary),
            on_primary: c(self.on_primary, other.on_primary),
            primary_container: c(self.primary_container, other.primary_container),
            on_primary_container: c(self.on_primary_container, other.on_primary_container),
            secondary: c(self.secondary, other.secondary),
            on_secondary: c(self.on_secondary, other.on_secondary),
            secondary_container: c(self.secondary_container, other.secondary_container),
            on_secondary_container: c(self.on_secondary_container, other.on_secondary_container),
            background: c(self.background, other.background),
            surface: c(self.surface, other.surface),
            on_surface: c(self.on_surface, other.on_surface),
            surface_variant: c(self.surface_variant, other.surface_variant),
            on_surface_variant: c(self.on_surface_variant, other.on_surface_variant),
            surface_container: c(self.surface_container, other.surface_container),
            surface_container_high: c(self.surface_container_high, other.surface_container_high),
            inverse_surface: c(self.inverse_surface, other.inverse_surface),
            on_inverse_surface: c(self.on_inverse_surface, other.on_inverse_surface),
            outline: c(self.outline, other.outline),
            outline_variant: c(self.outline_variant, other.outline_variant),
            error: c(self.error, other.error),
            on_error: c(self.on_error, other.on_error),
            scrim: c(self.scrim, other.scrim),
            shadow: c(self.shadow, other.shadow),
        }
    }
}

/// A set of style tokens.
///
/// The [`ColorScheme`] (`theme.scheme`) is the **source of truth** for the colors;
/// the "flat" fields (`background`, `surface`, `primary`, …) are **convenience
/// views** of the most used roles, derived from the scheme — the widgets'
/// historical API stays intact. `focus`/`selection` are interaction accents
/// specific to frus (outside the M3 roles).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// The color roles (the source of truth).
    pub scheme: ColorScheme,
    /// The application's background (= `scheme.background`).
    pub background: Color,
    /// The surfaces' background (= `scheme.surface`).
    pub surface: Color,
    /// The accent color (= `scheme.primary`).
    pub primary: Color,
    /// Text and content on `primary` (= `scheme.on_primary`).
    pub on_primary: Color,
    /// The default text on surfaces (= `scheme.on_surface`).
    pub on_surface: Color,
    /// Secondary text and discreet elements (= `scheme.on_surface_variant`).
    pub muted: Color,
    /// Borders at rest (= `scheme.outline`).
    pub border: Color,
    /// The focus accent (a frus interaction accent, outside the scheme).
    pub focus: Color,
    /// The text selection highlight (likewise).
    pub selection: Color,
    /// The tonal accent container (= `scheme.primary_container`).
    pub primary_container: Color,
    /// Content on `primary_container` (= `scheme.on_primary_container`).
    pub on_primary_container: Color,
    /// The error or danger color (= `scheme.error`).
    pub error: Color,
    /// Content on `error` (= `scheme.on_error`).
    pub on_error: Color,
    /// The discreet outline variant (= `scheme.outline_variant`).
    pub outline_variant: Color,
    /// The named typographic scale (Material's 15 steps).
    pub text: TextTheme,
    /// The default corner radius.
    pub radius: f32,
    /// The base spacing unit.
    pub spacing: f32,
    /// The ambient **reading and layout direction** (LTR by default). In RTL, the
    /// driver mirrors the layout horizontally. Carried here (an ambient context
    /// threaded down to paint) pending a dedicated `Env` (§2).
    pub direction: TextDirection,
}

impl Theme {
    /// Builds a theme from a scheme: the flat fields are **derived** from the roles,
    /// so there is a single source of truth.
    pub fn from_scheme(scheme: ColorScheme, focus: Color, selection: Color) -> Self {
        Self {
            scheme,
            background: scheme.background,
            surface: scheme.surface,
            primary: scheme.primary,
            on_primary: scheme.on_primary,
            on_surface: scheme.on_surface,
            muted: scheme.on_surface_variant,
            border: scheme.outline,
            focus,
            selection,
            primary_container: scheme.primary_container,
            on_primary_container: scheme.on_primary_container,
            error: scheme.error,
            on_error: scheme.on_error,
            outline_variant: scheme.outline_variant,
            text: TextTheme::default(),
            radius: 10.0,
            spacing: 8.0,
            direction: TextDirection::Ltr,
        }
    }

    /// The same theme in **right-to-left** (Arabic, Hebrew…).
    pub fn rtl(mut self) -> Self {
        self.direction = TextDirection::Rtl;
        self
    }

    /// The dark theme.
    pub fn dark() -> Self {
        Self::from_scheme(
            ColorScheme::dark(),
            Color::rgb8(90, 158, 242),
            Color::rgba(0.35, 0.62, 0.95, 0.40),
        )
    }

    /// The light theme.
    pub fn light() -> Self {
        Self::from_scheme(
            ColorScheme::light(),
            Color::rgb8(40, 120, 220),
            Color::rgba(0.20, 0.50, 0.90, 0.30),
        )
    }

    /// A theme generated from a **seed color** (see [`ColorScheme::from_seed`]). The
    /// focus ring and the selection derive from the scheme's primary (interaction
    /// roles specific to frus, outside the M3 scheme).
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        let scheme = ColorScheme::from_seed(seed, dark);
        let focus = scheme.primary;
        let selection = scheme.primary.with_alpha(if dark { 0.40 } else { 0.30 });
        Self::from_scheme(scheme, focus, selection)
    }

    /// Applies the Material **state layer** over `base`: it overlays the content
    /// color `on` at low opacity according to the interaction state — hover 8%,
    /// focus 10%, press 12% — taking the animated progressions into account
    /// (`hover_progress`/`focus_progress`). This is the state rule **baked** into
    /// the theme: widgets stay declarative (they pass their base color and their
    /// content color, and the theme decides on the overlay).
    pub fn state_layer(&self, base: Color, on: Color, status: &Status) -> Color {
        let mut overlay = 0.08 * status.hover_progress.clamp(0.0, 1.0)
            + 0.10 * status.focus_progress.clamp(0.0, 1.0);
        if status.interaction == Interaction::Pressed {
            overlay += 0.12;
        }
        base.lerp(on, overlay.min(1.0))
    }
}

impl Theme {
    /// Interpolates toward `other` at progress `t` (`0` = `self`, `1` = `other`).
    /// Used for the theme fade when switching light and dark. The **scheme** is
    /// interpolated role by role and the flat fields are re-derived from it (a
    /// single source of truth, even mid-fade).
    pub fn lerp(&self, other: &Theme, t: f32) -> Theme {
        let t = t.clamp(0.0, 1.0);
        let f = |a: f32, b: f32| a + (b - a) * t;
        let mut out = Theme::from_scheme(
            self.scheme.lerp(&other.scheme, t),
            self.focus.lerp(other.focus, t),
            self.selection.lerp(other.selection, t),
        );
        // Typography takes no part in the fade (it is identical light and dark).
        out.text = self.text;
        out.radius = f(self.radius, other.radius);
        out.spacing = f(self.spacing, other.spacing);
        // Direction is discrete: keep the fade target's.
        out.direction = other.direction;
        out
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The WCAG contrast between two colors (a ratio ≥ 1).
    fn contrast(a: Color, b: Color) -> f32 {
        let (la, lb) = (a.compute_luminance() + 0.05, b.compute_luminance() + 0.05);
        if la > lb {
            la / lb
        } else {
            lb / la
        }
    }

    #[test]
    fn from_seed_generates_contrasting_pairs() {
        // Every X / on_X pair must stay legible (≥ 4.5:1, the AA requirement), for
        // any seed at all — even a barely chromatic one.
        for seed in [
            Color::rgb8(0x42, 0x85, 0xF4), // Google blue
            Color::rgb8(0x9C, 0x27, 0xB0), // violet
            Color::rgb8(0x80, 0x80, 0x80), // gris (chroma quasi nul)
        ] {
            for dark in [false, true] {
                let s = ColorScheme::from_seed(seed, dark);
                for (name, base, on) in [
                    ("primary", s.primary, s.on_primary),
                    ("secondary", s.secondary, s.on_secondary),
                    ("surface", s.surface, s.on_surface),
                    ("error", s.error, s.on_error),
                    ("inverse", s.inverse_surface, s.on_inverse_surface),
                ] {
                    let ratio = contrast(base, on);
                    assert!(
                        ratio >= 4.5,
                        "contraste {name} insuffisant ({ratio:.2}) — graine {seed:?}, dark={dark}"
                    );
                }
            }
        }
    }

    #[test]
    fn from_seed_light_and_dark_share_the_hue() {
        // Both modes decline the same hue (the dark primary is the tone-80 version
        // of the tone-40 light primary).
        let seed = Color::rgb8(0x42, 0x85, 0xF4);
        let light = ColorScheme::from_seed(seed, false);
        let dark = ColorScheme::from_seed(seed, true);
        let hue = |c: Color| frus_core::Hct::from_color(c).hue;
        let delta = (hue(light.primary) - hue(dark.primary)).abs();
        let delta = delta.min(360.0 - delta);
        assert!(
            delta < 12.0,
            "teintes clair/sombre divergentes ({delta:.1}°)"
        );
        // Et le sombre est bien… sombre.
        assert!(dark.background.compute_luminance() < 0.1);
        assert!(light.background.compute_luminance() > 0.85);
    }

    #[test]
    fn dark_and_light_differ() {
        assert_ne!(Theme::dark().background, Theme::light().background);
        assert_ne!(Theme::dark().on_surface, Theme::light().on_surface);
    }

    #[test]
    fn flat_fields_mirror_the_scheme() {
        // The flat fields are views derived from the scheme — including mid-fade
        // (the lerp goes through the scheme).
        for theme in [
            Theme::dark(),
            Theme::light(),
            Theme::dark().lerp(&Theme::light(), 0.37),
        ] {
            assert_eq!(theme.background, theme.scheme.background);
            assert_eq!(theme.surface, theme.scheme.surface);
            assert_eq!(theme.primary, theme.scheme.primary);
            assert_eq!(theme.on_primary, theme.scheme.on_primary);
            assert_eq!(theme.on_surface, theme.scheme.on_surface);
            assert_eq!(theme.muted, theme.scheme.on_surface_variant);
            assert_eq!(theme.border, theme.scheme.outline);
            assert_eq!(theme.primary_container, theme.scheme.primary_container);
            assert_eq!(theme.error, theme.scheme.error);
            assert_eq!(theme.outline_variant, theme.scheme.outline_variant);
        }
    }

    #[test]
    fn state_layer_darkens_toward_content_on_interaction() {
        let theme = Theme::dark();
        let base = Color::rgb(0.4, 0.4, 0.4);
        let on = Color::BLACK;

        // Au repos : aucune superposition.
        let idle = Status::default();
        assert_eq!(theme.state_layer(base, on, &idle), base);

        // Fully hovered: the base is pulled 8% toward `on` (darker here).
        let hovered = Status {
            hover_progress: 1.0,
            ..Default::default()
        };
        let h = theme.state_layer(base, on, &hovered);
        assert!(h.r < base.r && (base.r - h.r - 0.4 * 0.08).abs() < 1e-4);

        // Pressed: a stronger overlay than hover alone.
        let pressed = Status {
            interaction: Interaction::Pressed,
            ..Default::default()
        };
        assert!(theme.state_layer(base, on, &pressed).r < h.r);
    }

    #[test]
    fn lerp_hits_endpoints() {
        let d = Theme::dark();
        let l = Theme::light();
        assert_eq!(d.lerp(&l, 0.0).background, d.background);
        assert_eq!(d.lerp(&l, 1.0).background, l.background);
        // In the middle: neither one nor the other.
        let mid = d.lerp(&l, 0.5).background;
        assert_ne!(mid, d.background);
        assert_ne!(mid, l.background);
    }
}

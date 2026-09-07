//! Floating-point RGBA colour (components in 0.0..=1.0).

use bytemuck::{Pod, Zeroable};

/// An RGBA colour. Components lie in `[0.0, 1.0]`.
///
/// Colours are expressed in **sRGB**, the way a colour picker states them. The
/// conversion to linear ([`Color::to_linear`]) happens at the last moment, at the
/// GPU boundary, because the render surface is sRGB (see the colour-management
/// milestone).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    /// Builds an opaque colour.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Builds a colour with an alpha channel.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Builds an opaque colour from 8-bit components (0..=255).
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba8(r, g, b, 255)
    }

    /// Builds a colour from 8-bit components (0..=255).
    pub const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Parses a CSS hex code (sRGB values), with or without a leading `#`.
    ///
    /// Accepted formats: `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`, in either case.
    /// Returns `None` if the string is invalid. See [`Color::hex`] for the
    /// convenient variant, which panics on invalid input.
    pub fn try_hex(s: &str) -> Option<Color> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        // Expand the short forms (#RGB / #RGBA) by duplicating each nibble.
        let expand = |c: u8| (c << 4) | c;
        let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
        let nib = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).ok().map(expand);
        match s.len() {
            3 => Some(Self::rgb8(nib(0)?, nib(1)?, nib(2)?)),
            4 => Some(Self::rgba8(nib(0)?, nib(1)?, nib(2)?, nib(3)?)),
            6 => Some(Self::rgb8(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Self::rgba8(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    /// Builds a colour from a CSS hex code (see [`Color::try_hex`]).
    ///
    /// Panics if the string is invalid — convenient for known literals such as
    /// `Color::hex("#3B82F6")`. For dynamic input, prefer `try_hex`.
    pub fn hex(s: &str) -> Color {
        Self::try_hex(s).unwrap_or_else(|| panic!("invalid hex colour code: {s:?}"))
    }

    /// The `[r, g, b, a]` array form, ready for the GPU.
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Multiplies the opacity (alpha channel) by `opacity`, clamped to `0.0..=1.0`.
    pub fn fade(self, opacity: f32) -> Color {
        Color::rgba(self.r, self.g, self.b, self.a * opacity.clamp(0.0, 1.0))
    }

    /// Replaces the alpha channel (clamped to `0.0..=1.0`), leaving RGB alone.
    pub fn with_alpha(self, alpha: f32) -> Color {
        Color::rgba(self.r, self.g, self.b, alpha.clamp(0.0, 1.0))
    }

    /// Builds a colour from an `0xAARRGGBB` integer (alpha first).
    pub const fn from_argb_u32(argb: u32) -> Color {
        let a = (argb >> 24) & 0xFF;
        let r = (argb >> 16) & 0xFF;
        let g = (argb >> 8) & 0xFF;
        let b = argb & 0xFF;
        Color::rgba8(r as u8, g as u8, b as u8, a as u8)
    }

    /// Relative luminance (WCAG), computed on the **linearised** channels — the
    /// basis of any contrast calculation. Ignores alpha. Result in `[0, 1]`.
    pub fn compute_luminance(self) -> f32 {
        let lin = self.to_linear();
        0.2126 * lin.r + 0.7152 * lin.g + 0.0722 * lin.b
    }

    /// Converts one sRGB component (`0..1`) to linear.
    fn srgb_to_linear(c: f32) -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Converts one linear component (`0..1`) to sRGB.
    fn linear_to_srgb(c: f32) -> f32 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    /// The **linear** version of this colour (`Color` values are sRGB). Apply this
    /// before handing anything to the GPU: an sRGB target re-encodes linear back to
    /// sRGB, so sending linear values reproduces the colour you asked for. Alpha is
    /// left unchanged.
    pub fn to_linear(self) -> Color {
        Color::rgba(
            Self::srgb_to_linear(self.r),
            Self::srgb_to_linear(self.g),
            Self::srgb_to_linear(self.b),
            self.a,
        )
    }

    /// The inverse of [`Color::to_linear`]: linear → sRGB.
    pub fn to_srgb(self) -> Color {
        Color::rgba(
            Self::linear_to_srgb(self.r),
            Self::linear_to_srgb(self.g),
            Self::linear_to_srgb(self.b),
            self.a,
        )
    }

    /// Linear interpolation towards `other` (`t` clamped to `0.0..=1.0`).
    pub fn lerp(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color::rgba(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }

    /// The **surface tint** for a raised surface: `self` with `tint` laid over it at the
    /// opacity Material 3 gives that elevation.
    ///
    /// A raised surface in Material 3 is not lit, it is **tinted**. A shadow says a thing
    /// is above the page; the tint says how far, and it is what keeps a raised card
    /// legible against a dark background where a shadow shows nothing at all.
    ///
    /// The opacities are the specification's own table, interpolated between its levels
    /// and clamped outside them (see [`surface_tint_opacity`]).
    ///
    /// The blend is a plain channel mix, **not** the framework's usual warning about
    /// translucency: the result here is an opaque colour computed once and handed to the
    /// renderer to paint, so nothing is composited and there is no linear-space step to
    /// get wrong. Laying the tint on as a translucent layer instead would go through
    /// compositing and come out darker — the same trap as every other token with an alpha.
    ///
    /// A fully transparent `tint` leaves the surface alone, as the reference's does.
    pub fn surface_tint(self, tint: Color, elevation: f32) -> Color {
        if tint.a <= 0.0 {
            return self;
        }
        self.lerp(tint, surface_tint_opacity(elevation))
    }
}

/// How strongly a Material 3 surface is tinted at a given elevation.
///
/// The specification gives six levels rather than a curve:
///
/// | elevation | 0 | 1 | 3 | 6 | 8 | 12 |
/// |---|---|---|---|---|---|---|
/// | opacity | 0 | 0.05 | 0.08 | 0.11 | 0.12 | 0.14 |
///
/// Between two levels it interpolates; outside them it clamps, so a bar at 40 is tinted
/// exactly as much as one at 12 and no more. That is the reference's rule too, and the
/// table is the one its token generator emits.
pub fn surface_tint_opacity(elevation: f32) -> f32 {
    const LEVELS: [(f32, f32); 6] = [
        (0.0, 0.0),
        (1.0, 0.05),
        (3.0, 0.08),
        (6.0, 0.11),
        (8.0, 0.12),
        (12.0, 0.14),
    ];
    if elevation <= LEVELS[0].0 {
        return LEVELS[0].1;
    }
    for pair in LEVELS.windows(2) {
        let (low_e, low_o) = pair[0];
        let (high_e, high_o) = pair[1];
        if elevation <= high_e {
            let t = (elevation - low_e) / (high_e - low_e);
            return low_o + t * (high_o - low_o);
        }
    }
    LEVELS[LEVELS.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The tint table is the specification's, levels and all.**
    ///
    /// Six levels, interpolated between and clamped outside — a bar at 40 is tinted
    /// exactly as much as one at 12. Checked at the levels themselves, between two of
    /// them, and past the end, because those are the three ways a lookup table goes
    /// wrong.
    #[test]
    fn the_surface_tint_follows_the_material_levels() {
        assert_eq!(surface_tint_opacity(0.0), 0.0);
        assert_eq!(surface_tint_opacity(1.0), 0.05);
        assert_eq!(surface_tint_opacity(3.0), 0.08);
        assert_eq!(surface_tint_opacity(12.0), 0.14);
        // Halfway between level 1 and level 2.
        assert!((surface_tint_opacity(2.0) - 0.065).abs() < 1e-6);
        // Below the first level and above the last: clamped, not extrapolated.
        assert_eq!(surface_tint_opacity(-4.0), 0.0);
        assert_eq!(surface_tint_opacity(40.0), 0.14);
    }

    /// **A transparent tint leaves the surface exactly as it was.**
    ///
    /// The reference returns the colour unmodified rather than blending towards
    /// nothing, and the difference shows: blending towards a transparent colour would
    /// drag the surface's own alpha down with it.
    #[test]
    fn a_transparent_tint_changes_nothing() {
        let surface = Color::rgb(0.2, 0.2, 0.25);
        assert_eq!(surface.surface_tint(Color::TRANSPARENT, 6.0), surface);
        // And at elevation zero there is nothing to tint with either.
        let tint = Color::rgb(0.4, 0.2, 0.9);
        assert_eq!(surface.surface_tint(tint, 0.0), surface);
    }
    #[test]
    fn lerp_midpoint() {
        let a = Color::rgb(0.0, 0.0, 0.0);
        let b = Color::rgb(1.0, 0.5, 0.0);
        let mid = a.lerp(b, 0.5);
        assert_eq!(mid, Color::rgb(0.5, 0.25, 0.0));
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
    }

    #[test]
    fn srgb_linear_roundtrip_and_values() {
        // Fixed points.
        assert_eq!(
            Color::rgb(0.0, 0.0, 0.0).to_linear(),
            Color::rgb(0.0, 0.0, 0.0)
        );
        let white = Color::rgb(1.0, 1.0, 1.0).to_linear();
        assert!((white.r - 1.0).abs() < 1e-4);
        // sRGB midpoint 0.5 → about 0.214 linear.
        let mid = Color::rgb(0.5, 0.5, 0.5).to_linear();
        assert!(
            (mid.r - 0.214).abs() < 0.005,
            "0.5 sRGB → linear = {}",
            mid.r
        );
        // Round trip.
        let c = Color::rgba(0.2, 0.6, 0.9, 0.5);
        let round = c.to_linear().to_srgb();
        assert!((round.r - 0.2).abs() < 1e-3 && (round.g - 0.6).abs() < 1e-3);
        // Alpha untouched.
        assert_eq!(c.to_linear().a, 0.5);
    }

    #[test]
    fn hex_parses_css_codes() {
        assert_eq!(Color::hex("#000000"), Color::BLACK);
        assert_eq!(Color::hex("#FFFFFF"), Color::WHITE);
        // No '#', either case.
        assert_eq!(Color::try_hex("ffffff"), Some(Color::WHITE));
        // Short form: #RGB → #RRGGBB.
        assert_eq!(Color::hex("#f00"), Color::rgb8(255, 0, 0));
        // Alpha.
        assert_eq!(Color::hex("#00000080").a, 128.0 / 255.0);
        assert_eq!(Color::try_hex("#0008").unwrap().a, 0x88 as f32 / 255.0);
        // A concrete component.
        assert_eq!(Color::hex("#3B82F6"), Color::rgb8(0x3B, 0x82, 0xF6));
        // Invalid input.
        assert_eq!(Color::try_hex("#12"), None);
        assert_eq!(Color::try_hex("#gggggg"), None);
        assert_eq!(Color::try_hex(""), None);
    }

    #[test]
    fn fade_scales_alpha() {
        let c = Color::rgba(0.2, 0.4, 0.6, 0.8);
        assert_eq!(c.fade(0.5), Color::rgba(0.2, 0.4, 0.6, 0.4));
        assert_eq!(c.fade(1.0), c);
        assert_eq!(c.fade(0.0).a, 0.0);
    }

    #[test]
    fn with_alpha_replaces_alpha() {
        let c = Color::rgba(0.2, 0.4, 0.6, 0.8);
        assert_eq!(c.with_alpha(0.3), Color::rgba(0.2, 0.4, 0.6, 0.3));
        assert_eq!(c.with_alpha(2.0).a, 1.0, "clamped to 1");
    }

    #[test]
    fn from_argb_u32_decodes_channels() {
        // 0x80FF0000 = red at 50% alpha.
        let c = Color::from_argb_u32(0x80FF_0000);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 128.0 / 255.0);
        assert_eq!(Color::from_argb_u32(0xFFFF_FFFF), Color::WHITE);
    }

    #[test]
    fn luminance_orders_black_grey_white() {
        assert!(Color::BLACK.compute_luminance() < 1e-6);
        assert!((Color::WHITE.compute_luminance() - 1.0).abs() < 1e-4);
        let grey = Color::rgb(0.5, 0.5, 0.5).compute_luminance();
        assert!(grey > 0.0 && grey < 1.0);
        // Green weighs more than red, which weighs more than blue (WCAG).
        assert!(
            Color::rgb(0.0, 1.0, 0.0).compute_luminance()
                > Color::rgb(1.0, 0.0, 0.0).compute_luminance()
        );
        assert!(
            Color::rgb(1.0, 0.0, 0.0).compute_luminance()
                > Color::rgb(0.0, 0.0, 1.0).compute_luminance()
        );
    }
}

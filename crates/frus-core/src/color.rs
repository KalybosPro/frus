//! Couleur RGBA en virgule flottante (composantes 0.0..=1.0).

use bytemuck::{Pod, Zeroable};

/// Une couleur RGBA. Les composantes sont dans `[0.0, 1.0]`.
///
/// Les couleurs sont exprimées en **sRGB** (comme un sélecteur de couleur). La
/// conversion en linéaire ([`Color::to_linear`]) se fait au dernier moment, à la
/// frontière GPU, car la surface de rendu est sRGB (voir jalon colorimétrie).
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

    /// Construit une couleur opaque.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Construit une couleur avec canal alpha.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Construit une couleur à partir de composantes 8 bits (0..=255), opaque.
    pub fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba8(r, g, b, 255)
    }

    /// Construit une couleur à partir de composantes 8 bits (0..=255).
    pub fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Parse un code hexadécimal CSS (valeurs sRGB), avec ou sans `#` en tête.
    ///
    /// Formats acceptés : `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA` (casse libre).
    /// Retourne `None` si la chaîne est invalide. Voir [`Color::hex`] pour la
    /// variante ergonomique (panique sur entrée invalide).
    pub fn try_hex(s: &str) -> Option<Color> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        // Développe les formes courtes (#RGB / #RGBA) en dupliquant chaque quartet.
        let expand = |c: u8| (c << 4) | c;
        let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
        let nib = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).ok().map(|c| expand(c));
        match s.len() {
            3 => Some(Self::rgb8(nib(0)?, nib(1)?, nib(2)?)),
            4 => Some(Self::rgba8(nib(0)?, nib(1)?, nib(2)?, nib(3)?)),
            6 => Some(Self::rgb8(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Self::rgba8(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    /// Construit une couleur depuis un code hexadécimal CSS (voir [`Color::try_hex`]).
    ///
    /// Panique si la chaîne est invalide — pratique pour des littéraux connus
    /// (`Color::hex("#3B82F6")`). Pour une entrée dynamique, préférer `try_hex`.
    pub fn hex(s: &str) -> Color {
        Self::try_hex(s).unwrap_or_else(|| panic!("code couleur hex invalide : {s:?}"))
    }

    /// Représentation en tableau `[r, g, b, a]`, prête pour le GPU.
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Multiplie l'opacité (canal alpha) par `opacity` (borné à `0.0..=1.0`).
    pub fn fade(self, opacity: f32) -> Color {
        Color::rgba(self.r, self.g, self.b, self.a * opacity.clamp(0.0, 1.0))
    }

    /// Remplace le canal alpha (borné à `0.0..=1.0`), sans toucher au RGB.
    pub fn with_alpha(self, alpha: f32) -> Color {
        Color::rgba(self.r, self.g, self.b, alpha.clamp(0.0, 1.0))
    }

    /// Construit une couleur depuis un entier `0xAARRGGBB` (alpha en tête).
    pub fn from_argb_u32(argb: u32) -> Color {
        let a = (argb >> 24) & 0xFF;
        let r = (argb >> 16) & 0xFF;
        let g = (argb >> 8) & 0xFF;
        let b = argb & 0xFF;
        Color::rgba8(r as u8, g as u8, b as u8, a as u8)
    }

    /// Luminance relative (WCAG), calculée sur les canaux **linéarisés** — la base
    /// d'un calcul de contraste. Ignore l'alpha. Résultat dans `[0, 1]`.
    pub fn compute_luminance(self) -> f32 {
        let lin = self.to_linear();
        0.2126 * lin.r + 0.7152 * lin.g + 0.0722 * lin.b
    }

    /// Convertit une composante sRGB (`0..1`) en linéaire.
    fn srgb_to_linear(c: f32) -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Convertit une composante linéaire (`0..1`) en sRGB.
    fn linear_to_srgb(c: f32) -> f32 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    /// Version **linéaire** de cette couleur (les valeurs `Color` sont en sRGB).
    /// À appliquer avant l'envoi au GPU : une cible sRGB ré-encode linéaire→sRGB,
    /// donc envoyer du linéaire restitue la couleur voulue. L'alpha est inchangé.
    pub fn to_linear(self) -> Color {
        Color::rgba(
            Self::srgb_to_linear(self.r),
            Self::srgb_to_linear(self.g),
            Self::srgb_to_linear(self.b),
            self.a,
        )
    }

    /// Inverse de [`Color::to_linear`] : linéaire → sRGB.
    pub fn to_srgb(self) -> Color {
        Color::rgba(
            Self::linear_to_srgb(self.r),
            Self::linear_to_srgb(self.g),
            Self::linear_to_srgb(self.b),
            self.a,
        )
    }

    /// Interpolation linéaire vers `other` (`t` borné à `0.0..=1.0`).
    pub fn lerp(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color::rgba(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Points fixes.
        assert_eq!(
            Color::rgb(0.0, 0.0, 0.0).to_linear(),
            Color::rgb(0.0, 0.0, 0.0)
        );
        let white = Color::rgb(1.0, 1.0, 1.0).to_linear();
        assert!((white.r - 1.0).abs() < 1e-4);
        // Milieu sRGB 0.5 → ~0.214 linéaire.
        let mid = Color::rgb(0.5, 0.5, 0.5).to_linear();
        assert!(
            (mid.r - 0.214).abs() < 0.005,
            "0.5 sRGB → linéaire = {}",
            mid.r
        );
        // Aller-retour.
        let c = Color::rgba(0.2, 0.6, 0.9, 0.5);
        let round = c.to_linear().to_srgb();
        assert!((round.r - 0.2).abs() < 1e-3 && (round.g - 0.6).abs() < 1e-3);
        // Alpha inchangé.
        assert_eq!(c.to_linear().a, 0.5);
    }

    #[test]
    fn hex_parses_css_codes() {
        assert_eq!(Color::hex("#000000"), Color::BLACK);
        assert_eq!(Color::hex("#FFFFFF"), Color::WHITE);
        // Sans '#', casse libre.
        assert_eq!(Color::try_hex("ffffff"), Some(Color::WHITE));
        // Forme courte : #RGB → #RRGGBB.
        assert_eq!(Color::hex("#f00"), Color::rgb8(255, 0, 0));
        // Alpha.
        assert_eq!(Color::hex("#00000080").a, 128.0 / 255.0);
        assert_eq!(Color::try_hex("#0008").unwrap().a, 0x88 as f32 / 255.0);
        // Composante concrète.
        assert_eq!(Color::hex("#3B82F6"), Color::rgb8(0x3B, 0x82, 0xF6));
        // Invalides.
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
        assert_eq!(c.with_alpha(2.0).a, 1.0, "borné à 1");
    }

    #[test]
    fn from_argb_u32_decodes_channels() {
        // 0x80FF0000 = rouge à 50 % d'alpha.
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
        // Le vert pèse plus que le rouge, lui-même plus que le bleu (WCAG).
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

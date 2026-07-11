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

    /// Représentation en tableau `[r, g, b, a]`, prête pour le GPU.
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Multiplie l'opacité (canal alpha) par `opacity` (borné à `0.0..=1.0`).
    pub fn fade(self, opacity: f32) -> Color {
        Color::rgba(self.r, self.g, self.b, self.a * opacity.clamp(0.0, 1.0))
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
        assert_eq!(Color::rgb(0.0, 0.0, 0.0).to_linear(), Color::rgb(0.0, 0.0, 0.0));
        let white = Color::rgb(1.0, 1.0, 1.0).to_linear();
        assert!((white.r - 1.0).abs() < 1e-4);
        // Milieu sRGB 0.5 → ~0.214 linéaire.
        let mid = Color::rgb(0.5, 0.5, 0.5).to_linear();
        assert!((mid.r - 0.214).abs() < 0.005, "0.5 sRGB → linéaire = {}", mid.r);
        // Aller-retour.
        let c = Color::rgba(0.2, 0.6, 0.9, 0.5);
        let round = c.to_linear().to_srgb();
        assert!((round.r - 0.2).abs() < 1e-3 && (round.g - 0.6).abs() < 1e-3);
        // Alpha inchangé.
        assert_eq!(c.to_linear().a, 0.5);
    }

    #[test]
    fn fade_scales_alpha() {
        let c = Color::rgba(0.2, 0.4, 0.6, 0.8);
        assert_eq!(c.fade(0.5), Color::rgba(0.2, 0.4, 0.6, 0.4));
        assert_eq!(c.fade(1.0), c);
        assert_eq!(c.fade(0.0).a, 0.0);
    }
}

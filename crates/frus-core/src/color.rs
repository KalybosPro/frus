//! Couleur RGBA en virgule flottante (composantes 0.0..=1.0).

use bytemuck::{Pod, Zeroable};

/// Une couleur RGBA. Les composantes sont dans `[0.0, 1.0]`.
///
/// À ce stade, les couleurs sont transmises telles quelles au GPU. La gestion
/// fine de l'espace colorimétrique (sRGB vs linéaire) sera traitée dans un
/// jalon ultérieur dédié à la colorimétrie.
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
    fn fade_scales_alpha() {
        let c = Color::rgba(0.2, 0.4, 0.6, 0.8);
        assert_eq!(c.fade(0.5), Color::rgba(0.2, 0.4, 0.6, 0.4));
        assert_eq!(c.fade(1.0), c);
        assert_eq!(c.fade(0.0).a, 0.0);
    }
}

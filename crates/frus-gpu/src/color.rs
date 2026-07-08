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
}

//! Primitives géométriques exprimées en pixels logiques.
//!
//! Convention de coordonnées de frus : origine en **haut à gauche**, axe X vers
//! la droite, axe Y vers le **bas** (comme CSS / Flutter). La conversion vers
//! l'espace NDC du GPU est faite dans le shader.

/// Un rectangle aligné sur les axes, en pixels logiques.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Bord gauche.
    pub x: f32,
    /// Bord supérieur.
    pub y: f32,
    /// Largeur (vers la droite).
    pub width: f32,
    /// Hauteur (vers le bas).
    pub height: f32,
}

impl Rect {
    /// Crée un rectangle depuis sa position (coin haut-gauche) et sa taille.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Représentation `[x, y, width, height]`, prête pour le GPU.
    pub const fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

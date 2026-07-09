//! Primitives géométriques exprimées en pixels logiques.
//!
//! Convention de coordonnées de frus : origine en **haut à gauche**, axe X vers
//! la droite, axe Y vers le **bas** (comme CSS / Flutter).

/// Un point 2D.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Marges (intérieures ou extérieures) par côté, en pixels logiques.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }

    /// La même marge sur les quatre côtés.
    pub const fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }
}

/// Une taille 2D (largeur × hauteur).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

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

    /// Crée un rectangle depuis un point d'origine et une taille.
    pub const fn from_point_size(origin: Point, size: Size) -> Self {
        Self::new(origin.x, origin.y, size.width, size.height)
    }

    /// Coin haut-gauche.
    pub const fn origin(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Taille du rectangle.
    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Représentation `[x, y, width, height]`, prête pour le GPU.
    pub const fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_from_point_size_roundtrips() {
        let r = Rect::from_point_size(Point::new(3.0, 4.0), Size::new(10.0, 20.0));
        assert_eq!(r, Rect::new(3.0, 4.0, 10.0, 20.0));
        assert_eq!(r.origin(), Point::new(3.0, 4.0));
        assert_eq!(r.size(), Size::new(10.0, 20.0));
        assert_eq!(r.to_array(), [3.0, 4.0, 10.0, 20.0]);
    }
}

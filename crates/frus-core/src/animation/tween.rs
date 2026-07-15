//! Interpolation typée : une progression `[0,1]` pilote n'importe quelle valeur
//! interpolable (nombre, couleur, point, taille…).

use crate::{Color, Point, Size};

/// Une valeur interpolable linéairement.
pub trait Lerp: Copy {
    /// Interpole de `self` (à `t=0`) vers `other` (à `t=1`).
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for Color {
    fn lerp(self, other: Self, t: f32) -> Self {
        Color::lerp(self, other, t)
    }
}

impl Lerp for Point {
    fn lerp(self, other: Self, t: f32) -> Self {
        Point::new(self.x.lerp(other.x, t), self.y.lerp(other.y, t))
    }
}

impl Lerp for Size {
    fn lerp(self, other: Self, t: f32) -> Self {
        Size::new(self.width.lerp(other.width, t), self.height.lerp(other.height, t))
    }
}

/// Interpole une valeur entre deux bornes selon une progression `[0,1]`.
///
/// Un seul pilote `[0,1]` (contrôleur, ressort…) anime ainsi arbitrairement de
/// valeurs typées, chacune avec ses propres bornes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween<T> {
    /// Valeur à `t = 0`.
    pub begin: T,
    /// Valeur à `t = 1`.
    pub end: T,
}

impl<T: Lerp> Tween<T> {
    /// Crée un tween de `begin` à `end`.
    pub fn new(begin: T, end: T) -> Self {
        Self { begin, end }
    }

    /// Valeur à la progression `t` (généralement issue d'une [`super::Curve`]).
    pub fn eval(&self, t: f32) -> T {
        self.begin.lerp(self.end, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_tween_interpolates() {
        let t = Tween::new(10.0, 20.0);
        assert_eq!(t.eval(0.0), 10.0);
        assert_eq!(t.eval(0.5), 15.0);
        assert_eq!(t.eval(1.0), 20.0);
    }

    #[test]
    fn color_tween_interpolates() {
        let t = Tween::new(Color::BLACK, Color::WHITE);
        let mid = t.eval(0.5);
        assert_eq!(mid, Color::rgb(0.5, 0.5, 0.5));
    }

    #[test]
    fn point_tween_interpolates() {
        let t = Tween::new(Point::new(0.0, 0.0), Point::new(10.0, 20.0));
        assert_eq!(t.eval(0.5), Point::new(5.0, 10.0));
    }
}

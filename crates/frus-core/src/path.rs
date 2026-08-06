//! Chemins vectoriels : de la géométrie 2D **arbitraire** (segments droits et
//! courbes de Bézier), remplie et/ou tracée. C'est la brique des icônes et du
//! dessin personnalisé (`CustomPaint`), au-delà des rectangles.
//!
//! Un [`Path`] est purement déclaratif — une suite de [`PathVerb`]. Il ne sait
//! rien du GPU : `frus-gpu` le tessellise (→ triangles) au moment du rendu.
//! Coordonnées en pixels logiques, mêmes conventions que [`crate::Rect`]
//! (origine haut-gauche, Y vers le bas).

use crate::{Color, Point, Rect};

/// Une commande élémentaire de tracé.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathVerb {
    /// Lève puis repose le crayon : démarre un nouveau **sous-chemin** en ce point.
    MoveTo(Point),
    /// Segment droit depuis le point courant jusqu'à `to`.
    LineTo(Point),
    /// Courbe de Bézier **quadratique** (un point de contrôle).
    QuadTo { ctrl: Point, to: Point },
    /// Courbe de Bézier **cubique** (deux points de contrôle).
    CubicTo { c1: Point, c2: Point, to: Point },
    /// Ferme le sous-chemin courant (rejoint son point de départ).
    Close,
}

impl PathVerb {
    fn scaled(self, f: f32) -> PathVerb {
        let s = |p: Point| Point::new(p.x * f, p.y * f);
        match self {
            PathVerb::MoveTo(p) => PathVerb::MoveTo(s(p)),
            PathVerb::LineTo(p) => PathVerb::LineTo(s(p)),
            PathVerb::QuadTo { ctrl, to } => PathVerb::QuadTo {
                ctrl: s(ctrl),
                to: s(to),
            },
            PathVerb::CubicTo { c1, c2, to } => PathVerb::CubicTo {
                c1: s(c1),
                c2: s(c2),
                to: s(to),
            },
            PathVerb::Close => PathVerb::Close,
        }
    }

    fn translated(self, dx: f32, dy: f32) -> PathVerb {
        let t = |p: Point| Point::new(p.x + dx, p.y + dy);
        match self {
            PathVerb::MoveTo(p) => PathVerb::MoveTo(t(p)),
            PathVerb::LineTo(p) => PathVerb::LineTo(t(p)),
            PathVerb::QuadTo { ctrl, to } => PathVerb::QuadTo {
                ctrl: t(ctrl),
                to: t(to),
            },
            PathVerb::CubicTo { c1, c2, to } => PathVerb::CubicTo {
                c1: t(c1),
                c2: t(c2),
                to: t(to),
            },
            PathVerb::Close => PathVerb::Close,
        }
    }
}

/// Un **contour** (le trait d'un tracé) : couleur et épaisseur, en pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

impl Stroke {
    /// Un contour de couleur et d'épaisseur données.
    pub const fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
}

/// Un chemin vectoriel : une suite de [`PathVerb`]. Se construit façon *builder*
/// (chaînable) ; se consomme au rendu.
///
/// ```
/// use frus_core::{Path, Point};
/// let triangle = Path::new()
///     .move_to(Point::new(0.0, 0.0))
///     .line_to(Point::new(10.0, 0.0))
///     .line_to(Point::new(5.0, 8.0))
///     .close();
/// assert_eq!(triangle.verbs().len(), 4);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    verbs: Vec<PathVerb>,
}

impl Path {
    /// Un chemin vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Démarre un sous-chemin en `p`.
    pub fn move_to(mut self, p: Point) -> Self {
        self.verbs.push(PathVerb::MoveTo(p));
        self
    }

    /// Segment droit jusqu'à `p`.
    pub fn line_to(mut self, p: Point) -> Self {
        self.verbs.push(PathVerb::LineTo(p));
        self
    }

    /// Courbe quadratique (un point de contrôle `ctrl`) jusqu'à `to`.
    pub fn quad_to(mut self, ctrl: Point, to: Point) -> Self {
        self.verbs.push(PathVerb::QuadTo { ctrl, to });
        self
    }

    /// Courbe cubique (deux points de contrôle `c1`, `c2`) jusqu'à `to`.
    pub fn cubic_to(mut self, c1: Point, c2: Point, to: Point) -> Self {
        self.verbs.push(PathVerb::CubicTo { c1, c2, to });
        self
    }

    /// Ferme le sous-chemin courant.
    pub fn close(mut self) -> Self {
        self.verbs.push(PathVerb::Close);
        self
    }

    /// Les commandes du chemin, dans l'ordre.
    pub fn verbs(&self) -> &[PathVerb] {
        &self.verbs
    }

    /// `true` si le chemin ne contient aucune commande.
    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }

    /// Un rectangle plein (contour fermé, sens horaire).
    pub fn rect(r: Rect) -> Self {
        Self::new()
            .move_to(Point::new(r.x, r.y))
            .line_to(Point::new(r.x + r.width, r.y))
            .line_to(Point::new(r.x + r.width, r.y + r.height))
            .line_to(Point::new(r.x, r.y + r.height))
            .close()
    }

    /// Un cercle, approximé par **quatre arcs cubiques** (constante de Bézier
    /// `0.5523` pour un cercle exact aux nœuds).
    pub fn circle(center: Point, radius: f32) -> Self {
        const K: f32 = 0.552_284_75;
        let (cx, cy, r) = (center.x, center.y, radius);
        let k = r * K;
        Self::new()
            .move_to(Point::new(cx, cy - r))
            .cubic_to(
                Point::new(cx + k, cy - r),
                Point::new(cx + r, cy - k),
                Point::new(cx + r, cy),
            )
            .cubic_to(
                Point::new(cx + r, cy + k),
                Point::new(cx + k, cy + r),
                Point::new(cx, cy + r),
            )
            .cubic_to(
                Point::new(cx - k, cy + r),
                Point::new(cx - r, cy + k),
                Point::new(cx - r, cy),
            )
            .cubic_to(
                Point::new(cx - r, cy - k),
                Point::new(cx - k, cy - r),
                Point::new(cx, cy - r),
            )
            .close()
    }

    /// Copie mise à l'échelle par `factor` (autour de l'origine) — conversion
    /// logique → physique, ou adaptation d'une icône `24×24` à sa taille réelle.
    pub fn scaled(&self, factor: f32) -> Path {
        Path {
            verbs: self.verbs.iter().map(|v| v.scaled(factor)).collect(),
        }
    }

    /// Copie translatée de `(dx, dy)` — pour placer une icône dans sa boîte.
    pub fn translated(&self, dx: f32, dy: f32) -> Path {
        Path {
            verbs: self.verbs.iter().map(|v| v.translated(dx, dy)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_records_verbs_in_order() {
        let p = Path::new()
            .move_to(Point::new(1.0, 2.0))
            .line_to(Point::new(3.0, 4.0))
            .quad_to(Point::new(5.0, 6.0), Point::new(7.0, 8.0))
            .close();
        assert_eq!(
            p.verbs(),
            &[
                PathVerb::MoveTo(Point::new(1.0, 2.0)),
                PathVerb::LineTo(Point::new(3.0, 4.0)),
                PathVerb::QuadTo {
                    ctrl: Point::new(5.0, 6.0),
                    to: Point::new(7.0, 8.0)
                },
                PathVerb::Close,
            ]
        );
    }

    #[test]
    fn rect_is_a_closed_quad() {
        let p = Path::rect(Rect::new(0.0, 0.0, 10.0, 20.0));
        // move + 3× line + close = 5 commandes (le 4ᵉ côté est fermé par `close`).
        assert_eq!(p.verbs().len(), 5);
        assert!(matches!(p.verbs().last(), Some(PathVerb::Close)));
    }

    #[test]
    fn scaled_and_translated_transform_points() {
        let p = Path::new()
            .move_to(Point::new(2.0, 3.0))
            .line_to(Point::new(4.0, 5.0));
        let s = p.scaled(2.0);
        assert_eq!(s.verbs()[0], PathVerb::MoveTo(Point::new(4.0, 6.0)));
        let t = p.translated(10.0, 100.0);
        assert_eq!(t.verbs()[1], PathVerb::LineTo(Point::new(14.0, 105.0)));
    }

    #[test]
    fn circle_starts_at_top_and_closes() {
        let c = Path::circle(Point::new(12.0, 12.0), 10.0);
        assert_eq!(c.verbs()[0], PathVerb::MoveTo(Point::new(12.0, 2.0)));
        assert!(matches!(c.verbs().last(), Some(PathVerb::Close)));
    }
}

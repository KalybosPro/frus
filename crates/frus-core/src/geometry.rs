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

    /// Multiplie les coordonnées par `factor` (conversion logique → physique).
    pub fn scale(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor)
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

/// Les insets **fenêtre**, séparés par nature (façon `MediaQuery` de Flutter) :
/// `padding` = zones occupées **en permanence** par le système (barres d'état/
/// navigation, encoche — statiques) ; `view_insets` = zones couvertes par une UI
/// **transitoire** (clavier logiciel — dynamiques). L'évitement du clavier
/// consiste à écarter le contenu de [`WindowInsets::safe`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowInsets {
    /// Zones système permanentes (barres, encoche).
    pub padding: Insets,
    /// Zones transitoires (clavier logiciel) — seul le bas bouge en pratique.
    pub view_insets: Insets,
}

impl WindowInsets {
    /// Aucun inset.
    pub const ZERO: Self = Self {
        padding: Insets::ZERO,
        view_insets: Insets::ZERO,
    };

    /// La zone à éviter au total : le **maximum** côté à côté des deux natures
    /// (le clavier recouvre la barre de navigation — on n'additionne pas).
    pub fn safe(&self) -> Insets {
        Insets::new(
            self.padding.top.max(self.view_insets.top),
            self.padding.right.max(self.view_insets.right),
            self.padding.bottom.max(self.view_insets.bottom),
            self.padding.left.max(self.view_insets.left),
        )
    }

    /// Sépare des insets bruts en `(padding, view_insets)` à partir d'une
    /// **référence sans clavier**. Un excédent **bas** au-delà de la référence
    /// signale le clavier ; `view_insets.bottom` mesure alors l'occultation
    /// **totale depuis le bord** de la fenêtre (barre comprise — la convention
    /// `MediaQuery.viewInsets`, qui rend la combinaison par `max` correcte).
    pub fn from_baseline(baseline: Insets, current: Insets) -> WindowInsets {
        let keyboard = (current.bottom - baseline.bottom).max(0.0);
        WindowInsets {
            padding: Insets::new(
                current.top,
                current.right,
                current.bottom - keyboard,
                current.left,
            ),
            view_insets: Insets::new(
                0.0,
                0.0,
                if keyboard > 0.0 { current.bottom } else { 0.0 },
                0.0,
            ),
        }
    }
}

impl Default for WindowInsets {
    fn default() -> Self {
        Self::ZERO
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
    /// Un rectangle « infini » servant de découpe neutre (aucun clipping).
    pub const UNBOUNDED: Rect = Rect::new(-1.0e7, -1.0e7, 2.0e7, 2.0e7);

    /// Crée un rectangle depuis sa position (coin haut-gauche) et sa taille.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Décale le rectangle de `(dx, dy)`.
    pub fn translate(self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// Multiplie position et taille par `factor` (conversion logique → physique).
    pub fn scale(self, factor: f32) -> Self {
        Self::new(
            self.x * factor,
            self.y * factor,
            self.width * factor,
            self.height * factor,
        )
    }

    /// Intersection de deux rectangles (taille nulle s'ils sont disjoints).
    pub fn intersect(self, other: Rect) -> Self {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.width).min(other.x + other.width);
        let y1 = (self.y + self.height).min(other.y + other.height);
        Self::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }

    /// `true` si `point` est dans le rectangle (bord gauche/haut inclus).
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
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
    fn window_insets_split_and_safe_area() {
        // Référence sans clavier : barres système haut/bas.
        let baseline = Insets::new(84.0, 0.0, 45.0, 0.0);

        // Clavier fermé : tout est du padding statique, rien de transitoire.
        let closed = WindowInsets::from_baseline(baseline, baseline);
        assert_eq!(closed.padding, baseline);
        assert_eq!(closed.view_insets, Insets::ZERO);
        assert_eq!(closed.safe(), baseline);

        // Clavier ouvert (excédent bas 300) : `view_insets.bottom` mesure
        // l'occultation totale depuis le bord (345, barre comprise).
        let open = WindowInsets::from_baseline(baseline, Insets::new(84.0, 0.0, 345.0, 0.0));
        assert_eq!(open.padding, baseline);
        assert_eq!(open.view_insets, Insets::new(0.0, 0.0, 345.0, 0.0));
        // La zone sûre = max côté à côté (le clavier recouvre la barre).
        assert_eq!(open.safe(), Insets::new(84.0, 0.0, 345.0, 0.0));

        // Bas courant SOUS la référence (barres masquées) : pas de clavier négatif.
        let hidden = WindowInsets::from_baseline(baseline, Insets::new(84.0, 0.0, 10.0, 0.0));
        assert_eq!(hidden.view_insets, Insets::ZERO);
        assert_eq!(hidden.padding.bottom, 10.0);
    }

    #[test]
    fn rect_from_point_size_roundtrips() {
        let r = Rect::from_point_size(Point::new(3.0, 4.0), Size::new(10.0, 20.0));
        assert_eq!(r, Rect::new(3.0, 4.0, 10.0, 20.0));
        assert_eq!(r.origin(), Point::new(3.0, 4.0));
        assert_eq!(r.size(), Size::new(10.0, 20.0));
        assert_eq!(r.to_array(), [3.0, 4.0, 10.0, 20.0]);
    }
}

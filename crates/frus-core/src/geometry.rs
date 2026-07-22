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

/// La **direction de lecture** du texte et de la mise en page. Contexte
/// ambiant (façon `Directionality` de Flutter) : en RTL, les rangées, l'aligne-
/// ment et les marges directionnelles se retournent horizontalement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextDirection {
    /// Gauche → droite (latin, par défaut).
    #[default]
    Ltr,
    /// Droite → gauche (arabe, hébreu…).
    Rtl,
}

impl TextDirection {
    /// `true` si droite-à-gauche.
    pub fn is_rtl(self) -> bool {
        matches!(self, TextDirection::Rtl)
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

/// Marges **directionnelles** : `start`/`end` au lieu de `left`/`right`. En LTR,
/// `start` = gauche ; en RTL, `start` = droite. Résolues en [`Insets`] concrets
/// au moment de la mise en page (façon `EdgeInsetsDirectional` de Flutter).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct InsetsDirectional {
    pub top: f32,
    pub end: f32,
    pub bottom: f32,
    pub start: f32,
}

impl InsetsDirectional {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(top: f32, end: f32, bottom: f32, start: f32) -> Self {
        Self { top, end, bottom, start }
    }

    /// Marge `start`/`end` symétrique horizontale (façon `symmetric`).
    pub const fn horizontal(value: f32) -> Self {
        Self::new(0.0, value, 0.0, value)
    }

    /// Résout en marges concrètes selon la direction : en RTL, `start`↔droite.
    pub fn resolve(self, direction: TextDirection) -> Insets {
        let (left, right) = if direction.is_rtl() {
            (self.end, self.start)
        } else {
            (self.start, self.end)
        };
        Insets::new(self.top, right, self.bottom, left)
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

/// Où poser un enfant dans la boîte qui le contient, façon `Alignment` de Flutter :
/// deux fractions **continues** `[-1, 1]`. `x = -1` colle à gauche, `0` centre,
/// `+1` colle à droite ; `y = -1` en haut, `+1` en bas. Étant continu, il
/// **s'interpole** ([`crate::Lerp`]) — un `Tween<Alignment>` glisse un enfant d'un
/// ancrage à l'autre. Les neuf ancrages usuels sont fournis comme constantes
/// (`Alignment::CENTER`, `Alignment::TOP_LEFT`…).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Alignment {
    /// Fraction horizontale : `-1` (gauche) … `0` (centre) … `+1` (droite).
    pub x: f32,
    /// Fraction verticale : `-1` (haut) … `0` (centre) … `+1` (bas).
    pub y: f32,
}

impl Alignment {
    pub const TOP_LEFT: Self = Self::new(-1.0, -1.0);
    pub const TOP_CENTER: Self = Self::new(0.0, -1.0);
    pub const TOP_RIGHT: Self = Self::new(1.0, -1.0);
    pub const CENTER_LEFT: Self = Self::new(-1.0, 0.0);
    pub const CENTER: Self = Self::new(0.0, 0.0);
    pub const CENTER_RIGHT: Self = Self::new(1.0, 0.0);
    pub const BOTTOM_LEFT: Self = Self::new(-1.0, 1.0);
    pub const BOTTOM_CENTER: Self = Self::new(0.0, 1.0);
    pub const BOTTOM_RIGHT: Self = Self::new(1.0, 1.0);

    /// Un ancrage `(x, y)`, fractions dans `[-1, 1]`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Fraction horizontale ramenée dans `[0, 1]` (`0` = gauche, `1` = droite) —
    /// la part de l'espace libre à laisser **avant** l'enfant sur l'axe x.
    pub fn fraction_x(self) -> f32 {
        ((self.x + 1.0) * 0.5).clamp(0.0, 1.0)
    }

    /// Fraction verticale ramenée dans `[0, 1]` (`0` = haut, `1` = bas).
    pub fn fraction_y(self) -> f32 {
        ((self.y + 1.0) * 0.5).clamp(0.0, 1.0)
    }
}

/// Ancrage **directionnel** (façon `AlignmentDirectional` de Flutter) : l'axe
/// horizontal est exprimé **début → fin** au lieu de gauche → droite. `x_start = -1`
/// colle au bord de **début** (gauche en LTR, droite en RTL), `+1` au bord de
/// **fin**. Résolu en [`Alignment`] physique au rendu selon la direction de lecture
/// — l'ancrage suit le texte sans que l'appelant ne teste le sens.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct AlignmentDirectional {
    /// Fraction début→fin : `-1` (début) … `0` (centre) … `+1` (fin).
    pub x_start: f32,
    /// Fraction verticale : `-1` (haut) … `+1` (bas).
    pub y: f32,
}

impl AlignmentDirectional {
    pub const TOP_START: Self = Self::new(-1.0, -1.0);
    pub const TOP_CENTER: Self = Self::new(0.0, -1.0);
    pub const TOP_END: Self = Self::new(1.0, -1.0);
    pub const CENTER_START: Self = Self::new(-1.0, 0.0);
    pub const CENTER: Self = Self::new(0.0, 0.0);
    pub const CENTER_END: Self = Self::new(1.0, 0.0);
    pub const BOTTOM_START: Self = Self::new(-1.0, 1.0);
    pub const BOTTOM_CENTER: Self = Self::new(0.0, 1.0);
    pub const BOTTOM_END: Self = Self::new(1.0, 1.0);

    /// Un ancrage directionnel `(x_start, y)`, fractions dans `[-1, 1]`.
    pub const fn new(x_start: f32, y: f32) -> Self {
        Self { x_start, y }
    }

    /// Résout en ancrage **physique** : en RTL, début ↔ droite (x inversé) ; en
    /// LTR, début = gauche (x inchangé). Le `y` ne dépend pas de la direction.
    pub fn resolve(self, direction: TextDirection) -> Alignment {
        let x = if direction.is_rtl() { -self.x_start } else { self.x_start };
        Alignment::new(x, self.y)
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
    fn directional_alignment_resolves_by_direction() {
        // start = bord de lecture : gauche en LTR, droite en RTL.
        let start = AlignmentDirectional::CENTER_START;
        assert_eq!(start.resolve(TextDirection::Ltr), Alignment::CENTER_LEFT);
        assert_eq!(start.resolve(TextDirection::Rtl), Alignment::CENTER_RIGHT);
        // Le centre et le vertical ne dépendent pas de la direction.
        let top = AlignmentDirectional::TOP_CENTER;
        assert_eq!(top.resolve(TextDirection::Rtl), Alignment::TOP_CENTER);
    }

    #[test]
    fn directional_insets_flip_start_end() {
        // start=10, end=4 : en LTR start→gauche ; en RTL start→droite.
        let d = InsetsDirectional::new(1.0, 4.0, 2.0, 10.0);
        let ltr = d.resolve(TextDirection::Ltr);
        assert_eq!((ltr.left, ltr.right, ltr.top, ltr.bottom), (10.0, 4.0, 1.0, 2.0));
        let rtl = d.resolve(TextDirection::Rtl);
        assert_eq!((rtl.left, rtl.right), (4.0, 10.0), "start passe à droite en RTL");
        // Le vertical ne bouge jamais.
        assert_eq!((rtl.top, rtl.bottom), (1.0, 2.0));
    }

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

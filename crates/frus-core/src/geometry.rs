//! Primitives géométriques exprimées en pixels logiques.
//!
//! Convention de coordonnées de frus : origine en **haut à gauche**, axe X vers
//! la droite, axe Y vers le **bas** (comme CSS / Flutter).

/// Une **transformation affine 2D** (matrice 2×3). Un point `(x, y)` devient
/// `(a·x + c·y + e, b·x + d·y + f)` — soit une partie linéaire 2×2 `[[a, c], [b, d]]`
/// (échelle, rotation, cisaillement) plus une translation `(e, f)`.
///
/// Les composées se lisent « de droite à gauche » : `a.then(b)` applique `a` **puis**
/// `b`. C'est la représentation unifiée des transformations de peinture d'un
/// [`crate::LayerTransform`] : translation, échelle (uniforme ou par axe) et rotation
/// se fondent en **une seule** matrice, sans approximation de composition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    /// `[a, b, c, d, e, f]` : `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
    pub m: [f32; 6],
}

impl Default for Affine {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine {
    /// L'identité (aucune transformation).
    pub const IDENTITY: Affine = Affine { m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] };

    /// Une translation pure de `(tx, ty)`.
    pub const fn translation(tx: f32, ty: f32) -> Affine {
        Affine { m: [1.0, 0.0, 0.0, 1.0, tx, ty] }
    }

    /// Une mise à l'échelle par axe (autour de l'origine).
    pub const fn scale(sx: f32, sy: f32) -> Affine {
        Affine { m: [sx, 0.0, 0.0, sy, 0.0, 0.0] }
    }

    /// Une rotation d'`angle` radians (sens horaire, y vers le bas), autour de
    /// l'origine.
    pub fn rotation(angle: f32) -> Affine {
        let (s, c) = angle.sin_cos();
        Affine { m: [c, s, -s, c, 0.0, 0.0] }
    }

    /// Composition : `self.then(next)` = appliquer `self` **puis** `next`.
    pub fn then(self, next: Affine) -> Affine {
        // next ∘ self : on développe next(self(p)).
        let [a, b, c, d, e, f] = self.m;
        let [a2, b2, c2, d2, e2, f2] = next.m;
        Affine {
            m: [
                a2 * a + c2 * b,
                b2 * a + d2 * b,
                a2 * c + c2 * d,
                b2 * c + d2 * d,
                a2 * e + c2 * f + e2,
                b2 * e + d2 * f + f2,
            ],
        }
    }

    /// La même transformation, mais **autour de `pivot`** (le pivot reste fixe) :
    /// `T(pivot) ∘ self ∘ T(-pivot)`.
    pub fn about(self, pivot: Point) -> Affine {
        Affine::translation(-pivot.x, -pivot.y)
            .then(self)
            .then(Affine::translation(pivot.x, pivot.y))
    }

    /// Applique la transformation à un point.
    pub fn apply(self, p: Point) -> Point {
        let [a, b, c, d, e, f] = self.m;
        Point::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f)
    }

    /// La transformation inverse (l'identité si la matrice est dégénérée).
    pub fn inverse(self) -> Affine {
        let [a, b, c, d, e, f] = self.m;
        let det = a * d - b * c;
        if det.abs() < f32::EPSILON {
            return Affine::IDENTITY;
        }
        let inv = 1.0 / det;
        Affine {
            m: [
                d * inv,
                -b * inv,
                -c * inv,
                a * inv,
                (c * f - d * e) * inv,
                (b * e - a * f) * inv,
            ],
        }
    }
}

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

    /// Multiplie les coordonnées **par axe** (`sx`, `sy`).
    pub fn scale_xy(self, sx: f32, sy: f32) -> Self {
        Self::new(self.x * sx, self.y * sy)
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

/// Un ancrage **résoluble** — soit physique ([`Alignment`]), soit directionnel
/// ([`AlignmentDirectional`]) : l'abstraction commune que Flutter nomme
/// `AlignmentGeometry`. Un widget l'accepte indifféremment (via `Into`) et le
/// résout selon la direction de lecture au rendu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlignmentGeometry {
    /// Ancrage physique (gauche/droite absolus).
    Physical(Alignment),
    /// Ancrage directionnel (début/fin, retourné en RTL).
    Directional(AlignmentDirectional),
}

impl AlignmentGeometry {
    /// Résout en ancrage physique selon la direction de lecture (un ancrage déjà
    /// physique est renvoyé tel quel).
    pub fn resolve(self, direction: TextDirection) -> Alignment {
        match self {
            AlignmentGeometry::Physical(a) => a,
            AlignmentGeometry::Directional(d) => d.resolve(direction),
        }
    }
}

impl From<Alignment> for AlignmentGeometry {
    fn from(a: Alignment) -> Self {
        AlignmentGeometry::Physical(a)
    }
}

impl From<AlignmentDirectional> for AlignmentGeometry {
    fn from(d: AlignmentDirectional) -> Self {
        AlignmentGeometry::Directional(d)
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

    /// Met le rectangle à l'échelle par `factor` **autour de `pivot`** (le pivot
    /// reste fixe) : `pos' = pivot + (pos - pivot) * factor`, taille × `factor`.
    pub fn scale_about(self, pivot: Point, factor: f32) -> Self {
        self.scale_about_xy(pivot, factor, factor)
    }

    /// Met le rectangle à l'échelle **par axe** autour de `pivot`.
    pub fn scale_about_xy(self, pivot: Point, sx: f32, sy: f32) -> Self {
        Self::new(
            pivot.x + (self.x - pivot.x) * sx,
            pivot.y + (self.y - pivot.y) * sy,
            self.width * sx,
            self.height * sy,
        )
    }

    /// Multiplie position et taille **par axe** (`sx`, `sy`).
    pub fn scale_xy(self, sx: f32, sy: f32) -> Self {
        Self::new(self.x * sx, self.y * sy, self.width * sx, self.height * sy)
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
    fn affine_composes_scale_then_rotate_about_a_pivot() {
        use std::f32::consts::FRAC_PI_2;
        let pivot = Point::new(10.0, 10.0);
        // Échelle ×2 puis rotation +90°, toutes deux autour du pivot.
        let m = Affine::scale(2.0, 2.0)
            .about(pivot)
            .then(Affine::rotation(FRAC_PI_2).about(pivot));
        // Le pivot est fixe.
        let c = m.apply(pivot);
        assert!((c.x - 10.0).abs() < 1e-3 && (c.y - 10.0).abs() < 1e-3, "pivot fixe : {c:?}");
        // Partie linéaire = rotation(90°) ∘ échelle(2) = [0, 2, -2, 0].
        assert!(m.m[0].abs() < 1e-3 && (m.m[1] - 2.0).abs() < 1e-3);
        assert!((m.m[2] + 2.0).abs() < 1e-3 && m.m[3].abs() < 1e-3);
    }

    #[test]
    fn affine_inverse_round_trips() {
        use std::f32::consts::FRAC_PI_3;
        let m = Affine::scale(1.5, 0.5)
            .about(Point::new(4.0, 7.0))
            .then(Affine::rotation(FRAC_PI_3).about(Point::new(2.0, 3.0)))
            .then(Affine::translation(5.0, -2.0));
        let inv = m.inverse();
        for p in [Point::new(0.0, 0.0), Point::new(12.0, -3.0), Point::new(-5.0, 8.0)] {
            let back = inv.apply(m.apply(p));
            assert!((back.x - p.x).abs() < 1e-2 && (back.y - p.y).abs() < 1e-2, "{p:?} -> {back:?}");
        }
    }

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
    fn alignment_geometry_unifies_physical_and_directional() {
        // Physique : résolu tel quel, quelle que soit la direction.
        let phys: AlignmentGeometry = Alignment::CENTER_LEFT.into();
        assert_eq!(phys.resolve(TextDirection::Rtl), Alignment::CENTER_LEFT);
        // Directionnel : suit le sens de lecture.
        let dir: AlignmentGeometry = AlignmentDirectional::CENTER_START.into();
        assert_eq!(dir.resolve(TextDirection::Ltr), Alignment::CENTER_LEFT);
        assert_eq!(dir.resolve(TextDirection::Rtl), Alignment::CENTER_RIGHT);
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

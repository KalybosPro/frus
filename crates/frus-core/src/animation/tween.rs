//! Interpolation typée : une progression `[0,1]` pilote n'importe quelle valeur
//! interpolable (nombre, couleur, point, taille…).

use super::controller::{AnimationController, Status};
use super::curve::Curve;
use crate::{Alignment, BorderRadius, Color, Insets, Point, Size};

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

impl Lerp for Insets {
    fn lerp(self, other: Self, t: f32) -> Self {
        Insets::new(
            self.top.lerp(other.top, t),
            self.right.lerp(other.right, t),
            self.bottom.lerp(other.bottom, t),
            self.left.lerp(other.left, t),
        )
    }
}

impl Lerp for BorderRadius {
    fn lerp(self, other: Self, t: f32) -> Self {
        BorderRadius {
            top_left: self.top_left.lerp(other.top_left, t),
            top_right: self.top_right.lerp(other.top_right, t),
            bottom_right: self.bottom_right.lerp(other.bottom_right, t),
            bottom_left: self.bottom_left.lerp(other.bottom_left, t),
        }
    }
}

impl Lerp for Alignment {
    fn lerp(self, other: Self, t: f32) -> Self {
        Alignment::new(self.x.lerp(other.x, t), self.y.lerp(other.y, t))
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

/// Une valeur **façonnable** par une progression `[0,1]` — l'abstraction que
/// partagent tweens et courbes (Flutter `Animatable`). C'est le pont entre le
/// versant *explicite* (un [`AnimationController`] qui produit un `[0,1]` frame par
/// frame) et une valeur *typée* : `tween.animate(&controller).value()` lit à tout
/// instant la couleur / taille / point courant, sans que la vue ne connaisse le
/// contrôleur autrement que par cette valeur.
pub trait Animatable {
    /// Le type de valeur produit (couleur, taille, nombre…).
    type Output;

    /// Valeur à la progression `t ∈ [0,1]`.
    fn evaluate(&self, t: f32) -> Self::Output;

    /// Enchaîne une [`Curve`] **avant** l'évaluation : `t` est d'abord façonné par
    /// la courbe (façon `CurveTween` de Flutter). Une seule progression linéaire
    /// pilote ainsi une valeur au timing non linéaire.
    fn curved(self, curve: Curve) -> Curved<Self>
    where
        Self: Sized,
    {
        Curved { inner: self, curve }
    }

    /// Lie cette animation à la progression d'un [`AnimationController`], produisant
    /// une [`Animation`] dont `value()` suit la valeur courante du contrôleur. La
    /// valeur du contrôleur est **normalisée** par ses bornes, si bien qu'un
    /// contrôleur non unitaire pilote quand même un `[0,1]` complet.
    fn animate<'a>(&'a self, controller: &'a AnimationController) -> Animation<'a, Self>
    where
        Self: Sized,
    {
        Animation { animatable: self, controller }
    }
}

impl<T: Lerp> Animatable for Tween<T> {
    type Output = T;

    fn evaluate(&self, t: f32) -> T {
        self.eval(t)
    }
}

/// Un [`Animatable`] dont la progression est d'abord façonnée par une [`Curve`]
/// (résultat de [`Animatable::curved`]).
#[derive(Clone, Debug)]
pub struct Curved<A> {
    inner: A,
    curve: Curve,
}

impl<A: Animatable> Animatable for Curved<A> {
    type Output = A::Output;

    fn evaluate(&self, t: f32) -> Self::Output {
        self.inner.evaluate(self.curve.transform(t))
    }
}

/// Une valeur typée **vivante** : un [`Animatable`] lié à un [`AnimationController`]
/// (résultat de [`Animatable::animate`]). `value()` échantillonne le contrôleur à
/// l'instant présent — c'est ce que la vue lit au paint.
pub struct Animation<'a, A: Animatable> {
    animatable: &'a A,
    controller: &'a AnimationController,
}

impl<A: Animatable> Animation<'_, A> {
    /// Valeur typée courante : progression normalisée du contrôleur, évaluée.
    pub fn value(&self) -> A::Output {
        let (lower, upper) = self.controller.bounds();
        let t = if upper > lower {
            (self.controller.value() - lower) / (upper - lower)
        } else {
            0.0
        };
        self.animatable.evaluate(t.clamp(0.0, 1.0))
    }

    /// Statut du contrôleur sous-jacent.
    pub fn status(&self) -> Status {
        self.controller.status()
    }

    /// `true` si le contrôleur sous-jacent anime encore.
    pub fn is_animating(&self) -> bool {
        self.controller.is_animating()
    }
}

/// Une suite de segments [`Animatable`] enchaînés sur la progression `[0,1]`, façon
/// `TweenSequence` de Flutter : chaque segment reçoit une **part** proportionnelle à
/// son poids. Ainsi une seule progression traverse plusieurs étapes — un morph en
/// plusieurs temps (couleur A → B → C), un rebond (grossir puis revenir), une
/// séquence à rythmes distincts (un segment `.curved`, l'autre linéaire).
///
/// `TweenSequence` est **lui-même** un `Animatable` : il se `.curved()` et
/// s'`.animate()` comme n'importe quel tween.
pub struct TweenSequence<T> {
    /// `(segment, poids)`. Toujours au moins un (garanti par [`new`](Self::new)).
    items: Vec<(Box<dyn Animatable<Output = T>>, f32)>,
    total_weight: f32,
}

impl<T> TweenSequence<T> {
    /// Démarre une suite avec son premier segment et son poids (poids négatif borné
    /// à zéro).
    pub fn new(first: impl Animatable<Output = T> + 'static, weight: f32) -> Self {
        let w = weight.max(0.0);
        Self { items: vec![(Box::new(first), w)], total_weight: w }
    }

    /// Enchaîne un segment de plus, occupant `weight` de la progression totale.
    pub fn then(mut self, next: impl Animatable<Output = T> + 'static, weight: f32) -> Self {
        let w = weight.max(0.0);
        self.total_weight += w;
        self.items.push((Box::new(next), w));
        self
    }
}

impl<T> Animatable for TweenSequence<T> {
    type Output = T;

    fn evaluate(&self, t: f32) -> T {
        let t = t.clamp(0.0, 1.0);
        let last = self.items.len() - 1;
        // Tous les poids nuls : pas de partition possible → dernier segment.
        if self.total_weight <= 0.0 {
            return self.items[last].0.evaluate(t);
        }
        let target = t * self.total_weight;
        let mut acc = 0.0;
        for (i, (seg, w)) in self.items.iter().enumerate() {
            // Le dernier segment attrape le reste (robuste aux arrondis).
            if i == last || target <= acc + *w {
                let local = if *w > 0.0 { (target - acc) / *w } else { 0.0 };
                return seg.evaluate(local.clamp(0.0, 1.0));
            }
            acc += *w;
        }
        unreachable!("le dernier segment attrape toujours")
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

    /// `tween.animate(&controller)` : la valeur typée suit le contrôleur — au repos
    /// bas → `begin`, puis vers `end` une fois l'animation terminée.
    #[test]
    fn animate_follows_controller() {
        let mut ctrl = AnimationController::unit();
        let tween = Tween::new(Size::new(100.0, 40.0), Size::new(200.0, 80.0));
        assert_eq!(tween.animate(&ctrl).value(), Size::new(100.0, 40.0));
        assert_eq!(tween.animate(&ctrl).status(), Status::Dismissed);

        ctrl.forward(0.2, Curve::Linear);
        while ctrl.tick(0.016) {}
        assert_eq!(tween.animate(&ctrl).value(), Size::new(200.0, 80.0));
        assert_eq!(tween.animate(&ctrl).status(), Status::Completed);
    }

    /// `.curved(...)` façonne la progression avant l'évaluation : à mi-course d'un
    /// `ease_in`, la valeur est **en deçà** du milieu linéaire.
    #[test]
    fn curved_reshapes_progression() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.5);
        let linear = Tween::new(0.0f32, 100.0);
        let eased = linear.curved(Curve::ease_in());
        let mid = eased.animate(&ctrl).value();
        assert!(mid < 50.0, "ease_in en deçà du milieu linéaire : {mid}");
        assert!(mid > 0.0);
        // Les bornes restent atteintes (à la tolérance du solveur de bézier près).
        ctrl.set_value(0.0);
        assert!(eased.animate(&ctrl).value().abs() < 0.5);
        ctrl.set_value(1.0);
        assert!((eased.animate(&ctrl).value() - 100.0).abs() < 0.5);
    }

    /// Un contrôleur non unitaire pilote quand même un `[0,1]` complet : la valeur
    /// est normalisée par ses bornes.
    #[test]
    fn non_unit_bounds_are_normalized() {
        let mut ctrl = AnimationController::new(0.0, 2.0);
        ctrl.set_value(1.0); // milieu de [0,2] → t = 0.5
        let tween = Tween::new(Color::BLACK, Color::WHITE);
        assert_eq!(tween.animate(&ctrl).value(), Color::rgb(0.5, 0.5, 0.5));
    }

    #[test]
    fn alignment_tween_slides_between_anchors() {
        let slide = Tween::new(Alignment::TOP_LEFT, Alignment::BOTTOM_RIGHT);
        assert_eq!(slide.eval(0.0), Alignment::TOP_LEFT);
        assert_eq!(slide.eval(0.5), Alignment::CENTER);
        assert_eq!(slide.eval(1.0), Alignment::BOTTOM_RIGHT);
    }

    #[test]
    fn insets_and_radius_tween_interpolate() {
        let pad = Tween::new(Insets::uniform(0.0), Insets::uniform(20.0));
        assert_eq!(pad.eval(0.5), Insets::uniform(10.0));
        let radius = Tween::new(BorderRadius::uniform(4.0), BorderRadius::uniform(24.0));
        assert_eq!(radius.eval(0.5), BorderRadius::uniform(14.0));
    }

    /// `TweenSequence` à poids égaux : deux segments qui se relaient à `t = 0.5`,
    /// chacun parcouru en entier sur sa moitié.
    #[test]
    fn tween_sequence_relays_equal_weight_segments() {
        let seq = TweenSequence::new(Tween::new(0.0f32, 10.0), 1.0)
            .then(Tween::new(10.0, 30.0), 1.0);
        assert_eq!(seq.evaluate(0.0), 0.0);
        assert_eq!(seq.evaluate(0.25), 5.0); // milieu du 1er segment
        assert_eq!(seq.evaluate(0.5), 10.0); // couture
        assert_eq!(seq.evaluate(0.75), 20.0); // milieu du 2e segment
        assert_eq!(seq.evaluate(1.0), 30.0);
    }

    /// Poids inégaux : le segment le plus lourd occupe une plus grande part de la
    /// progression.
    #[test]
    fn tween_sequence_honors_weights() {
        // 3 parts pour le 1er, 1 part pour le 2e → couture à t = 0.75.
        let seq = TweenSequence::new(Tween::new(0.0f32, 100.0), 3.0)
            .then(Tween::new(100.0, 200.0), 1.0);
        assert_eq!(seq.evaluate(0.75), 100.0); // fin du 1er / début du 2e
        assert_eq!(seq.evaluate(0.375), 50.0); // milieu du 1er (0.375 / 0.75)
        assert_eq!(seq.evaluate(0.875), 150.0); // milieu du 2e
    }

    /// La suite est elle-même un `Animatable` : elle se pilote par un contrôleur.
    #[test]
    fn tween_sequence_drives_from_controller() {
        let seq = TweenSequence::new(Tween::new(Color::BLACK, Color::WHITE), 1.0)
            .then(Tween::new(Color::WHITE, Color::BLACK), 1.0);
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.5); // couture → blanc
        assert_eq!(seq.animate(&ctrl).value(), Color::WHITE);
        ctrl.set_value(1.0); // retour au noir
        assert_eq!(seq.animate(&ctrl).value(), Color::BLACK);
    }
}

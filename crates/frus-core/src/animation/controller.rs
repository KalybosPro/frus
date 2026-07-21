//! Le pilote : un [`AnimationController`] qui échantillonne une [`Simulation`]
//! frame par frame et expose une **valeur courante** + un **statut**.
//!
//! Tout ce que fait le contrôleur — une transition d'une courbe sur une durée,
//! ou un fling physique — est exprimé comme une `Simulation` (fonction pure
//! `x(t)`). Une seule boucle de tick pilote donc tout : *un pilote, des fonctions
//! temps→valeur enfichables*. C'est l'abstraction que le shell instanciera par
//! identité (`child_id`) ; la vue lit `value()` au paint.

use super::curve::Curve;
use super::simulation::{Simulation, SpringDescription, SpringSimulation, Tolerance};

/// L'avancement d'un contrôleur d'animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// À la borne basse, au repos.
    Dismissed,
    /// En mouvement vers la borne haute.
    Forward,
    /// En mouvement vers la borne basse.
    Reverse,
    /// À la borne haute, au repos.
    Completed,
}

/// Une simulation « interpolation d'une courbe sur une durée » : le versant
/// *explicite* (durée + courbe) exprimé, comme tout le reste, en `Simulation`.
#[derive(Clone, Debug)]
struct InterpolationSimulation {
    begin: f32,
    end: f32,
    duration: f32,
    curve: Curve,
}

impl Simulation for InterpolationSimulation {
    fn x(&self, time: f32) -> f32 {
        if self.duration <= 0.0 {
            return self.end;
        }
        let t = (time / self.duration).clamp(0.0, 1.0);
        let shaped = self.curve.transform(t);
        self.begin + (self.end - self.begin) * shaped
    }

    fn dx(&self, time: f32) -> f32 {
        let h = 1e-3;
        (self.x(time + h) - self.x(time - h)) / (2.0 * h)
    }

    fn is_done(&self, time: f32) -> bool {
        time >= self.duration
    }
}

/// Mode de **répétition** : chaque cycle dure `period` secondes, façonné par
/// `curve` ; `reverse` fait l'aller-retour (sinon redémarre du bas).
#[derive(Clone)]
struct Repeat {
    period: f32,
    reverse: bool,
    curve: Curve,
}

/// Pilote une valeur bornée `[lower, upper]` via une simulation enfichable.
pub struct AnimationController {
    value: f32,
    velocity: f32,
    lower: f32,
    upper: f32,
    status: Status,
    /// Simulation active + temps écoulé depuis son démarrage ; `None` au repos.
    active: Option<(Box<dyn Simulation>, f32)>,
    /// Sens visé, pour classer le statut au repos (`Completed` vs `Dismissed`).
    heading_up: bool,
    /// Répétition en boucle active ; `None` = animation à un coup.
    repeat: Option<Repeat>,
}

impl AnimationController {
    /// Contrôleur bornant sa valeur dans `[lower, upper]`, démarré à `lower`.
    pub fn new(lower: f32, upper: f32) -> Self {
        Self {
            value: lower,
            velocity: 0.0,
            lower,
            upper,
            status: Status::Dismissed,
            active: None,
            heading_up: false,
            repeat: None,
        }
    }

    /// Contrôleur standard sur `[0, 1]`.
    pub fn unit() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Bornes `[lower, upper]` du contrôleur.
    pub fn bounds(&self) -> (f32, f32) {
        (self.lower, self.upper)
    }

    /// Valeur courante.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Vitesse courante (unités par seconde).
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Statut courant.
    pub fn status(&self) -> Status {
        self.status
    }

    /// `true` si une simulation est en cours (une frame est nécessaire).
    pub fn is_animating(&self) -> bool {
        self.active.is_some()
    }

    /// Fixe la valeur immédiatement (sans animation), et met le statut à jour.
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.lower, self.upper);
        self.velocity = 0.0;
        self.active = None;
        self.repeat = None;
        self.status = self.rest_status();
    }

    /// Remet la valeur à la borne basse, sans animation (`set_value(lower)`).
    pub fn reset(&mut self) {
        self.set_value(self.lower);
    }

    /// Arrête toute animation (y compris une boucle [`AnimationController::repeat`])
    /// et fige la valeur courante.
    pub fn stop(&mut self) {
        self.active = None;
        self.repeat = None;
        self.velocity = 0.0;
        self.status = self.rest_status();
    }

    /// Anime vers la borne haute sur `duration` secondes selon `curve`.
    pub fn forward(&mut self, duration: f32, curve: Curve) {
        self.animate_to(self.upper, duration, curve);
    }

    /// Anime vers la borne basse sur `duration` secondes selon `curve`.
    pub fn reverse(&mut self, duration: f32, curve: Curve) {
        self.animate_to(self.lower, duration, curve);
    }

    /// Anime la valeur courante vers `target` sur `duration` secondes. Annule une
    /// éventuelle boucle `repeat` (animation à un coup).
    pub fn animate_to(&mut self, target: f32, duration: f32, curve: Curve) {
        self.repeat = None;
        self.start_interpolation(target, duration, curve);
    }

    /// Démarre une interpolation vers `target` **sans** toucher au mode `repeat`
    /// (partagé par `animate_to` et le redémarrage de boucle dans `tick`).
    fn start_interpolation(&mut self, target: f32, duration: f32, curve: Curve) {
        let target = target.clamp(self.lower, self.upper);
        self.heading_up = target >= self.value;
        self.status = if self.heading_up {
            Status::Forward
        } else {
            Status::Reverse
        };
        self.active = Some((
            Box::new(InterpolationSimulation {
                begin: self.value,
                end: target,
                duration,
                curve,
            }),
            0.0,
        ));
    }

    /// **Répète** l'animation en boucle, chaque cycle durant `period` secondes et
    /// façonné par `curve`. `reverse` : aller-retour (`0→1→0…`) ; sinon redémarre
    /// du bas à chaque cycle (`0→1, 0→1…`). Ne s'arrête qu'à [`stop`](Self::stop)
    /// (ou `set_value`/`animate_to`…). Idiome des animations continues pilotées
    /// (pulsation, indicateur, halo) — façon `AnimationController.repeat` de Flutter.
    pub fn repeat(&mut self, period: f32, reverse: bool, curve: Curve) {
        self.repeat = Some(Repeat { period, reverse, curve: curve.clone() });
        // Démarre le premier cycle vers le haut depuis la valeur courante.
        self.start_interpolation(self.upper, period, curve);
    }

    /// Lance un fling physique : un ressort critique amorcé par `velocity`, qui
    /// s'arrête sur la position (bornée à `[lower, upper]`).
    pub fn fling(&mut self, velocity: f32) {
        self.repeat = None;
        // Ressort critique par défaut, à la manière de Flutter (`fling`).
        let spring = SpringDescription::with_damping_ratio(1.0, 500.0, 1.0);
        let target = if velocity < 0.0 { self.lower } else { self.upper };
        self.heading_up = velocity >= 0.0;
        self.status = if self.heading_up {
            Status::Forward
        } else {
            Status::Reverse
        };
        self.active = Some((
            Box::new(SpringSimulation::new(
                spring,
                self.value,
                target,
                velocity,
                Tolerance::default(),
            )),
            0.0,
        ));
    }

    /// Anime la valeur courante vers `target` par un **ressort** décrit par
    /// `spring`, amorcé par `velocity` (unités/s). C'est le chemin des transitions
    /// interruptibles amorcées par un geste (détente de glissement, retour amorcé
    /// par l'élan du doigt) : le ressort part de la position *et* de la vitesse
    /// courantes.
    pub fn spring_to(&mut self, target: f32, spring: SpringDescription, velocity: f32) {
        let target = target.clamp(self.lower, self.upper);
        let heading_up = target >= self.value;
        let sim = SpringSimulation::new(spring, self.value, target, velocity, Tolerance::default());
        self.drive(Box::new(sim), heading_up);
    }

    /// Pilote une simulation arbitraire (momentum de scroll, courbe sur mesure…).
    pub fn drive(&mut self, simulation: Box<dyn Simulation>, heading_up: bool) {
        self.repeat = None;
        self.heading_up = heading_up;
        self.status = if heading_up {
            Status::Forward
        } else {
            Status::Reverse
        };
        self.active = Some((simulation, 0.0));
    }

    /// Statut de repos correspondant à la valeur courante.
    fn rest_status(&self) -> Status {
        if (self.value - self.upper).abs() < 1e-4 {
            Status::Completed
        } else if (self.value - self.lower).abs() < 1e-4 {
            Status::Dismissed
        } else if self.heading_up {
            Status::Completed
        } else {
            Status::Dismissed
        }
    }

    /// Fait avancer l'animation de `dt` secondes. Met à jour valeur, vitesse et
    /// statut. Renvoie `true` si l'animation continue (une frame est à redemander).
    pub fn tick(&mut self, dt: f32) -> bool {
        let Some((sim, elapsed)) = self.active.as_mut() else {
            return false;
        };
        *elapsed += dt;
        let time = *elapsed;
        let raw = sim.x(time);
        let velocity = sim.dx(time);
        let done = sim.is_done(time);
        // Emprunt de `self.active` terminé ici (plus d'usage de `sim`/`elapsed`).
        self.value = raw.clamp(self.lower, self.upper);
        self.velocity = velocity;

        // Fin : simulation terminée, ou valeur sortie des bornes (fling qui tape
        // un bord — on s'arrête net, pas de rebond au-delà).
        let out_of_bounds = raw <= self.lower - 1e-4 || raw >= self.upper + 1e-4;
        if done || out_of_bounds {
            // Boucle active : redémarre un cycle au lieu de se reposer.
            if let Some(rep) = self.repeat.clone() {
                let target = if rep.reverse {
                    // Aller-retour : repart vers le bord opposé à celui atteint.
                    if self.heading_up { self.lower } else { self.upper }
                } else {
                    // Sawtooth : repart du bas vers le haut.
                    self.value = self.lower;
                    self.upper
                };
                self.start_interpolation(target, rep.period, rep.curve);
                return true;
            }
            self.velocity = 0.0;
            self.active = None;
            self.status = self.rest_status();
            false
        } else {
            true
        }
    }
}

impl Default for AnimationController {
    /// Un contrôleur standard sur `[0, 1]`, au repos à `0`.
    fn default() -> Self {
        Self::unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Avance le contrôleur jusqu'au repos (ou `max_frames`), pas de 16 ms.
    fn settle(ctrl: &mut AnimationController, max_frames: usize) -> usize {
        let mut frames = 0;
        while ctrl.tick(0.016) {
            frames += 1;
            if frames >= max_frames {
                break;
            }
        }
        frames
    }

    #[test]
    fn forward_reaches_upper_and_completes() {
        let mut ctrl = AnimationController::unit();
        assert_eq!(ctrl.status(), Status::Dismissed);
        ctrl.forward(0.3, Curve::ease_in_out());
        assert_eq!(ctrl.status(), Status::Forward);
        assert!(ctrl.is_animating());
        settle(&mut ctrl, 1000);
        assert!((ctrl.value() - 1.0).abs() < 1e-3, "value = {}", ctrl.value());
        assert_eq!(ctrl.status(), Status::Completed);
        assert!(!ctrl.is_animating());
    }

    #[test]
    fn reverse_reaches_lower_and_dismisses() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(1.0);
        assert_eq!(ctrl.status(), Status::Completed);
        ctrl.reverse(0.3, Curve::ease_in_out());
        assert_eq!(ctrl.status(), Status::Reverse);
        settle(&mut ctrl, 1000);
        assert!(ctrl.value().abs() < 1e-3);
        assert_eq!(ctrl.status(), Status::Dismissed);
    }

    #[test]
    fn value_is_monotonic_during_forward() {
        let mut ctrl = AnimationController::unit();
        ctrl.forward(0.5, Curve::ease_in_out());
        let mut prev = ctrl.value();
        while ctrl.tick(0.016) {
            assert!(ctrl.value() >= prev - 1e-4, "recule : {} < {}", ctrl.value(), prev);
            assert!(ctrl.value() <= 1.0 + 1e-4);
            prev = ctrl.value();
        }
    }

    #[test]
    fn fling_upward_settles_at_upper() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.5);
        ctrl.fling(3.0); // vitesse positive → vers le haut
        assert_eq!(ctrl.status(), Status::Forward);
        settle(&mut ctrl, 2000);
        assert!((ctrl.value() - 1.0).abs() < 1e-2, "value = {}", ctrl.value());
        assert_eq!(ctrl.status(), Status::Completed);
    }

    #[test]
    fn fling_downward_settles_at_lower() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.5);
        ctrl.fling(-3.0);
        settle(&mut ctrl, 2000);
        assert!(ctrl.value().abs() < 1e-2, "value = {}", ctrl.value());
        assert_eq!(ctrl.status(), Status::Dismissed);
    }

    #[test]
    fn spring_to_settles_at_target_from_velocity() {
        let mut ctrl = AnimationController::unit();
        ctrl.set_value(0.2);
        // Détente amorcée par un « flick » : ressort ~critique vers la borne haute.
        let spring = SpringDescription::new(1.0, 220.0, 30.0);
        ctrl.spring_to(1.0, spring, 5.0);
        assert_eq!(ctrl.status(), Status::Forward);
        settle(&mut ctrl, 2000);
        assert!((ctrl.value() - 1.0).abs() < 1e-2, "value = {}", ctrl.value());
        assert_eq!(ctrl.status(), Status::Completed);
    }

    /// `repeat` (sawtooth) : ne se repose jamais, atteint le haut, puis **retombe**
    /// (redémarrage du cycle).
    #[test]
    fn repeat_never_settles_and_restarts() {
        let mut ctrl = AnimationController::unit();
        ctrl.repeat(0.1, false, Curve::Linear);
        let (mut max, mut saw_restart, mut prev) = (0.0f32, false, ctrl.value());
        for _ in 0..100 {
            // ~1.6 s = 16 cycles
            ctrl.tick(0.016);
            assert!(ctrl.is_animating(), "une boucle reste toujours animée");
            assert!(ctrl.value() >= -1e-4 && ctrl.value() <= 1.0 + 1e-4);
            max = max.max(ctrl.value());
            if ctrl.value() + 1e-3 < prev {
                saw_restart = true; // la valeur a chuté → nouveau cycle
            }
            prev = ctrl.value();
        }
        assert!(max > 0.9, "atteint le haut (max = {max})");
        assert!(saw_restart, "redémarre (la valeur retombe)");
    }

    /// `repeat(reverse)` : aller-retour — la valeur monte puis redescend.
    #[test]
    fn repeat_reverse_ping_pongs() {
        let mut ctrl = AnimationController::unit();
        ctrl.repeat(0.1, true, Curve::Linear);
        let (mut went_up, mut came_down, mut prev) = (false, false, ctrl.value());
        for _ in 0..40 {
            ctrl.tick(0.016);
            if ctrl.value() > prev + 1e-4 {
                went_up = true;
            }
            if went_up && ctrl.value() < prev - 1e-4 {
                came_down = true;
            }
            prev = ctrl.value();
        }
        assert!(went_up && came_down, "aller-retour (montée puis descente)");
        assert!(ctrl.is_animating());
    }

    /// `stop` met fin à une boucle ; `reset` ramène au bas.
    #[test]
    fn stop_and_reset_end_a_repeat() {
        let mut ctrl = AnimationController::unit();
        ctrl.repeat(0.1, false, Curve::Linear);
        ctrl.tick(0.05);
        assert!(ctrl.is_animating());
        ctrl.stop();
        assert!(!ctrl.is_animating(), "stop arrête la boucle");
        assert!(!ctrl.tick(0.016), "plus rien à animer après stop");

        ctrl.repeat(0.1, false, Curve::Linear);
        ctrl.tick(0.05);
        ctrl.reset();
        assert!(!ctrl.is_animating(), "reset arrête la boucle");
        assert_eq!(ctrl.value(), 0.0, "reset ramène à la borne basse");
    }

    #[test]
    fn zero_duration_snaps_immediately() {
        let mut ctrl = AnimationController::unit();
        ctrl.forward(0.0, Curve::Linear);
        // Une seule frame suffit à atteindre la cible.
        ctrl.tick(0.016);
        assert!((ctrl.value() - 1.0).abs() < 1e-4);
        assert!(!ctrl.is_animating());
    }
}

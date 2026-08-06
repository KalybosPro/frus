//! Simulations physiques : des fonctions **pures** `temps → valeur`.
//!
//! Une [`Simulation`] décrit un mouvement par sa position `x(t)`, sa vitesse
//! `dx(t)` et un critère d'arrêt `is_done(t)` — sans état mutable ni couplage au
//! rendu. C'est l'abstraction que tout le reste de l'animation partage : un
//! ressort, un fling à friction et le momentum de défilement passent par le même
//! chemin (un pilote échantillonne `x(t)` à chaque frame).
//!
//! Les maths (ressort en 3 régimes, friction en forme close) sont portées de
//! Flutter (`physics/*.dart`), mais exprimées comme des valeurs immuables
//! calculées une fois à la construction : aucune ré-entrance, aucun emprunt vers
//! le haut — le style colle à Rust.

/// Seuils en deçà desquels une simulation est considérée « au repos ».
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    /// Distance négligeable (unités de position).
    pub distance: f32,
    /// Vitesse négligeable (unités de position par seconde).
    pub velocity: f32,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            distance: 1e-3,
            velocity: 1e-3,
        }
    }
}

impl Tolerance {
    /// Une tolérance calibrée en **pixels** (mouvements d'UI) : on considère un
    /// déplacement de moins d'un demi-pixel et une vitesse de moins de 2 px/s
    /// comme immobiles.
    pub const PIXELS: Tolerance = Tolerance {
        distance: 0.5,
        velocity: 2.0,
    };
}

/// Un mouvement décrit comme une fonction pure du temps.
///
/// `time` est en **secondes** depuis le début du mouvement.
pub trait Simulation {
    /// Position à l'instant `time`.
    fn x(&self, time: f32) -> f32;
    /// Vitesse (dérivée de la position) à l'instant `time`.
    fn dx(&self, time: f32) -> f32;
    /// `true` quand le mouvement est terminé (dans les tolérances).
    fn is_done(&self, time: f32) -> bool;
    /// Seuils d'arrêt.
    fn tolerance(&self) -> Tolerance {
        Tolerance::default()
    }
}

/// Paramètres d'un ressort amorti.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringDescription {
    /// Masse `m` (> 0).
    pub mass: f32,
    /// Raideur `k` (> 0).
    pub stiffness: f32,
    /// Amortissement `c` (≥ 0).
    pub damping: f32,
}

impl SpringDescription {
    /// Ressort décrit directement par `(masse, raideur, amortissement)`.
    pub fn new(mass: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            mass,
            stiffness,
            damping,
        }
    }

    /// Ressort décrit par un **ratio d'amortissement** : `1` = critique (arrivée
    /// la plus rapide sans dépassement), `< 1` oscille, `> 1` est mou.
    /// `c = ratio · 2·√(m·k)`.
    pub fn with_damping_ratio(mass: f32, stiffness: f32, ratio: f32) -> Self {
        Self {
            mass,
            stiffness,
            damping: ratio * 2.0 * (mass * stiffness).sqrt(),
        }
    }
}

/// La forme close d'un ressort, choisie selon le discriminant `c² − 4mk`.
#[derive(Clone, Copy, Debug)]
enum Solution {
    /// Amortissement critique (`c² − 4mk == 0`) : arrivée la plus rapide sans
    /// oscillation.
    Critical { r: f32, c1: f32, c2: f32 },
    /// Suramorti (`c² − 4mk > 0`) : deux exponentielles réelles, pas d'oscillation.
    Overdamped { r1: f32, r2: f32, c1: f32, c2: f32 },
    /// Sous-amorti (`c² − 4mk < 0`) : oscillation amortie.
    Underdamped { w: f32, r: f32, c1: f32, c2: f32 },
}

impl Solution {
    fn x(&self, t: f32) -> f32 {
        match *self {
            Solution::Critical { r, c1, c2 } => (c1 + c2 * t) * (r * t).exp(),
            Solution::Overdamped { r1, r2, c1, c2 } => c1 * (r1 * t).exp() + c2 * (r2 * t).exp(),
            Solution::Underdamped { w, r, c1, c2 } => {
                (r * t).exp() * (c1 * (w * t).cos() + c2 * (w * t).sin())
            }
        }
    }

    fn dx(&self, t: f32) -> f32 {
        match *self {
            Solution::Critical { r, c1, c2 } => {
                let power = (r * t).exp();
                r * (c1 + c2 * t) * power + c2 * power
            }
            Solution::Overdamped { r1, r2, c1, c2 } => {
                c1 * r1 * (r1 * t).exp() + c2 * r2 * (r2 * t).exp()
            }
            Solution::Underdamped { w, r, c1, c2 } => {
                let power = (r * t).exp();
                let cosine = (w * t).cos();
                let sine = (w * t).sin();
                power * (c2 * w * cosine - c1 * w * sine) + r * power * (c2 * sine + c1 * cosine)
            }
        }
    }
}

/// Un ressort qui ramène une valeur de `start` vers `end`, amorcé par une vitesse
/// initiale `velocity`. La position rapportée est `end + solution(t)`.
#[derive(Clone, Copy, Debug)]
pub struct SpringSimulation {
    end: f32,
    solution: Solution,
    tolerance: Tolerance,
}

impl SpringSimulation {
    /// Construit un ressort tendant de `start` vers `end`, à vitesse initiale
    /// `velocity` (px/s), avec la tolérance donnée.
    pub fn new(
        spring: SpringDescription,
        start: f32,
        end: f32,
        velocity: f32,
        tolerance: Tolerance,
    ) -> Self {
        let m = spring.mass;
        let k = spring.stiffness;
        let c = spring.damping;
        let x0 = start - end;
        let v0 = velocity;
        let cmk = c * c - 4.0 * m * k;

        // On considère le régime critique dès que le discriminant est négligeable
        // devant `4mk` : `with_damping_ratio(1.0)` peut, par arrondi flottant,
        // produire un `cmk` minuscule non nul qu'on ne veut pas voir osciller.
        let solution = if cmk.abs() <= 1e-4 * (4.0 * m * k).max(1.0) {
            let r = -c / (2.0 * m);
            Solution::Critical {
                r,
                c1: x0,
                c2: v0 - r * x0,
            }
        } else if cmk > 0.0 {
            let root = cmk.sqrt();
            let r1 = (-c - root) / (2.0 * m);
            let r2 = (-c + root) / (2.0 * m);
            let c2 = (v0 - r1 * x0) / (r2 - r1);
            Solution::Overdamped {
                r1,
                r2,
                c1: x0 - c2,
                c2,
            }
        } else {
            let w = (4.0 * m * k - c * c).sqrt() / (2.0 * m);
            let r = -c / (2.0 * m);
            Solution::Underdamped {
                w,
                r,
                c1: x0,
                c2: (v0 - r * x0) / w,
            }
        };

        Self {
            end,
            solution,
            tolerance,
        }
    }
}

impl Simulation for SpringSimulation {
    fn x(&self, time: f32) -> f32 {
        self.end + self.solution.x(time)
    }

    fn dx(&self, time: f32) -> f32 {
        self.solution.dx(time)
    }

    fn is_done(&self, time: f32) -> bool {
        self.solution.x(time).abs() < self.tolerance.distance
            && self.solution.dx(time).abs() < self.tolerance.velocity
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// Décélération par friction (momentum de scroll/fling), en forme close.
///
/// `drag ∈ (0, 1)` : plus il est proche de 1, plus le mouvement roule longtemps.
#[derive(Clone, Copy, Debug)]
pub struct FrictionSimulation {
    drag: f32,
    drag_log: f32,
    x0: f32,
    v0: f32,
    tolerance: Tolerance,
}

impl FrictionSimulation {
    /// Friction depuis la position `position` à la vitesse `velocity`, avec le
    /// coefficient `drag`.
    pub fn new(drag: f32, position: f32, velocity: f32, tolerance: Tolerance) -> Self {
        Self {
            drag,
            drag_log: drag.ln(),
            x0: position,
            v0: velocity,
            tolerance,
        }
    }

    /// Friction calibrée pour passer par `start` (à `start_velocity`) **et**
    /// s'arrêter à `end` (à `end_velocity`) : en déduit le `drag`.
    /// `drag = e^((v₀ − v₁)/(x₀ − x₁))`.
    pub fn through(
        start: f32,
        end: f32,
        start_velocity: f32,
        end_velocity: f32,
        tolerance: Tolerance,
    ) -> Self {
        let drag = ((start_velocity - end_velocity) / (start - end)).exp();
        Self::new(drag, start, start_velocity, tolerance)
    }

    /// Position finale (limite quand `t → ∞`).
    pub fn final_x(&self) -> f32 {
        self.x0 - self.v0 / self.drag_log
    }
}

impl Simulation for FrictionSimulation {
    fn x(&self, time: f32) -> f32 {
        self.x0 + (self.v0 / self.drag_log) * (self.drag.powf(time) - 1.0)
    }

    fn dx(&self, time: f32) -> f32 {
        self.v0 * self.drag.powf(time)
    }

    fn is_done(&self, time: f32) -> bool {
        self.dx(time).abs() < self.tolerance.velocity
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// Épingle la **position** d'une simulation dans `[min, max]` (la vitesse
/// continue de rapporter celle de la simulation sous-jacente). Sert au scroll :
/// le momentum est calculé librement, mais l'offset ne sort pas des bornes.
pub struct ClampedSimulation<S: Simulation> {
    inner: S,
    min: f32,
    max: f32,
}

impl<S: Simulation> ClampedSimulation<S> {
    /// Enveloppe `inner`, contraignant sa position dans `[min, max]`.
    pub fn new(inner: S, min: f32, max: f32) -> Self {
        Self { inner, min, max }
    }
}

impl<S: Simulation> Simulation for ClampedSimulation<S> {
    fn x(&self, time: f32) -> f32 {
        self.inner.x(time).clamp(self.min, self.max)
    }

    fn dx(&self, time: f32) -> f32 {
        self.inner.dx(time)
    }

    fn is_done(&self, time: f32) -> bool {
        self.inner.is_done(time)
    }

    fn tolerance(&self) -> Tolerance {
        self.inner.tolerance()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Intègre numériquement `dx` et vérifie que ça colle à `x` : garantit que
    /// vitesse et position d'une simulation sont cohérentes.
    fn dx_matches_x<S: Simulation>(sim: &S, t: f32) {
        let h = 1e-4;
        let numeric = (sim.x(t + h) - sim.x(t - h)) / (2.0 * h);
        let analytic = sim.dx(t);
        // Tolérance relative : la différence finie centrée a une erreur de
        // troncature proportionnelle à la courbure (grande à haute vitesse).
        let scale = analytic.abs().max(1.0);
        assert!(
            (numeric - analytic).abs() < 1e-2 * scale,
            "dx incohérent à t={t} : numérique={numeric}, analytique={analytic}"
        );
    }

    #[test]
    fn critical_spring_settles_without_overshoot() {
        let spring = SpringDescription::with_damping_ratio(1.0, 500.0, 1.0);
        let sim = SpringSimulation::new(spring, 0.0, 1.0, 0.0, Tolerance::default());
        assert!((sim.x(0.0) - 0.0).abs() < 1e-5, "départ à start");
        // Monotone croissante, ne dépasse jamais 1 (pas d'oscillation).
        let mut prev = sim.x(0.0);
        let mut t = 0.0;
        while t < 2.0 {
            let v = sim.x(t);
            assert!(v <= 1.0 + 1e-4, "dépassement à t={t} : {v}");
            assert!(v >= prev - 1e-4, "non monotone à t={t}");
            prev = v;
            t += 0.01;
        }
        assert!((sim.x(2.0) - 1.0).abs() < 1e-2, "arrivée à end");
        assert!(sim.is_done(3.0), "au repos après 3 s");
        dx_matches_x(&sim, 0.2);
    }

    #[test]
    fn underdamped_spring_overshoots_then_returns() {
        let spring = SpringDescription::with_damping_ratio(1.0, 500.0, 0.3);
        let sim = SpringSimulation::new(spring, 0.0, 1.0, 0.0, Tolerance::default());
        // Faiblement amorti : dépasse la cible au moins une fois.
        let mut max = f32::MIN;
        let mut t = 0.0;
        while t < 2.0 {
            max = max.max(sim.x(t));
            t += 0.005;
        }
        assert!(max > 1.05, "devrait dépasser la cible, max={max}");
        // Finit par se poser sur la cible.
        assert!((sim.x(3.0) - 1.0).abs() < 1e-2);
        dx_matches_x(&sim, 0.1);
    }

    #[test]
    fn overdamped_spring_is_slow_and_monotonic() {
        let spring = SpringDescription::with_damping_ratio(1.0, 100.0, 2.5);
        let sim = SpringSimulation::new(spring, 0.0, 1.0, 0.0, Tolerance::default());
        let mut prev = sim.x(0.0);
        let mut t = 0.0;
        while t < 3.0 {
            let v = sim.x(t);
            assert!(v <= 1.0 + 1e-4, "pas de dépassement (suramorti) à t={t}");
            assert!(v >= prev - 1e-4, "monotone à t={t}");
            prev = v;
            t += 0.01;
        }
        dx_matches_x(&sim, 0.3);
    }

    #[test]
    fn spring_honours_initial_velocity() {
        let spring = SpringDescription::with_damping_ratio(1.0, 500.0, 1.0);
        let sim = SpringSimulation::new(spring, 0.0, 1.0, 50.0, Tolerance::default());
        // Vitesse initiale = 50 px/s vers la cible.
        assert!((sim.dx(0.0) - 50.0).abs() < 1e-2, "dx(0) = {}", sim.dx(0.0));
    }

    #[test]
    fn friction_decelerates_to_finite_limit() {
        let sim = FrictionSimulation::new(0.135, 0.0, 1000.0, Tolerance::PIXELS);
        let limit = sim.final_x();
        // La position tend vers la limite finie sans la dépasser.
        assert!(sim.x(0.0).abs() < 1e-4);
        assert!(sim.x(10.0) <= limit + 1e-3);
        assert!(
            (sim.x(10.0) - limit).abs() < 1.0,
            "proche de la limite après 10 s"
        );
        // La vitesse décroît vers 0.
        assert!(sim.dx(0.0) > sim.dx(1.0));
        assert!(sim.dx(5.0).abs() < sim.dx(0.0).abs());
        dx_matches_x(&sim, 0.5);
    }

    #[test]
    fn friction_through_hits_endpoints() {
        // Doit passer par x=0 à v=1000 et finir à x=200 à v≈0.
        let sim = FrictionSimulation::through(0.0, 200.0, 1000.0, 0.0, Tolerance::PIXELS);
        assert!((sim.x(0.0)).abs() < 1e-3);
        assert!(
            (sim.final_x() - 200.0).abs() < 1e-1,
            "final = {}",
            sim.final_x()
        );
    }

    #[test]
    fn clamped_pins_position_but_reports_velocity() {
        let inner = FrictionSimulation::new(0.135, 0.0, 1000.0, Tolerance::PIXELS);
        let uncapped = inner.x(10.0);
        assert!(uncapped > 100.0);
        let clamped = ClampedSimulation::new(inner, 0.0, 100.0);
        assert_eq!(clamped.x(10.0), 100.0, "position épinglée au max");
        // La vitesse reste celle du mouvement libre (non nulle près du bord).
        assert!(clamped.dx(0.1) > 0.0);
    }
}

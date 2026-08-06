//! Reconnaissance de gestes — **palier 1** du brief (§3) : un reconnaisseur
//! « tap-ou-appui-long » qui parle déjà le vocabulaire de l'**arène**, pour que le
//! passage à la vraie arène (palier 2) soit une substitution, pas une réécriture.
//!
//! - L'**appui long** accepte **avidement** au franchissement du délai : il évince
//!   le tap (le relâchement qui suit est avalé).
//! - Le **tap** accepte **passivement** : il gagne si rien ne l'a évincé au
//!   relâchement (le chemin de clic existant du shell).
//! - Un mouvement au-delà du **slop** rejette l'appui long (le geste devient un
//!   glissement/défilement) ; `Cancel` rejette tout.
//!
//! La machine est **pure** (aucun accès horloge : les instants sont passés en
//! paramètres) — donc testable au tick près.

use std::time::Duration;

use web_time::Instant;

use frus_widgets::{FrictionSimulation, Point, Tolerance};

/// Délai d'appui long (pression immobile) avant acceptation.
pub(crate) const LONG_PRESS_DELAY: Duration = Duration::from_millis(500);
/// Mouvement (px logiques) au-delà duquel l'appui long est rejeté.
const SLOP: f32 = 8.0;

/// Décélération du fling : fraction de la vitesse conservée après 1 s
/// (`dx(t) = v·drag^t`, la constante de friction usuelle du défilement).
const FLING_DRAG: f32 = 0.135;
/// Vitesse minimale (px/s) au relâchement pour déclencher un fling.
const FLING_MIN_VELOCITY: f32 = 50.0;

/// Destination **balistique** d'un fling de défilement : la position finale
/// d'une [`FrictionSimulation`] amorcée à `velocity` depuis `current` — le
/// momentum du doigt, en forme close. `None` sous le seuil de vitesse (le
/// relâchement lent n'entraîne pas le contenu).
pub(crate) fn fling_destination(current: f32, velocity: f32) -> Option<f32> {
    if velocity.abs() < FLING_MIN_VELOCITY {
        return None;
    }
    Some(FrictionSimulation::new(FLING_DRAG, current, velocity, Tolerance::PIXELS).final_x())
}

/// Un événement pointeur **normalisé** (souris ou doigt) : l'entrée unique du
/// routage — palier 0 du brief, avec `Cancel` explicite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PointerKind {
    Down,
    Move,
    Up,
    /// Geste interrompu (app en arrière-plan, doigt annulé) : abandonner sans
    /// callback de succès.
    Cancel,
}

/// L'événement normalisé : sa nature, sa position **logique**, et si la source
/// est tactile (le tactile arme le défilement au doigt).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PointerEvent {
    pub kind: PointerKind,
    pub position: Point,
    pub touch: bool,
}

/// États internes du reconnaisseur.
#[derive(Debug)]
enum State {
    /// Aucun appui suivi.
    Idle,
    /// Appui en cours, encore candidat à l'appui long.
    Possible { origin: Point, deadline: Instant },
    /// L'appui long a **accepté** (tiré) : le prochain relâchement est avalé.
    Fired,
    /// Candidat rejeté (mouvement > slop) : l'appui continue en tap/glissement.
    Rejected,
}

/// Reconnaisseur tap-ou-appui-long, une instance par pointeur actif.
pub(crate) struct PressRecognizer {
    state: State,
}

impl PressRecognizer {
    pub fn new() -> Self {
        Self { state: State::Idle }
    }

    /// Appui : arme le suivi si la cible est intéressée par l'appui long
    /// (`interested`), sinon reste inerte.
    pub fn down(&mut self, at: Point, now: Instant, interested: bool) {
        self.state = if interested {
            State::Possible {
                origin: at,
                deadline: now + LONG_PRESS_DELAY,
            }
        } else {
            State::Rejected
        };
    }

    /// Mouvement : au-delà du slop, l'appui long est rejeté (le geste est un
    /// glissement — « accepte avidement au franchissement de slop », côté drag).
    pub fn moved(&mut self, at: Point) {
        if let State::Possible { origin, .. } = self.state {
            let (dx, dy) = (at.x - origin.x, at.y - origin.y);
            if dx * dx + dy * dy > SLOP * SLOP {
                self.state = State::Rejected;
            }
        }
    }

    /// Échéance à laquelle l'appui long tirera, si encore candidat — à donner à
    /// `ControlFlow::WaitUntil` pour être réveillé pile au bon moment.
    pub fn deadline(&self) -> Option<Instant> {
        match self.state {
            State::Possible { deadline, .. } => Some(deadline),
            _ => None,
        }
    }

    /// Le temps a-t-il fait tirer l'appui long ? (`true` exactement une fois.)
    pub fn poll(&mut self, now: Instant) -> bool {
        if let State::Possible { deadline, .. } = self.state {
            if now >= deadline {
                self.state = State::Fired;
                return true;
            }
        }
        false
    }

    /// Relâchement : renvoie `true` si le clic doit être **avalé** (un appui long
    /// a déjà accepté — il évince le tap).
    pub fn up(&mut self) -> bool {
        let swallow = matches!(self.state, State::Fired);
        self.state = State::Idle;
        swallow
    }

    /// Interruption : abandonner sans rien émettre.
    pub fn cancel(&mut self) {
        self.state = State::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> Instant {
        Instant::now()
    }

    #[test]
    fn fling_projects_a_friction_final_position() {
        // Sous le seuil : pas de fling.
        assert_eq!(fling_destination(100.0, 0.0), None);
        assert_eq!(fling_destination(100.0, 30.0), None);

        // Momentum : destination = position + v / ln(1/drag) (forme close).
        let dest = fling_destination(0.0, 2000.0).expect("fling");
        let expected = 2000.0 / (1.0f32 / FLING_DRAG).ln();
        assert!(
            (dest - expected).abs() < 1.0,
            "dest = {dest}, attendu ≈ {expected}"
        );
        assert!(
            dest > 900.0 && dest < 1100.0,
            "≈ 1000 px de course : {dest}"
        );

        // Symétrique vers l'arrière.
        let back = fling_destination(500.0, -2000.0).expect("fling arrière");
        assert!((back - (500.0 - expected)).abs() < 1.0);
    }

    #[test]
    fn long_press_fires_after_the_delay_and_swallows_the_release() {
        let mut rec = PressRecognizer::new();
        let t0 = start();
        rec.down(Point::new(10.0, 10.0), t0, true);
        assert_eq!(rec.deadline(), Some(t0 + LONG_PRESS_DELAY));

        // Avant l'échéance : rien.
        assert!(!rec.poll(t0 + Duration::from_millis(499)));
        // À l'échéance : tire, exactement une fois.
        assert!(rec.poll(t0 + LONG_PRESS_DELAY));
        assert!(!rec.poll(t0 + Duration::from_millis(600)));
        // Le relâchement qui suit est avalé (l'appui long évince le tap).
        assert!(rec.up());
        // Et l'état est réinitialisé.
        assert!(rec.deadline().is_none());
    }

    #[test]
    fn release_before_the_delay_is_a_plain_tap() {
        let mut rec = PressRecognizer::new();
        let t0 = start();
        rec.down(Point::new(0.0, 0.0), t0, true);
        assert!(!rec.up(), "tap : le clic n'est pas avalé");
        assert!(!rec.poll(t0 + Duration::from_secs(1)), "plus rien à tirer");
    }

    #[test]
    fn movement_beyond_slop_rejects_the_long_press() {
        let mut rec = PressRecognizer::new();
        let t0 = start();
        rec.down(Point::new(0.0, 0.0), t0, true);
        // Sous le slop : encore candidat.
        rec.moved(Point::new(4.0, 4.0));
        assert!(rec.deadline().is_some());
        // Au-delà : rejeté, l'échéance disparaît, le temps ne tire plus.
        rec.moved(Point::new(10.0, 10.0));
        assert!(rec.deadline().is_none());
        assert!(!rec.poll(t0 + Duration::from_secs(1)));
        assert!(!rec.up(), "le clic normal reste possible");
    }

    #[test]
    fn uninterested_target_never_arms() {
        let mut rec = PressRecognizer::new();
        rec.down(Point::new(0.0, 0.0), start(), false);
        assert!(rec.deadline().is_none());
        assert!(!rec.poll(start() + Duration::from_secs(1)));
    }

    #[test]
    fn cancel_abandons_without_firing() {
        let mut rec = PressRecognizer::new();
        let t0 = start();
        rec.down(Point::new(0.0, 0.0), t0, true);
        rec.cancel();
        assert!(!rec.poll(t0 + Duration::from_secs(1)));
        assert!(!rec.up());
    }
}

//! État retenu au runtime entre les frames, **clé par identité de widget**.
//!
//! La *valeur* d'un champ reste contrôlée (état applicatif) ; ce qui vit ici est
//! l'état d'**interaction/édition** propre aux widgets : survol/focus, offsets de
//! défilement, et position curseur/sélection des champs. C'est la fondation
//! d'une reconciliation par identité (posée au Jalon 6).

use std::cell::RefCell;
use std::collections::HashMap;

use frus_core::Primitive;

use crate::interaction::{InputState, WidgetId};
use crate::relayout::LayoutCache;

/// Offsets de défilement `(x, y)`, par zone défilable.
pub type ScrollState = HashMap<WidgetId, (f32, f32)>;

/// État d'édition d'un champ de saisie : curseur + ancre de sélection.
///
/// Les indices sont en **caractères**. Ils peuvent dépasser la longueur de la
/// valeur (p. ex. `usize::MAX` pour « fin ») : les widgets les bornent à l'usage.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Edit {
    /// Position du curseur.
    pub cursor: usize,
    /// Ancre de sélection (`None` = pas de sélection).
    pub anchor: Option<usize>,
    /// Plage `(début, fin)` en **cours de composition** IME (texte provisoire,
    /// souligné à l'écran) ; `None` hors composition. En indices de caractères.
    pub composing: Option<(usize, usize)>,
}

impl Edit {
    /// Plage sélectionnée `(début, fin)`, non vide, sinon `None`.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.anchor
            .map(|anchor| (anchor.min(self.cursor), anchor.max(self.cursor)))
            .filter(|(start, end)| start < end)
    }
}

/// Durée des transitions, en secondes.
const ANIM_DURATION: f32 = 0.12;

/// Raideur du ressort de défilement (px·s⁻²).
const SCROLL_K: f32 = 200.0;
/// Amortissement du ressort de défilement.
const SCROLL_C: f32 = 28.0;
/// Rappel élastique de la cible vers les bornes valides (par seconde) — rebond.
const SCROLL_RETRACT: f32 = 14.0;

/// Un axe de défilement : rappel élastique de la cible dans `[0, max]`, puis
/// ressort de l'offset courant vers cette cible. Renvoie
/// `(offset, vitesse, cible, en_mouvement)`.
fn scroll_axis(current: f32, vel: f32, target: f32, max: f32, dt: f32) -> (f32, f32, f32, bool) {
    let clamp_t = target.clamp(0.0, max);
    // La cible est ramenée vers la borne valide (dépassement → rebond).
    let target = target + (clamp_t - target) * (1.0 - (-SCROLL_RETRACT * dt).exp());
    let (offset, vel, _) = spring_step(current, vel, target, dt, SCROLL_K, SCROLL_C);
    // Seuils en pixels (spring_step est calibré en fractions).
    let moving = (offset - target).abs() > 0.5 || vel.abs() > 2.0 || (target - clamp_t).abs() > 0.5;
    if moving {
        (offset, vel, target, true)
    } else {
        (clamp_t, 0.0, clamp_t, false)
    }
}

/// Progressions d'animation d'un widget (`0.0..=1.0`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Anim {
    pub hover: f32,
    pub focus: f32,
    /// Opacité (1 au repos ; démarrée à 0 au montage pour le fondu d'apparition).
    pub opacity: f32,
}

impl Default for Anim {
    fn default() -> Self {
        Self {
            hover: 0.0,
            focus: 0.0,
            opacity: 1.0,
        }
    }
}

/// Un pas de ressort amorti (Euler semi-implicite) faisant tendre `progress` vers
/// `target`, amorcé par `velocity`. `stiffness`/`damping` règlent la raideur et
/// l'amortissement (≈ `2·√stiffness` = amortissement critique, sans dépassement).
/// Renvoie `(progress, velocity, au_repos)`.
///
/// Util générique de mouvement : sert aux transitions d'écran et aux gestes
/// (détente amorcée par la vélocité du doigt).
pub fn spring_step(
    progress: f32,
    velocity: f32,
    target: f32,
    dt: f32,
    stiffness: f32,
    damping: f32,
) -> (f32, f32, bool) {
    let accel = stiffness * (target - progress) - damping * velocity;
    let velocity = velocity + accel * dt;
    let progress = progress + velocity * dt;
    let at_rest = (progress - target).abs() < 0.004 && velocity.abs() < 0.06;
    (progress, velocity, at_rest)
}

/// Courbe en **ressort** (réponse indicielle d'un ressort en amortissement
/// **critique**) remappant une progression linéaire `t ∈ [0,1]` : départ au
/// repos (pente nulle), montée franche, arrivée douce **sans dépassement** —
/// même sensation que les transitions d'écran, mais sous forme fermée (pas
/// d'état de vélocité). `f(0) = 0`, `f(1) = 1`, monotone croissante.
pub fn spring_ease(t: f32) -> f32 {
    // Réponse critique (`omega = 8`), désormais fournie par la couche d'animation
    // partagée de `frus-core` : une seule source de vérité pour cette courbe.
    frus_core::Curve::critical_spring().transform(t)
}

/// Fait tendre `value` vers `target` par pas de `step` ; note si ça bouge encore.
fn approach(value: &mut f32, target: f32, step: f32, animating: &mut bool) {
    if *value < target {
        *value = (*value + step).min(target);
    } else if *value > target {
        *value = (*value - step).max(target);
    }
    if (*value - target).abs() > 1e-3 {
        *animating = true;
    }
}

/// Contexte runtime transmis à `build_ui` : tout l'état retenu entre frames.
#[derive(Default)]
pub struct Runtime {
    /// Survol / pression / focus.
    pub input: InputState,
    /// Offsets de défilement **courants** (rendus), par zone.
    pub scroll: ScrollState,
    /// Offsets de défilement **visés** (le ressort y tend), par zone.
    pub scroll_target: ScrollState,
    /// Vitesse de défilement (pour le ressort), par zone.
    pub scroll_velocity: ScrollState,
    /// État d'édition, par champ de saisie.
    pub edits: HashMap<WidgetId, Edit>,
    /// Progressions d'animation (survol/focus/opacité), par widget.
    pub anims: HashMap<WidgetId, Anim>,
    /// Valeurs animées propres aux widgets (`Widget::anim_target`), par widget.
    pub values: HashMap<WidgetId, f32>,
    /// Widgets présents à la frame précédente (pour détecter les montages).
    pub mounted: std::collections::HashSet<WidgetId>,
    /// Instantanés des sous-arbres sortants, en cours de fondu de sortie :
    /// clé d'événement → (primitives capturées, opacité restante `1 → 0`).
    pub leaving: HashMap<u64, (Vec<Primitive>, f32)>,
    /// Temps écoulé (secondes) depuis le démarrage, pour les animations continues.
    pub time: f32,
    /// La dernière interaction était-elle **clavier** ? L'anneau de focus
    /// générique n'est peint que dans ce cas (`FocusHighlightMode` : un clic ne
    /// doit pas faire flasher d'anneau). Le focus lui-même reste actif.
    pub focus_visible: bool,
    /// Cache de frontière de relayout (rectangles retenus par racine de layout,
    /// d'une frame à l'autre). Mutabilité intérieure : `build_ui` le met à jour
    /// tout en ne tenant qu'une référence partagée au `Runtime`.
    pub layout_cache: RefCell<LayoutCache>,
}

impl Runtime {
    /// Progression de survol animée d'un widget.
    pub fn hover_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.hover).unwrap_or(0.0)
    }

    /// Progression de focus animée d'un widget.
    pub fn focus_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.focus).unwrap_or(0.0)
    }

    /// Opacité animée d'un widget (1 par défaut).
    pub fn opacity(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.opacity).unwrap_or(1.0)
    }

    /// Valeur animée d'un widget (0 par défaut).
    pub fn value(&self, id: WidgetId) -> f32 {
        self.values.get(&id).copied().unwrap_or(0.0)
    }

    /// Valeur animée d'un widget, ou `default` si **aucune** valeur n'est encore
    /// enregistrée (widget jamais animé — p. ex. rendu isolé sans boucle). Permet
    /// d'adopter la cible immédiatement, comme au montage.
    pub fn value_or(&self, id: WidgetId, default: f32) -> f32 {
        self.values.get(&id).copied().unwrap_or(default)
    }

    /// Fait tendre chaque valeur animée vers la cible déclarée par son widget
    /// (`Widget::anim_target`). Un widget vu pour la **première** fois adopte sa
    /// cible sans transition (pas d'animation au montage). Renvoie `true` s'il
    /// reste une valeur en mouvement.
    pub fn advance_values<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            out: &mut Vec<(WidgetId, f32)>,
        ) {
            if let Some(target) = widget.anim_target() {
                out.push((id, target));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(child.as_ref(), crate::ui::child_id(id, index, child.as_ref()), out);
            }
        }
        let mut targets: Vec<(WidgetId, f32)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

        // Oublie les valeurs des widgets disparus.
        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, _)| *id).collect();
        self.values.retain(|id, _| present.contains(id));

        let step = if ANIM_DURATION > 0.0 { dt / ANIM_DURATION } else { 1.0 };
        let mut animating = false;
        for (id, target) in targets {
            match self.values.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    approach(e.get_mut(), target, step, &mut animating);
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    // Montage : adopte la cible sans transition.
                    e.insert(target);
                }
            }
        }
        animating
    }

    /// Fait avancer les transitions (survol/focus) de `dt` secondes vers leurs
    /// cibles. Renvoie `true` si au moins une animation est encore en cours.
    pub fn advance(&mut self, dt: f32) -> bool {
        let hovered = self.input.hovered;
        let focused = self.input.focused;
        if let Some(id) = hovered {
            self.anims.entry(id).or_default();
        }
        if let Some(id) = focused {
            self.anims.entry(id).or_default();
        }

        let step = if ANIM_DURATION > 0.0 {
            dt / ANIM_DURATION
        } else {
            1.0
        };
        let mut animating = false;

        self.anims.retain(|id, anim| {
            let hover_target = if Some(*id) == hovered { 1.0 } else { 0.0 };
            let focus_target = if Some(*id) == focused { 1.0 } else { 0.0 };
            approach(&mut anim.hover, hover_target, step, &mut animating);
            approach(&mut anim.focus, focus_target, step, &mut animating);
            // L'opacité tend toujours vers 1 (fondu d'apparition).
            approach(&mut anim.opacity, 1.0, step, &mut animating);
            // On oublie les entrées entièrement au repos (rien à animer).
            !(hover_target == 0.0
                && focus_target == 0.0
                && anim.hover <= 0.0
                && anim.focus <= 0.0
                && anim.opacity >= 1.0)
        });

        animating
    }

    /// Fait tendre chaque offset de défilement **courant** vers sa **cible** par
    /// un ressort (défilement lissé), avec rappel élastique aux bords (rebond).
    /// `maxes` fournit `(max_x, max_y)` par zone (issus de la dernière frame).
    /// Renvoie `true` s'il reste un défilement en mouvement.
    pub fn advance_scroll(&mut self, maxes: &[(WidgetId, f32, f32)], dt: f32) -> bool {
        let ids: Vec<WidgetId> = self.scroll_target.keys().copied().collect();
        let mut animating = false;
        for id in ids {
            let (max_x, max_y) = maxes
                .iter()
                .find(|(i, _, _)| *i == id)
                .map(|(_, x, y)| (*x, *y))
                .unwrap_or((0.0, 0.0));
            let current = self.scroll.get(&id).copied().unwrap_or((0.0, 0.0));
            let target = self.scroll_target.get(&id).copied().unwrap_or(current);
            let vel = self.scroll_velocity.get(&id).copied().unwrap_or((0.0, 0.0));

            let (cx, vx, tx, ax) = scroll_axis(current.0, vel.0, target.0, max_x, dt);
            let (cy, vy, ty, ay) = scroll_axis(current.1, vel.1, target.1, max_y, dt);

            self.scroll.insert(id, (cx, cy));
            if ax || ay {
                self.scroll_target.insert(id, (tx, ty));
                self.scroll_velocity.insert(id, (vx, vy));
                animating = true;
            } else {
                // Au repos : on nettoie l'état d'animation (l'offset courant reste).
                self.scroll_target.remove(&id);
                self.scroll_velocity.remove(&id);
            }
        }
        animating
    }

    /// Fait décroître l'opacité des sous-arbres sortants ; oublie ceux arrivés à
    /// 0. Renvoie `true` s'il reste une sortie en cours.
    pub fn advance_leaving(&mut self, dt: f32) -> bool {
        let step = if ANIM_DURATION > 0.0 {
            dt / ANIM_DURATION
        } else {
            1.0
        };
        let mut animating = false;
        self.leaving.retain(|_, (_, opacity)| {
            *opacity -= step;
            if *opacity > 0.0 {
                animating = true;
                true
            } else {
                false
            }
        });
        animating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_rises_then_falls_and_clears() {
        let id = WidgetId::ROOT.child(0);
        let mut rt = Runtime::default();
        rt.input.hovered = Some(id);

        // Survolé : petites étapes → la progression monte sans atteindre 1.
        assert!(rt.advance(0.03)); // ~0.25, encore en cours
        assert!(rt.advance(0.03)); // ~0.5, encore en cours
        let p = rt.hover_progress(id);
        assert!(p > 0.4 && p < 0.6, "progression = {p}");

        // Grand pas : atteint 1.0 puis y reste (plus d'animation).
        rt.advance(1.0);
        assert_eq!(rt.hover_progress(id), 1.0);
        assert!(!rt.advance(0.03));

        // Fin du survol : redescend (en cours), puis arrive à 0 et l'entrée disparaît.
        rt.input.hovered = None;
        assert!(rt.advance(0.03));
        rt.advance(1.0);
        assert_eq!(rt.hover_progress(id), 0.0);
        assert!(rt.anims.is_empty());
    }

    #[test]
    fn focus_animates_independently() {
        let id = WidgetId::ROOT.child(1);
        let mut rt = Runtime::default();
        rt.input.focused = Some(id);
        rt.advance(1.0);
        assert_eq!(rt.focus_progress(id), 1.0);
        assert_eq!(rt.hover_progress(id), 0.0);
    }

    #[test]
    fn opacity_rises_to_one() {
        let id = WidgetId::ROOT.child(2);
        let mut rt = Runtime::default();
        // Montage : démarre transparent.
        rt.anims.insert(id, Anim { opacity: 0.0, ..Default::default() });
        assert!(rt.advance(0.03));
        let o = rt.opacity(id);
        assert!(o > 0.0 && o < 1.0, "opacité = {o}");
        rt.advance(1.0);
        assert_eq!(rt.opacity(id), 1.0);
        // Défaut sans entrée : opaque.
        assert_eq!(rt.opacity(WidgetId::ROOT), 1.0);
    }

    #[test]
    fn value_snaps_on_mount_then_animates() {
        let mut rt = Runtime::default();
        // Montage d'un interrupteur off : adopte la cible (0) sans animation.
        let off: crate::Switch<()> = crate::Switch::new(false);
        assert!(!rt.advance_values(&off, 1.0));
        assert_eq!(rt.value(WidgetId::ROOT), 0.0);

        // Bascule on : la valeur monte vers 1 par petits pas.
        let on: crate::Switch<()> = crate::Switch::new(true);
        assert!(rt.advance_values(&on, 0.03));
        let v = rt.value(WidgetId::ROOT);
        assert!(v > 0.0 && v < 1.0, "valeur = {v}");
        rt.advance_values(&on, 1.0);
        assert_eq!(rt.value(WidgetId::ROOT), 1.0);

        // Widget disparu : la valeur est oubliée.
        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_values(&empty, 1.0);
        assert!(rt.values.is_empty());
    }

    #[test]
    fn spring_ease_is_monotonic_no_overshoot() {
        assert!((spring_ease(0.0) - 0.0).abs() < 1e-6);
        assert!((spring_ease(1.0) - 1.0).abs() < 1e-6);
        // Croissante et bornée à [0,1] (aucun dépassement au-delà de 1).
        let mut prev = 0.0;
        for i in 0..=100 {
            let v = spring_ease(i as f32 / 100.0);
            assert!(v >= prev - 1e-6, "décroît en {i}");
            assert!(v <= 1.0 + 1e-6, "dépasse 1 en {i}");
            prev = v;
        }
        // Déjà bien avancée à mi-parcours (arrivée douce en fin).
        assert!(spring_ease(0.5) > 0.7);
        // Bornée hors domaine.
        assert_eq!(spring_ease(-1.0), 0.0);
        assert_eq!(spring_ease(2.0), 1.0);
    }

    #[test]
    fn scroll_springs_to_target_and_settles() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        rt.scroll_target.insert(id, (0.0, 100.0));
        rt.scroll_velocity.insert(id, (0.0, 0.0));
        let maxes = [(id, 0.0, 200.0)];
        for _ in 0..600 {
            if !rt.advance_scroll(&maxes, 0.016) {
                break;
            }
        }
        let (_, y) = rt.scroll.get(&id).copied().unwrap();
        assert!((y - 100.0).abs() < 1.0, "arrivé à la cible : {y}");
        assert!(!rt.scroll_target.contains_key(&id), "état d'animation nettoyé au repos");
    }

    #[test]
    fn scroll_overshoot_rubber_bands_back_to_max() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        // Cible au-delà de la borne (dépassement) → doit revenir à max.
        rt.scroll_target.insert(id, (0.0, 240.0));
        rt.scroll_velocity.insert(id, (0.0, 0.0));
        let maxes = [(id, 0.0, 200.0)];
        for _ in 0..1000 {
            if !rt.advance_scroll(&maxes, 0.016) {
                break;
            }
        }
        let (_, y) = rt.scroll.get(&id).copied().unwrap();
        assert!((y - 200.0).abs() < 1.0, "revenu à la borne max : {y}");
    }

    #[test]
    fn leaving_fades_out_and_clears() {
        let mut rt = Runtime::default();
        rt.leaving.insert(0, (Vec::new(), 1.0));
        assert!(rt.advance_leaving(0.06)); // ~0.5, encore en cours
        assert!(!rt.advance_leaving(0.06)); // atteint 0 → nettoyé
        assert!(rt.leaving.is_empty());
    }
}

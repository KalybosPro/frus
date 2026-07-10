//! État retenu au runtime entre les frames, **clé par identité de widget**.
//!
//! La *valeur* d'un champ reste contrôlée (état applicatif) ; ce qui vit ici est
//! l'état d'**interaction/édition** propre aux widgets : survol/focus, offsets de
//! défilement, et position curseur/sélection des champs. C'est la fondation
//! d'une reconciliation par identité (posée au Jalon 6).

use std::collections::HashMap;

use crate::interaction::{InputState, WidgetId};

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
    /// Offsets de défilement, par zone.
    pub scroll: ScrollState,
    /// État d'édition, par champ de saisie.
    pub edits: HashMap<WidgetId, Edit>,
    /// Progressions d'animation (survol/focus/opacité), par widget.
    pub anims: HashMap<WidgetId, Anim>,
    /// Widgets présents à la frame précédente (pour détecter les montages).
    pub mounted: std::collections::HashSet<WidgetId>,
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
}

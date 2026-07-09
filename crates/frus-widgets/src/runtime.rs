//! État retenu au runtime entre les frames, **clé par identité de widget**.
//!
//! La *valeur* d'un champ reste contrôlée (état applicatif) ; ce qui vit ici est
//! l'état d'**interaction/édition** propre aux widgets : survol/focus, offsets de
//! défilement, et position curseur/sélection des champs. C'est la fondation
//! d'une reconciliation par identité (posée au Jalon 6).

use std::collections::HashMap;

use crate::interaction::{InputState, WidgetId};

/// Offsets de défilement, par zone défilable.
pub type ScrollState = HashMap<WidgetId, f32>;

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

/// Durée d'une transition de survol, en secondes.
const HOVER_DURATION: f32 = 0.12;

/// Contexte runtime transmis à `build_ui` : tout l'état retenu entre frames.
#[derive(Default)]
pub struct Runtime {
    /// Survol / pression / focus.
    pub input: InputState,
    /// Offsets de défilement, par zone.
    pub scroll: ScrollState,
    /// État d'édition, par champ de saisie.
    pub edits: HashMap<WidgetId, Edit>,
    /// Progression d'animation de survol (`0.0..=1.0`), par widget.
    pub anims: HashMap<WidgetId, f32>,
}

impl Runtime {
    /// Progression de survol animée d'un widget (`0.0` = repos, `1.0` = survolé).
    pub fn hover_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).copied().unwrap_or(0.0)
    }

    /// Fait avancer les transitions de survol de `dt` secondes vers leur cible
    /// (le widget survolé tend vers `1.0`, les autres vers `0.0`). Renvoie `true`
    /// si au moins une animation est encore en cours (pour continuer à redessiner).
    pub fn advance_hover(&mut self, dt: f32) -> bool {
        let hovered = self.input.hovered;
        if let Some(id) = hovered {
            self.anims.entry(id).or_insert(0.0);
        }

        let step = if HOVER_DURATION > 0.0 {
            dt / HOVER_DURATION
        } else {
            1.0
        };
        let mut animating = false;

        self.anims.retain(|id, progress| {
            let target = if Some(*id) == hovered { 1.0 } else { 0.0 };
            if *progress < target {
                *progress = (*progress + step).min(target);
            } else if *progress > target {
                *progress = (*progress - step).max(target);
            }
            if (*progress - target).abs() > 1e-3 {
                animating = true;
            }
            // On oublie les entrées revenues au repos.
            !(target == 0.0 && *progress <= 0.0)
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
        assert!(rt.advance_hover(0.03)); // ~0.25, encore en cours
        assert!(rt.advance_hover(0.03)); // ~0.5, encore en cours
        let p = rt.hover_progress(id);
        assert!(p > 0.4 && p < 0.6, "progression = {p}");

        // Grand pas : atteint 1.0 puis y reste (plus d'animation).
        rt.advance_hover(1.0);
        assert_eq!(rt.hover_progress(id), 1.0);
        assert!(!rt.advance_hover(0.03));

        // Fin du survol : redescend (en cours), puis arrive à 0 et l'entrée disparaît.
        rt.input.hovered = None;
        assert!(rt.advance_hover(0.03));
        rt.advance_hover(1.0);
        assert_eq!(rt.hover_progress(id), 0.0);
        assert!(rt.anims.is_empty());
    }
}

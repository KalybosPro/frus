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

/// Contexte runtime transmis à `build_ui` : tout l'état retenu entre frames.
#[derive(Default)]
pub struct Runtime {
    /// Survol / pression / focus.
    pub input: InputState,
    /// Offsets de défilement, par zone.
    pub scroll: ScrollState,
    /// État d'édition, par champ de saisie.
    pub edits: HashMap<WidgetId, Edit>,
}

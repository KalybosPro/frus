//! Identité des widgets et état d'interaction.
//!
//! Un [`WidgetId`] identifie un widget par sa **position** dans l'arbre (chemin
//! racine → indices d'enfants), de façon stable d'une frame à l'autre tant que
//! la structure de l'arbre ne change pas. C'est la brique fondatrice d'une
//! future reconciliation, et ce qui permet ici de suivre le survol/pression.

/// Identité positionnelle d'un widget (hash du chemin dans l'arbre).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct WidgetId(u64);

impl WidgetId {
    /// Identité de la racine.
    pub(crate) const ROOT: WidgetId = WidgetId(0xcbf29ce484222325);

    /// Dérive l'identité du `index`-ième enfant de ce widget.
    pub(crate) fn child(self, index: usize) -> WidgetId {
        let mut h = self.0 ^ (index as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 29;
        WidgetId(h)
    }
}

/// État visuel d'interaction d'un widget pour une frame donnée.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Interaction {
    /// Ni survolé ni pressé.
    #[default]
    None,
    /// Le pointeur est au-dessus.
    Hovered,
    /// Le pointeur est enfoncé sur ce widget.
    Pressed,
}

/// État d'entrée retenu au runtime, transmis à la construction de l'interface.
#[derive(Copy, Clone, Debug, Default)]
pub struct InputState {
    /// Widget actuellement survolé.
    pub hovered: Option<WidgetId>,
    /// Widget sur lequel le pointeur est enfoncé.
    pub pressed: Option<WidgetId>,
}

impl InputState {
    /// Statut d'interaction d'un widget donné.
    pub(crate) fn status_for(&self, id: WidgetId) -> Interaction {
        if self.pressed == Some(id) && self.hovered == Some(id) {
            Interaction::Pressed
        } else if self.hovered == Some(id) {
            Interaction::Hovered
        } else {
            Interaction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_path_yields_same_id() {
        let a = WidgetId::ROOT.child(0).child(2);
        let b = WidgetId::ROOT.child(0).child(2);
        assert_eq!(a, b);
    }

    #[test]
    fn different_paths_differ() {
        assert_ne!(WidgetId::ROOT.child(0), WidgetId::ROOT.child(1));
        assert_ne!(WidgetId::ROOT.child(0).child(1), WidgetId::ROOT.child(1).child(0));
        assert_ne!(WidgetId::ROOT, WidgetId::ROOT.child(0));
    }

    #[test]
    fn status_precedence() {
        let id = WidgetId::ROOT.child(0);
        let other = WidgetId::ROOT.child(1);

        let hovered = InputState { hovered: Some(id), pressed: None };
        assert_eq!(hovered.status_for(id), Interaction::Hovered);

        let pressed = InputState { hovered: Some(id), pressed: Some(id) };
        assert_eq!(pressed.status_for(id), Interaction::Pressed);

        // Pressé mais pointeur ailleurs → pas "Pressed".
        let moved_away = InputState { hovered: Some(other), pressed: Some(id) };
        assert_eq!(moved_away.status_for(id), Interaction::None);
    }
}

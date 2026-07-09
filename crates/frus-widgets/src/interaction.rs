//! Identité des widgets, touches clavier et état d'interaction.
//!
//! Un [`WidgetId`] identifie un widget par sa **position** dans l'arbre (chemin
//! racine → indices d'enfants), stable d'une frame à l'autre tant que la
//! structure ne change pas. C'est la brique fondatrice de la reconciliation, et
//! ce qui permet de suivre le survol, la pression et le **focus**.

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

/// Une touche transmise au widget focalisé.
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    /// Texte saisi (un ou plusieurs caractères) — sert aussi au collage.
    Text(String),
    /// Retour arrière (supprime la sélection, sinon le caractère à gauche).
    Backspace,
    /// Suppression avant (sélection, sinon caractère à droite).
    Delete,
    /// Entrée.
    Enter,
    /// Flèche gauche (Shift : étend la sélection).
    Left { shift: bool },
    /// Flèche droite.
    Right { shift: bool },
    /// Début de ligne.
    Home { shift: bool },
    /// Fin de ligne.
    End { shift: bool },
}

/// État visuel d'interaction pointeur d'un widget.
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

/// Statut complet d'un widget pour une frame : interaction pointeur, focus, et
/// (pour les champs de saisie) position du curseur et plage de sélection.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Status {
    pub interaction: Interaction,
    pub focused: bool,
    /// Index (caractère) du curseur, si ce widget est un champ focalisé.
    pub cursor: Option<usize>,
    /// Plage `(début, fin)` sélectionnée, en indices de caractères.
    pub selection: Option<(usize, usize)>,
}

/// État d'entrée retenu au runtime, transmis à la construction de l'interface.
#[derive(Copy, Clone, Debug, Default)]
pub struct InputState {
    /// Widget actuellement survolé.
    pub hovered: Option<WidgetId>,
    /// Widget sur lequel le pointeur est enfoncé.
    pub pressed: Option<WidgetId>,
    /// Widget qui a le focus clavier.
    pub focused: Option<WidgetId>,
}

impl InputState {
    /// Statut d'un widget donné.
    pub(crate) fn status_for(&self, id: WidgetId) -> Status {
        let interaction = if self.pressed == Some(id) && self.hovered == Some(id) {
            Interaction::Pressed
        } else if self.hovered == Some(id) {
            Interaction::Hovered
        } else {
            Interaction::None
        };
        Status {
            interaction,
            focused: self.focused == Some(id),
            cursor: None,
            selection: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_path_yields_same_id() {
        assert_eq!(WidgetId::ROOT.child(0).child(2), WidgetId::ROOT.child(0).child(2));
    }

    #[test]
    fn different_paths_differ() {
        assert_ne!(WidgetId::ROOT.child(0), WidgetId::ROOT.child(1));
        assert_ne!(WidgetId::ROOT.child(0).child(1), WidgetId::ROOT.child(1).child(0));
        assert_ne!(WidgetId::ROOT, WidgetId::ROOT.child(0));
    }

    #[test]
    fn status_precedence_and_focus() {
        let id = WidgetId::ROOT.child(0);
        let other = WidgetId::ROOT.child(1);

        let hovered = InputState { hovered: Some(id), ..Default::default() };
        assert_eq!(hovered.status_for(id).interaction, Interaction::Hovered);

        let pressed = InputState { hovered: Some(id), pressed: Some(id), ..Default::default() };
        assert_eq!(pressed.status_for(id).interaction, Interaction::Pressed);

        // Pressé mais pointeur ailleurs → pas "Pressed".
        let moved_away = InputState { hovered: Some(other), pressed: Some(id), ..Default::default() };
        assert_eq!(moved_away.status_for(id).interaction, Interaction::None);

        // Focus indépendant de l'interaction pointeur.
        let focused = InputState { focused: Some(id), ..Default::default() };
        assert!(focused.status_for(id).focused);
        assert!(!focused.status_for(other).focused);
    }
}

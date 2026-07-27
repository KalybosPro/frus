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

    /// Valeur brute de l'identité (pour tagguer les primitives).
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Reconstruit une identité depuis sa valeur brute (inverse d'[`as_u64`]) —
    /// pour router une action venue d'une couche externe (accessibilité).
    ///
    /// [`as_u64`]: WidgetId::as_u64
    pub fn from_u64(raw: u64) -> WidgetId {
        WidgetId(raw)
    }

    /// Dérive l'identité du `index`-ième enfant de ce widget (positionnel).
    pub(crate) fn child(self, index: usize) -> WidgetId {
        let mut h = self.0 ^ (index as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 29;
        WidgetId(h)
    }

    /// Dérive l'identité d'un enfant **par clé** (stable quel que soit sa position).
    /// Distincte de [`WidgetId::child`] (constante et décalage différents).
    pub(crate) fn keyed(self, key: u64) -> WidgetId {
        let mut h = self.0 ^ key.wrapping_add(0x517c_c1b7_2722_0a95);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 31;
        WidgetId(h)
    }
}

/// Réponse d'un widget à une touche reçue pendant la **montée feuille→racine**
/// (le focalisé d'abord, puis ses ancêtres tant que le résultat est `Ignored`).
#[derive(Clone, Debug, PartialEq)]
pub enum KeyResponse<Msg> {
    /// Pas concerné : la touche continue de remonter.
    Ignored,
    /// Consommée (avec un message éventuel à émettre) : la montée s'arrête.
    Handled(Option<Msg>),
    /// Non consommée mais la montée s'arrête (et aucun repli n'est tenté).
    Skip,
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
    /// Échappement (fermer/annuler) — routé feuille→racine, jamais à l'édition.
    Escape,
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

/// Statut complet d'un widget pour une frame : interaction pointeur, focus,
/// curseur/sélection (champs), progressions d'animation et opacité.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Status {
    pub interaction: Interaction,
    pub focused: bool,
    /// Index (caractère) du curseur, si ce widget est un champ focalisé.
    pub cursor: Option<usize>,
    /// Plage `(début, fin)` sélectionnée, en indices de caractères.
    pub selection: Option<(usize, usize)>,
    /// Plage `(début, fin)` en **cours de composition** IME (texte provisoire,
    /// souligné) ; `None` hors composition. En indices de caractères.
    pub composing: Option<(usize, usize)>,
    /// Progression de la transition de survol (`0.0..=1.0`).
    pub hover_progress: f32,
    /// Progression de la transition de focus (`0.0..=1.0`).
    pub focus_progress: f32,
    /// Opacité à appliquer (fondu d'apparition) ; `1.0` = opaque.
    pub opacity: f32,
    /// Valeur animée propre au widget (p. ex. `0 → 1` d'un interrupteur), pilotée
    /// par `Widget::anim_target`.
    pub value: f32,
    /// Temps écoulé (secondes) depuis le démarrage — pour les animations
    /// continues pilotées par le temps (ex. `Spinner`).
    pub time: f32,
    /// Couleur **interpolée** d'un fond animé (`Container::animated_color`), si en
    /// transition ; `None` = pas de couleur animée (le widget utilise sa couleur).
    pub anim_color: Option<frus_core::Color>,
    /// Rayon de coin **interpolé** (`Container::animated_radius`), si en
    /// transition ; `None` = rayon fixe (le widget utilise le sien).
    pub anim_radius: Option<frus_core::BorderRadius>,
    /// Défilement vertical **retenu** de ce widget (px), pour les widgets qui
    /// défilent leur propre contenu (champ multi-lignes) ; `0` sinon.
    pub scroll_y: f32,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            interaction: Interaction::None,
            focused: false,
            cursor: None,
            selection: None,
            composing: None,
            hover_progress: 0.0,
            focus_progress: 0.0,
            opacity: 1.0,
            value: 0.0,
            time: 0.0,
            anim_color: None,
            anim_radius: None,
            scroll_y: 0.0,
        }
    }
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
            composing: None,
            hover_progress: 0.0,
            focus_progress: 0.0,
            opacity: 1.0,
            value: 0.0,
            time: 0.0,
            anim_color: None,
            anim_radius: None,
            scroll_y: 0.0,
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
    fn keyed_is_stable_and_distinct() {
        // Même clé sous le même parent → même identité (indépendante de la position).
        assert_eq!(WidgetId::ROOT.keyed(7), WidgetId::ROOT.keyed(7));
        // Clés différentes → identités différentes.
        assert_ne!(WidgetId::ROOT.keyed(7), WidgetId::ROOT.keyed(8));
        // Une clé n'entre pas en collision avec un indice positionnel de même valeur.
        assert_ne!(WidgetId::ROOT.keyed(0), WidgetId::ROOT.child(0));
        // Parents différents → identités différentes pour la même clé.
        assert_ne!(WidgetId::ROOT.child(0).keyed(7), WidgetId::ROOT.child(1).keyed(7));
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

//! Le trait [`Application`] : le contrat entre une **application** et le
//! framework. Le shell (fenêtre, renderer, entrées, runtime, animations) est
//! générique sur ce trait ; l'application ne fournit que sa logique.
//!
//! Une app minimale n'implémente que `update` et `view`. Les autres méthodes ont
//! des valeurs par défaut (thème sombre fixe, pas d'animation, pas de navigation).

use frus_widgets::{Theme, Widget};

/// Ce qu'une application fournit au framework (modèle à messages, façon Elm).
pub trait Application {
    /// Type de message émis par l'interface et consommé par [`Application::update`].
    type Message: Clone;

    /// Fait évoluer l'état applicatif en réponse à un message.
    fn update(&mut self, message: Self::Message);

    /// Construit l'arbre de widgets pour la taille et le thème courants.
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Self::Message>>;

    /// Thème affiché (peut être un mélange animé) — sombre par défaut.
    fn theme(&self) -> Theme {
        Theme::dark()
    }

    /// Fait avancer les animations **propres à l'app** (fondu de thème, transitions
    /// d'écran, détente de geste…) de `dt` secondes. Renvoie `true` si quelque
    /// chose bouge encore (le framework redemandera alors une frame).
    fn tick(&mut self, _dt: f32) -> bool {
        false
    }

    /// Titre de la fenêtre.
    fn title(&self) -> String {
        "frus".to_string()
    }

    /// L'app peut-elle revenir en arrière ? (active le geste retour depuis le bord).
    fn can_go_back(&self) -> bool {
        false
    }

    /// Le geste retour progresse (`0 → 1`) — pour prévisualiser via `view`.
    fn back_gesture(&mut self, _progress: f32) {}

    /// Le geste retour est relâché, avec la vélocité du doigt (fraction/s) — à
    /// l'app de valider ou d'annuler (généralement via une détente animée en `tick`).
    fn back_gesture_end(&mut self, _velocity: f32) {}
}

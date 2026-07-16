//! Le trait [`Application`] : le contrat entre une **application** et le
//! framework. Le shell (fenêtre, renderer, entrées, runtime, animations) est
//! générique sur ce trait ; l'application ne fournit que sa logique.
//!
//! Une app minimale n'implémente que `update` et `view`. Les autres méthodes ont
//! des valeurs par défaut (thème sombre fixe, pas d'animation, pas de navigation).

use frus_widgets::{Theme, Widget, WindowInsets};

use crate::command::Command;
use crate::subscription::Subscription;

/// Ce qu'une application fournit au framework (modèle à messages, façon Elm).
pub trait Application {
    /// Type de message émis par l'interface et consommé par [`Application::update`].
    ///
    /// `Send + 'static` car les effets ([`Command`]) traversent des threads.
    type Message: Clone + Send + 'static;

    /// Fait évoluer l'état en réponse à un message, et renvoie les **effets** à
    /// exécuter (I/O, tâches de fond…). Utiliser [`Command::none`] si aucun.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message>;

    /// Effet à exécuter **au démarrage** (chargement initial, etc.).
    fn init(&mut self) -> Command<Self::Message> {
        Command::none()
    }

    /// Sources **continues** de messages (timers…), déclarées selon l'état. Le
    /// framework les démarre/arrête par diff à chaque cycle.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

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

    /// Taille **logique** initiale souhaitée de la fenêtre (`None` = défaut système).
    fn window_size(&self) -> Option<(f32, f32)> {
        None
    }

    /// **Densité** de l'interface : un facteur de zoom **applicatif** (défaut
    /// `1.0`) appliqué par-dessus l'échelle DPI système. `1.2` agrandit toute
    /// l'UI de 20 % ; `0.9` la resserre. Change à chaud via l'état.
    fn density(&self) -> f32 {
        1.0
    }

    /// Appelé quand la taille **logique** de la surface change (redimensionnement
    /// de la fenêtre **ou** changement de densité). Permet à l'app de réagir au
    /// **changement de palier** dans sa logique (p. ex. fermer un tiroir en
    /// rétrécissant), au-delà du simple rendu.
    fn on_resize(&mut self, _width: f32, _height: f32) {}

    /// Appelé quand les **insets fenêtre** changent, séparés par nature :
    /// `padding` = barres système/encoche (statique), `view_insets` = clavier
    /// logiciel (dynamique — l'évitement du clavier consiste à écarter le contenu
    /// de `insets.safe()`). En px **logiques**. Sur bureau : toujours zéro.
    fn on_insets(&mut self, _insets: WindowInsets) {}

    /// **Live-reload** (dev) : instantané sérialisé de l'état, capturé juste
    /// avant qu'un binaire recompilé relance l'application (`FRUS_WATCH=1`).
    /// `None` (défaut) = l'app ne participe pas — le rechargement repart alors
    /// d'un état neuf. Le format des octets appartient à l'app.
    fn save_state(&self) -> Option<Vec<u8>> {
        None
    }

    /// Réhydrate l'état depuis un instantané [`Application::save_state`] pris
    /// par le binaire **précédent** (appelé avant [`Application::init`]). Les
    /// octets viennent d'une autre version du code : tolérer les formats
    /// inattendus (ignorer vaut mieux que paniquer).
    fn restore_state(&mut self, _bytes: &[u8]) {}

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

//! `frus-hello` — la plus petite application frus complète : un compteur.
//!
//! C'est le « Hello, world! » du framework et la source du template
//! `cargo generate` (voir `templates/app/`). Il tient en une struct d'état,
//! un `update` pur et une `view` — le modèle Elm au complet.
//!
//! Lancer sur bureau : `cargo run -p frus-hello`.

use std::time::Duration;

use frus_shell::{Application, Command, Subscription};
use frus_widgets::{button, column, text, Align, Container, Justify, Theme, Variant, Widget};

/// L'état de l'application : un simple compteur.
#[derive(Default)]
struct Counter {
    count: i32,
    /// Comptage automatique : quand actif, une **souscription** `every(1s)` incrémente
    /// le compteur. La même source continue tourne sur bureau (thread), Android et Web
    /// (`setInterval`) — l'app ne connaît que `Subscription::every`.
    auto: bool,
}

/// Les messages émis par l'interface.
#[derive(Clone)]
enum Msg {
    Increment,
    Decrement,
    /// (Dé)active le comptage automatique.
    ToggleAuto,
    /// Un tick de la souscription : incrémente.
    Tick,
}

impl Application for Counter {
    type Message = Msg;

    /// `update` est **pur** : il fait évoluer l'état et renvoie les effets
    /// éventuels (ici aucun). Testable sans GPU ni fenêtre.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Increment | Msg::Tick => self.count += 1,
            Msg::Decrement => self.count -= 1,
            Msg::ToggleAuto => self.auto = !self.auto,
        }
        Command::none()
    }

    /// Souscriptions **selon l'état** : en mode auto, émet un `Tick` par seconde ;
    /// sinon aucune. Le framework démarre/arrête la source par diff.
    fn subscription(&self) -> Subscription<Msg> {
        if self.auto {
            Subscription::every(Duration::from_secs(1), |_| Msg::Tick)
        } else {
            Subscription::none()
        }
    }

    /// `view` décrit l'interface pour l'état courant — une fonction pure de
    /// `(état, thème, taille)`. Le framework la (re)construit au besoin.
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        let content = column![
            text(format!("{}", self.count)).size(48.0),
            column![
                button("+", Msg::Increment).variant(Variant::Primary),
                button("−", Msg::Decrement).variant(Variant::Secondary),
                button(
                    if self.auto { "Stop auto" } else { "Start auto" },
                    Msg::ToggleAuto,
                )
                .variant(Variant::Secondary),
            ]
            .gap(8.0)
            .align(Align::Center),
        ]
        .gap(16.0)
        .align(Align::Center);

        // Centré à l'écran : un Flex plein-fenêtre qui centre son unique enfant
        // sur les deux axes, posé sur le fond du thème.
        let centered = column![content]
            .width(width)
            .height(height)
            .justify(Justify::Center)
            .align(Align::Center);

        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(theme.background)
                .child(centered),
        )
    }

    fn title(&self) -> String {
        "frus — counter".to_string()
    }
}

/// Point d'entrée **bureau** : ouvre la fenêtre et lance la boucle.
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
pub fn run_desktop() -> anyhow::Result<()> {
    frus_shell::run(Counter::default())
}

/// Point d'entrée **Web** : appelé automatiquement au chargement du module wasm
/// (`wasm-bindgen`). Attache un canvas et lance la boucle pilotée par le navigateur.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    if let Err(err) = frus_shell::run_web(Counter::default()) {
        log::error!("frus-hello (web) s'est arrêté : {err:#}");
    }
}

/// Point d'entrée **Android** : appelé par l'activité native.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: frus_shell::AndroidApp) {
    if let Err(err) = frus_shell::run_android(Counter::default(), android_app) {
        log::error!("frus-hello (android) s'est arrêté : {err:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'avantage Elm : `update` se teste sans GPU ni fenêtre.
    #[test]
    fn counting_is_pure() {
        let mut app = Counter::default();
        app.update(Msg::Increment);
        app.update(Msg::Increment);
        app.update(Msg::Decrement);
        assert_eq!(app.count, 1);
    }

    /// Le mode auto pilote la souscription : absente au repos, présente une fois
    /// activée (le framework la démarre/arrête par ce diff), et un `Tick` compte.
    #[test]
    fn auto_mode_drives_the_subscription() {
        let mut app = Counter::default();
        assert!(app.subscription().is_empty(), "au repos : aucune souscription");
        app.update(Msg::ToggleAuto);
        assert!(!app.subscription().is_empty(), "auto : une source every(1s)");
        app.update(Msg::Tick);
        assert_eq!(app.count, 1, "un tick incrémente");
        app.update(Msg::ToggleAuto);
        assert!(app.subscription().is_empty(), "auto coupé : plus de souscription");
    }
}

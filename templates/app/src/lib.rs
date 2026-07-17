//! `{{project-name}}` — une application frus (le modèle Elm : état, `update`, `view`).
//!
//! - Bureau : `cargo run`
//! - Android : `cargo apk run`   (voir https://… le guide de démarrage)

use frus_shell::{Application, Command};
use frus_widgets::{button, column, text, Align, Container, Justify, Theme, Variant, Widget};

/// L'état de l'application : un simple compteur.
#[derive(Default)]
struct App {
    count: i32,
}

/// Les messages émis par l'interface.
#[derive(Clone)]
enum Msg {
    Increment,
    Decrement,
}

impl Application for App {
    type Message = Msg;

    /// `update` est **pur** : il fait évoluer l'état et renvoie les effets
    /// éventuels (ici aucun). Testable sans GPU ni fenêtre.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Increment => self.count += 1,
            Msg::Decrement => self.count -= 1,
        }
        Command::none()
    }

    /// `view` décrit l'interface pour l'état courant — une fonction pure de
    /// `(état, thème, taille)`.
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        let content = column![
            text(format!("{}", self.count)).size(48.0),
            column![
                button("+", Msg::Increment).variant(Variant::Primary),
                button("−", Msg::Decrement).variant(Variant::Secondary),
            ]
            .gap(8.0)
            .align(Align::Center),
        ]
        .gap(16.0)
        .align(Align::Center);

        // Centré à l'écran, posé sur le fond du thème.
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
        "{{project-name}}".to_string()
    }
}

/// Point d'entrée **bureau** : ouvre la fenêtre et lance la boucle.
#[cfg(not(target_os = "android"))]
pub fn run_desktop() -> anyhow::Result<()> {
    frus_shell::run(App::default())
}

/// Point d'entrée **Android** : appelé par l'activité native.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: frus_shell::AndroidApp) {
    if let Err(err) = frus_shell::run_android(App::default(), android_app) {
        log::error!("{{project-name}} (android) s'est arrêté : {err:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'avantage Elm : `update` se teste sans GPU ni fenêtre.
    #[test]
    fn counting_is_pure() {
        let mut app = App::default();
        app.update(Msg::Increment);
        app.update(Msg::Increment);
        app.update(Msg::Decrement);
        assert_eq!(app.count, 1);
    }
}

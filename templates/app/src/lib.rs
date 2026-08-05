//! `{{project-name}}` — une application frus (le modèle Elm : état, `update`, `view`).
//!
//! - Bureau : `cargo run`
//! - Android : `cargo apk run`   (voir https://… le guide de démarrage)

// Une **seule** dépendance : la façade `frus` fournit tout (couche framework + widgets + DSL).
use frus::{button, column, text, Align, Application, Command, Container, Justify, Theme, Variant, Widget};

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

// **Point d'entrée unique** — façon Flutter (`void main() => runApp(App())`) : une seule
// déclaration engendre les entrées bureau / Android / Web (voir `frus::main!`). Le mince
// binaire `src/bin/{{project-name}}.rs` appelle la `run()` ainsi produite pour le bureau.
frus::main!(App::default());

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

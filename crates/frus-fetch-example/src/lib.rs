//! `frus-fetch-example` — la **chaîne réseau de bout en bout** en une écran :
//! un bouton lance un GET, l'écran passe par **chargement → donnée** (ou **erreur**).
//!
//! C'est le pendant frus du `FutureBuilder` de Flutter, et la démonstration de la pile
//! ajoutée aux jalons 270–272 :
//!
//! - [`frus::Command::perform_async`] mène une future à terme, sa valeur devient un message ;
//! - [`frus::fetch`] / [`frus::Request`] font l'aller-retour HTTP (ici en-têtes + timeout).
//!
//! Tout tient dans le modèle Elm : un `update` **pur** (la seule impureté, le réseau, est
//! reléguée dans une `Command`) et une `view` qui n'affiche que l'état courant.
//!
//! Lancer sur bureau : `cargo run -p frus-fetch-example` (ajouter `RUST_LOG=info` pour les logs).

use std::time::Duration;

// Une **seule** dépendance : la façade `frus` (feature `net` pour `fetch` / `Request`).
use frus::{
    button, column, text, Align, Application, Color, Command, Container, Justify, RemoteData,
    Request, Theme, Variant, Widget,
};

/// L'API interrogée : une blague renvoyée en **texte simple** (en-tête `Accept: text/plain`).
/// Elle autorise les requêtes navigateur (CORS), donc l'exemple marche aussi sur le Web.
const JOKE_URL: &str = "https://icanhazdadjoke.com/";

/// L'état : le statut de la requête, exprimé par l'idiome [`RemoteData`] du framework
/// (`NotAsked → Loading → Success | Failure`) plutôt qu'une machine à états maison.
#[derive(Default)]
struct FetchDemo {
    joke: RemoteData<String>,
}

/// Les messages émis par l'interface et par l'effet réseau.
#[derive(Clone)]
enum Msg {
    /// L'utilisateur a demandé un chargement.
    Fetch,
    /// L'effet réseau a abouti : `Ok(corps)` ou `Err(message)`.
    Got(Result<String, String>),
}

impl Application for FetchDemo {
    type Message = Msg;

    /// `update` reste **pur** : il fait évoluer l'état et, pour `Fetch`, renvoie l'**effet**
    /// réseau (l'unique impureté). Quand la future se résout, le shell rappelle `update`
    /// avec `Got(...)`. Aucun `await` ni GPU ici — donc testable tel quel.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Fetch => {
                self.joke = RemoteData::Loading;
                // Un GET avec en-tête (texte simple) et un timeout : si l'API ne répond
                // pas en 5 s, on récolte un `FetchError::Network` → branche `Failure`.
                return Command::perform_async(async {
                    let res = Request::get(JOKE_URL)
                        .header("Accept", "text/plain")
                        .header("User-Agent", "frus-fetch-example (github.com/frus)")
                        .timeout(Duration::from_secs(5))
                        .send()
                        .await;
                    Msg::Got(res.map_err(|err| err.to_string()))
                });
            }
            // Le `Result` de l'effet devient directement un `RemoteData` (corps rogné au passage).
            Msg::Got(res) => {
                self.joke = RemoteData::from_result(res.map(|body| body.trim().to_string()))
            }
        }
        Command::none()
    }

    /// `view` ne fait que peindre l'état : un bouton, puis le rendu du statut courant.
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        // Étiquette du bouton selon l'état (relance possible même après coup).
        let label = match self.joke {
            RemoteData::NotAsked => "Get a joke",
            RemoteData::Loading => "Loading…",
            _ => "Get another joke",
        };

        // La zone de résultat : on **replie** les quatre cas de `RemoteData`.
        let result: Box<dyn Widget<Msg>> = match self.joke.as_ref() {
            RemoteData::NotAsked => Box::new(text("Press the button to fetch a joke.").size(18.0)),
            RemoteData::Loading => Box::new(text("Loading…").size(18.0)),
            RemoteData::Success(body) => Box::new(text(body.clone()).size(22.0)),
            RemoteData::Failure(err) => Box::new(
                text(format!("Failed: {err}"))
                    .size(18.0)
                    .color(Color::rgb(0.85, 0.2, 0.2)),
            ),
        };

        let content = column![
            text("frus · fetch").size(14.0).color(theme.muted),
            button(label, Msg::Fetch).variant(Variant::Primary),
            Container::new().width(width.min(420.0)).child(result),
        ]
        .gap(20.0)
        .align(Align::Center);

        // Centré plein écran, sur le fond du thème.
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
        "frus — fetch".to_string()
    }
}

// **Point d'entrée unique** (façon Flutter) : engendre les entrées bureau / Android / Web.
frus::main!(FetchDemo::default());

#[cfg(test)]
mod tests {
    use super::*;

    /// `Fetch` bascule en chargement **et** renvoie un effet (la future réseau) — sans
    /// réseau ni GPU dans le test, on n'observe que l'intention.
    #[test]
    fn fetch_enters_loading_and_emits_an_effect() {
        let mut app = FetchDemo::default();
        assert_eq!(app.joke, RemoteData::NotAsked);
        let cmd = app.update(Msg::Fetch);
        assert!(app.joke.is_loading());
        assert!(!cmd.is_empty(), "Fetch doit produire un effet réseau");
    }

    /// La résolution de l'effet peint l'état : succès → donnée (rognée), échec → erreur.
    #[test]
    fn result_messages_drive_the_state() {
        let mut app = FetchDemo::default();

        app.update(Msg::Got(Ok("  a good joke  ".to_string())));
        assert_eq!(app.joke.value().map(String::as_str), Some("a good joke"));

        app.update(Msg::Got(Err("HTTP status 500".to_string())));
        assert_eq!(
            app.joke.error().map(String::as_str),
            Some("HTTP status 500")
        );
    }
}

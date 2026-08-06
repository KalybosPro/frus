# Jalon 273 — Exemple réseau de bout en bout (`frus-fetch-example`)

## Objectif

Les jalons 270–272 ont bâti la pile réseau (effets async, `fetch`, `Request` avec
POST/en-têtes/timeout) mais **aucun écran ne l'exerçait**. Ce jalon livre le petit exemple
manquant : **charger une API et l'afficher**, avec les états **chargement → donnée → erreur** —
le pendant frus du `FutureBuilder` de Flutter.

C'est aussi la preuve de l'ergonomie revendiquée : **une seule dépendance** (`frus`, feature
`net`), **un seul point d'entrée** (`frus::main!`), et le modèle Elm au complet.

## L'écran

Un bouton lance la requête ; l'écran peint le statut courant :

```rust
enum Status { Idle, Loading, Loaded(String), Failed(String) }

fn update(&mut self, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::Fetch => {
            self.status = Status::Loading;
            return Command::perform_async(async {
                let res = Request::get(JOKE_URL)
                    .header("Accept", "text/plain")
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await;
                Msg::Got(res.map_err(|e| e.to_string()))
            });
        }
        Msg::Got(Ok(body)) => self.status = Status::Loaded(body.trim().to_string()),
        Msg::Got(Err(err)) => self.status = Status::Failed(err),
    }
    Command::none()
}
```

- **`update` reste pur** : la seule impureté (le réseau) est reléguée dans la `Command` ;
  quand la future se résout, le shell rappelle `update` avec `Got(...)`. Testable sans GPU.
- **`view` ne peint que l'état** : bouton + rendu du `Status`.

## L'API interrogée

`https://icanhazdadjoke.com/` avec l'en-tête `Accept: text/plain` — une blague en **texte
simple** (pas de JSON à parser). L'endpoint autorise les requêtes navigateur (**CORS**), donc
l'exemple fonctionne aussi **sur le Web**, pas seulement bureau/Android. En-tête + timeout
exercent le `Request` du jalon 272.

## Vérification

- **Build bureau** : `cargo build -p frus-fetch-example` — compile.
- **Build wasm** (`--target wasm32-unknown-unknown`) : compile.
- **Tests** (2) : `fetch_enters_loading_and_emits_an_effect` (bascule en `Loading` **et**
  renvoie un effet non vide), `result_messages_drive_the_state` (`Ok` → `Loaded` rogné,
  `Err` → `Failed`). L'aller-retour réseau réel se lance à la main (`cargo run -p
  frus-fetch-example`) — non exécuté ici.

## Lancer

```
cargo run -p frus-fetch-example
```

# Jalon 272 — `Request` : POST, en-têtes et timeout sur `fetch` (feature `net`)

## Objectif

Le jalon 271 a livré le socle : `fetch(url)`, un **GET texte** cross-plateforme. Une vraie
app a besoin de plus — **poster** un corps, **fixer des en-têtes** (`Content-Type`,
`Authorization`…), **borner l'attente** par un timeout. Ce jalon ajoute un **constructeur
de requête** qui couvre tout ça, sans casser le raccourci `fetch`.

## API

Deux niveaux, une seule signature de sortie (`Result<String, FetchError>`) pour les trois
cibles :

```rust
use frus::{Command, Request};
use std::time::Duration;

// Raccourci inchangé : GET texte.
Msg::Load => Command::perform_async(async {
    match frus::fetch("https://example.com/api").await {
        Ok(body) => Msg::Loaded(body),
        Err(err) => Msg::Failed(err.to_string()),
    }
}),

// POST JSON, en-tête, délai maximal.
Msg::Save(json) => Command::perform_async(async move {
    let res = Request::post("https://example.com/api")
        .header("Content-Type", "application/json")
        .body(json)
        .timeout(Duration::from_secs(10))
        .send()
        .await;
    match res { Ok(_) => Msg::Saved, Err(e) => Msg::Failed(e.to_string()) }
}),
```

- `Request::{get, post, put, delete}(url)` ou `Request::new(Method, url)`.
- `.header(name, value)` — **cumulable** (plusieurs appels n'écrasent rien).
- `.body(text)` — corps de la requête (le dernier appel gagne).
- `.timeout(Duration)` — délai avant abandon (rendu en `FetchError::Network`).
- `.send().await -> Result<String, FetchError>`.
- `fetch(url)` reste, et vaut exactement `Request::get(url).send().await`.

`Method` : `Get`, `Post`, `Put`, `Delete`, `Patch`, `Head` (`as_str()` → verbe HTTP).

## Implémentation par plateforme

- **Natif** : `ureq::request(method, url)`, `.set(name, value)` par en-tête, `.timeout(dur)`,
  puis `.send_string(body)` si un corps est fourni, sinon `.call()`.
- **Web** : `window.fetch` via un `web_sys::Request` bâti depuis un `RequestInit` (méthode,
  `Headers`, corps). Le **timeout** est armé par un `AbortController` dont le signal est passé
  à la requête ; un `setTimeout` déclenche `abort()` au-delà du délai, et le minuteur est
  **désarmé** (`clearTimeout`) dès la réponse reçue.

Même chaînage, la seule différence est cachée derrière deux `#[cfg]`.

## Vérification

- **Build natif `--features net`** : `frus-shell` et la façade `frus` compilent (ureq + rustls).
- **Build wasm `--features net`** (`--target wasm32-unknown-unknown`) : compile — les bindings
  `Request`/`RequestInit`/`Headers`/`AbortController`/`AbortSignal` sont ajoutés aux features
  `web-sys`.
- **Tests** (4) : `error_display_is_readable`, `method_verbs`,
  `builder_accumulates_headers_body_and_timeout`, `fetch_shortcut_is_a_bare_get`. Un aller-retour
  réseau réel dépend du réseau/navigateur — non exécuté ici ; le transport est délégué à
  `ureq`/`web-sys`.

## Reste

- Flux (corps non bufferisé), réponses binaires, redirections fines — au besoin. Le cœur
  (méthode + en-têtes + corps + timeout) est là.

# Jalon 271 — Helper `fetch` cross-plateforme (feature `net`)

## Objectif

Le jalon 270 a donné le **mécanisme** d'effet asynchrone (`Command::perform_async`) mais laissait
l'app fournir la future — donc toucher `web-sys` / un client HTTP elle-même. Ce jalon livre le
**helper manquant** : un GET HTTP **cross-plateforme**, `frus::fetch(url).await`, une seule signature
pour les trois cibles.

## API

```rust
use frus::{Command, fetch};

Msg::Load => Command::perform_async(async {
    match fetch("https://example.com/api").await {
        Ok(body) => Msg::Loaded(body),
        Err(err) => Msg::Failed(err.to_string()),
    }
}),
```

- `async fn fetch(url: impl Into<String>) -> Result<String, FetchError>` — GET, corps en texte.
- `FetchError` : `Network(String)` (transport/DNS/TLS), `Status(u16)` (non-2xx), `Decode(String)`
  (corps illisible). Implémente `Display` + `Error`.

## Implémentation par plateforme

- **Web** (`wasm32`) : `window.fetch` via `web-sys` (+ feature `Response`), `await` réel — la future
  n'est **pas** `Send`, ce que tolère le `perform_async` du Web.
- **Natif** : le client bloquant **`ureq`** (TLS rustls inclus), exécuté **dans le corps de la
  future** — menée à terme sur le thread dédié de `perform_async`, où bloquer est sans risque. La
  future reste `Send`.

Même signature, la seule différence est cachée derrière deux `#[cfg]`.

## Derrière une feature (opt-in)

- **`frus-shell`** : `[features] net = ["dep:ureq"]` ; `ureq` est une dépendance **native optionnelle**
  ; module `net` et ré-exports (`fetch`, `FetchError`) gardés `#[cfg(feature = "net")]`.
- **`frus`** (façade) : `[features] net = ["frus-shell/net"]` + ré-export `frus::{fetch, net,
  FetchError}`.
- **Par défaut, `net` est éteinte** : une app qui ne réseau pas n'embarque **ni `ureq` ni sa pile
  TLS**. On l'active avec `frus = { path = "…", features = ["net"] }`.

## Vérification

- **Build par défaut** (`net` éteinte) : `frus-shell` compile, inchangé — aucun coût.
- **Build `--features net`** : `frus-shell` compile avec `ureq` + rustls.
- **Test** : `error_display_is_readable` (format des `FetchError`). Un GET réel dépend du réseau/d'un
  navigateur — non exécuté ici (et binaires de test bloqués par SAC cette session) ; la logique de
  transport est déléguée à `ureq`/`web-sys`.

## Reste

- En-têtes, **POST**/corps, timeouts, flux — au besoin. Le socle (`fetch` GET) est là.

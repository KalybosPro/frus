# Jalon 275 — JSON typé sur `Request` (feature `json`)

## Objectif

`fetch` / `Request::send` rendent une `String`. Une vraie app veut un **type métier** —
`RemoteData<User>`, pas `RemoteData<String>` qu'elle re-parse à la main à chaque écran. Ce
jalon ajoute, derrière une feature `json`, les deux bouts du pont JSON :

- **lire** : `Request::send_json::<T>()` désérialise la réponse en `T` ;
- **écrire** : `Request::json_body(&value)` poste une valeur sérialisable.

## API

```rust
use frus::{Request, RemoteData};

// Lire : réponse JSON → type métier.
#[derive(serde::Deserialize)]
struct User { id: u64, name: String }

let user: User = Request::get(url).send_json().await?;         // RemoteData<User> côté état

// Écrire : valeur → corps JSON (+ en-tête Content-Type: application/json).
#[derive(serde::Serialize)]
struct NewPost { title: String, body: String }

Request::post(url).json_body(&payload).send().await?;
```

- `send_json::<T: DeserializeOwned>() -> Result<T, FetchError>` = `send()` + `serde_json::from_str` ;
  un corps illisible ou non conforme à `T` donne un `FetchError::Decode`.
- `json_body<B: Serialize>(&B)` sérialise le corps et pose l'en-tête `Content-Type`. Le
  chaînage reste **fluide** : une erreur de sérialisation (rare) est **différée** et ressort à
  `send()` (motif du builder de `reqwest`), via un champ `error: Option<FetchError>` sur
  `Request`.

## Feature

- `frus-shell` : `json = ["net", "dep:serde", "dep:serde_json"]` — **`json` implique `net`**
  (le JSON n'a de sens qu'avec la couche HTTP). `serde`/`serde_json` sont **pur Rust**, donc
  valables sur les trois cibles (contrairement à `ureq`, natif seulement).
- `frus` (façade) : `json = ["frus-shell/json"]`.
- Par défaut `json` (comme `net`) est **éteinte** : aucune dépendance serde embarquée.

## Vérification

- **Tests** (2 nouveaux, pur, sans réseau) : `json_body_serializes_and_sets_content_type`
  (corps `{"x":1,"y":2}` + en-tête posé, pas d'erreur différée) et
  `decode_json_maps_valid_and_invalid_bodies` (parsing valide → type ; corps illisible →
  `FetchError::Decode`). Le décodage est isolé dans un helper `decode_json` testé à part de
  l'E/S.
- **Builds** : `--features json` (natif + `wasm32-unknown-unknown`), façade `--features json`,
  et les combinaisons `net` seul / défaut — toutes OK.

## Reste

- Statuts non-2xx avec corps d'erreur JSON, en-tête `Accept: application/json` automatique —
  au besoin. Le pont (lire/écrire du JSON typé) est là.

# Jalon 274 — `RemoteData<T, E>` : l'idiome Elm pour une donnée asynchrone

## Objectif

Dans l'exemple du jalon 273, l'écran écrivait **à la main** une machine à états
`Idle/Loading/Loaded/Failed`. C'est le motif que **toute** app réseau réécrit — et qu'on rate
souvent (un `Option<Result<T, E>>` ambigu, ou deux booléens `loading`/`error`
désynchronisables). Ce jalon livre l'idiome Elm consacré, [`RemoteData`], comme **type de
framework** (`frus::RemoteData`), puis refactore l'exemple dessus.

## Le type

```rust
pub enum RemoteData<T, E = String> {
    NotAsked,   // rien demandé encore (état initial, Default)
    Loading,    // requête en vol
    Success(T), // la donnée est arrivée
    Failure(E), // la requête a échoué
}
```

Les quatre états sont **exclusifs** ; replier dessus dans la `view` force le compilateur à
traiter chacun. `E` vaut `String` par défaut (le cas courant après `FetchError::to_string()`).

**Méthodes** : `from_result(Result<T, E>)` (le pont depuis un effet), `is_loading` /
`is_success` / `is_failure`, `value() -> Option<&T>`, `error() -> Option<&E>`,
`as_ref() -> RemoteData<&T, &E>` (replier sans consommer), `map` / `map_err` (transformer un
seul cas — ex. décoder un corps en type métier).

## Avant / après (dans `frus-fetch-example`)

```rust
// Avant : enum maison + deux match dans update.
enum Status { Idle, Loading, Loaded(String), Failed(String) }
Msg::Got(Ok(body)) => self.status = Status::Loaded(body.trim().to_string()),
Msg::Got(Err(err)) => self.status = Status::Failed(err),

// Après : un seul type de framework, un seul pont.
joke: RemoteData<String>,
Msg::Got(res) => self.joke = RemoteData::from_result(res.map(|b| b.trim().to_string())),
```

La `view` replie `self.joke.as_ref()` sur les quatre variantes — plus de type ad hoc à
maintenir par écran.

## Vérification

- **6 tests** sur `RemoteData` : `Default` = `NotAsked`, `from_result` (Ok/Err),
  prédicats + accesseurs, `map` (ne touche que `Success`), `map_err` (ne touche que
  `Failure`), `as_ref` (emprunte sans déplacer).
- **`frus-fetch-example`** refactoré : ses 2 tests passent, builds bureau **et** wasm OK.

## Reste

- Un helper `view` qui replie un `RemoteData` en widgets (squelette de chargement, encart
  d'erreur standard) — au besoin. Le type, lui, est là et suffit à structurer l'état.

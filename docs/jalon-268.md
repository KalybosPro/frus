# Jalon 268 — Crate-façade `frus` : **une seule dépendance**

## Objectif

Prolonger le point d'entrée unique (jalon 267) jusqu'à l'ergonomie Flutter complète : une app ne
dépend plus que d'**une** crate, `frus`, et écrit `frus::main!(App::default())`. Avant, il fallait
lister `frus-shell` **et** `frus-widgets` (et connaître leur découpage).

## La façade

- **`crates/frus`** (nouvelle crate) — ne contient **que des ré-exports** :
  - `pub use frus_widgets::*;` — widgets, thème, layout, et le DSL ;
  - `pub use frus_shell::{Application, Command, Lifecycle, Subscription};` — la couche framework ;
  - `pub use frus_shell::{anyhow, log};` — pour le binaire mince et la macro (l'app ne déclare pas
    ces crates) ;
  - `pub use frus_shell::main;` et `pub use frus_widgets::{column, row};` — les **macros**,
    nommées explicitement car un glob ne ré-exporte pas les `#[macro_export]`. D'où `frus::main!`,
    `frus::column!`, `frus::row!`.

## Le point subtil : `$crate` traverse la façade

`main!` est défini dans `frus-shell` et son corps utilise `$crate::run`, `$crate::AndroidApp`, etc.
`$crate` désigne **toujours la crate de définition** (`frus-shell`), même invoqué comme `frus::main!`
depuis une app qui ne dépend **que** de `frus`. Rust résout `$crate` de façon **transitive** (via le
graphe de dépendances) : `frus-shell` y est présent au travers de `frus`, donc l'expansion
(`frus_shell::run`, …) compile sans que l'app nomme `frus-shell`. Vérifié : `frus-hello` bâti via la
façade compile sur bureau **et** produit un `android_main` fonctionnel.

## Adoption

- **`frus-hello`** (exemple canonique) et le **modèle `templates/app`** : une seule dépendance
  `frus`, imports `use frus::{…}`, entrée `frus::main!(…)`, binaire
  `fn main() -> frus::anyhow::Result<()> { <crate>::run() }`. (La dépendance `wasm-bindgen` ciblée
  `wasm32` reste — l'entrée Web y renvoie.)
- **Internes non migrés** : `frus-demo` et `frus-transforms` gardent les crates directes +
  `frus_shell::main!` (ils touchent des détails de `frus-widgets` ; la façade vise **l'app du
  développeur**, dont le modèle est la vitrine).

## Vérification

- **Desktop** : `frus` (façade), `frus-hello` compilent ; `frus-hello` 2 tests verts ; le reste du
  workspace inchangé.
- **Android** : APK `frus-hello` bâti via la façade (l'`android_main` engendré par `frus::main!`
  s'assemble dans le `.so`).

## Reste

- Publication crates.io : à terme, `frus = "0.1"` (le modèle ré-écrit déjà `path` → version en
  commentaire).

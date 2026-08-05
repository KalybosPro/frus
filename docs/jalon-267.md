# Jalon 267 — Point d'entrée **unique** (façon Flutter)

## Objectif

En Flutter, le développeur écrit **un seul** point d'entrée : `void main() => runApp(App())`, et le
toolchain câble Android/iOS/Web en dessous. frus, lui, forçait chaque app à écrire **trois** fonctions
d'entrée conditionnelles (`run_desktop`, `android_main`, `start` wasm) plus un binaire mince — ~15
lignes de plomberie plateforme dupliquées dans chaque projet. Ce jalon les remplace par **une seule
déclaration**.

## La macro `frus_shell::main!`

Invoquée **une fois** dans la bibliothèque de l'app, elle engendre les points d'entrée de **chaque
plateforme**, tous délégant à la **même** application :

```rust
frus_shell::main!(App::default());
```

engendre (conditionnellement à la cible) :

- **bureau** — `pub fn run() -> anyhow::Result<()>` (que le mince binaire de l'app appelle) ;
- **Android** — `#[no_mangle] fn android_main(AndroidApp)` (le symbole natif attendu par l'activité) ;
- **Web** — `#[wasm_bindgen(start)] pub fn start()`.

L'argument est une **expression** qui construit l'application (réévaluée par plateforme, jamais
partagée). L'app n'écrit **plus rien** de spécifique à la plateforme.

## Détails

- **`frus-shell/src/lib.rs`** : la macro `main!` (`#[macro_export]`) + ré-exports `#[doc(hidden)] pub
  use anyhow; pub use log;` pour qu'elle soit **auto-suffisante** (l'app n'a pas à déclarer ces
  crates). L'entrée Web renvoie à `::wasm_bindgen` (la dépendance `wasm32` de l'app, comme une app
  Flutter garde `flutter` dans son `pubspec`).
- **Binaire mince** (bureau) : `fn main() -> frus_shell::anyhow::Result<()> { <crate>::run() }` —
  ossature que le **modèle** fournit, jamais éditée (l'équivalent des *runners* générés de Flutter).
  Un binaire reste requis car, côté cargo, `cargo run` cible un `[[bin]]` tandis que `cargo apk`
  compile le `cdylib` (`--lib`) : les deux cibles sont distinctes, mais le **code écrit par le dev**,
  lui, est unique.
- **Migrés** vers la macro : `frus-hello` (exemple canonique), le **modèle** `templates/app`,
  `frus-demo` et `frus-transforms`. Les `Cargo.toml` de `frus-hello` et du modèle perdent `anyhow` /
  `log` (ré-exportés par la macro) ; le modèle gagne la dépendance `wasm-bindgen` (ciblée `wasm32`)
  dont l'entrée Web a besoin.

## Vérification

- **Desktop** : `frus-hello`, `frus-demo`, `frus-transforms` compilent (binaires appelant `run()`) ;
  tests `frus-hello` 2, `frus-widgets` 396, `frus-shell` 27, `frus-demo` 36 — tous verts.
- **Android** : APK `frus-demo` construit **et lancé sur appareil** — l'`android_main` engendré par la
  macro démarre l'app complète (logcat : la souscription `chrono` égrène chaque seconde ; l'écran
  « My Tasks » s'affiche). Le symbole natif est donc correct de bout en bout.
- **Web** : entrée `#[wasm_bindgen(start)]` engendrée (non construite ici ; structurellement identique
  à l'ancienne, dépendance `wasm-bindgen` présente).

## Reste

- Un jour, une **crate-façade `frus`** ré-exportant `frus-shell` + `frus-widgets`, pour que l'app
  dépende d'un seul `frus` et écrive `frus::main!(...)` — encore plus proche de l'ergonomie Flutter.

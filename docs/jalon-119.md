# Jalon 119 — Vitrine animée : `frus-transforms`

## Analyse

Première démo **tangible** de la couche de transformation : un petit crate runnable
(`frus-transforms`, sur le modèle de `frus-hello`) qui **anime** l'arsenal
récemment construit — [`Transform`] composé (rotation + échelle), [`AspectRatio`] et
[`FractionallySizedBox`] — piloté par un [`Tween`] au fil du temps. C'est la première
occasion de **voir** la rotation et l'échelle rendues par le GPU, au-delà des tests
headless.

## Décisions techniques

- **Modèle Elm minimal.** État = le temps écoulé (`f32`). `update` avance l'horloge
  d'un **pas fixe** (`1/60 s`) → pur et testable. Une souscription
  `every(16 ms) → Frame` bat la mesure (~60 fps) tant que la fenêtre est ouverte ;
  chaque image, l'état change et la `view` est reconstruite.

- **Animation pilotée par `Tween`, dans une `view` pure.** À partir de l'instant, on
  dérive une phase d'aller-retour adoucie (`Curve::ease_in_out`) et on interpole
  chaque valeur par un `Tween` : échelle `1.0 → 1.4`, largeur fractionnaire
  `0.25 → 1.0`, plus une rotation continue. Aucune valeur animée n'est retenue hors de
  l'état temps.

- **Galerie de la palette `Transform`.** Deux rangées de tuiles couvrent tout
  l'éventail : `translate` (va-et-vient), `scale_xy` (écrasement/étirement, échelle
  **non uniforme**), `rotate + scale` (**composition**), `rotate @ corner` (pivot
  décalé, `rotate_from(TOP_LEFT)`) et `translate + rotate`. Puis une boîte
  `AspectRatio 16:9` et une barre `FractionallySizedBox` qui respire.

- **Interactif.** Un **bouton cliquable placé dans un `Transform` tourné** incrémente
  un compteur — preuve *visible* que le hit-test traverse la transformation (matrice
  inverse) ; un **curseur** pilote une échelle en direct ; un bouton **lecture/pause**
  fige l'animation (et coupe la souscription). L'ensemble défile (`Scroll`) pour rester
  utilisable sur une petite fenêtre.

- **Ré-export.** `Alignment` (et `AlignmentGeometry`/`AlignmentDirectional`/`Affine`)
  sont désormais ré-exportés par `frus-widgets` — les applications en ont besoin pour
  `Transform::rotate_from` / `Container::alignment`.

- **Conventions du projet.** Constructeurs de structs uniquement (`Text::new`,
  `Container::new`, `Flex::column`, `Transform::rotate`…), **aucun** helper libre ;
  textes d'interface en **anglais**.

## Implémentation

- `crates/frus-transforms/` : `Cargo.toml` (rlib + cdylib, métadonnées Android),
  `src/lib.rs` (l'app `Showcase` + points d'entrée bureau/Android), `src/bin/`.
  Inclus automatiquement au workspace (`members = ["crates/*"]`).

## Tests

- `frames_advance_the_clock` : `update` avance le temps d'un pas fixe (pur).
- `ticks_continuously` : la souscription n'est jamais vide (animation permanente).
- `renders_a_transformed_layer` : rendu **headless** d'une image — la `view` émet bien
  un `Primitive::Layer` **transformé** (le `Transform` composé), preuve que la vitrine
  câble la pile de bout en bout sans GPU.
- Suite verte ; workspace complet vert.

## Lancer

- Bureau : `cargo run -p frus-transforms`.
- Android : APK via `cargo-apk` (mêmes métadonnées que `frus-hello`).

## Reste

- Vérifier le rendu **sur device réel** (desktop + Android) — l'objectif : *voir* la
  rotation/échelle GPU.
- Cible Web (wasm + WebGPU).

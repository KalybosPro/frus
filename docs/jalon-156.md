# Jalon 156 — Curseur de plage (deux poignées)

## Analyse

Le `Slider` (curseur simple `0..=1`) ne couvrait pas un besoin courant : choisir un
**intervalle** (prix min/max, plage de dates…). Il fallait un curseur à **deux poignées**.

Le nœud technique : deux poignées sur **un seul** widget. Le mécanisme de glissement
existant enregistre **un** rectangle glissable par widget et livre `on_drag(fraction)` —
la fraction absolue sur toute la largeur du curseur. Comment savoir **quelle** poignée
bouge ?

## Décisions techniques

- **Un widget, poignée la plus proche.** Plutôt qu'un composite à deux enfants glissables
  (positionnement absolu + delta, lourd), `RangeSlider` reste un **widget feuille** qui
  réutilise `on_drag(fraction)` tel quel : la fraction déplace la poignée la plus proche.
  Règle déterministe évitant tout croisement :
  - `f ≤ low` → la poignée **basse** suit ;
  - `f ≥ high` → la poignée **haute** suit ;
  - entre les deux → la plus proche bouge.
  Aux extrêmes, le geste **passe la main** à l'autre poignée (on continue de glisser, le
  bord de la plage s'étend) ; les poignées ne se croisent jamais.

- **Contrôlé.** `on_change(low, high)` : l'application reçoit le nouvel intervalle et le
  repasse. `new(low, high)` **borne** à `0..=1` et **réordonne** (`low ≤ high`).

- **Rendu réutilisant le `Slider`.** Mêmes constantes (`H`, `TRACK_H`, `THUMB`) : piste,
  **segment actif** `primary` entre les deux poignées, deux poignées circulaires.
  `Semantics` `Slider` avec la valeur « low%–high% ».

## Implémentation

- `slider.rs` : `RangeSlider<Msg>` (`low`, `high`, `width`, `on_change`) ; `new` /
  `width` / `on_change` ; `Widget` (piste + segment + 2 poignées ; `draggable` ;
  `on_drag` → poignée la plus proche → `on_change(low, high)`).
- `lib.rs` : `pub use slider::{RangeSlider, Slider}`.
- `goldens.rs` : golden `range_slider` (intervalle `0.3..0.7`).

## Vérification

- **Unitaire** : glissement près du bas / du haut bouge la bonne poignée
  (`(0.25, 0.8)`, `(0.2, 0.75)`) ; au-delà des bornes, la poignée de ce côté suit et est
  **bornée** (`(0.2, 1.0)`, `(0.0, 0.8)`) ; `new(0.9, 0.1)` **réordonne** en `(0.1, 0.9)`.
- **Golden** `range_slider` **inspecté** : piste grise, **segment vert** entre deux
  poignées blanches cerclées `primary`.
- `cargo test --workspace` **vert**.

## Reste

- **Poignée « collante »** : mémoriser la poignée saisie à l'appui (état de glissement)
  pour qu'elle reste sélectionnée même après croisement, façon Material — demande un
  `on_drag` conscient de la poignée d'origine (ou deux poignées glissables distinctes).
- **Graduations / pas discret** (`divisions`) et **infobulles** de valeur au survol.

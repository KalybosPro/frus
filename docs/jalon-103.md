# Jalon 103 — `Animatable` : le pont explicite → valeur typée vivante

## Analyse

Le socle d'interpolation typée existait déjà — `Lerp` (nombre, couleur, point,
taille) et `Tween<T> { begin, end }.eval(t)` — mais il restait **inerte** : rien
ne reliait la valeur `[0,1]` qu'un [`AnimationController`] produit frame par frame
à un tween typé. La vue devait lire `controller.value()` puis lerper à la main.

Ce jalon pose le pont manquant, sur la forme éprouvée de Flutter
(`Animatable` / `CurveTween` / `Animation<T>`) : **une** progression `[0,1]`
pilote arbitrairement de valeurs typées, chacune avec ses bornes et sa courbe.

## Décisions techniques

- **`Animatable` (trait).** `type Output; fn evaluate(&self, t: f32) -> Output`.
  C'est l'abstraction que partagent tweens et courbes. `Tween<T: Lerp>`
  l'implémente (`evaluate = eval`).

- **`.curved(curve)` → `Curved<A>`** (façon `CurveTween`). Façonne `t` par la
  courbe **avant** l'évaluation : une progression linéaire pilote une valeur au
  timing non linéaire. Chaînable sur n'importe quel `Animatable`.

- **`.animate(&controller)` → `Animation<'a, A>`**. Lie l'animatable à un
  contrôleur. `value()` échantillonne le contrôleur **à l'instant présent** — c'est
  ce que la vue lit au paint, sans connaître le contrôleur autrement. La valeur du
  contrôleur est **normalisée par ses bornes** (`(v - lower) / (upper - lower)`),
  si bien qu'un contrôleur non unitaire pilote quand même un `[0,1]` complet.
  `Animation` expose aussi `status()` / `is_animating()` (délégués).

- **Emprunts, zéro allocation.** `Animation` emprunte `&animatable` et
  `&controller` : construction gratuite, jetable, recréée à chaque `view()`. Aucune
  dépendance rendu/plateforme — tout reste dans `frus-core`.

## Implémentation

- `frus-core/animation/tween.rs` : trait `Animatable` (+ défauts `curved`,
  `animate`), `impl Animatable for Tween<T>`, structs `Curved<A>` et
  `Animation<'a, A>`. Import de `Curve` / `AnimationController` / `Status`.
- Ré-exports : `animation/mod.rs` et `lib.rs` exposent `Animatable`, `Animation`,
  `Curved`.

## Tests

- `animate_follows_controller` : au repos bas → `begin` (`Dismissed`) ; après
  `forward` réglé → `end` (`Completed`), sur un `Tween<Size>`.
- `curved_reshapes_progression` : à mi-course d'un `ease_in`, la valeur est **en
  deçà** du milieu linéaire ; bornes atteintes (tolérance solveur bézier).
- `non_unit_bounds_are_normalized` : contrôleur `[0,2]` à `1.0` → `t = 0.5` → gris
  médian d'un `Tween<Color>`.
- Suite `frus-core` verte (81).

## Reste

- Idiome shell : instancier un `AnimationController` par identité et lire
  `tween.animate(&ctrl).value()` dans `view()` (démo dédiée).
- `Animatable` composés (séquence `TweenSequence`, `Tween` d'`Insets` /
  `BorderRadius`).

# Jalon 104 — `Animatable` composés : `TweenSequence` + tweens de boîte

## Analyse

Le J103 a posé le pont `Animatable` (tween → valeur typée vivante). Restaient deux
manques pour la parité Flutter :

1. Les **propriétés de boîte** — marge (`Insets`) et rayon (`BorderRadius`) —
   n'étaient pas interpolables : pas d'`impl Lerp`, donc pas de `Tween<Insets>` ni
   `Tween<BorderRadius>`.
2. Aucun moyen d'enchaîner **plusieurs étapes** sur une même progression (morph
   A → B → C, rebond grossir-puis-revenir, segments à rythmes distincts).

## Décisions techniques

- **`Lerp` pour `Insets` et `BorderRadius`** — côté par côté (chaque marge / coin
  interpolé indépendamment), comme `Size`/`Point`. `Tween<Insets>` et
  `Tween<BorderRadius>` en découlent *gratuitement* (le `Tween<T: Lerp>` générique
  les couvre).

- **`TweenSequence<T>`** — suite de segments **pondérés** (façon `TweenSequence` de
  Flutter). Chaque segment occupe une part de `[0,1]` proportionnelle à son poids ;
  `evaluate(t)` situe le segment actif et l'évalue sur sa **progression locale**
  `[0,1]`. Le dernier segment attrape le reste (robuste aux arrondis) ; poids nuls
  → dernier segment.

- **Segments arbitraires via `Box<dyn Animatable<Output = T>>`.** Un segment est
  n'importe quel `Animatable` : un `Tween`, un `Tween.curved(...)`, voire une autre
  `TweenSequence`. `Animatable` est *object-safe* (les défauts `curved`/`animate`
  sont `where Self: Sized`, hors vtable).

- **`TweenSequence` est lui-même un `Animatable`** : il se `.curved()` et
  s'`.animate(&controller)` comme n'importe quel tween. Composition uniforme.

- **Non-vide par construction.** `new(first, weight)` exige un premier segment ;
  `.then(next, weight)` en ajoute. `evaluate` n'a donc jamais de cas « vide ».

## Implémentation

- `frus-core/animation/tween.rs` : `impl Lerp for Insets`, `impl Lerp for
  BorderRadius` ; struct `TweenSequence<T>` (`new`, `then`, `impl Animatable`).
  Import de `BorderRadius` / `Insets`.
- Ré-exports : `animation/mod.rs` et `lib.rs` exposent `TweenSequence`.

## Tests

- `insets_and_radius_tween_interpolate` : `Tween<Insets>` / `Tween<BorderRadius>` à
  mi-course.
- `tween_sequence_relays_equal_weight_segments` : deux segments égaux se relaient à
  `t = 0.5`, chacun parcouru en entier sur sa moitié (0/5/10/20/30).
- `tween_sequence_honors_weights` : poids 3:1 → couture à `t = 0.75`.
- `tween_sequence_drives_from_controller` : la suite pilotée par un contrôleur
  (couleur noir→blanc→noir).
- Suite `frus-core` verte (85).

## Reste

- Idiome shell / démo lisant `sequence.animate(&ctrl).value()` dans `view()`.
- `Tween<Alignment>` une fois l'alignement introduit ; `decoration` composite.

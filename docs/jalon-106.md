# Jalon 106 — `Alignment` fractionnel + `Tween<Alignment>` (placement manuel)

## Analyse

Le J105 a introduit `Container::alignment` sur neuf ancrages **discrets**, traduits
en `justify`/`align` flex. Discret = non interpolable : un `Tween<Alignment>`
sauterait de position en position. Pour un ancrage **animable** (glisser un enfant
d'un coin à l'autre en douceur, façon `AlignTransition` de Flutter), il faut un
`Alignment` **continu** et un placement **manuel** (le flex ne fait pas de
fractions).

## Décisions techniques

- **`Alignment` continu (frus-core).** Struct `{ x, y }`, fractions `[-1, 1]`
  (`-1` = bord bas/gauche, `+1` = haut/droite selon l'axe), avec les neuf ancrages
  usuels en constantes (`Alignment::CENTER`, `TOP_LEFT`…). Étant continu, il
  implémente **`Lerp`** → `Tween<Alignment>` glisse gratuitement d'un ancrage à
  l'autre. `fraction_x/​y()` ramènent chaque axe dans `[0, 1]` (part d'espace libre
  à laisser avant l'enfant).

- **Placement manuel dans la marche.** `Container::style()` laisse taffy poser
  l'unique enfant en **haut-gauche** de la boîte de contenu (`Start`/`Start`, taille
  naturelle, pas d'étirement). La marche (`Builder::align_offset`) calcule ensuite
  l'espace libre `boîte_contenu − enfant` et **décale** l'enfant de
  `libre × fraction` via sa translation — qui cascade sur tout son sous-arbre
  (hit-test compris). Ne s'applique qu'à un **enfant unique** (l'ancrage vise un
  seul enfant, façon Flutter).

- **RTL.** taffy calcule en LTR puis `mirror` renvoie l'enfant à droite : la base
  est alors l'ancrage droit. On retranche `1` à la fraction x pour que l'ancrage
  reste **physique** (`x = +1` ⇒ droite dans les deux sens de lecture ; `Alignment`
  n'est pas directionnel, contrairement à `AlignmentDirectional`).

- **Trait `Widget::alignment()`** (défaut `None`), redescendu par `Box`, `Keyed`,
  `Responsive` et les widgets nommés. La cohérence du cache de relayout est
  gratuite : `style()` (qui porte `Start`/`Start`) est la source partagée par
  `build_layout` et l'empreinte (`layout_hash`).

## Implémentation

- `frus-core` : `Alignment` struct + constantes + `fraction_x/y` (geometry.rs) ;
  `impl Lerp for Alignment` (tween.rs) ; ré-export (suppression d'`AlignEdge`,
  éphémère du J105).
- `frus-widgets` : trait `alignment()` + forwarders ; `Container` (`style()` pose
  Start/Start si ancré, `alignment()`), `Builder::align_offset` + application dans
  la branche enfants du walk.

## Tests

- `alignment_tween_slides_between_anchors` (core) : `TOP_LEFT → BOTTOM_RIGHT`, milieu
  = `CENTER`.
- `fractional_alignment_places_child_proportionally` : ancrage `(0.5, -0.5)` →
  enfant à ~(60, 20) dans 100×100 (ce que le discret ne pouvait pas).
- `alignment_centers_the_child` / `alignment_anchors_child_to_a_corner` (J105,
  mis à jour aux constantes) restent verts sous le placement manuel.
- Suites vertes : frus-core 86, frus-widgets 197 ; workspace complet vert.

## Reste

- Ancrage dans un **élément de liste virtualisée** (`render_item` n'applique pas
  encore le décalage — chemin secondaire).
- `AlignmentDirectional` (start/end, résolu en RTL) si besoin.
- Idiome shell / démo : `align_tween.animate(&ctrl).value()` passé à `.alignment()`.

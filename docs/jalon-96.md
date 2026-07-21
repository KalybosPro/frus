# Jalon 96 — Opacité de groupe & `AnimatedOpacity`

## Analyse

L'arc J92→95 avait posé toutes les pièces : calques GPU composités
([`Primitive::Layer`], J92), cache de calques (J94) et animations implicites
courbées (J95). Manquait le **widget** qui les réunit : `Opacity` /
`AnimatedOpacity` de Flutter — appliquer une opacité à **tout un sous-arbre d'un
bloc** (et l'animer). C'était le dernier « reste » explicite de J92 (« intégrer un
widget Opacity dans le walk, comme `RepaintBoundary` »).

Fondre un sous-arbre **primitive par primitive** (`push_faded`) recréerait le
double-blend que J92 corrige sur les chevauchements. La bonne réponse est le
`saveLayer` de Flutter : rendre le groupe sur un calque, composer le calque entier
à l'opacité voulue.

## Décisions techniques

- **Plié dans `Container`** (idiome du framework, comme `repaint_boundary`) plutôt
  qu'un wrapper transparent : un wrapper « adopte » le nœud de son enfant et son
  scalaire d'animation entrerait en collision avec un enfant animé. `Container`
  gagne `.opacity(o)` (fixe) et `.animated_opacity(o, duration, curve)` (animée) —
  layout prouvé, un nœud propre, un scalaire propre. Aucun nouveau nœud de
  disposition surprise.

- **Nouveau point du trait** `Widget::opacity_group() -> Option<f32>` (défaut
  `None`) : renvoie l'opacité **cible** du groupe. Combiné à `anim_target`
  (opacité animée, J95), le fondu se déroule tout seul via `advance_values`.
  Forwardé par les wrappers (`Box`, `Keyed`, `Responsive`).

- **Drainage dans la marche de peinture** ([`crate::ui`]). À la rencontre d'un
  groupe : on peint le sous-arbre normalement dans la scène, puis on **draine** sa
  plage de primitives ([`Scene::split_off`]) dans un unique `Primitive::Layer` à
  l'opacité de groupe. L'opacité effective est la valeur **tweenée** par le runtime
  (fixe → la cible). **Totalement opaque (≈1) : aucun calque** (coût nul). Le
  hit-testing n'est pas affecté (le calque ne touche que le visuel).

## Limites assumées

- **Groupes imbriqués** : un `Layer` dans les primitives d'un autre n'est pas
  recompositionné (limite héritée de J92).
- Un **overlay** émis *dans* le groupe (différé hors de la scène) n'est pas fondu.
- Opacité bornée `[0,1]` ; à ≈0 le sous-arbre est tout de même peint (puis rendu
  invisible par le calque) — simple et correct.

## Implémentation

- `frus-core` : `Scene::split_off(start)` (déplace une plage de primitives).
- `frus-widgets` : trait `opacity_group()` + forwarders ; `Container` (champs
  `opacity`/`opacity_anim`, builders `.opacity`/`.animated_opacity`,
  `opacity_group()` + `anim_target`/`anim_duration`/`anim_curve` quand animé) ; la
  marche `ui::walk` enveloppe le sous-arbre dans un calque.

## Tests

- `frus-widgets` : `opacity_group_wraps_subtree_in_a_layer` (la scène contient un
  `Layer` à 0.5 enveloppant le contenu) ; `full_opacity_emits_no_layer` (opacité
  pleine → aucun calque) ; `animated_opacity_declares_anim_target` (cible/durée/
  courbe exposées ; `opacity` fixe → pas de valeur animée).
- `frus-test` : `group_opacity_fades_the_box` — **preuve pixel de bout en bout**
  (widget → walk → calque → GPU) : `opacity(0.5)` atténue nettement le rouge par
  rapport à `opacity(1.0)`.
- Goldens et suites existantes inchangés : le chemin ne se déclenche que si
  `opacity_group()` est `Some`.

## Reste

- Widget `Opacity`/`AnimatedOpacity` **nommé** (sucre au-dessus de `Container`),
  et des `Animated*` interpolant d'autres propriétés (couleur/taille/padding) via
  [`Tween`] — l'étape suivante des animations implicites.
- Recompositing des groupes imbriqués ; fondu des overlays.

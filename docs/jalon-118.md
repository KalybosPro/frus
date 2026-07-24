# Jalon 118 — `Transform` : focus/a11y suivent l'échelle (cas aligné)

## Analyse

Le jalon précédent (J117, matrice affine unifiée) laissait un compromis : sous une
transformation, les rectangles de **focus, défilement, glisser et accessibilité**
restaient **non transformés** (une matrice générale ne peut pas garder un rectangle
aligné sur les axes). On lève ce compromis **dans le cas courant** — quand la matrice
conserve l'alignement sur les axes (échelle et/ou translation, **sans rotation**),
l'image d'un rectangle *est* un rectangle : on la calcule exactement.

## Décisions techniques

- **`Affine::is_axis_aligned`** : la partie linéaire est diagonale (`b ≈ 0`,
  `c ≈ 0`) → pas de rotation ni de cisaillement.

- **`Affine::apply_rect`** : image d'un rectangle par la matrice — exacte quand la
  matrice est alignée sur les axes (sinon, boîte englobante).

- **Application conditionnelle dans le walk.** Après avoir enveloppé le sous-arbre
  transformé dans son calque, si la matrice `is_axis_aligned()`, on applique
  `apply_rect` aux surfaces **focus / défilement / glisser / accessibilité** émises. En
  présence d'une rotation, on les laisse (bornes approchées) — le **clic** reste juste
  dans tous les cas (via `M⁻¹` sur le point).

## Implémentation

- `frus-core/geometry.rs` : `Affine::is_axis_aligned`, `Affine::apply_rect`.
- `frus-widgets/ui.rs` : dans le bloc de transformation, re-capture des plages
  focus/scroll/drag/sémantique et transformation par `apply_rect` si la matrice est
  alignée sur les axes.

## Tests

- `axis_aligned_transform_scales_the_focus_rect` : un `Button` sous `scale(2.0)` — un
  point hors du bouton à plat mais dans son image agrandie devient focalisable, et son
  rectangle de focus est ~2× plus large.
- Suites vertes : frus-core 90, frus-widgets 212 ; workspace complet vert.

## Reste

- Sous **rotation**, focus/a11y restent aux bornes non tournées (limite géométrique
  d'un rectangle aligné sur les axes) — le clic, lui, reste exact.
- Une démo animée rassemblant l'arsenal (`Tween` pilotant un `Transform` composé).

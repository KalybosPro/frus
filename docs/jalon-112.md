# Jalon 112 — `Transform` : décalage de peinture (`translate`)

## Analyse

Dernier de la série des widgets de disposition manquants : **`Transform`**. Il
décale son enfant **à la peinture**, sans toucher la mise en page — l'enfant peut
déborder sa boîte, les frères ne bougent pas. C'est la brique des effets qui
*glissent* (pastille dans un coin, entrée qui coulisse, secousse d'erreur) et,
combinée à un `Tween` lu dans `view()`, d'un mouvement animé.

Ce jalon ne couvre que la **translation** (`Transform.translate` de Flutter).
L'échelle et la rotation demandent une **matrice affine** dans le pipeline — le
rendu GPU ne connaît aujourd'hui que des quads alignés sur les axes (un rect mis à
l'échelle reste un rect, mais un rect tourné ne l'est plus) — et sont donc
reportées à un jalon dédié.

## Décisions techniques

- **Réutilise la cascade de translation du walk.** Le décalage est ajouté à la
  translation propagée aux enfants (comme l'ancrage `Container::alignment`). Or
  primitives **et** toutes les surfaces d'interaction (clic, appui long, focus,
  scroll, glisser, accessibilité) dérivent de cette même translation : le décalage
  est donc **automatiquement correct partout**, hit-test compris — zéro post-
  traitement, zéro risque d'incohérence peinture/clic.

- **`align_offset` → `child_offset`.** L'ancienne fonction (offset de l'ancrage
  fractionnel) devient `child_offset` : elle **cumule** l'ancrage et le décalage
  `Transform::translate`. Appelée aux deux endroits du walk (arbre principal +
  éléments de listes virtualisées / `layout_builder`).

- **Correction RTL.** L'axe x du monde étant retourné en RTL, un `dx` logique
  positif (« vers la fin ») pointerait vers la gauche : on inverse son signe pour
  rester cohérent avec le sens de lecture.

- **Trait `transform_translate()`**, transmis par les wrappers transparents
  (`Box`, `Keyed`, `Responsive`, wrappers `animated`) selon le motif habituel.

## Implémentation

- `frus-widgets/transform.rs` : le widget `Transform` (`translate(dx, dy)`,
  `child`, `style()` passe-plat, `transform_translate() = Some((dx, dy))`).
- `frus-widgets/widget.rs` : méthode de trait `transform_translate` + forward `Box`.
- `keyed.rs` / `responsive.rs` / `animated.rs` : forwards.
- `ui.rs` : `align_offset` → `child_offset` (cumule ancrage + translate), aux deux
  sites d'appel.
- Export `Transform` dans `lib.rs`.

## Tests

- `translate_offsets_the_child_at_paint` : `translate(30, 10)` peint l'enfant
  (20×20) à ~(30, 10).
- `translate_does_not_affect_layout` : un frère placé après un enfant décalé de 50
  reste à sa position de mise en page (`y = 20`) — le décalage est purement visuel.
- Suite frus-widgets verte (205) ; workspace complet vert.

## Reste

- `Transform` **échelle** (`scale`) : post-traitement affine sur la plage de
  primitives du sous-arbre (via `split_off`, comme le calque d'opacité) **plus**
  transformation des rectangles d'interaction — reste axé-aligné, sans toucher le
  GPU.
- `Transform` **rotation** : matrice affine passée aux shaders (sommet + SDF), plus
  hit-test à transformation inverse — un jalon d'infrastructure de rendu.

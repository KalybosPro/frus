# Jalon 115 — `Transform` : échelle non uniforme (`scale_xy`)

## Analyse

Première moitié de la complétion de `Transform` : l'**échelle non uniforme**
(`scale_xy(sx, sy)`) — étirer ou aplatir un sous-arbre avec des facteurs différents
en X et en Y (barre qui s'allonge, vignette qui s'écrase). Jusqu'ici l'échelle était
forcément uniforme (un seul `factor`).

## Décisions techniques

- **Généralisation par axe de la mise à l'échelle des primitives.** `Primitive::
  scaled(factor)` (utilisé aussi pour le DPI) devient un cas particulier de
  `Primitive::scaled_xy(sx, sy)` : rectangles et images s'étirent **exactement** par
  axe ; les grandeurs **scalaires** sans axe (rayon d'arrondi, bordure, flou, chemin)
  suivent la moyenne des deux facteurs, la taille de police suit `sy` et la largeur de
  repli suit `sx`. Quand `sx == sy` (échelle uniforme, DPI), tout redevient exact — le
  comportement existant est préservé.

- **Réutilise le chemin de post-traitement de J113** (pas de calque, pas de GPU) : la
  plage de primitives émise et les rectangles d'interaction sont mis à l'échelle par
  axe autour du pivot (`scaled_about_xy`, `scale_about_xy`). Rendu et hit-test restent
  donc cohérents et **testables sans GPU**.

- **Helpers ajoutés** : `Point::scale_xy`, `Rect::scale_xy` / `scale_about_xy`,
  `Primitive::scaled_xy` / `scaled_about_xy`, `LayerTransform::scaled_xy`.

- **API.** `Transform::scale_xy(sx, sy)` (autour du centre) et `scale_xy_from(sx, sy,
  pivot)`. `scale(factor)` / `scale_from` deviennent des raccourcis uniformes.

## Implémentation

- `frus-core/geometry.rs` : `Point::scale_xy`, `Rect::scale_xy` / `scale_about_xy`.
- `frus-core/scene.rs` : `Primitive::scaled` délègue à `scaled_xy` ; `scaled_about`
  délègue à `scaled_about_xy` ; `LayerTransform::scaled_xy`.
- `frus-widgets/widget.rs` : `transform_scale() -> Option<(f32, f32, Alignment)>`
  (facteurs par axe) + forwards.
- `frus-widgets/transform.rs` : `scale_xy` / `scale_xy_from`.
- `frus-widgets/ui.rs` : le bloc d'échelle applique `scaled_about_xy` /
  `scale_about_xy`.

## Tests

- `scale_xy_stretches_per_axis` : `scale_xy(3.0, 1.0)` → fond ~60×20 (étiré en X
  seulement), toujours centré.
- Les tests d'échelle uniforme (J113) restent verts (cas `sx == sy`).
- Suites vertes : frus-core 88, frus-widgets 210 ; workspace complet vert.

## Reste

- **Composition** de plusieurs transformations dans un même `Transform` (translate +
  échelle + rotation appliquées ensemble) — seconde moitié de la complétion.
- Correction non uniforme des **rayons d'arrondi** (deviennent elliptiques) et des
  **chemins** — approximés ici par la moyenne des facteurs.

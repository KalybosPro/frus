# Jalon 114 — `Transform` : rotation (calque composité tourné)

## Analyse

Dernière transformation du widget `Transform` : la **rotation** (`Transform.rotate`
de Flutter). Contrairement à la translation (J112) et à l'échelle (J113), une
rotation **ne préserve pas l'alignement sur les axes** — un rect tourné n'est plus
un rect. Il fallait donc, pour la première fois, une **transformation affine dans
le pipeline de rendu**. C'est un jalon d'infrastructure : il équipe le compositeur
d'une passe de rotation réutilisable.

## Décisions techniques

- **Rotation d'un calque au compositing, pas de chaque primitive.** Le compositeur
  savait déjà rendre un sous-arbre **à plat** dans une texture puis le composer
  (opacité de groupe, façon `saveLayer`). On réutilise ce chemin : le calque est
  composité **tourné**. Une seule passe tourne ainsi tout un sous-arbre (rects,
  texte, images, chemins), **sans toucher les shaders de chaque type** de primitive.

- **Contre-rotation dans le fragment.** La texture contient le contenu à plat, à sa
  position écran. Pour peindre le calque tourné de `+angle` autour du pivot, le
  fragment échantillonne à la position **contre-tournée de `-angle`** : le pixel
  écran `p` reçoit le contenu qui, tourné de `+angle`, atterrit en `p`. Hors texture
  après contre-rotation → transparent.

- **`Primitive::Layer` porte une `Option<LayerTransform>`** (angle + pivot px).
  `None` = calque simplement composité (opacité). Suivie par `scaled` / `translated`
  (le pivot se met à l'échelle / se décale, l'angle est invariant).

- **Hit-test contre-tourné.** Une rotation ne peut pas transformer les rectangles de
  clic (ils cesseraient d'être alignés). On marque plutôt chaque cible de clic du
  sous-arbre d'une contre-transformation `(angle, pivot)` ; au test, le **point** est
  tourné de `-angle` avant `contains`. Exact pour une rotation ; les rotations
  imbriquées gardent la plus extérieure (approximation documentée).

- **API.** `Transform::rotate(radians)` (autour du centre) et
  `Transform::rotate_from(radians, pivot)`. Correction RTL : le monde étant retourné,
  l'angle est inversé. `angle ≈ 0` : rendu normal (coût nul).

## Implémentation

- `frus-core/scene.rs` : `LayerTransform { angle, pivot }` (+ `scaled` / `translated`) ;
  champ `transform` sur `Primitive::Layer` (propagé dans `scaled`/`translated`/`fade`/
  `layer`). Export dans `lib.rs`.
- `frus-gpu` : `LayerComposite`/`CompInstance` portent `(angle, pivot)` ;
  `composite.wgsl` contre-tourne l'échantillon (le fragment lit `viewport.size` →
  visibilité `VERTEX_FRAGMENT` du binding viewport).
- `frus-widgets/widget.rs` : trait `transform_rotate` + forwards
  (`Box`/`Keyed`/`Responsive`/`animated`).
- `frus-widgets/transform.rs` : `rotate` / `rotate_from`.
- `frus-widgets/ui.rs` : bloc de rotation dans `walk` (calque tourné + marquage des
  cibles) ; `Hit` gagne `xform` + `rotate_point` ; `hit` / `long_press_at` testent
  via `Hit::contains`.

## Tests

- `rotate_emits_a_rotated_layer` : `rotate(π/2)` produit un `Primitive::Layer` de
  `transform = Some(angle ≈ π/2, pivot = centre de l'enfant)`.
- `rotate_hit_test_counter_rotates_the_point` : enfant 40×20 tourné de +90° — un clic
  à la position **tournée** (20, 25) atteint la cible, l'ancienne position (35, 10)
  la rate.
- Suites vertes : frus-core 88, frus-gpu 16, frus-widgets 209 ; workspace complet
  vert. (Le rendu tourné lui-même n'est pas vérifié en CI — pas de GPU ; la
  correction du fragment est validée par construction, le hit-test par test unitaire.)

## Reste

`Transform` couvre désormais translation, échelle et rotation. Extensions possibles :
échelle non-uniforme (`scaleX`/`scaleY`), composition de plusieurs transformations en
une matrice unique, et une démo animée rassemblant l'arsenal (alignement + `Tween` +
`Transform`).

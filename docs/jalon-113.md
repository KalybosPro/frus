# Jalon 113 — `Transform` : échelle de peinture (`scale`)

## Analyse

Deuxième transformation du widget `Transform` : la **mise à l'échelle**
(`Transform.scale` de Flutter). Elle agrandit ou rétrécit un sous-arbre **à la
peinture**, sans toucher la mise en page — effet « pop » d'un bouton au survol,
zoom d'une vignette, respiration d'une icône. Comme la translation (J112), elle
reste **alignée sur les axes** (un rect mis à l'échelle reste un rect), donc
**aucune matrice** dans le pipeline GPU n'est nécessaire — contrairement à la
rotation, reportée au jalon suivant.

## Décisions techniques

- **Post-traitement de la plage de primitives**, comme le calque d'opacité. On
  peint le sous-arbre normalement, puis (via `Scene::split_off`) on met à l'échelle
  **autour d'un pivot** chaque primitive émise, réinsérée dans l'ordre.

- **Hit-test cohérent.** Une échelle change la géométrie (contrairement à
  l'opacité) : on transforme donc **aussi** les rectangles d'interaction émis par
  le sous-arbre — clic, appui long, focus, glisser, scroll, accessibilité — avec la
  même transformation. Rendu et hit-test restent alignés.

- **Pivot sur la boîte de l'enfant.** Le pivot est un `Alignment` (défaut : centre)
  résolu sur la boîte de l'**enfant** (le nœud suivant dans l'ordre préfixe), pas
  sur celle du `Transform` : cette dernière peut être étirée par le parent (flex
  `stretch`), ce qui éloignerait le pivot du contenu réellement mis à l'échelle.

- **Primitives de scène : `translated` + `scaled_about`.** Ajout dans `frus-core`
  de `Primitive::translated(dx, dy)` (miroir de `scaled`) et de
  `Primitive::scaled_about(pivot, factor) = scaled(f).translated(pivot·(1−f))`,
  plus `Rect::scale_about` pour les rectangles d'interaction. La mise à l'échelle
  touche position, taille, police, rayons et traits.

- **API.** `Transform::scale(factor)` (autour du centre) et
  `Transform::scale_from(factor, pivot)`. Facteur `≈ 1.0` : rendu normal (coût nul).

## Implémentation

- `frus-core/scene.rs` : `Primitive::translated`, `Primitive::scaled_about`.
- `frus-core/geometry.rs` : `Rect::scale_about`.
- `frus-widgets/widget.rs` : trait `transform_scale() -> Option<(f32, Alignment)>`
  + forwards (`Box`, `Keyed`, `Responsive`, `animated`).
- `frus-widgets/transform.rs` : `scale` / `scale_from` sur le widget `Transform`.
- `frus-widgets/ui.rs` : bloc d'échelle dans `walk` (drain + transformée des
  primitives et des surfaces d'interaction).

## Tests

- `scale_grows_the_child_about_its_center` : `scale(2.0)` → fond ~40×40, même
  centre (10, 10).
- `scale_from_pins_the_pivot_corner` : `scale_from(2.0, TOP_LEFT)` → coin
  haut-gauche fixe à (0, 0), fond doublé.
- Suites vertes : frus-core 88, frus-widgets 207 ; workspace complet vert.

## Reste

- `Transform` **rotation** : matrice affine passée aux shaders (sommet + SDF) et
  hit-test à transformation inverse — jalon d'infrastructure de rendu.
- Échelle **non-uniforme** (`scaleX` / `scaleY`) — extension simple du même
  post-traitement si le besoin apparaît.
- Un sous-arbre **défilable** à l'intérieur d'un `Transform::scale` : la piste de
  barre de défilement n'est pas transformée (combinaison rare — non couverte).

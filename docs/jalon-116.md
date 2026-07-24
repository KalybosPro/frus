# Jalon 116 — `Transform` : composition (translate + échelle + rotation)

## Analyse

Seconde moitié de la complétion de `Transform` : **composer** plusieurs
transformations dans un même widget. Jusqu'ici un `Transform` ne portait qu'une seule
opération (translation *ou* échelle *ou* rotation). On veut « grossir **et** tourner »
(effet pop qui pivote), « décaler **et** mettre à l'échelle », etc.

## Décisions techniques

- **Enchaîneurs sur le widget.** Aux constructeurs mono-opération s'ajoutent
  `and_translate`, `and_scale` / `and_scale_xy`, `and_rotate` : ils **cumulent** une
  opération sans effacer les autres. `Transform::scale(1.5).and_rotate(0.2)` porte les
  deux.

- **Ordre d'application fixe : translation → échelle → rotation.** La translation
  (déjà propagée via `child_offset`) est la plus **intérieure** ; l'échelle
  post-traite les primitives à plat ; la rotation enveloppe le tout dans un calque
  composité tourné (la plus **extérieure**). C'est l'ordre naturel : on positionne, on
  redimensionne, puis on pivote.

- **Fusion des deux passes du walk.** Les blocs séparés « échelle » et « rotation »
  deviennent **un seul** bloc : on peint le sous-arbre à plat une fois, puis on
  applique l'échelle (si présente) *puis* la rotation (si présente) sur la même plage
  de primitives. La rotation enveloppe donc les primitives **déjà mises à l'échelle**.

- **Hit-test composé cohérent.** L'échelle transforme les rectangles de clic ; la
  rotation marque leur contre-rotation. Au test, le point écran est d'abord
  contre-tourné (rotation extérieure) puis testé contre le rect mis à l'échelle —
  l'inverse exact de l'ordre de peinture.

- **Pivots.** Chaque opération garde son propre pivot (`Alignment`), résolu sur la
  boîte de l'enfant. Pour le cas courant (pivots au centre), le centre est invariant
  par l'échelle centrée, donc rotation et échelle partagent le même centre ; les
  combinaisons hors-centre sont approximées (documenté).

## Implémentation

- `frus-widgets/transform.rs` : enchaîneurs `and_translate` / `and_scale` /
  `and_scale_xy` / `and_rotate`.
- `frus-widgets/ui.rs` : les deux blocs (échelle, rotation) fusionnés en un seul, qui
  applique l'échelle puis la rotation à la même plage.

## Tests

- `scale_and_rotate_compose` : `scale(2.0).and_rotate(π/2)` produit un **calque
  tourné** (angle ≈ π/2) *contenant* un rectangle **agrandi** (~40×40) — les deux
  transformations composées, dans le bon ordre.
- Les tests mono-opération (translate / scale / scale_xy / rotate) restent verts.
- Suites vertes : frus-core 88, frus-widgets 211 ; workspace complet vert.

## Reste

`Transform` couvre désormais translation, échelle (uniforme et par axe), rotation, et
leur composition. Extensions possibles : une vraie **matrice affine unique** (fusion
des trois passes en une seule multiplication, y compris pour le hit-test) et une démo
animée rassemblant l'arsenal.

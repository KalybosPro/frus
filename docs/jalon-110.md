# Jalon 110 — `AspectRatio` : boîte à rapport largeur/hauteur

## Analyse

Premier des widgets de disposition manquants (face à Flutter) : **`AspectRatio`**.
Impossible, jusqu'ici, de tenir un rapport largeur/hauteur constant (vignette
vidéo 16:9, carré d'avatar, carte 4:3) sans figer *les deux* dimensions en dur —
donc sans s'adapter à la largeur disponible. Flutter le fait via
`AspectRatio(aspectRatio:)`.

## Décisions techniques

- **`aspect_ratio: Option<f32>` dans `frus_layout::Style`.** taffy 0.7 gère
  nativement le rapport (`width / height`, même convention que Flutter). Ajouté :
  champ, hachage dans `layout_hash` (il change la géométrie → invalide le cache de
  relayout) et transmission dans `to_taffy`.

- **La boîte prend la largeur, dérive la hauteur.** Une largeur seulement
  *étirée* (`align: stretch`) ne suffit **pas** à taffy pour dériver l'autre axe :
  vérifié empiriquement (sonde `probe_aspect_ratio`), une boîte étirée + rapport
  restait à hauteur 0. Il faut une dimension **connue**. `AspectRatio` pose donc
  `width: Percent(1.0)` (largeur pleine du parent) ; taffy en dérive alors la
  hauteur. Cas le plus courant : `AspectRatio` dans une colonne / un contexte
  pleine largeur.

- **Widget de disposition pur.** `AspectRatio::new(ratio).child(...)` ne peint
  rien ; l'enfant hérite de la boîte (étirement en hauteur via `align: stretch`,
  remplissage en largeur s'il grandit — `flex`, une image, un fond plein).

## Implémentation

- `frus-layout/style.rs` : champ `aspect_ratio`, défaut `None`, `layout_hash`,
  `to_taffy`.
- `frus-widgets/aspectratio.rs` : le widget `AspectRatio` (`new` borne le rapport
  à `> 0`, `child`, `style()` = `width: Percent(1.0)` + `aspect_ratio`).
- `frus-widgets/flex.rs` : passe `aspect_ratio: None` (constructeur de `Style`
  énuméré — seul `Flex` l'énumère ; les autres widgets utilisent
  `..Default::default()`).
- Export `AspectRatio` dans `lib.rs`.

## Tests

- `derives_free_dimension_from_ratio` : dans une colonne large de 100, un
  `AspectRatio(2.0)` donne une boîte 100×50 (l'enfant qui remplit peint ~100×50).
- `ratio_below_one_is_taller_than_wide` : `AspectRatio(0.5)` → 100×200 (plus haut
  que large).
- Suites vertes : frus-layout 16, frus-widgets 201 ; workspace complet vert.

## Reste

- `FractionallySizedBox` (taille en fraction du parent), `Transform`
  (rotation/échelle/translation d'un enfant) — autres widgets de disposition de
  Flutter.
- `AspectRatio` dérivé depuis la **hauteur** contrainte (cas d'un `Row`) — non
  couvert : le brique cible le cas pleine largeur, le plus courant.

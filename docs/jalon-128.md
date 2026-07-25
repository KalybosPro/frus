# Jalon 128 — Vitrine : ClipPath + RotatedBox + FittedBox

## Analyse

J125–J127 ont complété la famille (découpe par coin, `RotatedBox`/`FittedBox`,
`ClipPath`) sans les montrer. Ce jalon les rend **tangibles** dans `frus-transforms` —
et *voir* a de nouveau valeur : le rendu confirme d'un coup d'œil la découpe par chemin
(étoile), la rotation qui **change la boîte**, et l'ajustement `Contain`, sans
chevauchement des voisins.

## Décisions techniques

- **`ClipPath` en étoile.** Un chemin d'étoile à 5 branches (`star_path`, coordonnées
  locales) découpe un carré dégradé — bords anticrénelés par le masque GPU, aux côtés de
  `ClipRRect` et `ClipOval` (galerie 3).

- **`RotatedBox` visible par le texte.** « ROTATED » tourné de 3 quarts devient
  **vertical** (sa boîte passe haute et étroite) — la preuve *visible* que la rotation
  affecte la mise en page, contrairement à `Transform`.

- **`FittedBox·Contain`.** Un grand « Fit » (48 px) est mis à l'échelle pour **tenir**
  dans un cadre 120×80 — l'échelle découle de la boîte (galerie 4).

- **`view` toujours pure**, conventions respectées (constructeurs de structs, textes en
  anglais).

## Implémentation

- `crates/frus-transforms/src/lib.rs` : imports `ClipPath` / `RotatedBox` / `FittedBox` /
  `BoxFit` / `Path` / `Point` ; helper `star_path` ; galerie 3 étendue (tuile étoile) ;
  galerie 4 (`RotatedBox` + `FittedBox`) ; en-têtes et titre mis à jour.

## Tests

- `renders_clip_shapes` : la `view` émet aussi un `ClipShape::Path` (l'étoile) en plus
  de `RRect` et `Oval`.
- Garde-fous existants verts (calque transformé émis, contenu posé dans la fenêtre).
- Rendu visuel (hors commit) confirmé : étoile nette, texte vertical, « Fit » ajusté,
  **aucun chevauchement** sous la galerie `RotatedBox`. Suite `frus-transforms` : 7.

## Lancer / voir

- Bureau : `cargo run -p frus-transforms` — faire défiler ; glisser/zoomer la fenêtre
  interactive ; observer étoile, rotation, ajustement.
- Android : APK via `cargo-apk`.

## Reste

- Vérification **sur device réel** (desktop + Android) : le *voir* final.
- Une tuile animant un `ClipPath` (chemin qui pulse) illustrerait la découpe dynamique.

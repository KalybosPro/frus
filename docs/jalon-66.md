# Jalon 66 — `BorderRadius` : rayons d'arrondi **par coin** (SDF)

Dernier manque du modèle de boîte §5 : les coins ne pouvaient être arrondis
qu'uniformément (un seul `f32` traversait scène → GPU). Impossible d'exprimer une
feuille montante aux seuls coins hauts arrondis, un onglet, un segment de groupe.

## `BorderRadius` (frus-core, `Copy`)

`{ top_left, top_right, bottom_right, bottom_left }` avec `uniform`, `top`,
`bottom`, `inflate` (enveloppe d'ombre), `scale` (DPI), et **`clamped`** (rayons
négatifs bornés à zéro avant rendu, comme le préconise le brief).

**`impl From<f32>`** est la clé de la migration : tous les points d'entrée
(`Scene::draw_rect`/`gradient_rect`/`shadow`, `BoxDecoration::radius`,
`Container::radius`) prennent désormais `impl Into<BorderRadius>` — **chaque appel
existant passant un `f32` compile et rend à l'identique**, et un appel passant un
`BorderRadius` obtient le par-coin. Conforme à la règle « personnalisable comme
Flutter » : `Container::new().radius(BorderRadius::top(12.0))`.

## Le pipeline

- `Primitive::Rect.radius` devient `BorderRadius` ; `scaled` met les 4 rayons à
  l'échelle DPI.
- **Instance GPU** : nouvel attribut `radii: vec4` (tl, tr, br, bl), bornés à zéro
  côté peintre ; l'ancien slot `params.x` est libéré.
- **Shader** : `corner_radius(p, radii)` choisit le rayon du **quadrant** du
  fragment (coordonnées centrées, y vers le bas), puis la SDF classique inchangée —
  bordure, flou d'ombre et dégradé fonctionnent tels quels avec le rayon par coin.

## Validation

- **Preuve GPU par readback** : `per_corner_radius_rounds_only_selected_corners` —
  un rectangle au seul coin haut-gauche arrondi (30 px) : pixel (0,0) découpé,
  les trois autres coins **carrés**, centre plein. Sur vrai device wgpu.
- Rétro-compatibilité : `rounded_rect_leaves_corner_transparent` (rayon uniforme
  via `f32`) passe inchangé — le chemin `From<f32>` est pixel-identique.
- **238 tests** au total, tout vert ; build sans avertissement ; démo sans panique.

## Suite (§5 restants)

Décorations de texte (souligné/barré), `letter_spacing`/`line_height`,
consolidation `ColorScheme` (+ `from_seed` HCT), `content_padding` → taffy,
`Alignment`, RTL (§14). Adoption opportuniste du par-coin (BottomSheet aux coins
hauts, onglets, segments) au fil de l'eau.

# Jalon 125 — Découpe arrondie **par coin** (`ClipRRect` + `BorderRadius`)

## Analyse

La découpe en forme (J121) n'offrait qu'un **rayon uniforme** (`RRect(f32)`). Flutter
permet un rayon **par coin** (`ClipRRect(borderRadius: BorderRadius.only(…))`) : un
en-tête aux seuls coins hauts arrondis, une bulle asymétrique, une carte dont un côté
épouse un bord. Ce jalon porte la découpe arrondie au rayon **par coin**.

## Décisions techniques

- **`ClipShape::RRect` porte un [`BorderRadius`]** (4 rayons `tl, tr, br, bl`) au lieu
  d'un `f32`. `BorderRadius` est `Copy` : `ClipShape` le reste (pas de ripple sur les
  `Layer`). Un rayon uniforme reste `BorderRadius::uniform(r)`.

- **Sélection par quadrant dans le shader.** `composite.wgsl` reçoit les 4 rayons (5ᵉ
  attribut d'instance) et choisit celui du **coin du fragment** (`corner_radius`,
  identique au peintre de rectangles `quad.wgsl`) avant le SDF de rectangle arrondi.
  Chaque rayon est borné à la demi-plus-petite dimension.

- **API rétro-compatible.** `ClipRRect::new(f32)` (uniforme) inchangé ; nouveau
  `ClipRRect::rounded(BorderRadius)` pour le par-coin. Les rayons sont `clamped()` (un
  rayon négatif n'a pas de sens).

- **Suivi des transformations.** `ClipShape::scaled_xy` met chaque rayon à l'échelle
  (via `BorderRadius::scale`) — la découpe reste correcte sous un changement de densité.

## Implémentation

- `frus-core` : `ClipShape::RRect(BorderRadius)` ; `scaled_xy` échelonne les 4 rayons.
- `frus-gpu` : `LayerComposite` / `CompInstance` portent `radii: [tl, tr, br, bl]` (5ᵉ
  attribut) ; `composite.wgsl` sélectionne le rayon par quadrant (`corner_radius`).
- `frus-widgets` : `ClipRRect` stocke un `BorderRadius` ; `new` (uniforme) +
  `rounded(BorderRadius)`.

## Tests

- `frus-test` (au pixel, GPU réel) : `rrect_clip_rounds_only_the_specified_corner` —
  seul le coin haut-gauche (rayon 16) est gommé, les trois autres restent **nets** ;
  les cas uniformes et `RRect(0) = rectangle` tiennent toujours.
- `frus-widgets` / `frus-transforms` : formes de découpe émises mises à jour.
- Workspace complet vert : frus-core 91, frus-gpu 16, frus-widgets 227, frus-test
  clip 4.

## Reste — `ClipPath` (chemin arbitraire)

La découpe à un **chemin quelconque** est un chantier distinct : elle demande un
**pipeline de masque** (rendre le chemin dans une texture de couverture, ou un tampon
stencil, échantillonné au compositing) — pas une simple extension du SDF au fragment,
qui ne couvre que les formes analytiques (rect / rrect / ellipse). À traiter comme sa
propre brique (`ClipShape::Path` + texture de masque par calque) plutôt que de la
bâcler ici.

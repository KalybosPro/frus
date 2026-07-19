# Jalon 89 — Chemins vectoriels & icônes

## Analyse

La `Scene` ne connaissait que **trois primitives** : `Rect` (coins arrondis,
bordure, dégradé, ombre — via SDF), `Text`, `RichText`. Aucun **tracé
arbitraire**. Conséquences directes : pas de vraies icônes (elles auraient été
des glyphes texte/emoji), pas de `CustomPainter`, pas de graphiques, pas de
formes libres. Pour un framework « façon Flutter », c'est la brique fondatrice
la plus basse : icônes, dessin personnalisé et (plus tard) graphiques s'appuient
tous dessus.

Ce jalon ajoute la primitive **chemin** de bout en bout — modèle, GPU, widgets —
et l'expose par un widget `Icon` (jeu embarqué) et un widget `CustomPaint`
(toile libre).

## Architecture

```
frus-core                       frus-gpu                     frus-widgets
─────────                       ────────                     ────────────
Path (verbs)  ── Primitive::Path ──► PathPainter             Icon ─┐
Stroke                              (lyon → triangles         icons │→ Scene::fill_path
Scene::fill_path/stroke_path/        indexés + clip)          Custom┘  (Primitive::Path)
          paint_path                 shaders/path.wgsl        Paint
```

### `frus-core` — le modèle (`path.rs`)
- `PathVerb` : `MoveTo · LineTo · QuadTo · CubicTo · Close` (segments droits +
  Bézier quadratique/cubique).
- `Path` : suite de verbes, **builder chaînable** (`move_to().line_to()…`), plus
  des constructeurs (`rect`, `circle` — quatre arcs cubiques, constante `0.5523`)
  et des transformations `scaled` / `translated` (pour adapter une icône `24×24`
  à sa boîte, et pour le passage logique→physique DPI).
- `Stroke { color, width }`.
- `Primitive::Path { path, fill: Option<Color>, stroke: Option<Stroke>, clip,
  owner }` — intégré aux trois passes transverses existantes : `owner()`,
  `scaled()` (met à l'échelle géométrie **et** épaisseur de trait), `push_faded()`
  (fond de sortie : fade du fill et du stroke).
- `Scene::fill_path` / `stroke_path` / `paint_path`.

### `frus-gpu` — le rendu (`path.rs` + `shaders/path.wgsl`)
- **Tessellation CPU par lyon** : `FillTessellator` (règle *non-zero*) et
  `StrokeTessellator` (épaisseur), chacun via un `Ctor` qui injecte **couleur +
  découpe** dans chaque sommet produit. Tous les chemins d'une frame sont
  fusionnés dans un seul `VertexBuffers<PathVertex, u32>` (lyon décale les
  indices automatiquement), puis téléversés en un buffer de sommets + un buffer
  d'indices (agrandis par puissance de deux au besoin).
- **Pipeline indexé** (`TriangleList`), sommet = `pos(px) · color(sRGB) ·
  clip`. Le shader projette px→NDC et **découpe au fragment** (même convention
  que `quad.wgsl`), sRGB→linéaire à l'écriture. Tessellateurs et géométrie sont
  **retenus** d'une frame à l'autre (zéro réallocation en régime permanent).
- Câblé dans **le renderer fenêtré et le rendu hors-écran** dans l'ordre
  `rectangles → chemins → texte` (les icônes passent au-dessus des fonds, sous
  le texte).

### `frus-widgets`
- **`Icon`** : rend une icône du jeu, mise à l'échelle (`size/24`) et centrée
  dans sa boîte ; couleur = `on_surface` du thème par défaut, **surchargeable**
  (`.color(...)`) — conforme à la règle « personnalisable comme Flutter ».
- **`icons.rs`** : `IconName` (Check, Close, Add, Menu, Star, Heart, Circle,
  Square, Play, ChevronLeft/Right) — silhouettes pleines sur grille `24×24`
  (polygones, étoile/plus procéduraux, cœur en Bézier, croix/menu en
  sous-chemins).
- **`CustomPaint`** : toile de taille fixe qui délègue sa peinture à une closure
  `Fn(&mut Scene, Rect, &Theme)` — le pendant du `CustomPainter` de Flutter, qui
  se thème au moment de peindre.

## Décisions techniques

- **lyon vs tessellateur maison.** lyon 1.0 est la référence Rust (robuste,
  courbes + strokes + fill rules). Écrire un tessellateur correct (auto-
  intersections, joints de trait) serait un projet à soi seul. On l'adopte.
- **Tessellation CPU, pas de compute.** Simple, portable (y compris le futur
  Web), suffisant à cette échelle ; la géométrie est mise en cache par frame.
- **Passes séparées** (rect/path/text) plutôt qu'une passe unique ordonnée : on
  **prolonge le modèle en calques déjà en place**. Limite assumée : un chemin ne
  peut passer *sous* un rectangle émis après lui (comme le texte est toujours
  au-dessus). Une passe unifiée triée viendra avec le compositing.

## Explications & limites

- **Anti-aliasing.** La géométrie tessellisée est **nette mais non lissée** (pas
  de MSAA ici, pour un readback déterministe sous le GPU logiciel de WSL). Les
  bords obliques d'icône sont donc légèrement crénelés ; le lissage (MSAA ou AA
  géométrique de lyon) arrivera avec le compositing.
- **Remplissage uni.** `fill` est une couleur unie ; dégradés/textures sur
  chemin viendront plus tard (les dégradés existent déjà pour les rectangles).

## Tests

- `frus-core` : builder/ordre des verbes, `rect`/`circle`, `scaled`/`translated`
  ; doctest du builder.
- `frus-gpu` (readback GPU, la preuve pixel) : `fills_a_vector_triangle`
  (intérieur peint, extérieur au clear) ; `strokes_a_path_outline_only` (le trait
  est peint, le centre reste vide).
- `frus-widgets` : `Icon` émet **un** chemin rempli, la surcharge de couleur
  l'emporte sur le thème, la taille pilote la boîte ; chaque `IconName` produit
  un chemin non vide (étoile = 10 sommets, menu = 3 sous-chemins) ; `CustomPaint`
  invoque la closure avec sa boîte résolue.
- Aucune régression : les widgets existants n'émettent pas de chemin → les
  goldens et toutes les suites restent identiques.

## Démo

La carte principale affiche une **rangée d'icônes vectorielles** (coche à la
couleur d'accent, étoile, cœur, menu, chevron) — chemins tessellisés rendus par
le nouveau pipeline.

## Reste

- Anti-aliasing (MSAA / AA géométrique).
- Dégradés & motifs sur chemin ; passe unifiée triée (avec le compositing).
- Jeu d'icônes élargi ; chargement de chemins depuis SVG.

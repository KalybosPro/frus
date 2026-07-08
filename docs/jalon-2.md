# Jalon 2 — Moteur de mise en page (flexbox via taffy)

Ajoute la couche qui transforme un **arbre de nœuds stylés** en **rectangles
positionnés**, prêts pour le renderer. On ne positionne plus en pixels absolus :
on décrit des règles (flex, tailles, padding, gap) qui s'adaptent à la fenêtre.

## Ce qui est livré

- **Nouveau crate `frus-core`** : types fondamentaux partagés sans logique ni
  dépendance (`Point`, `Size`, `Rect`, `Color`). `frus-gpu` les ré-exporte.
- **Nouveau crate `frus-layout`** : moteur de mise en page au-dessus de
  [taffy](https://docs.rs/taffy), **caché** derrière une API frus stable.
  - `Style` (width/height, flex_grow, flex_direction, padding, gap),
  - `Layout<T>` : arbre avec donnée `T` par nœud (ici une `Color`),
  - `absolute_rects()` : rectangles en **coordonnées absolues**.
- **Démo** pilotée par layout : colonne (barre + rangée sidebar/main), adaptée à
  la taille de la fenêtre.

## Architecture

```
        frus-core  (Point, Size, Rect, Color) — zéro dépendance
        ╱        ╲
  frus-gpu       frus-layout (wrap taffy)
        ╲         ╱
          frus-shell  (layout -> Scene -> GPU)
```

Flux d'une frame :

```
arbre de nœuds (Style + Color)
      │ taffy::compute_layout
      ▼
positions relatives ──(accumulation d'offsets)──► rects ABSOLUS
      │ Scene::fill_rect
      ▼
   frus-gpu ─► écran
```

## Décisions

- **taffy** pour le layout (flexbox/grid mûr, utilisé par Bevy/Zed) — réutilise
  l'écosystème plutôt que réécrire.
- **`frus-core` partagé** : évite le couplage `frus-layout → frus-gpu` et la
  duplication du type `Rect`.
- **API mince** au-dessus de taffy : `Style` frus traduit en `taffy::Style`.
  taffy reste un détail d'implémentation remplaçable.
- **Coordonnées absolues** calculées côté frus (taffy donne du relatif) :
  directement rendables et testables.

## Tests

- `frus-core` : construction/`to_array` de `Rect`.
- `frus-layout` : une rangée flex `[fixe 120px, grow:1]` dans 400×100 avec
  padding 10 / gap 8 → vérifie les rects absolus (`A = (10,10,120,80)`,
  `B = (138,10,252,80)`).

## Lancer

```sh
bash scripts/wsl-run.sh   # fenêtre : barre verte + sidebar rouge + zone bleue
cargo test                # dans WSL
```

## Limites (à traiter plus tard)

- Sous-ensemble flexbox seulement (pas encore d'alignements, marges par côté).
- Pas d'arbre de widgets par-dessus : la démo construit l'arbre à la main. Le
  jalon widgets viendra s'appuyer sur cette couche.

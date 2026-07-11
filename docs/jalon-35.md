# Jalon 35 — Layout : grille (`Grid`)

Le jalon layout dédié : une **grille** à colonnes égales, via le **CSS Grid de
taffy** (déjà dans la dépendance) plutôt qu'un composite bricolé.

## Approche

Le manque de `Grid` venait d'un piège : un composite « builder » ne peut pas
reconstruire des enfants `Box<dyn Widget>`. La bonne réponse n'est pas un widget
qui reconstruit des lignes, mais **déléguer la disposition au moteur de layout**.

- `Style` gagne `grid_columns: Option<usize>`.
- `to_taffy` : si `Some(n)` → `display: Grid` + `grid_template_columns = n × 1fr`.
  Les enfants se placent **automatiquement** (auto-flow, ligne par ligne) ; les
  lignes sont dimensionnées au contenu → la hauteur du conteneur suit toute seule.
- `Grid` est donc un **conteneur normal** : `cell()` n'est qu'un `push` d'enfant,
  aucune branche spéciale dans `build_ui`, aucun problème de propriété.

## API

```rust
Grid::new(3).gap(10.0).width(360.0)
    .cell(a).cell(b).cell(c)   // [a b c] / [d …]
    .cell(d)
```

## Démo

Onglet « À propos » des Réglages : une grille **3 colonnes** de tuiles de
statistiques (Total / Actives / Terminées).

## Tests

- `cells_flow_into_rows_and_columns` : dans une grille 2 colonnes, `a`/`b` sur la
  même ligne (même `y`, `b` à droite), `c` sous `a` (même `x`, `y` plus bas), `d`
  aligné (colonne de `b`, ligne de `c`), et **colonnes égales** (`a.width ==
  b.width`). Preuve directe que la grille dispose correctement.
- 69 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- **Colonnes égales** (`1fr`) uniquement ; pas encore de largeurs de colonnes
  variables (`px` / `auto` / `minmax`) ni de spans de cellules.
- Pas de `Table` (en-têtes / bordures de cellules) — candidat pour le prochain
  lot de widgets, bâti sur `Grid`.

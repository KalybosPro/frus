# Jalon 232 — DataTable auto-triant (widget réutilisable)

## Analyse

Le `Table` est purement contrôlé : il émet la colonne cliquée (`on_sort`), affiche l'indicateur
`sorted(...)`, mais **ne trie pas** — l'application réordonne ses lignes elle-même. Résultat : la
logique de tri (comparaison, sens, casse) est recopiée à la main dans chaque reducer (cf. la grille
de la démo). Ce jalon ouvre le domaine **DataTable** : un tableau qui **trie ses propres données**
pour l'affichage, tout en restant contrôlé.

## Décisions techniques

- **`sort_rows` / `compare_cells` — logique pure, publique.** `compare_cells(a, b)` compare
  **numériquement** si les deux cellules se lisent comme des nombres, sinon lexicalement **insensible
  à la casse**. `sort_rows(rows, col, asc)` renvoie une copie triée. Fonctions libres exportées :
  réutilisables aussi hors widget (un reducer peut trier ses données de la même façon).

- **`DataTable` encapsule le tri d'affichage.** On lui passe les lignes brutes et l'état
  `sorted(col, sens)` ; il reconstruit un `Table` interne avec les lignes déjà triées + l'indicateur,
  et lui relaie `on_sort`. L'état de tri reste **dans l'app** (modèle contrôlé) — seule la
  transformation d'affichage est encapsulée.

- **Composition, pas héritage.** `DataTable` **délègue** les cinq méthodes `Widget` que `Table`
  surcharge (`style`, `children`, `paint` vide, `on_click`, `stack`) à un `Table` interne reconstruit
  à chaque builder — même motif de `rebuild()` que `Table`. Type par défaut `Msg = ()` pour
  l'ergonomie.

## Implémentation

- `frus-widgets/src/datatable.rs` (nouveau) : `compare_cells`, `sort_rows`, `DataTable` (builders
  `column_widths`, `sorted`, `on_sort` ; `rebuild` interne).
- `frus-widgets/src/lib.rs` : `mod datatable;` + `pub use datatable::{compare_cells, sort_rows, DataTable};`.

## Vérification

- **Widget** `sort_rows_is_numeric_aware_and_case_insensitive` : colonne numérique 2 < 9 < 10 (et
  non "10" < "2" < "9"), colonne texte alice < Bob < Carol, sens décroissant inversé.
- **Widget** `compare_cells_prefers_numbers_then_text`, `data_table_builds_a_non_empty_tree`.
- **Doctest** du `DataTable`.
- **Golden** `data_table_sorted` : tri par « Score » décroissant (12, 10, 9, 2) + indicateur « ▼ ».
- Widgets 367 ; goldens 65.

## Reste

- **Pagination** interne (tranche de page + `Pagination` sous le tableau).
- Wirer `DataTable` dans la démo (remplacer le tri recopié du reducer de grille).
- Clé de tri **personnalisée** par colonne (dates, montants formatés).

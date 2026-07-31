# Jalon 237 — Démo : écran Tableau de données (DataTable câblé)

## Analyse

`DataTable` (jalons 232/233/236) était testé isolément mais absent de l'application. Ce jalon
l'**ancre dans la démo** : un nouvel écran en lecture seule qui trie et pagine un vrai jeu de
données, câblé à l'état — la preuve d'ergonomie de bout en bout.

Contraste voulu avec la **grille éditable** (route `Grid`) : celle-ci trie côté reducer
(`app.grid.sort_by`) parce que ses cellules sont des `TextInput` liés à l'index de ligne. Le
`DataTable`, lui, est en lecture seule et **trie son affichage lui-même** — l'app ne recopie aucun
tri.

## Décisions techniques

- **Nouvelle route `Data`** (index 6) : ajoutée à l'`enum`, au dispatch `screen`, à
  `save_state`/`restore_state` (live-reload) et au tiroir (« Data table → »).

- **État minimal : `(data_sort, data_page, data_page_size)`.** Le reducer ne fait que basculer le
  sens de tri, changer de page, changer de taille — **jamais** réordonner les données. `DataSort` et
  `DataPageSize` ramènent à la page 1. Les `0` (défauts dérivés) sont coercés en valeurs de départ
  (page 1, taille 5) dans l'écran.

- **`data_screen`.** `DataTable::new(headers, rows).on_sort(DataSort).paginated(page, per, DataPage)
  .page_sizes([5,10], DataPageSize)`, plus `.sorted(col, asc)` si un tri est actif. Jeu de 12 lignes
  (nom, rôle, score) → pagination réelle.

## Implémentation

- `frus-demo/src/lib.rs` : `Route::Data` + plomberie ; champs d'état + `Msg::{DataSort, DataPage,
  DataPageSize}` + arms de reduce ; `DATA_PEOPLE` + `data_screen` ; entrée de tiroir ; import
  `DataTable`.

## Vérification

- **Démo** `data_table_screen_sorts_and_paginates_without_touching_data` : l'écran se rend ; premier
  clic d'en-tête = croissant, re-clic = décroissant, le tri ramène à la page 1 ; changer la taille
  ramène à la page 1. Les données source ne sont jamais réordonnées (le widget trie l'affichage).
- Démo 33 ; widgets/goldens inchangés.

## Reste

- Wirer un `DatePicker` **filtré/borné** dans la démo (jalon 238).
- Un état « ligne sélectionnée » (`on_select_row`) sur l'écran data.

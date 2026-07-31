# Jalon 236 — DataTable : taille de page + libellé « N–M of T »

## Analyse

La pagination (jalon 233) posait un sélecteur de numéros de page, mais il manquait deux repères
usuels d'un tableau de données : **combien** de lignes sont montrées (« 1–3 of 7 ») et un moyen de
**changer la taille de page**. Ce jalon enrichit le pied.

## Décisions techniques

- **Libellé de tranche `page_range_label`, pur et public.** `N–M of T` (tiret demi-cadratin), ou
  `0 of 0` si vide, avec la page ramenée dans l'intervalle. Réutilisable hors widget, comme
  `sort_rows`/`page_rows`. Toujours affiché à gauche du pied quand le tableau est paginé.

- **`.page_sizes(sizes, on_page_size)`.** Optionnel : un `SegmentedControl` des tailles proposées, à
  droite du pied, avec la taille courante présélectionnée. `on_page_size(taille)` au changement —
  l'app met à jour la taille (et revient en général à la page 1). Sans effet si non paginé.

- **Pied = ligne flex.** `[libellé] [spacer flex] [Pagination] [sélecteur de taille?]`. Le
  `Box<dyn Widget>` interne (jalon 233) passe de `[table, pager]` à `[table, pied]`.

## Implémentation

- `frus-widgets/src/datatable.rs` : `page_range_label` ; champs `page_sizes`/`on_page_size` ; builder
  `page_sizes` ; `rebuild` compose le pied (libellé + pager + `SegmentedControl`).
- `frus-widgets/src/lib.rs` : `page_range_label` ajouté au `pub use`.

## Vérification

- **Widget** `page_range_label_describes_the_slice` : `1–3 of 7`, `4–6 of 7`, dernière page partielle
  `7–7 of 7`, `0 of 0` si vide, page hors bornes ramenée.
- **Widget** `page_size_selector_appears_in_the_footer` : pied à **3** enfants (libellé+spacer+pager),
  **4** avec le sélecteur de taille.
- **Golden** `data_table_paginated` (enrichi) : « 1–3 of 7 » · ‹ 1 2 3 › · 3|5|10 (3 actif).
- Widgets 373 ; goldens 68 ; doctest OK.

## Reste

- Wirer `DataTable` (tri + pagination + taille) dans la démo, en retirant le tri recopié du reducer.
- Clé de tri **personnalisée** par colonne (dates, montants formatés).

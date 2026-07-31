# Jalon 233 — Pagination interne du DataTable

## Analyse

Le `DataTable` (jalon 232) trie ses lignes mais les affiche **toutes**. Pour un vrai tableau de
données, il faut les **paginer** : n'afficher qu'une tranche et offrir un sélecteur de page. Le
`Pagination` existe déjà comme contrôle pur (numéros de page) ; ce jalon le **compose** avec le
`DataTable`, qui découpe lui-même la tranche.

## Décisions techniques

- **Helpers purs `page_count` / `page_rows`, publics.** `page_count(len, per)` = nombre de pages
  (au moins 1) ; `page_rows(rows, current, per)` = la tranche de la page (1-indexée, ramenée dans
  l'intervalle si elle déborde). Réutilisables hors widget, comme `sort_rows`.

- **`.paginated(current, per_page, on_page)`.** Découpe la tranche sur les lignes **déjà triées**
  (tri d'abord, page ensuite) et pose un [`Pagination`](crate::Pagination) sous le tableau ; le
  nombre de pages est calculé sur le **total** trié. `on_page(page)` remonte le clic — l'app garde la
  page courante (modèle contrôlé).

- **`inner` devient `Box<dyn Widget>`.** Pour coiffer le `Table` d'un `Pagination`, le rendu interne
  passe d'un `Table` à un `Box<dyn Widget>` : soit le `Table` seul, soit une `Flex` colonne
  `[table, pager]`. La délégation `Widget` (style/children/paint/on_click/stack) vise ce `Box`.

## Implémentation

- `frus-widgets/src/datatable.rs` : `page_count`, `page_rows` ; champs `page`/`on_page` ; builder
  `paginated` ; `rebuild` découpe la page et compose `Table` + `Pagination` ; `inner: Box<dyn Widget>`.
- `frus-widgets/src/lib.rs` : `page_count`, `page_rows` ajoutés au `pub use`.

## Vérification

- **Widget** `pagination_slices_rows_and_counts_pages` : 7 lignes / 3 = 3 pages ; page 1 = `[1,2,3]`,
  page 3 = `[7]`, page hors bornes ramenée à la dernière.
- **Widget** `data_table_with_pagination_builds_table_and_pager` : l'arbre a **2** enfants (table +
  sélecteur).
- **Golden** `data_table_paginated` : top 3 par Score décroissant (15, 12, 10) + sélecteur ‹ 1 2 3 ›.
- Widgets 369 ; goldens 66.

## Reste

- Wirer `DataTable` (tri + pagination) dans la démo, en retirant le tri recopié du reducer de grille.
- Sélecteur de **taille de page** ; libellé « N–M sur T ».

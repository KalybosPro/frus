# Jalon 242 — DataTable : recherche/filtre

## Analyse

Le `DataTable` encapsule déjà des **transforms d'affichage** — tri (jalon 232), pagination (233/236),
et la traduction de sélection à travers eux (239/241). La recherche en est un de plus : filtrer les
lignes source à celles qui correspondent à une requête, **avant** tri et pagination. En le plaçant en
amont du même pipeline d'index source, la sélection (simple ou multiple) continue de fonctionner sur le
sous-ensemble visible, sans code supplémentaire.

## Décisions techniques

- **`searchable(query, on_query)`.** Un champ de recherche (un [`TextInput`]) coiffe le tableau ;
  `on_query(texte)` remonte chaque frappe à l'application (qui met à jour `query` et, en général,
  revient à la page 1). Le widget ne stocke pas la requête : elle vient de l'app à chaque rendu
  (modèle contrôlé).

- **Filtre en tête de `sorted_order`.** Le pipeline d'index commence par ne garder que les lignes
  correspondantes (`row_matches`), puis trie, puis découpe la page. `page_indices` reste une liste
  d'**index source** (sous-ensemble) → la traduction position affichée ↔ source (clic, case,
  surlignage) et le total du pied (« N–M of <filtré> ») suivent automatiquement.

- **`row_matches(row, query)`.** Sous-chaîne **insensible à la casse** sur **toutes** les colonnes ;
  requête vide/blanche = tout passe. Fonction publique réutilisable (un reducer peut filtrer pareil).

## Implémentation

- `frus-widgets/src/datatable.rs` : helper `row_matches` ; champs `query`/`on_query` + builder
  `searchable` ; filtre en tête de `sorted_order` ; `rebuild` coiffe le bloc d'un `TextInput` quand
  `on_query` est posé. Tests `row_matches_is_case_insensitive_substring_over_all_cells` et
  `search_filters_rows_before_sort_and_keeps_source_indices` (requête « a » → filtré puis trié en
  index source `[2, 0]`).
- `frus-widgets/src/lib.rs` : ré-export de `row_matches`.
- `frus-demo/src/lib.rs` : état `data_query` + `Msg::DataSearch` (met à jour le filtre, page → 1) ;
  `data_screen` câble `.searchable(app.data_query, Msg::DataSearch)`.

## Vérification

- **Widgets** : `row_matches` (casse, sous-chaîne, colonnes, vide) ; `search_filters…` (filtre en
  amont du tri, index source préservés).
- **Golden** `data_table_search` : champ « ar » + seules `Bob (Paris)` et `Carol (Berlin)` parmi
  quatre — inspecté visuellement.
- **Démo** `data_table_screen_…` étendu : la frappe met à jour `data_query` et ramène à la page 1.
- Widgets 378 ; goldens 72 ; démo 34 ; shell compile.

## Reste

- Actions **groupées** (barre d'actions quand des lignes sont cochées).
- Message **« aucun résultat »** quand le filtre vide le tableau.
- Un nouveau domaine de widgets (`Tabs`/`Tree`/`Kanban`).

# Jalon 241 — DataTable : sélection multiple (cases à cocher)

## Analyse

Le `Table` de base offre déjà une **sélection multiple** : une colonne de cases à cocher coiffée d'un
« tout cocher » (`checkboxes(on_check, on_check_all)`), l'état coché reflétant `selected`. Mais, comme
pour la sélection simple (jalon 239), le `Table` raisonne en **positions affichées** : sous tri +
pagination du `DataTable`, la case de la 2ᵉ ligne affichée ne coche pas la 2ᵉ ligne source.

Ce jalon expose la sélection multiple au niveau du `DataTable`, avec la **même traduction** position
affichée ↔ index source déjà en place pour le tri, la page et la sélection simple.

## Décisions techniques

- **`checkboxes(on_check, on_check_all)`.** `on_check(ligne_source)` reçoit l'index de la **ligne
  source** (traduit via `page_indices`, comme `on_select_row`) ; `on_check_all` est un message
  transmis tel quel — l'application décide ce que « tout » recouvre (toutes les lignes source, ou la
  page). L'état coché **réutilise** [`selected`](DataTable::selected) : mêmes index source, même
  traduction vers les positions visibles.

- **Coexiste avec `on_select_row` (façon Gmail).** La case gère la **sélection groupée** (surlignage +
  coche), tandis qu'un clic sur le **corps** de la ligne reste un clic de ligne (focus/détail). Les
  deux ciblent des cellules différentes (case vs texte) — le hit-test du plus profond les sépare.

- **Modèle contrôlé.** L'ensemble coché vit dans l'app (`data_checked: Vec<usize>` d'index source) ;
  le widget ne fait que traduire et afficher.

## Implémentation

- `frus-widgets/src/datatable.rs` : champs `on_check`/`on_check_all` + builder `checkboxes` ; `rebuild`
  câble `Table::checkboxes` avec la traduction position → source ; test
  `checkbox_click_reports_the_source_row_through_sort_and_page` (page 2 d'un tri croissant → la case
  renvoie l'index source, sentinelle `999` de la case de tête filtrée).
- `frus-demo/src/lib.rs` : état `data_checked` + `Msg::{DataCheck, DataCheckAll}` (toggle / tout
  cocher-décocher) ; `data_screen` câble `.checkboxes(...).selected(&data_checked)` en plus du clic de
  ligne, avec un résumé « N checked ».

## Vérification

- **Widgets** `checkbox_click…` : page 2 (taille 2) d'un tri croissant `[1,2,0]` → la case renvoie
  l'index source `0`.
- **Golden** `data_table_checkboxes` : tri « Score » décroissant `[Bob, Dan, Ada, Carol]`, lignes
  **source** 0 (Ada) et 3 (Dan) cochées → deux cases cochées à leurs positions triées (2ᵉ/3ᵉ) et la
  case de tête **indéterminée** (2 sur 4) — inspecté visuellement.
- **Démo** `data_table_screen_…` étendu : toggle d'une ligne (coche/décoche), « tout cocher » = 12
  lignes, re-« tout cocher » = tout décocher.
- Widgets 376 ; goldens 71 ; démo 34 ; shell compile.

## Reste

- **Filtre/recherche** au-dessus du `DataTable` (l'app filtre les lignes source ; le widget
  trie/pagine/sélectionne le sous-ensemble) — jalon 242.
- Actions **groupées** (barre d'actions quand des lignes sont cochées).

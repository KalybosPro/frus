# Jalon 239 — DataTable : ligne sélectionnée (traduction index source ↔ position affichée)

## Analyse

Le `Table` de base sait déjà **surligner** une ligne (`selected`) et émettre au clic (`on_select_row`),
mais il raisonne en **positions affichées** (0..n de ce qu'il montre). Le `DataTable`, lui, **trie** et
**pagine** son affichage : la ligne que l'utilisateur voit en 2ᵉ position n'est pas forcément la 2ᵉ ligne
des données source. Sans traduction, l'application recevrait un index affiché — inutilisable pour désigner
la donnée, et le surlignage se décalerait au moindre tri.

Ce jalon ajoute la **sélection de ligne** au `DataTable` en gardant l'identité de la donnée source à
travers tri + pagination — exactement le service que le widget rend déjà pour le tri et la page.

## Décisions techniques

- **Index de lignes source partout.** `rebuild` ne trie/découpe plus des `Vec<String>` mais une liste
  d'**index** `0..rows.len()` (`sorted_order`, tri **stable**), puis en prend la tranche de page. Cette
  liste `page_indices` conserve, pour chaque position affichée, l'index d'origine de la ligne.

- **`on_select_row(f)`.** Le clic sur la position affichée `d` renvoie `f(page_indices[d])` — donc
  l'index de la **ligne source**, quel que soit le tri ou la page courante.

- **`selected(&[source…])`.** L'application marque des lignes par leur index **source** ; le `DataTable`
  ne surligne que celles présentes dans la tranche courante (traduction source → position affichée).

- **Modèle contrôlé inchangé.** L'état de sélection vit dans l'app (`data_selected: Option<usize>`) ; le
  widget ne fait que la traduction d'affichage, comme pour `sort`/`page`.

- **Helpers de pagination rendus publics.** `page_count`, `page_rows`, `page_range_label` (docs déjà
  « réutilisable hors widget ») sont désormais ré-exportés — `rebuild` inline son propre découpage
  d'index, ils restent l'API réutilisable annoncée.

## Implémentation

- `frus-widgets/src/datatable.rs` : champs `on_select`/`selected` + builders `on_select_row`/`selected` ;
  `sorted_order()` (tri stable d'index) ; `rebuild` réécrit autour de `page_indices` (traduction dans les
  deux sens) ; test `selection_click_reports_the_source_row_through_sort_and_page` (collecte des messages
  `on_click` de l'arbre → vérifie que le clic renvoie l'index source à travers tri **et** pagination).
- `frus-widgets/src/lib.rs` : ré-export de `page_count`, `page_rows`, `page_range_label`.
- `frus-demo/src/lib.rs` : état `data_selected` + `Msg::DataSelectRow` (bascule au re-clic) ; `data_screen`
  câble `.on_select_row(...).selected(&[i])` et affiche un **détail** de la ligne sélectionnée (lu dans les
  données source par son index).

## Vérification

- **Widgets** `selection_click…` : tri croissant sur la clé → ordre source `[1,2,0]` ; page 1 (taille 2) →
  le clic renvoie `[1,2]`, page 2 → `[0]`. La traduction survit au tri et à la pagination.
- **Golden** `data_table_selected` : tri par « Score » décroissant `[Bob 12, Dan 10, Ada 9, Carol 2]` ; la
  ligne **source** 3 (Dan) apparaît surlignée en **2ᵉ** position — inspecté visuellement.
- **Démo** `data_table_screen_…` étendu : clic = sélection de la ligne source, clic ailleurs = déplace,
  re-clic = désélection.
- Widgets 374 ; goldens 69 ; démo 34 ; shell compile.

## Reste

- Clé de tri **personnalisée** par colonne du `DataTable` (dates, montants formatés) — jalon 240.
- Sélection **multiple** dans le `DataTable` (cases à cocher, comme le `Table`).

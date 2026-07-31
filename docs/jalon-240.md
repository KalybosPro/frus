# Jalon 240 — DataTable : clé de tri personnalisée par colonne

## Analyse

Le `DataTable` trie ses lignes avec [`compare_cells`] : **numérique** si les deux cellules se lisent
comme des nombres, sinon **texte insensible à la casse**. Cela suffit pour « Name » ou « Score », mais
échoue dès qu'une colonne porte des valeurs que ce défaut classe mal :

- **priorités** (`High`/`Medium`/`Low`) → triées alphabétiquement (`High, Low, Medium`), pas sémantiquement ;
- **dates formatées** (`Mar 2024`) → tri lexical ≠ chronologique ;
- **montants formatés** (`$1.2M`, `$950k`) → ne parsent pas en nombre, donc tri texte erroné.

Ce jalon laisse l'application fournir un **comparateur par colonne**, tout en gardant le modèle
contrôlé (l'état de tri reste `(colonne, sens)` dans l'app).

## Décisions techniques

- **`sort_with(col, cmp)`.** Un comparateur `Fn(&str, &str) -> Ordering` par colonne, stocké dans un
  `Vec<Option<…>>` indexé (comme les actions d'en-tête du `Table`). Il définit l'ordre **croissant** ;
  le sens (`sorted(_, ascending)`) s'applique par-dessus (inversion si décroissant).

- **Intégré à `sorted_order` (jalon 239).** Le tri d'index consulte le comparateur de la colonne triée
  s'il existe, sinon retombe sur `compare_cells`. La traduction index source ↔ position affichée (donc
  la sélection et la pagination) fonctionne à l'identique — c'est le **même** tri d'index.

- **Local au widget.** Le comparateur n'affecte que le tri d'affichage du `DataTable` ; le helper
  réutilisable [`sort_rows`] (tri par défaut) reste inchangé pour les reducers qui l'utilisent.

## Implémentation

- `frus-widgets/src/datatable.rs` : champ `comparators` + builder `sort_with` ; `sorted_order` choisit
  comparateur personnalisé ou défaut ; test `custom_comparator_orders_a_column_semantically`
  (`Low < Medium < High` via la collecte des messages `on_click` → l'ordre affiché est sémantique,
  pas alphabétique).
- `frus-demo/src/lib.rs` : colonne **« Level »** ajoutée à `DATA_PEOPLE` (`High`/`Medium`/`Low`) +
  helper `level_rank` ; `data_screen` câble `.sort_with(3, |a,b| level_rank(a).cmp(&level_rank(b)))` ;
  le détail de ligne affiche la priorité.

## Vérification

- **Widgets** `custom_comparator…` : trois lignes `High/Low/Medium` triées croissant → index source
  `[1,2,0]` (Low, Medium, High), et non `[0,1,2]` du tri texte.
- **Golden** `data_table_custom_sort` : colonne « Priority » triée croissant → affichage
  `Low, Medium, High` (et non `High, Low, Medium`) — inspecté visuellement.
- **Démo** `data_table_screen_…` étendu : `level_rank` sémantique ; trier la colonne Level se rend.
- Widgets 375 ; goldens 70 ; démo 34 ; shell compile.

## Reste

- Sélection **multiple** dans le `DataTable` (cases à cocher, comme le `Table`).
- Un **filtre**/recherche au-dessus du `DataTable` (l'app filtre les lignes source).

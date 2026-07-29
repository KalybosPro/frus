# Jalon 178 — Tableau : colonnes gelées

## Analyse

Une large grille (beaucoup de colonnes) déborde horizontalement. On veut alors **figer** les
premières colonnes (un identifiant, un nom) pendant que le reste **défile horizontalement** —
motif « frozen columns » des tableurs. Le tableau n'avait pas de défilement horizontal ni de
colonne épinglée.

## Décisions techniques

- **Composition : bloc figé + défilement horizontal.** En mode gelé, la racine devient une
  `Flex` **rangée** de deux blocs : à gauche un `Flex` **colonne** des cellules figées
  (en-tête + rangées, colonnes `0..n`), à droite un `Scroll` **horizontal** contenant un
  `Flex` colonne des cellules restantes (colonnes `n..`). L'**en-tête des colonnes défilantes
  est dans le même `Scroll`** : il suit ses colonnes au défilement, tandis que les colonnes
  gelées restent en place. Réutilise le `Scroll` existant — pas de nouvelle machinerie.

- **Alignement par hauteurs identiques.** Les deux blocs sont des `Flex` colonnes à même
  `gap` et à rangées de hauteur `ROW_H` : la rangée `r` du bloc figé s'aligne pixel à pixel
  avec la rangée `r` du bloc défilant. Les largeurs des blocs (somme des colonnes + écarts)
  se complètent pour tenir dans la largeur totale.

- **Chemin dédié, garanti sans régression.** `build_frozen()` ne s'active que si les
  conditions sont réunies (largeur totale + colonnes **toutes fixes** + `n` dans `1..columns`,
  texte, hors virtualisation/cases) ; sinon il renvoie `None` et le tableau retombe sur sa
  disposition normale. Les tableaux existants (qui n'appellent pas `frozen_columns`) sont
  **inchangés**.

## Implémentation

- `table.rs` : champ `frozen` + builder `frozen_columns(n)` ; `build_frozen()` (bloc figé +
  `Scroll` horizontal) et `frozen_header_cell()` ; court-circuit en tête de `rebuild`.
- `goldens.rs` : `table_frozen_columns` (colonne « Name » figée, Q1/Q2 visibles, Q3 hors cadre).

## Vérification

- **Unitaire** : `frozen_columns_split_into_pinned_and_scrolling_blocks` — racine à deux blocs ;
  une zone défilable **horizontale** (max_x > 0) ; cellule **gelée** cliquable (sélection) ;
  en-tête **défilant** triable.
- **Golden** `table_frozen_columns` **inspecté** : « Name ▲ » figée, Q1/Q2 visibles, Q3
  coupée, ascenseur horizontal, rangées alignées — aucune régression sur les 33 autres goldens.
- `cargo test --workspace` **vert**.

## Reste

- **Colonnes gelées + virtualisation / cases à cocher / rangées-widgets** : chemins
  aujourd'hui exclusifs ; les combiner (grand tableau figé **et** virtualisé) demanderait
  d'imbriquer défilements vertical (virtualisé) et horizontal (gelé) — jalon dédié.
- **Ombre de séparation** entre bloc figé et zone défilante (repère visuel du gel), et **gel
  à droite** (colonnes d'actions) : extensions visuelles possibles.

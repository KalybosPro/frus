# Jalon 148 — Tableau : sélection multiple & colonnes à largeur variable

## Analyse

Le `Table` (jalon 145) reposait sur `Grid` → **colonnes strictement égales** et aucune
sélection multiple. Pour une vraie table de gestion, il fallait :

- **Sélection multiple** : une colonne de **cases à cocher** par ligne, coiffée d'un
  « **tout cocher** » dans l'en-tête.
- **Largeurs de colonnes variables** : fixes (px) ou flexibles (part de l'espace restant).

## Décisions techniques

- **Rangées `Flex` au lieu de la `Grid`.** Une grille à pistes égales ne permet ni colonne
  étroite (cases à cocher) ni largeurs mixtes. Le tableau est désormais une **colonne de
  rangées `Flex`**, chaque cellule portant sa largeur : `Length(px)` fixe (`flex_grow = 0`)
  ou `Auto` flexible (`flex_grow = 1`). Comme toutes les rangées appliquent les **mêmes
  largeurs dans le même ordre**, les colonnes restent alignées ; la largeur totale fixée
  (`width`) est répartie par le moteur de layout.

- **Sélection multiple pilotée par l'app.** `checkboxes(on_check, on_check_all)` ajoute la
  colonne de cases (à gauche). Chaque case reflète `selected` ; l'en-tête est coché quand
  **toutes** les lignes le sont. `on_check(ligne)` bascule une ligne, `on_check_all` bascule
  tout — le tableau n'a toujours **aucun état** propre. Le clic-ligne (`on_select_row`) et
  les cases coexistent.

- **Case dessinée, coche = icône `Check`.** La `CheckCell` peint un carré (bordure si
  décoché, aplat `primary` + coche si coché) ; la coche réutilise le chemin vectoriel de
  `IconName::Check` — cohérent avec le reste, net à toute taille.

- **Facteurs partagés.** Fond de cellule (en-tête teinté / ligne surlignée / survol) et
  style de cellule (largeur + hauteur de rangée) sont deux fonctions communes aux cellules
  texte et cases, pour éviter la duplication.

- **API compatible.** `header`/`row`/`width`/`on_sort`/`sorted`/`on_select_row`/`selected`
  inchangés ; ajouts `column_widths(&[f32])` et `checkboxes(..)`.

## Implémentation

- `table.rs` : réécrit en rangées `Flex` ; `Cell` gagne une largeur ; nouveau `CheckCell` ;
  `column_widths`, `checkboxes` ; helpers `cell_background`, `cell_style`, `col_width`,
  `all_selected`.
- `goldens.rs` : `data_table` régénéré (même rendu, layout `Flex`) ; nouveau
  `data_table_multiselect`.

## Vérification

- **Unitaire** : structure en rangées (en-tête + données) ; clic en-tête → `Sort`, clic
  ligne → `Select`, clic case ligne → `Check(r)`, clic case en-tête → `CheckAll` ; colonne
  fixe de 80 px positionne bien la colonne suivante au-delà.
- **Golden** : `data_table` (inchangé visuellement) et `data_table_multiselect` (cases,
  « tout cocher » partiel décoché, lignes cochées surlignées, 1re colonne fixe) rendus et
  **inspectés**. `cargo test --workspace` vert.

## Reste

- **État indéterminé** du « tout cocher » (quand *certaines* lignes sont cochées).
- **Cellules-widgets** (pas seulement du texte) : la reconstruction (`rebuild`) régénère
  depuis des `String` ; accueillir des widgets arbitraires demanderait de ne pas les
  reconstruire.
- **Redimensionnement de colonnes** à la souris (poignées entre en-têtes).

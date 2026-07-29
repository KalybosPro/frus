# Jalon 179 — Colonnes gelées : ombre de séparation & gel à droite

## Analyse

Le gel de colonnes (jalon 178) figeait les **premières** colonnes à gauche, sans repère
visuel du bord de gel. Deux manques : (1) une **ombre de séparation** pour signaler que le
contenu défile derrière les colonnes figées ; (2) le gel à **droite** (colonnes d'actions,
totaux) — fréquent quand on veut garder des boutons de ligne en vue.

## Décisions techniques

- **Gel des deux bords.** Le compte de colonnes gelées devient un couple `(gauche, droite)` :
  `frozen_columns(n)` fige à gauche, `frozen_columns_right(m)` à droite. La disposition
  devient `Flex` rangée `[bloc figé gauche?, Scroll horizontal(milieu), bloc figé droite?]` ;
  chaque bloc est bâti par un helper commun `frozen_block(cols, w)`. Le milieu défilant (avec
  son en-tête) porte les colonnes centrales.

- **Ombre de séparation en calque.** Un `FrozenShadow` (calque de pile **inerte** — ne capte
  aucun clic) peint un dégradé `scrim → transparent` (via `gradient_rect`) au **bord intérieur**
  de chaque bloc gelé, **par-dessus** la zone défilante. La racine gelée devient une `Stack`
  `[rangée de blocs, ombre]`. Comme l'ombre n'a pas d'`on_click`, les cellules dessous restent
  cliquables (tri / sélection).

- **Piège évité.** Un calque de pile en `Style` par défaut (`Auto`) se réduit à `0×0` (aucun
  enfant) et ne peint rien ; `FrozenShadow` reçoit donc une **taille explicite** (largeur/hauteur
  totales) pour remplir la pile.

## Implémentation

- `table.rs` : `frozen` devient `(usize, usize)` ; builders `frozen_columns` /
  `frozen_columns_right` ; helper `frozen_block` ; `build_frozen` gère gauche/milieu/droite +
  calque `FrozenShadow` (dégradé `gradient_rect`, taille explicite).
- `goldens.rs` : `table_frozen_columns` (régénéré, ombre visible) ; `table_frozen_both_edges`.

## Vérification

- **Unitaire** : `freezing_both_edges_pins_left_and_right_columns` — gel 1 gauche + 1 droite,
  milieu défilant (max_x > 0) ; en-tête figé **à droite** (« Act ») et **à gauche** (« Name »)
  triables. `frozen_columns_split…` (gel gauche seul) reste vert, l'ombre ne bloquant pas les clics.
- **Golden** : `table_frozen_both_edges` **inspecté** (Name figée, Q1/Q2 défilantes, Act figée à
  droite, ombres aux deux bords) ; `table_frozen_columns` régénéré (ombre au bord du gel) —
  aucune régression sur les 33 autres goldens.
- `cargo test --workspace` **vert**.

## Reste

- **Épaisseur/opacité de l'ombre thématisables** : aujourd'hui `scrim` à 0.28 — pourrait
  suivre l'élévation du thème.
- **Gel + virtualisation** : toujours exclusif (défilements vertical virtualisé et horizontal
  gelé à imbriquer) — jalon dédié.

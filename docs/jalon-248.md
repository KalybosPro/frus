# Jalon 248 — Kanban : aperçu de dépôt vertical

## Analyse

Le glisser-déposer des cartes (jalon 247) route correctement le déplacement, mais l'**aperçu** de
glisser du shell était conçu pour les colonnes de `Table` : le fantôme ne suit que l'axe **horizontal**
(`dx`) et les voisins se réagencent en colonnes. Pour une carte que l'on glisse **verticalement**, le
fantôme ne descendait pas et aucun repère d'insertion n'apparaissait.

Ce jalon donne au shell un **indice d'axe** par widget réordonnable et une branche d'aperçu **verticale**.

## Décisions techniques

- **`Widget::reorder_axis() -> ReorderAxis`** (défaut `Horizontal`). Additif : les colonnes de `Table`
  gardent l'aperçu horizontal existant sans changement ; les cartes (et zones de dépôt) de `Kanban`
  renvoient `Vertical`.

- **Branche verticale de l'aperçu.** Pour un axe vertical, le shell : (1) fait suivre le fantôme en
  **2D** (`dx, dy`) au lieu de `dx` seul ; (2) **n'applique pas** le réagencement horizontal des
  colonnes ; (3) pose une **ligne d'insertion** (bandeau `primary`) au bord supérieur de l'emplacement
  survolé (carte ou zone de dépôt).

- **Géométrie pure et testable.** La position de la ligne est calculée par `drop_insertion_line(target,
  thickness)` — fonction pure, testée sans GPU (comme `draw_ghost_card`).

## Implémentation

- `frus-widgets/src/widget.rs` : enum `ReorderAxis` + méthode `reorder_axis` (défaut `Horizontal`) +
  transfert dans l'impl `Box<dyn Widget>` ; export.
- `frus-widgets/src/kanban.rs` : `Card` et `DropZone` renvoient `ReorderAxis::Vertical` ; test
  `cards_declare_vertical_reorder_axis`.
- `frus-shell/src/app.rs` : `paint_reorder_preview` branche sur l'axe (fantôme 2D + ligne d'insertion
  en vertical, comportement horizontal inchangé) ; helper `reorder_drop_line` + fonction pure
  `drop_insertion_line` ; test `insertion_line_sits_on_the_target_top_edge`.

## Vérification

- **Widgets** : les cartes/zones déclarent l'axe **vertical**.
- **Shell** : `drop_insertion_line` place le bandeau sur le bord supérieur de la cible, à sa largeur.
- **Non-régression** : la branche horizontale (colonnes de `Table`) est inchangée — tests shell et
  goldens `Table` inchangés.
- Widgets 385 ; shell 26 ; goldens 76 ; démo 36.

## Notes

- L'aperçu de glisser est un état **runtime** du shell (il n'apparaît que pendant un glissement) : il
  n'est pas capturable par un golden (rendu d'arbre **statique**). Les parties **pures** (axe, géométrie
  de la ligne) sont couvertes par des tests ; le rendu **live** n'est pas inspecté au GPU dans cet
  environnement.

## Reste

- Cartes **riches** (widgets) + ajout/suppression de carte dans le Kanban.
- Indicateur d'insertion **inter-cartes** plus fin (au-dessus/au-dessous selon la moitié survolée).

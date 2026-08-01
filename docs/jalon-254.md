# Jalon 254 — Revue transverse du glisser-déposer : correctifs Table + Kanban

## Analyse

Revue croisée du domaine **glisser-déposer/réordonnancement** (shell + `reorder.rs` + `kanban.rs` +
`table.rs` + le registre `reorderables`), pour lever les défauts réels — pas seulement du style. Elle a
mis au jour un **bug d'intégration critique** qui rendait **inopérants en application** les jalons
248–253 côté Kanban, plus plusieurs bugs de correction/accessibilité désormais atteignables.

## Correctifs

### 1. `widget_rect` retombe sur le registre réordonnable (**critique**)
`Ui::widget_rect` ne cherchait que dans `focusables`. Or les **cartes Kanban** (et les en-têtes
réordonnables mais **non triables**) ne sont pas focusables : `widget_rect` renvoyait `None`, donc
`paint_reorder_preview` **sortait aussitôt** (`let Some(src) = ui.widget_rect(id) else { return }`) — ni
fantôme, ni ligne d'insertion, ni réagencement vertical — **et** le routage *insert-after* retombait
sur `false`. Autrement dit, tout l'aperçu vertical (jalons 251–253) ne s'exécutait jamais à la souris.
`widget_rect` a désormais un **repli** sur le registre `reorderables` (dont les bornes existaient déjà).
Cela répare aussi le cas d'un en-tête de `Table` réordonnable **sans** tri.

### 2. Annonce du lecteur d'écran dépendante de l'axe
Le dépôt annonçait toujours `« Column moved to position {to+1} »`. Pour une carte, `to` est un index
**plat** (`col×STRIDE+pos`) → annonce absurde (« position 1001 ») et mauvais nom. Désormais :
horizontal → position de colonne (1-based) ; vertical → `« Card moved »` (sans numéro dénué de sens).

### 3. Dépôt d'une carte **sur elle-même**
Le garde `to != from` laissait passer un dépôt en **moitié basse** de la carte saisie (où `to = from+1`)
→ message de déplacement **nul** + annonce parasites (le reducer les annulait ensuite). Ajout d'un garde
`self_drop` (cible == source) qui neutralise le dépôt sur soi, quelle que soit la moitié.

### 4. Ressort horizontal borné à l'axe horizontal
Le ressort `reorder_x` (coulissement lissé des colonnes) était avancé pour **tout** glisser
réordonnable, y compris vertical où il est **inutilisé** — calcul mort, et `reorder_animating` scrutait
le mauvais axe. Il est désormais gardé à l'axe **horizontal** (nouvel accès `dragged_reorder_axis`).

### 5. La zone de dépôt n'est plus une **source** de glisser
`reorderable_at` (côté shell, à l'appui) démarrait un glisser sur n'importe quel réordonnable, y compris
une `DropZone` — on soulevait un fantôme vide qui ne déplaçait rien. Nouvelle méthode de trait
`reorder_draggable()` (défaut `true`) ; `DropZone` renvoie `false`. Le **dépôt** continue de la viser
(via `Ui::reorderable_at`), seule la **saisie** l'ignore.

### 6. Garde de débordement `STRIDE`
`kanban_slot(col, pos)` porte un `debug_assert!(pos < STRIDE)` : au-delà, `pos` déborderait sur le champ
colonne (l'index plat viserait silencieusement la colonne suivante).

## Vérification

- **Widgets 392** : `widget_rect` retombe sur le registre (carte retrouvée là où `focusables` échoue) ;
  cartes saisissables **et** zone de dépôt cible-seule (`reorder_draggable`).
- **Shell 27** ; **goldens 77 inchangés** (l'aperçu n'existe qu'en glisser) ; **démo (lib) 36** ;
  doctests 6.
- Les correctifs d'annonce/self-drop/ressort vivent dans le `pointer_up`/tick du shell (méthode à état),
  non isolables en test pur sans harnais complet ; leur logique est simple et documentée, et le pivot
  `widget_rect` (qui les débloque) est couvert.

## Notes

- Le rendu **live** du glisser reste non inspecté au GPU ici ; mais le bug critique n°1 explique
  pourquoi les jalons 251–253 ne pouvaient pas se voir en application — il est levé.
- La revue a aussi relevé des points de **consolidation/style** non traités ici (voir Reste).

## Reste

- **Style** : couleur d'ombre du fantôme (`Color::BLACK.fade`), décalage/flou et rayon d'insertion sont
  des littéraux dans la peinture DnD — à porter sur le thème (règle « customizable like Flutter »).
- **Consolidation** : factoriser les deux boucles de parcours quasi identiques d'`ui.rs`
  (`focusables/scrollables/draggables/reorderables/semantics`) et unifier `reflow_reorder_columns` /
  `reflow_reorder_cards` (même idée sur axes transposés).
- **Couverture** : test du réagencement **même-colonne** (chevauchement source/cible → décalage net nul),
  et harnais shell pour les branches de routage (`insert-after`, self-drop, annonce).
- Inertie/ressort **vertical** du coulissement (parité avec l'horizontal).

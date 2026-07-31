# Jalon 247 — Kanban : colonnes + cartes, glisser-déposer inter-colonnes

## Analyse

Un tableau **Kanban** (colonnes titrées de cartes, déplacées par glisser-déposer) est un patron
d'application courant. Le framework possède déjà un mécanisme de **réordonnancement par glisser**
(`reorder_index` / `on_reorder`) : le shell route `source.on_reorder(cible.reorder_index())` au dépôt,
**quels que soient** les deux widgets. Ce jalon l'exploite pour un déplacement de carte, sans nouveau
code de shell.

## Décisions techniques

- **Index plat d'emplacement.** Chaque emplacement `(col, pos)` porte un index plat
  `col * STRIDE + pos` ([`kanban_slot`]). C'est le `reorder_index` d'une carte — à la fois **source**
  (saisie) et **cible** (dépôt). La carte saisie **décode** l'index de la cible survolée pour émettre
  `on_move(from_col, from_pos, to_col, to_pos)`.

- **Zone de dépôt par colonne.** Une zone en bas de chaque colonne porte l'index `(col, nb_cartes)` :
  cible d'insertion **en fin**, et seule cible d'une colonne **vide**.

- **Contrôlé.** L'application tient les cartes par colonne et applique le déplacement (retrait +
  insertion, avec correction du décalage d'index dans une même colonne). Le widget ne fait que rendre
  et router.

- **Personnalisable.** Le fond de panneau d'une colonne est **thémé** (dérivé de `surface`/`on_surface`),
  pas codé en dur.

## Implémentation

- `frus-widgets/src/kanban.rs` (nouveau) : `Kanban::new(on_move).column(title, cards)` ; widgets
  internes `Card` (source+cible, `reorder_index`/`on_reorder`), `DropZone` (cible de fin), `Column`
  (panneau thémé) ; helper public [`kanban_slot`]. Tests : `slot_encoding_roundtrips`,
  `dropping_a_card_routes_a_cross_column_move` (une carte source expose son index plat et route le bon
  `Move` quand on la dépose sur un autre emplacement), `board_lays_out_one_widget_per_column`.
- `frus-widgets/src/lib.rs` : `mod kanban;` + export de `Kanban`/`kanban_slot`.
- `frus-demo/src/lib.rs` : route `Board` (+ plomberie tiroir/save-restore) ; `kanban: Option<Vec<Vec<
  String>>>` + helper `kanban_cols` ; `Msg::KanbanMove` + reducer (retrait/insertion, décalage géré) ;
  `board_screen` sur `Kanban::new(Msg::KanbanMove).column(...)`.

## Vérification

- **Widgets** : encodage/décodage d'emplacement ; le dépôt d'une carte route un `Move` inter-colonnes.
- **Golden** `kanban` : trois colonnes titrées (To do/Doing/Done), cartes en tuiles, zone de dépôt en
  bas de chaque colonne — inspecté visuellement.
- **Démo** `kanban_move_relocates_a_card` : déplacer une carte la retire de la source et l'insère dans
  la cible ; un déplacement intra-colonne réordonne sans dupliquer (décalage d'index géré).
- Widgets 384 ; goldens 76 ; démo 36 ; shell compile.

## Notes

- Le glisser-déposer réutilise l'aperçu de réordonnancement du shell (fantôme suivant le curseur),
  initialement conçu pour les colonnes de `Table` — l'affinage de cet aperçu pour des cartes
  verticales reste une amélioration possible.

## Reste

- Aperçu de dépôt dédié aux cartes (indicateur d'insertion vertical).
- Cartes **riches** (widgets) plutôt que texte ; ajout/suppression de carte.

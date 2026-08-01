# Jalon 253 — Décalage des cartes voisines à l'insertion verticale (le « trou »)

## Analyse

À l'horizontale (colonnes de `Table`), l'aperçu de réordonnancement **réagence** les voisines
(`reflow_reorder_columns`, jalon précédent) : le trou de la colonne soulevée se referme, la place de
dépôt s'ouvre, le tout **suivant le curseur**. À la **verticale** (cartes Kanban), l'aperçu ne posait
qu'une **ligne d'insertion** (jalons 248, 252) : les cartes restaient figées, sans « trou » sous la
barre. Ce jalon apporte le **pendant vertical** du réagencement.

## Décisions techniques

- **`reflow_reorder_cards`** dans `frus-widgets::reorder`, jumeau vertical de `reflow_reorder_columns`,
  **purement géométrique** (aucune connaissance de l'arbre) :
  - la **colonne source** (bande x de la carte soulevée) voit ses cartes situées **sous** la carte
    soulevée **remonter** d'un cran → le trou se referme ;
  - la **colonne cible** (bande x de la ligne d'insertion) voit ce qui est **au niveau/en dessous**
    de la ligne **descendre** d'un cran → la place s'ouvre.
- **Cran = hauteur de la carte** (`src.height`). Un bloc plus **haut** que `1.5×` ce cran est un fond
  de colonne/page (pas une carte) : laissé en place — pendant strict du garde `max_cell` horizontal.
- **Pas de cisaillement.** Chaque primitive coulisse selon le **centre** de ses bornes. Les lignes
  d'insertion se posant aux **bords** des cartes (jamais en plein centre), toutes les primitives d'une
  même carte tombent du même côté → la carte bouge d'un bloc (y compris une carte **riche**,
  multi-primitives).
- **Réutilise `owners`** (jalon 251) : le sous-arbre de la carte soulevée est retiré de l'aperçu (elle
  flotte déjà en fantôme).

## Implémentation

- `frus-widgets/src/reorder.rs` : `pub fn reflow_reorder_cards(prims, src, line, lifted)` (+ export).
  Tests : `lifting_a_card_closes_the_source_gap`, `insertion_line_opens_a_hole_in_the_target_column`
  (source **et** cible, colonnes distinctes), `tall_backgrounds_stay_put`.
- `frus-shell/src/app.rs` : la branche **verticale** de `paint_reorder_preview` réagence la scène via
  `reflow_reorder_cards` (comme la branche horizontale), puis pose la ligne d'insertion par-dessus. Le
  calcul de `owners` remonte avant le `match` (partagé fantôme + réagencement).

## Vérification

- **Widgets 391** (+3) : le réagencement est couvert par des tests **purs** — remontée en colonne
  source, ouverture du trou en colonne cible, immobilité des grands fonds.
- **Non-régression** : l'aperçu n'existe qu'en glisser (hors goldens) — **goldens 77 inchangés** ;
  démo 36 ; shell 27. L'axe **horizontal** est intact (branche séparée).

## Notes

- Le coulissement est **immédiat** (proportionnel à la position), sans le lissage à ressort de
  l'horizontal (`reorder_x`) — affinage possible (inertie verticale).
- Comme les jalons 250–252, le rendu **live** du glisser reste un état runtime non inspecté au GPU
  ici ; la vérification porte sur la géométrie pure du réagencement.

## Reste

- Inertie/ressort vertical du coulissement (parité avec l'horizontal).
- Consolidation/revue transverse du domaine glisser-déposer (Table + Kanban).

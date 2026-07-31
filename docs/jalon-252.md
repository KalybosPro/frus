# Jalon 252 — Indicateur d'insertion inter-cartes (moitié survolée)

## Analyse

L'aperçu de glisser **vertical** (jalon 248) posait toujours la ligne d'insertion sur le bord
**supérieur** de l'emplacement survolé, et le dépôt insérait **avant** cette cible. Impossible, donc,
de déposer une carte **entre** deux cartes en visant la seconde : viser une carte signifiait toujours
« juste au-dessus ». Comme la liste réordonnable de Flutter, l'intention doit dépendre de la **moitié**
survolée : moitié **haute** → insérer **avant** ; moitié **basse** → insérer **après**.

## Décisions techniques

- **Découpe au milieu (midpoint).** Pour un emplacement à réordonnancement **vertical**, le curseur
  au-dessus du milieu insère **avant** (bord supérieur), en dessous **après** (bord inférieur,
  index +1). Sur l'axe **horizontal** (colonnes de `Table`), aucun changement : la logique de dépôt
  reste identique.

- **Visuel et routage cohérents.** Le même prédicat (`reorder_insert_after`) pilote **à la fois** la
  ligne d'insertion peinte **et** l'index de dépôt effectif — la barre montre exactement où la carte
  atterrira. `to_pos` étant un **index d'insertion** (le reducer insère à cet index, avec l'ajustement
  −1 pour un déplacement vers l'aval dans la **même** colonne), « après » se traduit simplement par
  `+1`.

- **Zone de dépôt finale.** Sa moitié basse donne `slot(col, len) + 1`, borné par le reducer à `len` :
  l'insertion reste en fin de colonne (inoffensif).

## Implémentation

- `frus-shell/src/app.rs` :
  - `drop_insertion_line(target, thickness, after)` — bord **supérieur** (`after = false`) ou
    **inférieur** (`after = true`) de la cible.
  - `TodoApp::reorder_insert_after(target, rect)` — vrai si la cible est **verticale** et le curseur
    dans sa moitié basse (faux à l'horizontale ou hors cible).
  - `reorder_drop_line` peint la ligne au bord retenu ; le **dépôt** décale l'index effectif de `+1`
    quand `reorder_insert_after`.

## Vérification

- **Shell 27** : `insertion_line_sits_on_the_target_top_edge` (avant → bord haut) **et**
  `insertion_line_sits_on_the_target_bottom_edge_when_inserting_after` (après → bord bas, `y = bas −
  ép/2`).
- Sémantique d'insertion (`to_pos` = index, ajustement même-colonne) déjà couverte côté reducer
  (`kanban_move_relocates_a_card`).
- **Non-régression** : rendu statique inchangé (l'indicateur n'apparaît qu'en glisser) — goldens 77
  inchangés ; démo 36 ; widgets 388. Axe horizontal (Table) : `reorder_insert_after` renvoie toujours
  `false`, comportement de dépôt strictement identique.

## Notes

- La ligne d'insertion et la préhension restent des états **runtime** non inspectés au GPU dans cet
  environnement ; la géométrie de la barre (deux bords) et la sémantique d'index sont couvertes par
  des tests purs.

## Reste

- Décalage visuel des cartes voisines à l'insertion verticale (ouvrir un « trou » sous la barre),
  comme le `reflow` horizontal des colonnes.
- Nouveau domaine de widgets, ou consolidation/revue transverse.

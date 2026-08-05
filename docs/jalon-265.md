# Jalon 265 — Inertie verticale du glisser (ligne d'insertion à ressort)

## Objectif

Donner au réordonnancement **vertical** des cartes (Kanban) la même **inertie** que le
réordonnancement **horizontal** des colonnes (`Table`). Côté horizontal (jalons antérieurs), l'abscisse
lissée `reorder_x` rejoint le curseur par ressort et alimente le réagencement des colonnes
(`reflow_reorder_columns`) : les voisines *coulissent* au lieu de sauter. Côté vertical, la **ligne
d'insertion** et le **trou** *sautaient* d'un cran de carte à l'autre. Ce jalon leur donne le pendant :
une ordonnée lissée `reorder_y`.

## Approche

Un ressort `reorder_y` (mêmes constante de temps et fonction `spring_toward` que `reorder_x`) rejoint
non pas le curseur brut mais le **bord d'emplacement retenu** — l'ordonnée que calcule déjà
`reorder_drop_line` (bord supérieur/inférieur selon la moitié survolée, jalon 252). La ligne **snappe**
donc toujours à un cran valide, mais **glisse** entre crans au lieu de sauter.

Cette ordonnée lissée alimente **à la fois** l'indicateur peint **et** le réagencement des cartes
(`reflow_reorder_cards`) : ligne et trou bougent ensemble ; à mesure que `reorder_y` balaie l'écart,
les cartes intermédiaires basculent une à une (cascade), exact analog vertical du coulissement des
colonnes. Le **routage du dépôt** reste inchangé (fondé sur la position réelle du curseur via
`reorder_insert_after`) : le lissage n'anime que l'**approche**, et au repos `reorder_y == cible`.

## Implémentation (`frus-shell/src/app.rs`)

- Champ `reorder_y: f32` (init 0), posé à `cursor.y` au début du glisser (pas de ressaut initial).
- Boucle de frame : le calcul du ressort de réordonnancement passe d'un `if horizontal` à un `match`
  sur l'axe — **Horizontal** → `reorder_x` vers `cursor.x` (inchangé) ; **Vertical** → `reorder_y`
  vers `reorder_drop_line(...).y` (le bord retenu), animant tant que l'écart dépasse 0,5 px.
- `paint_reorder_preview` (branche verticale) : la ligne d'insertion est l'emplacement retenu dont on
  **remplace l'ordonnée** par `reorder_y` (`Rect { y: self.reorder_y, ..target }`), puis on la passe à
  `reflow_reorder_cards` **et** on la peint — ligne et trou glissent de concert.

## Vérification

- **Desktop** : compile ; shell **27** tests OK, dont `insertion_line_sits_on_the_target_top_edge`
  (logique de snap **inchangée** : `reorder_drop_line` n'est pas modifiée) et
  `spring_approaches_target_monotonically_and_settles` (le ressort converge, sans dépassement).
- **Appareil** : l'inertie effective (glisse de la ligne + cascade des cartes) est **runtime/GPU** —
  à confirmer au doigt sur le board.

## Reste

- Rien de bloquant. Réglage fin possible de la constante de temps (0,07 s) après retour d'usage.

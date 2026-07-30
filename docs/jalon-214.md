# Jalon 214 — Grille : cycle entre les fautes

## Analyse

Le jalon 210 menait à la **première** cellule fautive ; sur une grille qui en compte plusieurs,
l'utilisateur voulait ensuite passer à la **suivante**, et ainsi de suite. Le bouton devient un
**cycle** sur toutes les fautes.

## Décisions techniques

- **Cycle avec bouclage.** `Next error` (ex-`Go to first error`) focalise la faute **suivant** la
  dernière visée, en ordre ligne par ligne, et **reboucle** sur la première après la dernière. La
  position visée est mémorisée dans `grid_error_cursor`.

- **Une seule énumération des fautes.** `grid_faults` liste toutes les cellules invalides (ordre
  ligne par ligne) via `grid_cell_error` ; `grid_first_error` (jalon 210) et `grid_next_error` en
  dérivent — la règle de validité reste unique.

- **Retour visuel à l'arrivée.** `Command::focus` place le focus clavier sur la cellule visée :
  l'anneau de focus existant la met en évidence. (Un *halo bref* dédié demanderait une animation
  transitoire — laissé au Reste.)

## Implémentation

- `frus-demo/src/lib.rs` : champ `grid_error_cursor` ; `grid_faults` ; `grid_next_error`
  (bouclage) ; `grid_first_error` délègue ; `GridFocusError` cycle ; bouton renommé `Next error`.

## Vérification

- `grid_next_error_cycles_through_all_faults` : sur trois fautes `(0,0)`, `(0,2)`, `(1,2)`, quatre
  appels successifs donnent `(0,0) → (0,2) → (1,2) → (0,0)` (bouclage). Le test du jalon 210 reste
  vert (`grid_first_error` inchangé fonctionnellement).

## Reste

- **Halo bref** (pulse animé) sur la cellule à l'arrivée, **défiler** jusqu'à elle si la grille
  dépasse le viewport, et sauter la faute **courante** si on la corrige avant de cycler.

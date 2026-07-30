# Jalon 210 — Grille : Save désactivé + accès à la première faute

## Analyse

Le jalon 207 gardait la soumission dans le `reduce` (toast « Fix N errors »), mais le bouton `Save`
restait cliquable — on n'apprenait l'échec qu'après coup, sans savoir **où** corriger. Deux
améliorations : rendre l'invalide **inatteignable**, et offrir un **raccourci** vers la faute.

## Décisions techniques

- **`Save` désactivé quand invalide.** `button(...).enabled(errors == 0)` : le bouton est grisé et
  n'émet rien tant qu'une cellule est en faute. L'état d'invalidité devient visible *avant* le clic,
  pas après. La garde du `reduce` (jalon 207) reste en défense.

- **Raccourci vers la première faute.** Quand `errors > 0`, un bouton `Go to first error` apparaît ;
  `Msg::GridFocusError` focalise la première cellule invalide via `Command::focus(("grid", r, c))`
  — l'utilisateur est amené droit à la correction.

- **Une seule règle de faute.** `grid_first_error` réutilise `grid_cell_error` (jalons 204/207),
  parcouru ligne par ligne : même définition de validité partout.

## Implémentation

- `frus-demo/src/lib.rs` : `Msg::GridFocusError` ; `grid_first_error` ; arm `GridFocusError`
  (focus) ; `grid_screen` : `Save` conditionnellement activé + bouton `Go to first error` inséré
  dynamiquement (`Flex::child`) quand il y a des erreurs.

## Vérification

- `grid_focus_error_targets_the_first_faulty_cell` : `grid_first_error` pointe la première faute
  `(1, 0)` (Name vide), `GridFocusError` émet un focus ; une fois tout corrigé, plus de cible et
  aucune commande.

## Reste

- **Défiler** jusqu'à la cellule focalisée si la grille dépasse le viewport, cycler entre *toutes*
  les fautes (pas seulement la première), et un halo bref sur la cellule visée à l'arrivée.

# Jalon 207 — Grille : soumission gardée par la validation

## Analyse

Le jalon 204 signale les cellules invalides individuellement, mais rien n'empêchait de « soumettre »
une grille incohérente, et l'utilisateur n'avait pas de vue d'ensemble. Il fallait **agréger** l'état
de validation au niveau du tableau et **garder** la soumission.

## Décisions techniques

- **Un compteur pur.** `grid_error_count(grid)` somme les cellules invalides via le
  `grid_cell_error` du jalon 204 — une seule source de vérité pour la validation, réutilisée par la
  barre d'état et par la soumission.

- **Barre d'état vive.** Sous le tableau, `All cells valid` (accent) ou `N error(s)` (couleur
  d'erreur), recalculée à chaque frame depuis l'état — elle suit la saisie sans machinerie.

- **Soumission gardée.** `Msg::GridSave` n'aboutit (toast `Grid saved`) que si le compteur est nul ;
  sinon un toast `Fix N error(s) before saving` renvoie le décompte. La validation vit dans le
  `reduce` (testable), pas dans la vue.

## Implémentation

- `frus-demo/src/lib.rs` : `Msg::GridSave` ; `grid_error_count` ; arm `GridSave` (toast selon le
  compteur) ; `grid_screen` gagne une ligne d'actions `Add row` / `Save` / barre d'état.

## Vérification

- `grid_save_is_gated_on_cell_errors` : deux erreurs → `GridSave` bloque et compte
  (`Fix 2 errors before saving`) ; après correction, `grid_error_count == 0` et `GridSave` aboutit
  (`Grid saved`).

## Reste

- **Désactiver** visuellement le bouton `Save` tant qu'il y a des erreurs (plutôt que de le laisser
  cliquable et rapporter), **Échap** pour annuler l'édition d'une ligne (nécessite un instantané par
  ligne), et surligner la **première** cellule fautive au clic sur `Save`.

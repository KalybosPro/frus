# Jalon 204 — Grille : tri par en-tête + validation par cellule

## Analyse

Le jalon 201 a fait de la grille un vrai tableur clavier (cellules toujours éditables, Tab/Entrée,
ajout/suppression de lignes). Manquaient les deux gestes qu'attend tout tableur : **trier** en
cliquant un en-tête, et **signaler** une saisie invalide. La `Table` sait déjà émettre `on_sort` et
afficher une flèche de tri (`sorted`) ; il ne reste qu'à câbler la démo.

## Décisions techniques

- **Tri piloté par l'application.** `Table::on_sort(Msg::GridSort)` rend les en-têtes cliquables ;
  `reduce` bascule croissant/décroissant sur la colonne cliquée et **trie les lignes** (comparaison
  insensible à la casse). La flèche d'en-tête suit l'état via `.sorted(col, asc)`. La `Table` ne
  trie jamais elle-même — elle n'émet que la colonne (jalon 199).

- **Validation pure par cellule.** `grid_cell_error(col, value) -> Option<&str>` : `Name` (col 0)
  obligatoire, `Email` (col 2) doit contenir `@` et `.` une fois saisi. La cellule invalide passe
  par `TextInput::error(...)` (bordure + message, déjà au widget). Fonction pure, testable sans
  rendu.

- **Entrée sur la dernière ligne crée une ligne.** Prolonge le jalon 201 : `GridEnter` sur la
  dernière ligne pousse une ligne vide et y descend le focus, au lieu de rester en place — la saisie
  continue au clavier sans toucher la souris.

## Implémentation

- `frus-demo/src/lib.rs` : `Msg::GridSort(usize)` ; champ `grid_sort: Option<(usize, bool)>` ;
  arms `GridSort` (tri) et `GridEnter` (création en fin) ; `grid_cell_error` ; `grid_screen` câble
  `on_sort` + `sorted` + `error` par cellule ; indice mis à jour.

## Vérification

- `grid_edit_navigate_and_resize` : mis à jour — Entrée sur la dernière ligne **crée** une ligne.
- `grid_sort_toggles_and_validates` : tri col 0 croissant puis décroissant (ordre vérifié) ;
  `grid_cell_error` sur Name vide, email malformé, et les cas valides (email vide toléré).

## Reste

- Tri **numérique** pour les colonnes chiffrées (ici tout est texte), tri stable multi-colonnes,
  validation croisée entre lignes (unicité d'email), et blocage de la soumission tant qu'une cellule
  est en erreur.

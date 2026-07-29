# Jalon 197 — Grille éditable : câblage interactif

## Analyse

Le jalon 196 a prouvé (golden) que `Table` sait afficher un `TextInput` par cellule ; il restait à
le **câbler** en vrai : cliquer une cellule pour l'éditer, saisir, valider — un mini-tableur dans
le démo. C'est l'application concrète du motif d'édition en ligne.

## Décisions techniques

- **Une route dédiée, un état minimal.** `Route::Grid` (accessible depuis le tiroir) affiche une
  grille dont l'état tient en deux champs de `TodoApp` : `grid: Vec<Vec<String>>` (les données) et
  `grid_edit: Option<(ligne, colonne)>` (la cellule en édition). Données de démonstration semées
  dans `init`.

- **Échange de widget par cellule.** `grid_screen` construit une `Table::widget_row` par ligne ;
  chaque cellule est une **fabrique** qui rend, selon `grid_edit` :
  - au repos, un `Container` cliquable (`on_click` → `GridEdit(r, c)`) affichant la valeur ;
  - en édition, un `TextInput` lié (`on_input` → `GridInput`, `on_submit` → `GridCommit`).
  Aucun code de `Table` modifié : c'est la composition du jalon 196, pilotée par l'état.

- **Focus immédiat de la cellule (jalon 198).** `GridEdit` enveloppe le futur `TextInput` dans
  `keyed(("grid", r, c))` et renvoie `Command::focus(("grid", r, c))` : au prochain build, le
  curseur se pose **dans** la cellule cliquée — le clic ouvre et focalise d'un coup.

- **Cycle d'édition pur.** `reduce` gère `GridEdit` (ouvre + focalise), `GridInput` (met à jour la
  cellule ciblée), `GridCommit` (referme). Tout dérive de `grid` / `grid_edit`.

## Implémentation

- `frus-demo/src/lib.rs` : `Route::Grid` (+ `save_state`/`restore_state`) ; champs `grid` /
  `grid_edit` (semés dans `init`) ; `Msg::{GridEdit, GridInput, GridCommit}` (+ arms `reduce`) ;
  `grid_screen` ; entrée tiroir.

## Vérification

- **Intégration** (`grid_click_edit_commit`) : la grille se rend ; cliquer une cellule l'ouvre en
  édition **et** demande son focus (`!cmd.is_empty()`) ; la saisie met à jour la bonne cellule ;
  le commit referme ; les autres cellules restent intactes. Les 19 tests démo restent **verts**
  (20 au total).
- **Visuel** : identique au golden `table_editable` (jalon 196) — une cellule en `TextInput` parmi
  des cellules texte cliquables ; ce jalon en rend le comportement interactif.
- `cargo build -p frus-demo` **propre**.

## Reste

- **Navigation clavier** (Tab → cellule suivante, Entrée → ligne suivante) — chaîner les clés de
  focus `("grid", r, c)`.
- **Ajout / suppression de lignes**, tri, validation par cellule (`TextInput::error` + `Form`).

# Jalon 196 — Table : édition en ligne des cellules

## Analyse

`Table` savait afficher du texte, des widgets, des cases à cocher, geler des colonnes,
virtualiser… mais l'**édition en ligne** (cliquer une cellule pour la saisir, façon tableur)
n'avait jamais été démontrée. La question : faut-il un nouveau mécanisme, ou est-ce déjà
composable ?

## Décisions techniques

- **Aucun nouveau mécanisme — pure composition.** `Table::widget_row` accepte déjà une cellule
  comme **widget arbitraire** (fabrique `Fn() -> Box<dyn Widget>`). L'édition en ligne se réduit
  donc à un choix de widget par cellule, piloté par l'état applicatif :
  - cellule **au repos** : un [`Container`] cliquable (`on_click`) affichant la valeur — le clic
    émet « éditer la cellule (ligne, colonne) » ;
  - cellule **en édition** : un [`TextInput`] lié à la valeur (`on_input` → maj, `on_submit` →
    valider).
  L'application tient un `editing: Option<(row, col)>` et échange le widget de la cellule visée.
  Rien à ajouter au framework : `Container::on_click` + `TextInput` + `widget_row` suffisent.

- **Ce jalon est une preuve de capacité.** Il fixe le motif (et le verrouille par un golden)
  plutôt que d'ajouter du code : la flexibilité de `Table` (jalons data-table) rend l'édition en
  ligne « gratuite ». Le câblage interactif complet (état `editing`, commit/annulation) est un
  branchement applicatif direct de ce motif.

## Implémentation

- `goldens.rs` : `table_editable` — une grille 3 colonnes où toutes les cellules sont des
  `Container` cliquables **sauf** une, rendue par un `TextInput` (cellule en cours d'édition).

## Vérification

- **Golden** `table_editable` **inspecté** : la cellule « Cryptographer » (ligne 2, colonne
  Role) est un champ de saisie bordé ; toutes les autres sont du texte statique cliquable —
  l'édition en ligne se compose sans code framework.

## Reste

- **Câblage interactif dans le démo** : route/section « grille éditable » avec `editing:
  Option<(row, col)>`, `on_input`/`on_submit` reliés — application directe du motif.
- **Navigation clavier entre cellules** (Tab/Entrée pour passer à la cellule suivante) — au
  niveau applicatif, ou un futur mode « grille » intégré à `Table`.
- **Validation par cellule** (bordure d'erreur) — réutilise `TextInput::error` + `Form`.

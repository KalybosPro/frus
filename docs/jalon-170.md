# Jalon 170 — Tableau : widget d'action dans l'en-tête

## Analyse

Un en-tête savait porter un libellé et, depuis le jalon 168, une icône décorative — mais
tout l'en-tête n'était qu'**une seule zone cliquable** (le tri). Les grilles réelles posent
souvent un **bouton** dans l'en-tête (filtre, menu de colonne) qui doit réagir **pour
lui-même**, sans déclencher le tri. Il fallait un **widget d'action** en en-tête, cliquable
indépendamment, **tout en conservant** tri et réordonnancement sur le reste de la cellule.

## Décisions techniques

- **L'action est un *enfant* de la cellule, pas une superposition.** Plutôt qu'un calque
  flottant (qui aurait exigé de connaître les bords de colonnes, donc des largeurs fixes),
  le widget d'action devient un **enfant** de la `Cell` d'en-tête, posé à **droite**
  (`justify: End`). Le hit-test descend au **plus profond** : cliquer le bouton renvoie
  **son** message ; cliquer ailleurs dans l'en-tête renvoie celui de la cellule (le tri).
  Aucune géométrie de bords requise — marche pour toute largeur (fixe **ou** flexible).

- **Fabrique par colonne, rappelée à chaque reconstruction.** `Table::header_action(col,
  make)` stocke une fabrique `Fn() -> Box<dyn Widget>` (comme les cellules-widgets) :
  le tableau se rebâtissant à chaque réglage, elle produit un widget **frais**. Le libellé
  triable, l'icône et l'indicateur de tri restent peints à gauche ; l'action flotte à droite.

## Implémentation

- `table.rs` : champ `Cell.action: Vec<Box<dyn Widget>>` (0/1, exposé via `children`) ;
  `Cell::style` bascule en `justify: End` quand une action est présente ; champ
  `Table.header_actions` + builder `header_action(col, make)` ; câblage dans `rebuild`.
- `goldens.rs` : `table_header_action` (bouton « Filter » à droite de l'en-tête « Status »).

## Vérification

- **Unitaire** : `header_action_widget_captures_its_click` — la cellule d'en-tête porte
  l'action ; un clic sur le bouton renvoie **son** message (`Filter`), un clic ailleurs dans
  l'en-tête **trie** (`Sort(1)`).
- **Golden** `table_header_action` **inspecté** : bouton « Filter » à droite de l'en-tête,
  indicateur de tri ▲ conservé sur « Name » — aucune régression sur les autres goldens.
- `cargo test --workspace` **vert**.

## Reste

- **Focus clavier de l'action** : le bouton se clique à la souris ; l'atteindre au Tab
  suppose que le widget fourni soit `focusable` (le cas des boutons frus). RAS à faire côté
  tableau, à garder à l'esprit pour un menu déroulant piloté au clavier.
- **En-tête *entièrement* remplacé par un widget** (sans libellé texte du tout) : possible
  extension via une `widget_header`, si un cas concret le demande.

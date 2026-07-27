# Jalon 145 — Tableau : en-tête triable & lignes sélectionnables

## Analyse

Le `Table` existant (bâti sur `Grid`) affichait un en-tête stylé et des lignes de texte,
mais **statique** : aucun tri, aucune sélection, aucune interaction. Pour en faire la base
d'une application de gestion, il fallait :

- **En-tête triable** : cliquer une colonne émet un message ; un **indicateur** (triangle
  ▲/▼) marque la colonne triée et son sens.
- **Lignes sélectionnables** : cliquer une ligne émet un message ; les lignes
  sélectionnées sont **surlignées**.

## Décisions techniques

- **Le tableau n'ordonne rien.** Fidèle à l'architecture Elm de frus, il **émet** au clic
  (`on_sort(colonne)`, `on_select_row(ligne)`) et n'**affiche** que l'état qu'on lui passe
  (`sorted(col, asc)`, `selected(&rows)`). C'est l'application qui trie la donnée et
  renvoie l'état — le widget reste une fonction pure de ses entrées. API compatible :
  `header`/`row`/`width` inchangés.

- **Données d'abord, grille reconstruite.** Comme `children()` doit renvoyer un sous-arbre
  déjà bâti, le `Table` stocke désormais ses **données** (`headers`, `rows`) et son **état**
  (tri, sélection, rappels), et **régénère** la `Grid` (`rebuild`) après chaque réglage.
  L'ordre des appels du builder n'importe donc pas : l'état final est cohérent (p. ex.
  `on_select_row` posé après les `row`).

- **Interactivité au niveau cellule.** Une cellule qui renvoie `on_click(msg)` devient une
  cible de clic (rect peint = zone de clic) — sans focus clavier requis. Chaque cellule
  d'en-tête porte le message de tri de sa colonne, chaque cellule de donnée le message de
  sélection de sa ligne ; toutes les cellules d'une ligne sélectionnée partagent le fond
  surligné, donnant une ligne surlignée d'un bout à l'autre.

- **Indicateur de tri vectoriel.** Faute d'icône flèche haut/bas, le triangle est un
  petit `Path` (3 segments) rempli après le libellé de l'en-tête trié — net à toute
  échelle, sans dépendre de la police.

## Implémentation

- `table.rs` : `Cell<Msg>` gagne `selected`, `sort`, `message` (clic → tri/sélection,
  survol via couche d'état) ; `Table<Msg>` stocke données + état + rappels et `rebuild` la
  grille ; nouveaux constructeurs `on_sort`, `sorted`, `on_select_row`, `selected`.
- `goldens.rs` : golden `data_table` (en-tête trié + ligne surlignée).

## Vérification

- **Unitaire** : clic sur l'en-tête colonne 1 → `Sort(1)`, clic sur la 2e ligne →
  `Select(1)` (via `ui.hit` + `ui.msg_for`) ; l'indicateur de tri peint un `Path`, la
  ligne sélectionnée peint un rect teinté `primary`. L'ancien test (6 cellules) reste vert.
- **Golden** `data_table` rendu et **inspecté** : « Name ▲ » avec le triangle, ligne
  « Bob » surlignée. `cargo test --workspace` vert, aucun golden existant déplacé.

## Reste

- **Sélection multiple / tout sélectionner** (case d'en-tête), **cellules-widgets** (pas
  seulement du texte) et **largeurs de colonnes** variables (aujourd'hui égales via `Grid`).
- **Tri au clavier** (Entrée sur en-tête focalisé).

# Jalon 201 — Grille éditable : navigation clavier + lignes

## Analyse

Le jalon 197 câblait une grille **cliquer-pour-éditer** : une seule cellule en `TextInput` à la
fois. Pour un vrai mini-tableur, il manque le clavier (Tab de cellule en cellule, Entrée pour
descendre) et la gestion des lignes (ajouter / supprimer). Plutôt que d'empiler des raccourcis sur
le modèle « une cellule active », on adopte le modèle **tableur** : chaque cellule est **toujours
éditable**.

## Décisions techniques

- **Grille toujours éditable → Tab gratuit.** Chaque cellule est un `TextInput` `keyed(("grid", r,
  c))`. Le shell navigue déjà entre **focusables** avec Tab / Maj+Tab (jalon focus), en ordre de
  l'arbre — donc ligne par ligne, cellule par cellule. En rendant toutes les cellules focusables,
  **Tab devient la navigation de cellule** sans une ligne de code shell : on **compose** une brique
  existante. On supprime du même coup l'état `grid_edit` (plus de cellule « active » unique).

- **Entrée = descendre d'une ligne.** `on_submit` de chaque cellule émet `GridEnter(r, c)` ;
  `reduce` renvoie `Command::focus(("grid", r+1, c))` si la ligne suivante existe, sinon ne bouge
  pas. La saisie passe désormais les coordonnées : `on_input` émet `GridInput(r, c, valeur)`.

- **Ajouter / supprimer des lignes.** Un bouton « Add row » (`GridAddRow`) pousse une ligne vide et
  **focalise sa première cellule** ; chaque ligne porte, en dernière colonne, un bouton « ✕ »
  (`GridDeleteRow(r)`). Ce bouton est un `Container` **non focusable** (défaut du trait) : **Tab le
  saute**, la navigation reste de cellule à cellule.

## Implémentation

- `frus-demo/src/lib.rs` : `Msg::{GridInput(r,c,v), GridEnter(r,c), GridAddRow, GridDeleteRow(r)}`
  (remplacent `GridEdit/GridInput/GridCommit`) ; suppression du champ `grid_edit` ; `grid_screen`
  réécrit (cellules toujours éditables, colonne de suppression, bouton d'ajout, indice mis à jour).

## Vérification

- **Intégration** (`grid_edit_navigate_and_resize`) : saisir met à jour la bonne case ; `GridEnter`
  descend d'une ligne (focus demandé) et **reste** sur la dernière ; `GridAddRow` ajoute une ligne
  vide (bonnes colonnes) et focalise ; `GridDeleteRow` retire la ligne, les suivantes remontent.
- **Manuel** : dans la grille, Tab / Maj+Tab parcourent les cellules ; Entrée descend ; les boutons
  gèrent les lignes.

## Reste

- **Entrée sur la dernière ligne → créer une ligne** (au lieu de rester), navigation par flèches,
  tri des colonnes, validation par cellule (`TextInput::error` + `Form`).

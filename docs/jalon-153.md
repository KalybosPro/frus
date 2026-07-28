# Jalon 153 — Tableau : réordonnancement des colonnes (glisser un en-tête)

## Analyse

Après le redimensionnement (jalon 151), il manquait au tableau le **réordonnancement**
des colonnes — déplacer une colonne en glissant son en-tête, geste attendu de toute
grille de données.

Le nœud du problème : un en-tête doit rester **triable au clic** *et* devenir
**réordonnable au glissé**. Or, contrairement aux poignées de redimensionnement (widget
séparé), c'est le **même** widget qui doit distinguer les deux gestes. Le shell savait
déjà le faire pour le pan / défilement tactile (seuil `TOUCH_SLOP`, drapeau `moved`,
`was_tap`) mais pas pour un glissement de widget.

## Décisions techniques

- **Tap-vs-drag par seuil, réutilisé.** Nouveau `Drag::Reorder { from, start, moved }` :
  l'appui sur un en-tête réordonnable l'arme **sans** engager le glissement ; en deçà du
  seuil `TOUCH_SLOP`, le relâchement reste un **tri** (`was_tap`) ; au-delà, c'est un
  **réordonnancement** et le clic est supprimé (comme tout glissement). Aucune logique
  neuve : on calque exactement le modèle `moved` du pan.

- **Colonne cible = table de hit-test, pas de nouveau registre.** Deux méthodes de trait :
  `reorder_index()` (cet en-tête est réordonnable → sa colonne) et `on_reorder(to)`
  (l'en-tête **source** connaît son index et le rappel). Au dépôt, le shell résout la
  colonne **cible** en relisant `reorder_index()` de l'en-tête sous le curseur — via la
  table de hit-test **existante** (les en-têtes triables sont déjà cliquables). Zéro
  nouvelle collecte à la construction de l'UI.

- **Contrôlé.** `on_reorder(from, to)` : l'application permute l'ordre de ses colonnes et
  reconstruit. Le tableau ne stocke aucun ordre « vivant ».

- **Pas de rendu fantôme (MVP).** La colonne dépose « sec » ; l'aperçu glissant
  (proxy semi-transparent, décalage des voisines) est laissé au reste — le geste et le
  routage sont en place.

## Implémentation

- `widget.rs` : `reorder_index` / `on_reorder` (défaut `None`) + relais `Box` ;
  `keyed.rs`, `responsive.rs` : relais.
- `app.rs` (shell) : `Drag::Reorder` ; `reorderable_at` (hit-test → en-tête → colonne) ;
  armement à l'appui (sans `return`, pour garder le tap = tri) ; suivi `moved` au seuil ;
  au relâchement, colonne cible sous le curseur → `on_reorder(from, to)` routé, sauf
  `to == from`.
- `table.rs` : `Cell` gagne `reorder: Option<(usize, Rc<…>)>` + `reorder_index`/`on_reorder`
  (en-têtes seulement) ; champ `on_reorder` (`Rc`) + `.on_reorder()`.

## Vérification

- **Unitaire** : chaque en-tête expose sa colonne (`reorder_index` = 0, 2…) et produit
  `Reorder(from, to)` ; le **clic trie toujours** (`on_click` = `Sort`) ; les cellules de
  **données** ne sont pas réordonnables. Tri / sélection / redimensionnement inchangés.
- **Non couvert par test unitaire** : le geste bout-en-bout (appui → seuil → dépôt) vit
  dans le shell, sans harnais d'événements pointeur (fenêtre réelle requise) ; il réplique
  fidèlement le modèle `moved` / `was_tap` du pan, déjà éprouvé.
- `cargo test --workspace` **vert**.

## Reste

- **Aperçu glissant** : proxy semi-transparent de l'en-tête saisi + décalage animé des
  colonnes voisines + surbrillance de la zone de dépôt (façon `ReorderableListView`).
- **Réordonnancement au clavier** (Ctrl+Flèches sur un en-tête focalisé).

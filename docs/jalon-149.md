# Jalon 149 — Tableau : « tout cocher » indéterminé & tri au clavier

## Analyse

La sélection multiple (jalon 148) avait deux manques par rapport à Material :

- Le « **tout cocher** » n'affichait que coché / décoché — jamais l'état **indéterminé**
  (quand *certaines* lignes seulement sont cochées).
- Le tri n'était accessible qu'à la **souris** ; au clavier, on ne pouvait ni atteindre ni
  activer un en-tête.

## Décisions techniques

- **Tri-état de la case « tout cocher ».** `CheckCell` gagne un drapeau `indeterminate` ;
  le tableau le calcule (`some_selected` = au moins une ligne cochée mais pas toutes). Rendu
  façon Material : case pleine `primary` barrée d'un **tiret** (au lieu de la coche). Ordre
  d'affichage : coché > indéterminé > décoché.

- **Tri au clavier « gratuit ».** Le shell active déjà tout widget **focusable** portant un
  `on_click` sur Entrée/Espace (jalon boutons). Il suffit donc de rendre **focusables** les
  cellules qui doivent l'être : les **en-têtes triables** (`header && message`) et les
  **cases à cocher** — pas les cellules de données (elles restent cliquables souris sans
  encombrer l'ordre de tabulation). Le focus clavier trie / coche sans aucune logique
  nouvelle, et l'anneau de focus est dessiné automatiquement.

- **Rappel de layout.** Les colonnes flexibles n'ont de largeur que si le tableau a une
  **largeur** (`width`) : sans contrainte, une rangée `Flex` en largeur automatique réduit
  ses cellules flexibles à zéro (et rien n'est alors focusable/cliquable). Documenté par un
  test qui fixe la largeur.

## Implémentation

- `table.rs` : `CheckCell` gagne `indeterminate` (rendu tiret) + `focusable` ; `Cell`
  gagne `focusable` (en-têtes triables) ; helper `some_selected` ; l'en-tête passe
  l'indéterminé.
- `goldens.rs` : `data_table_multiselect` régénéré (« tout cocher » indéterminé).

## Vérification

- **Unitaire** : `all_selected`/`some_selected` — rien coché `(false,false)`, partiel
  `(false,true)`, tout `(true,false)` ; seuls les **2 en-têtes** entrent dans le cycle Tab
  (cellules de données exclues) ; les tests de clic/tri/sélection restent verts.
- **Golden** : `data_table_multiselect` **inspecté** — case « tout cocher » barrée (tiret)
  en sélection partielle. `cargo test --workspace` vert.

## Reste

- **Redimensionnement de colonnes** à la souris (poignées entre en-têtes) : demande un
  état de glissement dédié dans le shell (façon barre de défilement) + un rappel
  `on_resize(colonne, largeur)` — un jalon à part entière.
- **Cellules-widgets** (au-delà du texte).

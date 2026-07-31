# Jalon 243 — DataTable : barre d'actions groupées

## Analyse

Une fois la sélection multiple en place (jalon 241), l'usage attendu est d'**agir** sur les lignes
cochées : supprimer, exporter, déplacer… Les tableaux Material affichent alors une **barre d'actions
contextuelle** — « N selected » et les boutons d'action — qui n'apparaît que lorsqu'une sélection
existe. Ce jalon l'ajoute au `DataTable`, en laissant l'application fournir les boutons (slot).

## Décisions techniques

- **`bulk_actions(make)`.** Une **fabrique** de widgets d'action (rappelée à la reconstruction, comme
  les actions d'en-tête du `Table`) : l'application construit ses [`Button`](crate::Button) avec les
  variantes et messages voulus. Le widget ne fige aucun style d'action — il ne fournit que
  l'**emplacement** et le compteur.

- **Visible seulement avec une sélection.** La barre est rendue **au-dessus** du tableau (sous le champ
  de recherche s'il existe) uniquement si [`selected`](DataTable::selected) est non vide ; sinon, rien.
  Le libellé « N selected » compte les lignes sélectionnées (toutes pages confondues).

- **Modèle contrôlé, actions honnêtes.** Les messages émis par les boutons sont traités par l'app.
  Dans la démo, `Delete` **supprime réellement** les lignes cochées : les données du tableau passent
  dans l'état (`data_rows`, `None` = jeu de départ) et sont modifiées, la sélection et le focus étant
  remis à zéro.

## Implémentation

- `frus-widgets/src/datatable.rs` : champ `bulk_actions` + builder ; `rebuild` préfixe le bloc d'une
  barre `Flex` (« N selected » + spacer + widgets d'action) quand une sélection existe ; test
  `bulk_actions_bar_shows_only_with_a_selection` (une action sentinelle apparaît avec sélection,
  disparaît sans).
- `frus-demo/src/lib.rs` : `data_rows: Option<Vec<…>>` + helper `TodoApp::data_rows` ; `Msg::{
  DataClearChecked, DataDeleteChecked}` (Clear vide la sélection ; Delete retire les lignes cochées,
  en index décroissant, puis remet sélection/focus à zéro) ; `DataCheckAll` compte sur les lignes
  courantes ; `data_screen` câble `.bulk_actions(|| [Clear, Delete])`.

## Vérification

- **Widgets** `bulk_actions_bar…` : barre absente sans sélection, présente (action émettable) dès
  qu'une ligne est sélectionnée.
- **Golden** `data_table_bulk_actions` : deux lignes cochées → « 2 selected » + `Clear` (secondaire) et
  `Delete` (danger) au-dessus du tableau — inspecté visuellement.
- **Démo** `data_table_screen_…` étendu : Clear vide la sélection sans toucher le focus ; Delete retire
  la ligne cochée (12 → 11) et remet sélection/focus à zéro.
- Widgets 379 ; goldens 73 ; démo 34 ; shell compile.

## Reste

- **État vide** : message « No results » quand le filtre/les données vident le tableau — jalon 244.
- Confirmation avant `Delete` (dialogue) dans la démo.

# Jalon 244 — DataTable : état vide (« No results »)

## Analyse

Avec la recherche (jalon 242) et le Delete groupé (jalon 243), le tableau peut se retrouver **sans
aucune ligne à afficher** : filtre trop restrictif, ou toutes les lignes supprimées. Un corps vide
surmonté d'un pied « 0 of 0 » est déroutant ; les tableaux soignés affichent un **message d'état
vide**. Ce jalon l'ajoute au `DataTable`.

## Décisions techniques

- **Détection en un point.** Le pipeline d'index (`sorted_order` → filtre → tri → page) donne déjà le
  **total** de lignes visibles. Quand il vaut `0`, `rebuild` bascule sur la disposition « vide » :
  l'**en-tête** (les colonnes restent lisibles) surmonte un **message centré**, et le **pied de
  pagination est retiré** (un pager sur zéro ligne n'apporte rien).

- **Message surchargeable.** Défaut **« No results »** ; `empty_text(...)` permet un texte adapté
  (« No people match your search ») — cohérent avec la ligne de conduite « personnalisable comme
  Flutter » (défaut thémé, surcharge libre).

- **Automatique.** Aucune API supplémentaire à câbler côté application : dès que les données/filtre
  ne montrent rien, l'état vide apparaît.

## Implémentation

- `frus-widgets/src/datatable.rs` : champ `empty_text` (défaut « No results ») + builder ; branche
  `total == 0` dans `rebuild` (en-tête + message centré, sans pied) ; test
  `empty_filter_drops_rows_and_pager` (filtre « zzz » → aucune ligne cliquable **et** pager retiré —
  sinon son unique bouton de page émettrait un message).
- `frus-demo/src/lib.rs` : `data_screen` surcharge `.empty_text("No people match your search")` ;
  test étendu (un filtre sans résultat se rend quand même).

## Vérification

- **Widgets** `empty_filter_drops_rows_and_pager` : filtre sans résultat → ni ligne ni pager, arbre
  non vide (en-tête + message).
- **Golden** `data_table_empty` : champ « zzz », en-tête conservé, « No people match your search »
  centré, aucun pied — inspecté visuellement.
- **Démo** `data_table_screen_…` étendu : un filtre sans résultat se rend (état vide).
- Widgets 380 ; goldens 74 ; démo 34 ; shell compile.

## Reste

- Confirmation avant `Delete` (dialogue) dans la démo.
- Un nouveau domaine de widgets (`Tabs` avancé, `Tree` view, `Kanban`).

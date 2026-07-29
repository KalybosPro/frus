# Jalon 177 — Tableau virtualisé : sélection multiple

## Analyse

La virtualisation (jalons 173/176) excluait les **cases à cocher** : une grande grille
virtualisée ne pouvait pas offrir de sélection multiple. C'est pourtant l'usage type d'un
tableau de milliers de lignes (tout cocher, cocher une plage). Il fallait la colonne de
cases **dans le mode virtualisé**, en gardant le « tout cocher » de l'en-tête épinglé.

## Décisions techniques

- **Case par ligne dans la fabrique virtualisée.** Quand `checkboxes` est actif, la fabrique
  de rangée de la `List` **préfixe** une `CheckCell` (comme les rangées matérialisées), alignée
  sur la case « tout cocher » de l'en-tête épinglé. `on_check` passe de `Box` à `Rc` pour être
  capturé dans la fabrique `'static`.

- **État « tout cocher » basé sur le compte effectif.** `all_selected` / `some_selected`
  comptaient `self.rows` — **vide** en virtualisé, donc l'en-tête montrait « décoché » à tort.
  Corrigé : un `row_count()` (compte **virtualisé** s'il existe) et un `selected_count()`
  **O(sélection)** (indices valides uniques) — l'indéterminé s'affiche correctement même sur
  des millions de lignes, sans balayer toute la plage.

## Implémentation

- `table.rs` : `on_check` en `Rc` ; `CheckCell` préfixée dans la fabrique virtualisée ;
  helpers `row_count` / `selected_count` ; `all_selected` / `some_selected` réécrits ; docs des
  builders virtualisés mises à jour (cases à cocher désormais prises en charge).
- `goldens.rs` : `table_virtual_checkboxes` (colonne de cases + « tout cocher » indéterminé).

## Vérification

- **Unitaire** : `virtual_table_supports_checkboxes` — « tout cocher » dans l'en-tête épinglé
  émet `CheckAll` ; la case d'une ligne visible émet `Check(i)`. `select_all_is_indeterminate…`
  (matériel) reste vert (comportement inchangé).
- **Golden** `table_virtual_checkboxes` **inspecté** : cases par ligne, deux lignes cochées,
  « tout cocher » en **indéterminé** (dash) — aucune régression sur les 32 autres goldens.
- `cargo test --workspace` **vert**.

## Reste

- **Hauteur de ligne variable en virtualisé** : la `List` reste à hauteur fixe (`ROW_H`) —
  la hauteur adaptative (jalon 166) ne s'y applique pas ; une `List` à hauteurs par index
  (sommes préfixes) serait un jalon dédié.
- **Colonnes gelées / défilement horizontal** : nécessite un viewport horizontal et une
  colonne épinglée — restructuration de mise en page, jalon dédié.

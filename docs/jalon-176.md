# Jalon 176 — Tableau virtualisé : rangées-widgets

## Analyse

La virtualisation (jalon 173) était **texte** : `virtual_rows` fournit des chaînes par
colonne. Or les grandes grilles réelles affichent aussi des **widgets** par ligne (avatars,
puces d'état, boutons) — qu'on veut virtualiser tout autant (des milliers de lignes riches
sans tout construire). Il fallait la variante **widget**.

## Décisions techniques

- **Fabrique de ligne unifiée (`VirtualBuild`).** Le mode virtualisé porte désormais un
  `enum VirtualBuild { Text(Rc<Fn(usize) -> Vec<String>>), Widgets(Rc<Fn(usize) ->
  Vec<Box<dyn Widget>>>) }`. La fabrique de la `List` **matche** dessus par index :
  texte → rangée de `Cell` ; widgets → rangée de `WidgetCell`. Une seule branche de
  virtualisation dans `rebuild`, deux entrées publiques.

- **`virtual_widget_rows` en miroir.** `Table::virtual_widget_rows(count, viewport_height,
  build)` où `build(index) -> Vec<Box<dyn Widget>>` (un widget par colonne). Seules les
  lignes visibles sont construites ; sélection au clic (la cellule capte le clic sous un
  contenu non cliquable) ; en-tête épinglé. Mêmes exclusions que le mode texte (cases à
  cocher / redimensionnement / réordonnancement).

## Implémentation

- `table.rs` : `enum VirtualBuild` ; `virtual_data` porte un `VirtualBuild` ; `virtual_rows`
  enveloppe `Text`, nouveau `virtual_widget_rows` enveloppe `Widgets` ; la fabrique de la
  `List` matche le type et bâtit `Cell` ou `WidgetCell`.
- `goldens.rs` : `table_virtual_widgets` (500 lignes d'avatars + puces).

## Vérification

- **Unitaire** : `virtual_widget_rows_builds_only_visible` — sur 3000 lignes, **< 20**
  construites ; en-tête « Item » épinglé + widget « W0 » peints ; ligne-widget **cliquable**.
- **Golden** `table_virtual_widgets` **inspecté** : en-tête épinglé, avatars + puces (« tag 1
  »…« tag 4 »), ascenseur fin (500 lignes) — aucune régression sur les 31 autres goldens.
- `cargo test --workspace` **vert**.

## Reste

- **Hauteur de ligne variable** : la `List` reste à hauteur fixe (`ROW_H`) ; un widget plus
  haut serait rogné en virtualisé (la hauteur adaptative du jalon 166 ne s'y applique pas).
- **Cases à cocher virtualisées** : la colonne de sélection multiple pourrait s'ajouter à la
  fabrique de rangée si un cas le demande.

# Jalon 173 — Tableau : lignes virtualisées

## Analyse

Le tableau construisait **toutes** ses lignes à chaque reconstruction (une `Flex` de
rangées). Pour un journal ou un export de milliers de lignes, c'est un coût par frame
proportionnel au **total** — inacceptable. Le framework possède déjà une primitive de
virtualisation (`List` : hauteur d'élément fixe, défilement vertical, éléments construits à
la demande). Il fallait l'**appliquer au tableau**, en gardant l'en-tête épinglé.

## Décisions techniques

- **En-tête épinglé + `List` de rangées.** En mode virtualisé, la racine devient une `Flex`
  colonne `[en-tête, List(count, ROW_H, build_row)]` : l'en-tête reste fixe, la `List`
  virtualise les données (coût par frame ∝ lignes **visibles**). Aucune nouvelle machinerie
  de défilement — on réutilise `List`.

- **Fabrique de rangée capturée, `'static`.** La closure de la `List` ne peut pas emprunter
  `self` (elle lui survit). On **capture des clones** des paramètres nécessaires (colonnes,
  largeurs, largeur totale, ensemble sélectionné, `on_select` — passé en `Rc`) et la fabrique
  de contenu de l'app (`index -> Vec<String>`). Elle bâtit une rangée de `Cell` alignée sur
  les mêmes colonnes que l'en-tête.

- **Périmètre v1 assumé : texte.** `virtual_rows(count, viewport_height, build)` fournit des
  **textes** par ligne. La **sélection** (clic) marche sur les lignes visibles. Cases à
  cocher / redimensionnement / réordonnancement / cellules-widgets ne se combinent pas à la
  virtualisation (le hors-écran n'a pas d'état retenu) — ignorés en mode virtualisé, documenté.

## Implémentation

- `table.rs` : `on_select` passe de `Box` à `Rc` (partage dans la closure) ; champ
  `virtual_data` + builder `virtual_rows` ; branche virtualisée dans `rebuild` (en-tête +
  `List`) ; helper libre `col_dimension` partagé par la voie directe et virtualisée.
- `goldens.rs` : `table_virtualized` (1000 lignes, en-tête épinglé + fenêtre visible).

## Vérification

- **Unitaire** : `virtual_table_builds_only_visible_rows` — sur 5000 lignes, **< 20**
  construites (la fenêtre visible, pas 5000) ; en-tête « Name » épinglé + « R0 » peints ;
  borne de défilement = `5000 × ROW_H − viewport` ; une ligne visible reste **cliquable**.
- **Golden** `table_virtualized` **inspecté** : en-tête épinglé, lignes 1..4 visibles,
  ascenseur fin (beaucoup de contenu) — aucune régression.
- `cargo test --workspace` **vert**.

## Reste

- **Rangées-widgets virtualisées** : v1 est texte ; une variante `virtual_widget_rows`
  (fabrique `index -> Vec<widget>`) suivrait le même schéma.
- **Hauteur de ligne variable** : `List` v1 est à hauteur fixe (`ROW_H`) — la hauteur
  adaptative (jalon 166) ne s'applique pas en virtualisé.
- **Épingler l'en-tête pendant le défilement horizontal** et colonnes gelées : extensions
  possibles.

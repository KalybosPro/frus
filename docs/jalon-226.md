# Jalon 226 — Pourcentages dans l'infobulle en mode 100 %

## Analyse

L'empilage 100 % (jalon 224) montre des **proportions** : les strates remplissent toute la hauteur et
l'axe est en pourcentages. Mais l'infobulle de survol continuait d'afficher les **valeurs brutes** —
incohérent avec ce que le graphique met en avant, et sans donner la part exacte survolée.

## Décisions techniques

- **`format_measure(value, percent_of)` partagé.** Une fonction libre formate une mesure d'infobulle :
  la valeur brute seule (`None`), ou `valeur (part%)` quand un dénominateur 100 % est fourni. Les deux
  graphiques l'utilisent — la valeur reste visible, la part est ajoutée entre parenthèses.

- **Dénominateur = total de la catégorie survolée.** Dans chaque infobulle, `percent_of` vaut
  `Some(category_total(hi))` en mode 100 % (respecte `hidden`, borné), `None` sinon. La part affichée
  est donc bien relative à la colonne/catégorie sous le pointeur, cohérente avec les strates.

- **Aucun impact hors survol.** Le changement est confiné au chemin infobulle (`status.hover_cursor`) :
  les goldens (rendus sans survol) sont inchangés.

## Implémentation

- `frus-widgets/src/chart.rs` : fonction `format_measure` ; les infobulles de `BarChart` et
  `LineChart` calculent `percent_of` selon `normalized` et formatent chaque mesure via `format_measure`.

## Vérification

- **Widget** `normalized_bar_tooltip_shows_percentages` : survol de la catégorie A (deux séries à 2)
  → l'infobulle contient `(50%)` en 100 %, aucun `%` en absolu.
- **Widget** `normalized_line_tooltip_shows_percentages` : idem pour les aires empilées.
- Widgets 359 ; goldens 63 inchangés (chemin infobulle hors rendu golden).

## Reste

- Sortir du domaine graphes (nouveau widget : `Calendar`/`DataTable` avancé).
- Un **libellé de part** directement sur les strates (dans la barre/bande) en mode 100 %.

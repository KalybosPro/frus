# Jalon 212 — BarChart au niveau de LineChart : groupées + légende + infobulle

## Analyse

`LineChart` a gagné les séries multiples (209), la légende (209) et l'infobulle de survol (211).
`BarChart` était resté mono-série. Ce jalon lui donne les mêmes capacités — barres **groupées**,
légende, infobulle — en **factorisant** ce qui est commun aux deux graphiques.

## Décisions techniques

- **Modèle multi-séries identique à LineChart (jalon 209).** `.name` / `.series(nom, couleur,
  valeurs)` / `.legend(bool)` ; `max_value` sur toutes les séries. Rétro-compatible : sans `.series`,
  rendu du jalon 199.

- **Barres groupées.** Par catégorie, un **groupe** centré de `s` barres côte à côte (largeur du
  groupe = `slot * BAR_FILL`, divisée en `s`). Les libellés de valeur ne s'affichent qu'en série
  unique (surcharge évitée) ; les libellés de catégorie une fois.

- **Trois helpers partagés, zéro duplication.** L'ajout de la légende et de l'infobulle à BarChart a
  été l'occasion de sortir `draw_legend`, `draw_tooltip` et `chart_plot_hit` en fonctions libres,
  utilisées par **les deux** graphiques. `LineChart` a été recâblé dessus (comportement inchangé,
  ses goldens et tests le confirment).

- **Infobulle et suivi réutilisent le jalon 208/211.** `BarChart::cursor_icon` active `hover_cursor`
  sur la zone de tracé via `chart_plot_hit` ; `paint` liste la valeur de chaque série à la catégorie
  survolée via `draw_tooltip`.

## Implémentation

- `frus-widgets/src/chart.rs` : helpers `draw_legend` / `draw_tooltip` / `chart_plot_hit` ;
  `LineChart` recâblé dessus ; `BarChart` gagne `name` / `extra` / `legend`, `max_value` /
  `has_legend`, paint groupé + légende + infobulle, et `cursor_icon`.
- `frus-test/tests/goldens.rs` : golden `bar_chart_grouped` (2 séries + axe + légende).

## Vérification

- **Unitaire** `grouped_series_draw_a_bar_per_series_and_a_legend` (6 barres = 3×2, 2 pastilles,
  noms en légende) ; `hovering_bars_shows_a_tooltip_guide` (guide au survol, `cursor_icon` Some sur
  la zone). Les tests LineChart existants passent (refactor sans régression).
- **Golden** `bar_chart_grouped` ; `line_chart_multi` et les autres inchangés.

## Reste

- Barres **empilées** (cumul par catégorie), infobulle suivant la **barre exacte** sous le pointeur
  (pas seulement la catégorie), et couleur de série additionnelle issue d'une palette par défaut si
  omise.

# Jalon 224 — Empilage 100 % (proportions normalisées)

## Analyse

L'empilage (jalons 213/216) montre des **valeurs absolues** cumulées : la hauteur d'une colonne
dépend de son total, ce qui écrase la lecture des **proportions** quand les totaux varient beaucoup.
Le graphique « 100 % » (barres ou aires) répond à une autre question — *quelle part chaque série
prend-elle dans chaque catégorie ?* — en normalisant chaque catégorie à son propre total.

## Décisions techniques

- **`.normalized(bool)` sur les deux graphiques.** N'a d'effet qu'en mode empilé multi-séries
  (`normalized = self.normalized && stacked`). Additif : par défaut `false`, le paint est **inchangé**
  (goldens saufs).

- **Dénominateur par catégorie.** `category_total(i)` somme les séries **visibles** de la catégorie
  `i` (respecte `hidden`, borné à `1e-6` pour éviter la division par zéro). En 100 %, chaque strate
  est tracée sur `valeur / category_total(i)` au lieu de `valeur / échelle_globale` : chaque colonne
  (barres) ou chaque catégorie (aires) remplit alors **toute la hauteur**.

- **Axe en pourcentages.** `draw_grid` gagne un paramètre `percent` : en 100 %, les graduations
  affichent `0%..100%` au lieu des valeurs. Partagé par les deux graphiques.

- **Bascule dans l'app.** Un `Switch` « 100% stacking » (visible seulement pour les types empilés)
  pilote `chart_normalized` ; `dashboard_chart` passe `.normalized(app.chart_normalized)` aux branches
  empilées (aires empilées, barres empilées).

## Implémentation

- `frus-widgets/src/chart.rs` : champ `normalized` + builder `.normalized` + `category_total` sur
  `BarChart` et `LineChart` ; `draw_grid` gagne `percent` ; la branche empilée de chaque paint
  utilise le dénominateur par catégorie (`denom` pour les barres, closure `spt` pour les aires).
- `frus-demo/src/lib.rs` : état `chart_normalized`, `Msg::SetChartNormalized`, `reduce`, `Switch` dans
  `charts_screen`, `.normalized(...)` sur les deux branches empilées de `dashboard_chart`.

## Vérification

- **Widget** `normalized_stacked_bars_fill_each_column` : la colonne A (total 5) est **pleine** en
  100 % mais partielle en absolu (le max, 8, est en B) ; l'axe affiche `100%`.
- **Widget** `normalized_stacked_areas_fill_to_the_top` : le trait du bord supérieur est **plat** à
  100 % (plot_top) partout en normalisé, mais suit les totaux en absolu.
- **Démo** `normalized_toggle_applies_to_stacked_kinds` : la bascule met `chart_normalized`, les deux
  types empilés se rendent.
- **Goldens** `bar_chart_normalized` + `line_chart_normalized` (63 au total) : colonnes/aires pleines,
  axe en pourcentages.
- Widgets 357, démo 31, shell 25 ; suite verte.

## Reste

- Un **désépinglage** (re-clic sur l'élément sélectionné pour effacer `chart_sel`/`chart_pin`).
- Étiquettes de **pourcentage** dans l'infobulle en mode 100 % (aujourd'hui : valeurs brutes).

# Jalon 222 — Clic sur une barre : détail épinglé (BarChart::on_point)

## Analyse

Le jalon 221 a rendu les **points** d'une `LineChart` cliquables (`on_point(cat, série)`), mais les
`BarChart` restaient inertes : un tableau de bord qui bascule en « barres groupées » ou « barres
empilées » perdait l'interaction. Ce jalon donne aux barres la **parité** avec les points de ligne.

## Décisions techniques

- **`BarChart::on_point(cat, série)`.** Symétrique de `LineChart::on_point`. Après le test de
  légende, `positional_click` reconstruit la géométrie du paint et cherche le **rectangle** (barre
  groupée ou strate empilée) qui contient le point local ; renvoie `on_point(catégorie, série)`.

- **Hit-test des deux dispositions.** En **groupé**, chaque série `j` occupe une sous-barre
  `[bx, bx + draw_w]` (avec le facteur `inner = 0.86` et le décalage `(bar_w - draw_w)/2`, identiques
  au paint) de hauteur `(valeur/max)·plot_h`. En **empilé**, chaque strate est un segment pleine
  largeur `[sbx, sbx + group_w]` entre son cumul bas et haut. Une série **masquée** ne compte pas
  (barre non tracée = non cliquable), exactement comme au paint.

- **L'app réutilise `Msg::ChartPoint`.** Le message et le formatage `série · catégorie = valeur`
  (jalon 221) sont indépendants de la famille : brancher `.on_point(Msg::ChartPoint)` sur la
  `BarChart` du tableau de bord suffit à épingler le détail au clic d'une barre.

## Implémentation

- `frus-widgets/src/chart.rs` : `BarChart` gagne le champ `on_point` + le builder `.on_point` ;
  `positional_click` passe de « légende seule » à « légende **puis** barres » (structure identique à
  `LineChart`), avec le hit-test groupé/empilé et le respect de `hidden`.
- `frus-demo/src/lib.rs` : la branche `BarChart` de `dashboard_chart` câble
  `.on_point(Msg::ChartPoint)` quand la légende est active (graphique principal).

## Vérification

- **Widget** `clicking_a_bar_emits_category_and_series` : clic au centre de la 2e barre de la
  catégorie A (série additionnelle) → `(0, 1)` ; au-dessus de la barre → `None` ; en empilé, clic bas
  de colonne → strate `0` ; barre d'une série **masquée** → `None`.
- **Démo** `grouped_bars_are_clickable_in_dashboard` : le graphique principal en barres groupées
  émet `ChartPoint` sur au moins un point de sa zone (balayage, indépendant des constantes internes).
- Widgets 353, démo 29 ; goldens sans régression (changement additif, paint inchangé).

## Reste

- Une barre/point **épinglé mis en évidence** (anneau persistant) dans le graphique — jalon 223.
- Normaliser l'empilage en **100 %** (proportions plutôt que valeurs absolues).

# Jalon 200 — Charts : graphique en lignes (LineChart)

## Analyse

Le jalon 199 a ouvert le domaine « graphes » avec la [`BarChart`] : idéale pour **comparer** des
grandeurs. Pour lire une **tendance** (une série dans le temps), la forme naturelle est la
**polyligne** — des points reliés par des segments. C'est le deuxième widget du domaine, et le
premier consommateur widget de `Scene::stroke_path` (contour de chemin, sans remplissage).

## Décisions techniques

- **Même géométrie que la BarChart.** `LineChart` réutilise à l'identique la mise en page : bande
  des valeurs en haut, libellés de catégorie sous la ligne de base, échelle `0..max`. Un point par
  catégorie, centré dans sa « case », de hauteur proportionnelle à la valeur. On lit donc une
  BarChart et une LineChart de la même série **au même endroit**.

- **Trait vectoriel plutôt que rectangles.** La courbe est un `Path` (un `move_to` puis des
  `line_to`) rendu par `scene.stroke_path` — le premier usage côté widgets du **contour** de chemin.
  Chaque point porte un **marqueur** rond (`Path::circle` rempli) pour rester lisible même à plat.

- **Auto-peint, non générique, thémé (façon BarChart / Icon).** Aucun enfant, pas de `Msg` : c'est
  une **vue** de données. `color` surcharge le trait (défaut `primary`), `height` la hauteur
  (défaut 200) ; `width: Percent(1.0)` — le parent doit donc être **dimensionné**.

## Implémentation

- `frus-widgets/src/chart.rs` : `LineChart` (`new`, `color`, `height`) ; `paint` calcule les points,
  trace la polyligne (`stroke_path`), pose les marqueurs (`fill_path` de cercles), les valeurs et les
  libellés. Constantes `MARKER_R`, `LINE_W` ; réutilise `format_value` et la géométrie de la BarChart.
- `frus-widgets/src/lib.rs` : export `LineChart`.
- `frus-test/tests/goldens.rs` : golden `line_chart` (même série que `bar_chart`).

## Vérification

- **Unitaire** (`line_empty_series_paints_nothing`, `line_connects_all_points`) : série vide → rien ;
  trois points → une polyligne tracée (chemin `stroke: Some, fill: None`) de **deux** segments, un
  marqueur rempli par point, valeurs et libellés dessinés.
- **Golden** `line_chart` : la série `Mon..Fri` en courbe, marqueurs, valeurs, ligne de base.

## Reste

- **Axe des ordonnées** (graduations + grille horizontale), aire remplie sous la courbe, séries
  multiples (légende), survol d'un point → infobulle.

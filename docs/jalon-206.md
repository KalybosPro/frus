# Jalon 206 — Charts : aire remplie sous la courbe

## Analyse

La `LineChart` (jalon 200) trace la **tendance** ; pour souligner le **volume** (cumul, part), les
graphiques Flutter/Recharts remplissent l'aire entre la courbe et la ligne de base. C'est un ajout
naturel, réutilisant le remplissage de chemin non-zero déjà en place.

## Décisions techniques

- **Opt-in, un polygone fermé.** `.area(bool)` (défaut `false`). Quand actif, on construit un chemin
  `Path` : depuis la ligne de base sous le premier point, on relie tous les points de la courbe, puis
  on redescend à la ligne de base sous le dernier point. Le remplissage non-zero referme
  automatiquement le contour.

- **Peint dessous.** L'aire est remplie **avant** la polyligne et les marqueurs (couleur du trait
  fortement atténuée, `AREA_ALPHA = 0.16`), donc le trait reste net par-dessus.

- **Se compose avec l'axe.** L'aire et l'axe des ordonnées (jalon 203) sont indépendants : le golden
  combine `.grid(4).area(true)`.

## Implémentation

- `frus-widgets/src/chart.rs` : constante `AREA_ALPHA` ; champ `fill: bool` + `.area(bool)` ;
  remplissage du polygone avant le trait dans le paint de `LineChart`.
- `frus-test/tests/goldens.rs` : golden `line_chart_area` (`grid(4)` + `area(true)`).

## Vérification

- **Unitaire** `area_fills_a_polygon_under_the_curve` : avec `.area(true)`, exactement **un** chemin
  rempli fait de segments droits (`LineTo`) — l'aire — ; sans, **zéro** (seuls les marqueurs, des
  cercles, sont remplis).
- **Golden** `line_chart_area` : aire translucide sous la courbe, trait et marqueurs au-dessus.

## Reste

- **Séries multiples** + légende (empilées ou superposées), dégradé vertical de l'aire (plutôt
  qu'aplat), et interpolation lissée (courbes de Bézier) entre points.

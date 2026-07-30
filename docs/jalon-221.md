# Jalon 221 — Clic sur un point de graphique → détail épinglé

## Analyse

La légende cliquable (jalon 215) route déjà un clic de sous-région vers un message. Le geste
suivant attendu d'un tableau de bord : **cliquer un point** de la courbe pour épingler sa valeur.
Le hit-test d'un point exige la **hauteur** de la boîte — que `positional_click(local_x, local_y,
width)` ne fournissait pas.

## Décisions techniques

- **`positional_click` gagne `height`.** La signature du trait passe à `(local_x, local_y, width,
  height)` — le shell passe déjà `rect`, il transmet aussi `rect.height`. Changement mécanique
  propagé à tous les widgets (`TextInput`, `keyed`, `responsive`, défaut) ; seuls les graphiques
  s'en servent. Débloque tout hit-test de sous-région dépendant de la géométrie verticale.

- **`LineChart::on_point(cat, série)`.** Après le test de légende, `positional_click` reconstruit la
  géométrie du paint et cherche un **marqueur** (des séries **visibles**) dans un rayon
  `POINT_HIT_R`. Hors mode empilé, où les marqueurs individuels n'existent pas. Renvoie
  `on_point(catégorie, série)`.

- **L'app épingle le détail.** `Msg::ChartPoint(cat, série)` formate `série · catégorie = valeur`
  depuis les données partagées (`CHART_SERIES` / `CHART_CATS`) dans `chart_pin`, affiché en `Chip`
  sous le graphique. Les barres restent non cliquables (segments : voir Reste).

## Implémentation

- `frus-widgets` : `positional_click` gagne `height` (trait + `TextInput` / `keyed` / `responsive`
  / `Box`) ; `LineChart` gagne `on_point` + hit-test des points ; `POINT_HIT_R`.
- `frus-shell/src/app.rs` : passe `rect.height` à `positional_click`.
- `frus-demo/src/lib.rs` : `Msg::ChartPoint` + état `chart_pin` + `reduce` (formatage) ; câblage
  `.on_point(Msg::ChartPoint)` sur le graphique principal ; `Chip` d'épingle dans `charts_screen`.

## Vérification

- **Widget** `clicking_a_point_emits_category_and_series` : clic sur le point A de la série
  principale → `(0, 0)` ; loin d'un marqueur → `None` ; point d'une série **masquée** → `None`.
- **Démo** `clicking_a_point_pins_its_detail` : `ChartPoint(3, 0)` → `Sales · Thu = 8`, remplacé par
  `ChartPoint(1, 1)` → `Costs · Tue = 4`. Widgets 352, démo 28, shell 25 ; goldens sans régression.

## Reste

- Clic sur un **segment** de barre (`BarChart::on_point`, hit-test des rectangles) et sur une strate
  empilée, et un point **épinglé mis en évidence** (halo persistant) dans le graphique.

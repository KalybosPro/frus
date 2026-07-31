# Jalon 223 — Point/barre épinglé mis en évidence (halo + anneau persistants)

## Analyse

Les jalons 221/222 rendent points et barres **cliquables** et épinglent le détail dans un `Chip`.
Mais rien, dans le graphique, ne montrait **quel** élément était la source de l'épingle : l'accent au
survol (jalon 211/217) disparaît dès que le pointeur quitte la zone. Il manquait une mise en évidence
**persistante** de la sélection courante.

## Décisions techniques

- **`.selected(Option<(catégorie, série)>)` sur les deux graphiques.** Signature en `Option` pour
  brancher directement l'état de l'app (`Option<(usize, usize)>`). `None` = rien mis en évidence.
  Champ purement additif : par défaut `None`, le paint est **inchangé** (goldens saufs).

- **`LineChart` : halo + anneau sur le marqueur.** Après le tracé des séries (donc au-dessus), si un
  point est sélectionné — hors mode empilé, hors série masquée —, on pose un halo translucide
  (`MARKER_R + 6`, `α·0.22`) puis un anneau plein (`MARKER_R + 3`, 2 px) dans la couleur de la série.
  Indépendant du survol : la mise en évidence reste tant que la sélection tient.

- **`BarChart` : anneau contrasté autour de la barre.** Le rectangle de la barre/strate sélectionnée
  est **capturé** pendant la boucle de paint (même géométrie que le tracé), puis un anneau dilaté de
  2,5 px, à bordure 2 px en `on_surface` (couleur contrastée, lisible sur toute barre colorée), est
  tracé après coup. Fonctionne en groupé **et** en empilé.

- **L'app retient la sélection.** `Msg::ChartPoint(cat, série)` pose désormais aussi
  `chart_sel = Some((cat, série))` ; `dashboard_chart` passe `.selected(app.chart_sel)` au graphique
  **principal** (lignes ou barres). Cliquer un point/barre l'entoure aussitôt.

## Implémentation

- `frus-widgets/src/chart.rs` : champ `selected` + builder `.selected` sur `BarChart` et `LineChart` ;
  `BarChart::paint` capture `sel_rect` et trace l'anneau ; `LineChart::paint` trace halo + anneau sur
  le marqueur sélectionné.
- `frus-demo/src/lib.rs` : état `chart_sel` ; `reduce(ChartPoint)` le renseigne ; les deux branches de
  `dashboard_chart` câblent `.selected(app.chart_sel)`.

## Vérification

- **Widget** `selected_bar_draws_a_persistent_ring` : la barre épinglée ajoute un rectangle à bordure
  (0 sans sélection, 1 avec) ; une série masquée épinglée n'en ajoute pas.
- **Widget** `selected_point_draws_a_persistent_ring` : le point épinglé ajoute un cercle **contour**
  (0 sans sélection, 1 avec) ; série masquée épinglée : aucun.
- **Démo** `clicking_a_point_marks_it_selected` : `ChartPoint(3, 0)` → `chart_sel = Some((3, 0))`,
  suit le dernier clic.
- **Goldens** `line_chart_selected` + `bar_chart_selected` (61 au total) : halo/anneau visibles.
- Widgets 355, démo 30, shell 25 ; suite verte.

## Reste

- Normaliser l'empilage en **100 %** (proportions plutôt que valeurs absolues).
- Un **désépinglage** (re-clic sur l'élément sélectionné pour effacer `chart_sel`/`chart_pin`).

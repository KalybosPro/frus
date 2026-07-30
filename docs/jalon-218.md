# Jalon 218 — Démo : écran « Charts » à légende cliquable

## Analyse

Les jalons 209–217 ont fait des graphiques des widgets riches et interactifs, mais **aucun écran de
démo** ne les exerçait — la légende cliquable (jalon 215) n'était couverte que par un test unitaire.
Ce jalon ajoute un écran qui **boucle la boucle** : un vrai `LineChart` dans l'app, dont la légende
pilote l'état.

## Décisions techniques

- **Un nouvel écran routé.** `Route::Charts` rejoint la navigation (tiroir, raccourci `5`,
  save/restore d'état). `charts_screen` rend un `LineChart` à trois séries (`Sales` / `Costs` /
  `Profit`), avec axe (`grid(4)`), légende, et halo animé au survol (jalon 217).

- **La légende pilote l'état.** `.on_legend(Msg::ChartToggleSeries)` route le clic d'entrée vers le
  `reduce`, qui **bascule** l'index dans `chart_hidden` ; `.hidden(app.chart_hidden.clone())` reflète
  l'état au tracé. Aucun code shell : tout passe par `positional_click` (jalon 215) et la boucle
  Elm de l'app.

- **Rien de neuf côté widget.** L'écran n'assemble que des capacités existantes — la valeur est
  l'**intégration** de bout en bout (clic de sous-région → message → état → re-rendu).

## Implémentation

- `frus-demo/src/lib.rs` : `Route::Charts` (+ index actif, save/restore, entrée de tiroir) ;
  `Msg::ChartToggleSeries(usize)` ; état `chart_hidden: Vec<usize>` ; `reduce` (bascule) ;
  `charts_screen` ; import `LineChart`.

## Vérification

- `chart_legend_toggle_hides_and_shows_series` : l'écran se rend (`primitive_count > 0`) ; un clic
  masque la série (`chart_hidden == [1]`), un autre en masque une seconde (`[1, 2]`), un re-clic
  ré-affiche la première (`[2]`). Suite démo verte ; workspace sans régression.

## Reste

- Un **sélecteur de type** (lignes / barres / empilé) sur l'écran, un clic sur un **point** pour
  épingler son détail, et un second graphique (`BarChart`) partageant l'état de visibilité.

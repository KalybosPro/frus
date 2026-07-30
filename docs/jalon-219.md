# Jalon 219 — Démo Charts : sélecteur de type

## Analyse

L'écran Charts (jalon 218) n'affichait qu'un type de graphique. Un vrai tableau de bord laisse
**choisir** la présentation. Toutes les briques existent (lignes, aires empilées, barres groupées,
barres empilées) — il reste à les exposer derrière un sélecteur.

## Décisions techniques

- **Un `SegmentedControl` pilote le type.** État `chart_kind: usize` (0 lignes, 1 aires empilées,
  2 barres groupées, 3 barres empilées) ; `Msg::SetChartKind` le change. Le sélecteur réutilise le
  widget existant (même patron que le filtre de tâches).

- **Un seul constructeur, quatre variantes.** `dashboard_chart(app, height, legend)` bâtit le
  graphique selon `chart_kind` : `LineChart` (avec `.stacked` pour l'aire empilée) ou `BarChart`
  (avec `.stacked` pour les barres empilées). Toutes les variantes partagent **les mêmes données**
  (`CHART_CATS` / `CHART_SERIES` / `CHART_COLORS`), l'axe, et l'état `chart_hidden` — changer de type
  ne perd pas les séries masquées. Le paramètre `legend` prépare un graphique **compagnon** (jalon
  220).

## Implémentation

- `frus-demo/src/lib.rs` : constantes de données `CHART_CATS` / `CHART_SERIES` / `CHART_COLORS` ;
  état `chart_kind` ; `Msg::SetChartKind` + `reduce` ; `dashboard_chart` ; `charts_screen` gagne le
  `SegmentedControl`.

## Vérification

- `chart_kind_selector_switches_type_and_each_renders` : type par défaut (lignes) ; chaque type
  (aires empilées, barres groupées, barres empilées) se sélectionne et **se rend**
  (`primitive_count > 0`), exerçant les deux branches de `dashboard_chart`. Démo 26/26.

## Reste

- Persister `chart_kind` dans `save_state`, et un second graphique **compagnon** partageant la
  visibilité (jalon 220).

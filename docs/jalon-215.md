# Jalon 215 — Charts : légende cliquable + séries masquables

## Analyse

La légende (jalon 209/212) était décorative. Un tableau de bord réel laisse **cliquer** une entrée
pour **masquer/afficher** sa série. Émettre un message au clic impose que les graphiques deviennent
**génériques sur `Msg`** — la brique qui ouvre toute interaction future (clic sur barre/point).

## Décisions techniques

- **Graphiques génériques.** `BarChart` → `BarChart<Msg = ()>`, idem `LineChart`. Le paramètre par
  défaut `()` garde toutes les constructions existantes inférables (goldens, doctests, tests) sans
  annotation. Seul endroit d'usage hors du crate : les goldens — vérifiés inchangés.

- **Clic de légende routé comme une sous-région.** `positional_click` (le mécanisme du suffixe de
  `TextInput`, jalons 198/205) reconstruit la disposition de la légende via `legend_hit` et renvoie
  `on_legend(index)`. Le shell le route déjà — aucun code shell.

- **Masquage côté données.** `.hidden([indices])` retire les séries du tracé (courbes, aires,
  barres, bandes empilées, infobulle) tout en gardant leur entrée de légende **atténuée** (pour
  pouvoir les réafficher). L'application toggle `hidden` en réponse à `on_legend`.

- **Échelle stable.** `max_value` reste calculé sur **toutes** les séries : masquer/afficher ne fait
  pas sauter l'axe.

## Implémentation

- `frus-widgets/src/chart.rs` : helper partagé `legend_hit` ; `BarChart` et `LineChart` génériques
  avec champs `hidden` / `on_legend`, builders `.hidden` / `.on_legend`, `series_names`, saut des
  séries masquées dans tous les chemins de tracé, atténuation en légende, et `positional_click`.
- `frus-test/tests/goldens.rs` : golden `line_chart_hidden`.

## Vérification

- **Unitaire** `legend_click_emits_the_series_index` (clic sur l'entrée *i* → `on_legend(i)`, hors
  bande / sans `on_legend` → `None`) ; `hidden_series_is_not_drawn` (une série masquée = une ligne en
  moins). Les 16 tests chart et les 58 goldens passent.
- **Golden** `line_chart_hidden` : « Costs » masqué (non tracé, atténué en légende).

## Reste

- **Démo** : un écran graphiques exerçant le toggle de légende (le domaine n'a pas encore d'écran de
  démo). Clic sur une **barre/un point** (même `positional_click`) pour un détail, et sélection
  multiple de séries.

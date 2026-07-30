# Jalon 216 — BarChart : barres empilées

## Analyse

Le jalon 212 a donné les barres **groupées** (comparaison côte à côte). Comme la LineChart empilée
(jalon 213), la BarChart doit aussi savoir **empiler** — une seule barre par catégorie, segmentée par
série, pour lire un total et sa composition.

## Décisions techniques

- **`.stacked(bool)`.** Actif avec plusieurs séries, chaque catégorie n'a plus qu'**une** barre
  (largeur du groupe), segmentée du bas vers le haut : chaque série est un rectangle entre son cumul
  bas et son cumul haut, dans sa couleur (segments à angles droits, radius 0, pour un empilement net).

- **Échelle au total.** `max` = `stacked_max` = le maximum de la **somme** des séries par catégorie
  — l'axe (jalon 203) contient toute la pile. Miroir exact de `LineChart::stacked_max` (jalon 213).

- **Compose avec le reste.** Les séries **masquées** (jalon 215) ne comptent pas dans la pile ;
  légende et infobulle (valeur propre de chaque série) fonctionnent à l'identique. Sans `.stacked`,
  le rendu groupé (jalon 212) est inchangé, série unique comprise.

## Implémentation

- `frus-widgets/src/chart.rs` : champ `stacked` + `.stacked(bool)` sur `BarChart` ; `stacked_max` ;
  branche empilée dans le paint (segments cumulés par catégorie) vs branche groupée.

## Vérification

- **Unitaire** `stacked_bars_share_one_column_per_category` : `stacked_max = max(2+3, 4+1) = 5` ;
  quatre segments (2 catégories × 2 séries) tous à la **pleine largeur** du groupe (empilés dans une
  colonne, vs groupés côte à côte). Le golden `bar_chart_grouped` reste inchangé.
- **Golden** `bar_chart_stacked` : une barre segmentée par catégorie, échelle au total.

## Reste

- Empilage **normalisé** (100 %), coin arrondi sur le **segment supérieur** seulement, et infobulle
  suivant le **segment exact** sous le pointeur (pas seulement la catégorie).

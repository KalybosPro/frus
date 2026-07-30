# Jalon 213 — LineChart : aires empilées

## Analyse

Le jalon 209 superpose les séries (comparaison) ; pour lire un **total** et sa **composition** (part
de chaque série dans la somme), il faut les **empiler** — les aires cumulées, classiques d'un
graphe de répartition dans le temps.

## Décisions techniques

- **`.stacked(bool)`.** Actif avec plusieurs séries, chaque série devient une **bande** entre son
  cumul bas et son cumul haut ; les bandes s'ajoutent du bas vers le haut, le trait suit le bord
  supérieur. Implique le remplissage (opacité `STACK_ALPHA`, plus soutenue que l'aire simple pour
  distinguer les strates).

- **Échelle au total.** En empilé, `max` = `stacked_max` = le maximum de la **somme** des séries par
  catégorie — l'axe (jalon 203) contient donc la pile entière.

- **Infobulle cohérente.** Au survol, l'infobulle liste la valeur **propre** de chaque série (pas le
  cumul) ; les marqueurs accentués sont omis en empilé (une hauteur individuelle n'a pas de sens sur
  une strate cumulée), le guide et la boîte restent.

- **Sans régression.** Sans `.stacked`, le chemin de rendu est celui des jalons 200/206/209
  (superposition), inchangé.

## Implémentation

- `frus-widgets/src/chart.rs` : champ `stacked` + `.stacked(bool)` ; `stacked_max` ; branche
  empilée dans le paint de `LineChart` (bandes cumulées + trait supérieur) ; garde des marqueurs
  d'infobulle ; constante `STACK_ALPHA`.
- `frus-test/tests/goldens.rs` : golden `line_chart_stacked`.

## Vérification

- **Unitaire** `stacked_areas_fill_a_band_per_series` : `stacked_max` = `max(2+3, 4+1) = 5` ; deux
  bandes remplies (chemins pleins avec segments) en empilé, zéro sans.
- **Golden** `line_chart_stacked` : deux bandes cumulées, échelle au total, légende.

## Reste

- Empilage **normalisé** (100 %, parts relatives), aires empilées **lissées** (Bézier), et la même
  option pour la **BarChart** (barres empilées vs groupées du jalon 212).

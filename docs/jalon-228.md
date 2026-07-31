# Jalon 228 — Total au sommet des colonnes empilées absolues

## Analyse

Une `BarChart` à série unique écrit la **valeur** au-dessus de chaque barre (lecture immédiate). Les
barres **empilées absolues** n'avaient aucun repère chiffré : on voyait la composition mais pas le
total de la colonne. Ce jalon rétablit la **parité** — le total au sommet de chaque colonne.

Réservé au mode **absolu** : en 100 % (jalon 224) la colonne est pleine par construction et déjà
étiquetée strate par strate (jalon 227), un total « 100 % » n'apporterait rien.

## Décisions techniques

- **Total = cumul des séries visibles.** À la fin de la boucle des strates, `lower` vaut déjà la
  somme des séries **visibles** de la catégorie (les masquées sont ignorées) : on l'écrit centrée
  au-dessus de la strate supérieure, à `top_y - VALUE_SIZE - 2`, exactement comme la valeur d'une
  barre simple (même taille `VALUE_SIZE`, même couleur `on_surface`, même décalage).

- **Rien si la colonne est vide.** `lower > 0.0` évite un « 0 » flottant sur une catégorie sans
  données visibles.

## Implémentation

- `frus-widgets/src/chart.rs` : dans la branche empilée de `BarChart::paint`, après les strates et
  seulement si `!normalized`, écriture du total de la colonne au-dessus.

## Vérification

- **Widget** `stacked_absolute_bars_show_the_column_total` : deux colonnes de total 5 → le texte `5`
  apparaît **2** fois en absolu ; en 100 %, aucun total brut (parts en `%`).
- **Golden** `bar_chart_stacked` régénéré : totaux (5, 12, 11, 12, 7) au sommet de chaque colonne.
- Widgets 361 ; goldens 63.

## Reste

- Sortir du domaine graphes (nouveau widget : `Calendar`/`DataTable` avancé).
- Valeur par **strate** (dans le segment) en empilé absolu, comme le `%` en 100 %.

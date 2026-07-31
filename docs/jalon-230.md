# Jalon 230 — Valeur/part dans chaque bande (aires empilées)

## Analyse

Les jalons 227/229 étiquettent chaque strate d'une `BarChart` empilée (part `%` en 100 %, valeur en
absolu). Les **aires empilées** (`LineChart`) n'avaient ces chiffres qu'au survol (jalon 226). Ce
jalon porte la même parité aux bandes : à chaque catégorie, la valeur (ou la part) au centre de la
bande, si elle y est assez épaisse.

## Décisions techniques

- **Une bande = des strates par catégorie.** Une aire empilée est continue, mais sa « strate » à la
  catégorie `i` est l'épaisseur entre `lower[i]` et `upper[i]`. On y écrit un libellé centré
  (horizontalement sur le point, verticalement au milieu de la bande) — même logique et même seuil
  (`STRATA_LABEL_SIZE + 4`) que les barres, même couleur `on_primary`.

- **Contenu selon le mode.** La part (`%` de `category_total(i)`) en 100 %, la valeur brute en
  absolu. Cohérent avec le libellé de strate des barres.

- **Bornage horizontal.** Contrairement aux barres (insérées via `BAR_FILL`), les sommets d'aire
  tombent aux bords de la zone ; le `x` du libellé est borné à `[plot_left, plot_left + plot_w - lw]`
  pour ne pas déborder aux catégories de bord.

- **Sans clic ni survol.** Rendu statique, en plus de l'infobulle (jalon 226) qui reste disponible.

## Implémentation

- `frus-widgets/src/chart.rs` : dans la branche empilée de `LineChart::paint`, après le tracé de
  chaque bande, boucle sur les catégories pour écrire la valeur/part au centre des segments assez
  épais (borné horizontalement).

## Vérification

- **Widget** `stacked_areas_label_each_band_with_value_or_percentage` : en absolu, les valeurs de
  bandes (`3`, `4`, `5`, `6`) sont présentes ; en 100 %, des libellés en `%` apparaissent.
- **Goldens** `line_chart_stacked` (valeurs) et `line_chart_normalized` (parts %) régénérés.
- Widgets 363, démo 32 ; goldens 63.

## Reste

- Sortir du domaine graphes (nouveau widget : `Calendar`/`DataTable` avancé).
- Réglage d'opacité/densité des libellés quand les catégories sont nombreuses (éviter la surcharge).

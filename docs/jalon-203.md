# Jalon 203 — Charts : axe des ordonnées + grille (partagé)

## Analyse

Les jalons 199/200 ont donné les barres et la courbe, mais sans **repère de lecture** : impossible
d'estimer une valeur sans son étiquette. Un axe des ordonnées (graduations + lignes de grille
horizontales) répond à ça, et doit être **commun** aux deux graphiques — ils partagent déjà leur
géométrie.

## Décisions techniques

- **Un axe partagé, opt-in.** Une fonction libre `draw_grid(...)` trace `divisions` lignes
  horizontales réparties entre la ligne de base et le haut de la zone de tracé, chacune étiquetée de
  sa valeur (`0..max`) alignée à droite dans une marge de gauche. `BarChart` et `LineChart` gagnent
  le même `.grid(divisions)` (défaut `0` = aucun axe) et appellent `draw_grid` avant de peindre.

- **Non-cassant.** Sans `.grid(...)`, `axis_width` renvoie `0`, la zone de tracé reste pleine
  largeur et le rendu est **identique** aux jalons 199/200 (leurs goldens sont inchangés). Avec un
  axe, une marge `Y_AXIS_W` décale barres et points vers la droite pour loger les graduations.

- **La grille se lit derrière.** Lignes de grille en `theme.border` atténué, graduations en
  `theme.muted` : présentes sans masquer les données (façon Flutter).

## Implémentation

- `frus-widgets/src/chart.rs` : constantes `Y_AXIS_W`, `AXIS_SIZE` ; fonctions libres `axis_width`
  et `draw_grid` ; champ `grid: usize` + `.grid(n)` sur `BarChart` **et** `LineChart` ; paints
  décalés de la marge d'axe (`plot_left`, `plot_w`).
- `frus-test/tests/goldens.rs` : golden `line_chart_axis` (`grid(4)`).

## Vérification

- **Unitaire** (`grid_draws_horizontal_lines_and_axis_labels`, `no_grid_by_default_keeps_full_width`)
  : avec `grid(4)`, au moins 5 lignes fines (4 grille + base) et les graduations `0` et `8` sont
  dessinées ; sans grille, aucune graduation.
- **Golden** `line_chart_axis` : la série `Mon..Fri` avec grille horizontale et échelle à gauche.

## Reste

- **Aire remplie** sous la courbe, **séries multiples** + légende, graduations « rondes » (pas
  d'échelle jolis multiples), axe des abscisses libellé indépendant du nombre de points.

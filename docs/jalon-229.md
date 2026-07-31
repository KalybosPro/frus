# Jalon 229 — Valeur dans chaque strate (barres empilées absolues)

## Analyse

Le jalon 227 écrit la part (`%`) dans chaque strate en mode 100 % ; le jalon 228 le total au sommet
d'une colonne empilée absolue. Manquait le pendant naturel : la **valeur** de chaque strate, à
l'intérieur du segment, en empilé absolu — pour lire la composition sans survol, comme le `%` le fait
en 100 %.

## Décisions techniques

- **Un seul chemin de libellé de strate.** La branche empilée écrit désormais un libellé centré dans
  chaque strate assez haute, quel que soit le mode : la **part (`%`)** en 100 %, la **valeur brute**
  en absolu. Même seuil (`STRATA_LABEL_SIZE + 4`), même couleur (`on_primary`, lisible sur fond
  saturé), même centrage. Le comportement 100 % (jalon 227) est **inchangé** — seul l'absolu gagne
  le libellé.

- **Cohabite avec le total (jalon 228).** Total au sommet + valeur par strate = lecture complète
  (composition **et** somme), sans redondance : le total est au-dessus de la colonne, les valeurs
  dedans.

## Implémentation

- `frus-widgets/src/chart.rs` : le libellé de strate de la branche empilée passe de « `%` si
  `normalized` » à « `%` si `normalized`, sinon `format_value(value)` » (le garde de hauteur est
  désormais commun aux deux modes).

## Vérification

- **Widget** `stacked_absolute_bars_label_each_strata_with_its_value` : les valeurs de strates
  (`3`, `4`, `6`) sont présentes, et les totaux de colonne (`7`, `11`) restent au sommet.
- Les tests 100 % existants (`normalized_bars_label_each_strata_with_its_percentage`) restent verts
  (comportement normalisé préservé).
- **Golden** `bar_chart_stacked` régénéré : chaque strate porte sa valeur (3/2, 7/5, 5/6, 8/4, 4/3),
  total au sommet.
- Widgets 362 ; goldens 63.

## Reste

- Sortir du domaine graphes (nouveau widget : `Calendar`/`DataTable` avancé).
- Valeur par strate pour les **aires empilées** (lignes) — aujourd'hui au survol seulement.

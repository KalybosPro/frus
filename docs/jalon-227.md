# Jalon 227 — Libellé de part (%) dans chaque strate (barres 100 %)

## Analyse

En empilage 100 % (jalon 224), les strates montrent les proportions, mais la valeur exacte n'était
lisible qu'au survol (jalon 226). Pour un graphique à barres 100 %, l'usage est d'écrire la **part
(%) directement dans chaque strate** : lecture immédiate, sans interaction.

Réservé aux **barres** : chaque strate est un rectangle discret qui accueille un label centré. Les
aires empilées (lignes) n'ont pas cette découpe nette par catégorie — un label par bande/catégorie
surchargerait ; elles gardent la part au survol (jalon 226).

## Décisions techniques

- **Part au centre de la strate.** Dans la branche empilée de `BarChart::paint`, en mode 100 %,
  chaque segment visible reçoit `{part}%` (arrondie) centré horizontalement (sur `cx`) et
  verticalement (milieu de `[y_top, y_bottom]`).

- **Seuil de hauteur.** Le label n'est tracé que si la strate mesure au moins `STRATA_LABEL_SIZE + 4`
  px — une part trop fine reste sans texte (illisible sinon), la valeur restant accessible au survol.

- **Texte lisible sur fond saturé.** Couleur `theme.on_primary` (le rôle « texte sur surface
  colorée » du thème), conforme à la directive « customizable like Flutter » : dérivé du thème, donc
  surchargéable, jamais une couleur codée en dur.

## Implémentation

- `frus-widgets/src/chart.rs` : constante `STRATA_LABEL_SIZE` ; la branche empilée de `BarChart`
  écrit la part `%` au centre de chaque strate assez haute quand `normalized`.

## Vérification

- **Widget** `normalized_bars_label_each_strata_with_its_percentage` : 2 catégories × 2 séries
  visibles = **4** strates étiquetées en 100 % (sans axe : aucune graduation `%` parasite), **0** en
  absolu.
- **Golden** `bar_chart_normalized` régénéré : chaque colonne affiche ses parts (60 %/40 %, 64 %/36 %…)
  sommant à 100 %, texte blanc lisible sur les fills.
- Widgets 360 ; goldens 63.

## Reste

- Sortir du domaine graphes (nouveau widget : `Calendar`/`DataTable` avancé).
- Libellé de **valeur** au sommet des barres empilées **absolues** (parité avec les barres simples).

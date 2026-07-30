# Jalon 209 — Charts : séries multiples + légende

## Analyse

La `LineChart` ne traçait qu'une série. Comparer (ventes vs coûts, cette année vs l'an dernier)
demande **plusieurs séries** partageant les mêmes catégories et la même échelle, plus une **légende**
pour les distinguer. C'est la brique qui fait passer le graphe de démo à outil réel.

## Décisions techniques

- **Série principale + additionnelles, alignées par index.** `new(...)` fournit les catégories et la
  première série ; `.series(name, color, values)` en ajoute d'autres (valeurs alignées par index).
  `.name(...)` nomme la principale. Rétro-compatible : sans `.series`, le rendu est celui du jalon
  200/206.

- **Couleur explicite par série additionnelle** (façon customisable Flutter) : pas de palette cachée
  imposée — l'appelant fournit la couleur, la principale garde `color`/`theme.primary`.

- **Échelle et axe communs.** `max_value` englobe **toutes** les séries ; grille et graduations
  (jalon 203) sont partagées, si bien que les courbes sont directement comparables.

- **Moins de bruit en multi-séries.** Les libellés de valeur par point (jalon 200) et l'aire (jalon
  206) ne s'affichent qu'en **série unique** ; en multi-séries, on s'appuie sur l'axe et la légende.

- **Légende.** `.legend(bool)` dessine une bande en haut (pastille de couleur + nom par série), et
  réserve `LEGEND_H` au-dessus de la zone de tracé. Ne s'affiche que si activée **et** au moins une
  série est nommée.

## Implémentation

- `frus-widgets/src/chart.rs` : champs `name` / `extra` / `legend` sur `LineChart` ; builders
  `.name` / `.series` / `.legend` ; `max_value` sur toutes les séries ; `has_legend` ; paint
  restructuré (boucle par série + bande de légende) ; constantes `LEGEND_*`.
- `frus-test/tests/goldens.rs` : golden `line_chart_multi` (2 séries + axe + légende).

## Vérification

- **Unitaire** `multi_series_draws_each_line_and_a_legend` : deux polylignes, deux pastilles
  `~10x10`, et les noms `Sales`/`Costs` en légende. `max_value_spans_all_series` : l'échelle prend
  bien le max de la série additionnelle.
- **Golden** `line_chart_multi` : deux courbes colorées, légende en haut, axe partagé.

## Reste

- Séries **empilées** (aires cumulées), **BarChart** groupée/empilée multi-séries, et légende
  cliquable pour masquer/afficher une série.

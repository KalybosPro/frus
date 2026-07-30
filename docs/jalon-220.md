# Jalon 220 — Démo Charts : graphique compagnon partageant la visibilité

## Analyse

Le sélecteur (jalon 219) montre **un** graphique à la fois. Pour illustrer que plusieurs vues
peuvent réagir au **même** état, l'écran gagne un graphique **compagnon** : masquer une série via la
légende du principal la masque aussi dans le compagnon.

## Décisions techniques

- **La famille complémentaire.** Le compagnon est toujours de l'autre famille que le principal :
  barres si le principal est en lignes, lignes si le principal est en barres. On voit ainsi en
  permanence une lecture *tendance* et une lecture *comparaison*.

- **Un seul constructeur, un `kind` explicite.** `dashboard_chart` prend désormais le `kind` en
  paramètre (au lieu de lire `app.chart_kind`). Le principal appelle `dashboard_chart(app,
  app.chart_kind, …, legend = true)` ; le compagnon `dashboard_chart(app, complément, …, legend =
  false)`. Zéro duplication.

- **État partagé, sans légende propre.** Les deux graphiques lisent le **même** `chart_hidden` ;
  le compagnon n'affiche pas sa propre légende (`legend = false`) — il **reflète** simplement la
  visibilité pilotée depuis le principal.

## Implémentation

- `frus-demo/src/lib.rs` : `dashboard_chart` gagne un paramètre `kind` ; `charts_screen` ajoute le
  compagnon (famille complémentaire, hauteur réduite, sans légende) sous le graphique principal.

## Vérification

- `companion_chart_renders_across_families_with_hidden` : une série masquée (partagée) ; l'écran se
  rend avec le principal en lignes (compagnon barres) **et** en barres (compagnon lignes). Démo
  27/27.

## Reste

- Un clic sur un **point** du principal pour épingler son détail (jalon 221), et une légende
  **synchronisée** cliquable sur le compagnon aussi.

# Jalon 225 — Désépinglage au re-clic

## Analyse

Depuis les jalons 221–223, cliquer un point/barre l'épingle (détail dans un `Chip`) et le met en
évidence (halo/anneau). Mais rien ne permettait de **retirer** la sélection : une fois épinglé, le
tableau de bord restait marqué jusqu'au clic d'un autre élément. Le geste manquant, standard d'un
sélecteur : **re-cliquer** l'élément déjà sélectionné pour le désélectionner.

## Décisions techniques

- **Bascule côté app, pas côté widget.** Les graphiques rapportent un clic via `on_point` ; c'est à
  l'application de décider qu'un second clic sur le **même** `(catégorie, série)` annule la sélection.
  `reduce(ChartPoint)` compare la cible à `chart_sel` : si elle est déjà sélectionnée, on remet
  `chart_sel` et `chart_pin` à `None` ; sinon on épingle comme avant. Le widget n'a pas changé — la
  mise en évidence disparaît d'elle-même dès que `selected` repasse à `None`.

- **Indice mis à jour.** Le texte d'aide passe à « click a point to pin it, or again to unpin ».

## Implémentation

- `frus-demo/src/lib.rs` : `reduce(Msg::ChartPoint)` bascule (désépingle si `chart_sel == Some((cat,
  s))`, épingle sinon) ; texte d'indication actualisé.

## Vérification

- **Démo** `re_clicking_a_selected_point_unpins_it` : `ChartPoint(2, 1)` épingle
  (`chart_sel = Some((2, 1))`, détail présent) ; un second `ChartPoint(2, 1)` **désépingle**
  (`chart_sel = None`, détail effacé) ; `ChartPoint(0, 0)` ré-épingle un autre point.
- Les tests existants (`clicking_a_point_pins_its_detail`, `clicking_a_point_marks_it_selected`)
  cliquent des points **distincts** : inchangés. Démo 32 ; widgets/goldens non touchés.

## Reste

- Étiquettes de **pourcentage** dans l'infobulle en mode 100 % (aujourd'hui : valeurs brutes).
- Sortir du domaine graphes (nouveau widget : `Calendar`/`DataTable` avancé).

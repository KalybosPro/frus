# Jalon 231 — DatePicker borné (jours désactivés hors [min, max])

## Analyse

Le `DatePicker` (calendrier mensuel contrôlé) rendait **tout** jour réel cliquable — aucun moyen
d'interdire des dates. Or un sélecteur de date « réel » a presque toujours des bornes : pas de dates
passées, une fenêtre de réservation, une échéance. Ce jalon ouvre le domaine **date picker avancé**
par la première brique manquante : les **jours désactivés**.

## Décisions techniques

- **Prédicat `enabled` interne, non-breaking.** `assemble` (privé) gagne un paramètre
  `enabled: Fn(u32) -> bool`. Les constructeurs publics existants (`new`, `range`, `range_dual`)
  passent `|_| true` — rendu **identique**, aucune signature publique modifiée.

- **Jour désactivé = case atténuée, non cliquable.** La case `Day` gagne un champ `disabled` : peinte
  en `muted` très atténué, **sans** fond ni bande, et son `message` est `None` (donc `on_click`/focus
  inactifs). Le modèle **contrôlé** est préservé : le widget ne décide de rien, il reflète les bornes.

- **Constructeur `bounded(min, max)`.** Superset de `new` avec deux bornes **optionnelles et
  incluses** (dates `(année, mois, jour)`) ; un jour est activé ssi `min <= date <= max`. Une seule
  borne (`None` de l'autre côté) borne d'un seul côté.

## Implémentation

- `frus-widgets/src/datepicker.rs` : champ `disabled` sur `Day` + rendu atténué ; `assemble` gagne
  `enabled` ; `new`/`range` passent `|_| true` ; nouveau constructeur `bounded`.

## Vérification

- **Widget** `bounded_disables_days_outside_the_range` : fenêtre `[10, 20]` juillet 2026 → 9 et 21
  non cliquables (`on_click() == None`), 10/15/20 cliquables ; sans borne max, le 31 reste cliquable.
- **Golden** `date_bounded` : jours 10–20 actifs (15 sélectionné en pastille), le reste atténué.
- Widgets 364 ; goldens : `date_range`/`date_range_dual` inchangés (défaut « tout activé »).

## Reste

- Wirer `bounded` dans la démo (écran date) avec une fenêtre réelle.
- Bornes en **mode plage** (`range` borné) et jours **blackout** arbitraires (prédicat/ensemble).

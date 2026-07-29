# Jalon 187 — TimePicker : plage horaire (créneau début → fin)

## Analyse

`TimePicker` choisit **une** heure. Réserver un créneau, fixer des horaires d'ouverture,
planifier une réunion : autant de cas qui demandent **deux** heures — un début et une fin. C'est
le pendant temporel du calendrier double (jalon 186) ; il manquait à frus.

## Décisions techniques

- **Composition de deux `TimePicker`.** `TimeRange` pose deux sélecteurs étiquetés « Start » et
  « End » côte à côte (chacun une colonne `[label, TimePicker]` dans une `Flex::row`). Toute la
  logique (grilles 24 h/12 h, aperçu, pas des minutes) est **réutilisée** telle quelle — aucune
  duplication.

- **Un seul rappel taggé.** Plutôt que quatre closures (heure/minute × début/fin), `TimeRange`
  prend **un** `on_change(Endpoint, TimeField, u32)` : chaque `TimePicker` interne enveloppe ses
  `on_hour`/`on_minute` pour préfixer la **borne** (`Start`/`End`) et le **champ** (`Hour`/
  `Minute`). Le rappel est mis en `Rc` pour alimenter les deux sélecteurs ; les valeurs restent en
  **24 h** (comme `TimePicker`). L'application reçoit un message unique et décide comment
  actualiser son état (et, si besoin, contraindre fin ≥ début — logique applicative).

- **Options propagées.** `hour12()` et `minute_step(n)` s'appliquent aux **deux** sélecteurs via
  `rebuild` (mêmes réglages de part et d'autre).

## Implémentation

- `timepicker.rs` : `enum Endpoint { Start, End }`, `enum TimeField { Hour, Minute }` ;
  `TimeRange<Msg>` (`new`/`hour12`/`minute_step`, `rebuild` construit les deux colonnes taggées,
  `Rc` du rappel partagé) ; `impl Widget` (rangée, sans peinture propre).
- `lib.rs` : `pub use timepicker::{Endpoint, TimeField, TimeRange}`.
- `goldens.rs` : `time_range` (Start 09:00 / End 17:30, minutes par pas de 15).

## Vérification

- **Unitaire** : `range_builds_start_and_end_pickers` (deux colonnes ; minutes pas de 15 → 4
  cases ; cliquer 09 h côté End émet `Set(End, Hour, 9)`) ; `hour12_applies_to_both_pickers`
  (section heures 12 h = label + AM/PM + grille des deux côtés). Tests `TimePicker` existants
  **verts**.
- **Golden** `time_range` **inspecté** : « Start » 09:00 (heure 09 + minute 00 surlignées),
  « End » 17:30 (heure 17 + minute 30), minutes 00/15/30/45.
- `cargo test -p frus-widgets timepicker::` **vert**.

## Reste

- **Contrainte fin ≥ début** intégrée (griser les heures antérieures de la borne End) — pour
  l'instant à la charge de l'application.
- **Durée** dérivée affichée entre les deux (ex. « 8 h 30 ») — extension de présentation.

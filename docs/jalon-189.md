# Jalon 189 — DateTimeRange : plage date + heure

## Analyse

On sait choisir une plage de **dates** (calendrier double, jalon 186) et une plage d'**heures**
(`TimeRange`, jalon 187), mais pas les deux ensemble. Réserver un créneau réel — « du 28 juillet
09:00 au 3 août 17:30 » — demande un seul écran combinant date **et** heure de début et de fin.
C'est le pendant « plage » de [`DateTimePicker`](../crates/frus-widgets/src/datetimepicker.rs).

## Décisions techniques

- **Composition pure, comme `DateTimePicker`.** `DateTimeRange` empile en colonne le calendrier
  double ([`DatePicker::range_dual`]) et la plage horaire ([`TimeRange`]), coiffés d'un
  **récapitulatif** « début → fin ». Aucune logique nouvelle : chaque brique garde la sienne, le
  composite ne fait que **relayer** les deux canaux de messages.

- **Deux canaux distincts, chacun à sa nature.** Les **dates** passent par `on_date((année, mois,
  jour))` (l'application décide quelle borne reçoit le jour cliqué — comme la plage de dates) et
  `on_nav(±1)` ; les **heures** par `on_time(borne, champ, valeur)` (les bornes y sont
  explicites, Start/End). La plage de dates traverse naturellement la frontière de mois (dates
  comparées en entier, jalon 184).

- **Récapitulatif conditionnel.** La ligne « July 28, 2026 09:00 → August 3, 2026 17:30 »
  n'apparaît que lorsque les **deux** dates sont posées (sélection terminée) ; sinon le composite
  se réduit à `[calendrier, heures]` — même règle que le récapitulatif de `DateTimePicker`.

## Implémentation

- `datetimerange.rs` : `DateTimeRange<Msg>` (`new`, composite `range_dual` + `TimeRange`,
  récapitulatif conditionnel) ; `impl Widget` (colonne, sans peinture propre).
- `lib.rs` : `mod datetimerange` + `pub use datetimerange::DateTimeRange`.
- `goldens.rs` : `datetime_range`.

## Vérification

- **Unitaire** : `summary_appears_only_with_both_dates` (0/1 borne → `[calendrier, heures]` ;
  2 bornes → `[récap, calendrier, heures]`) ; `renders_the_combined_summary` (texte exact
  « July 28, 2026 09:00 → August 3, 2026 17:30 »).
- **Golden** `datetime_range` **inspecté** : récapitulatif en tête, calendrier double (plage
  28/07 → 03/08 franchissant le mois), plage horaire Start 09:00 / End 17:30.
- `cargo test -p frus-widgets datetimerange::` **vert**.

## Reste

- **Contrainte fin ≥ début** (dates et heures) intégrée — pour l'instant à la charge de
  l'application.
- **Durée dérivée** affichée dans le récapitulatif (« 6 j 8 h 30 ») — extension de présentation.

# Jalon 147 — Flux date + heure, minutes fines & 12 h AM/PM

## Analyse

Le `DatePicker` (calendrier) et le `TimePicker` (heure) existaient séparément. Il manquait
le **flux combiné** façon `showDateTimePicker`, et le `TimePicker` était rigide : 24 h
uniquement, minutes figées au pas de 5. On complète la famille date/heure.

## Décisions techniques

- **`TimePicker` reconstruit son arbre.** Comme le `Table` (jalon 145), le picker stocke
  désormais son état (`hour`, `minute`, rappels) + ses options et **régénère** ses enfants
  (`rebuild`) : cela ouvre des réglages fluides (`hour12`, `minute_step`) sans multiplier
  les constructeurs.

- **12 h propre, un seul rappel 24 h.** En `hour12`, la grille passe de 0–23 à **1–12** et
  une **bascule AM/PM** apparaît. Le widget reste piloté par une heure **24 h** unique :
  chaque case 1–12 vise l'heure 24 h de la moitié courante, et AM/PM bascule l'heure
  courante d'±12 h. L'application ne gère donc qu'un seul `on_hour(h24)` — la conversion
  12 h ↔ 24 h est interne (`digit12`).

- **Pas des minutes réglable.** `minute_step(n)` (borné 1–60) contrôle la granularité ;
  la sélection ne s'allume que si la minute courante tombe sur un pas (l'aperçu, lui, reste
  exact).

- **`DateTimePicker` purement composite.** Il n'ajoute aucune logique : il empile le
  `DatePicker` et le `TimePicker`, relaie leurs quatre rappels (`on_day`, `on_nav`,
  `on_hour`, `on_minute`), et coiffe le tout d'un **récapitulatif** « Mois jour, année
  HH:MM » — affiché seulement quand un jour est choisi. L'état (date, heure) reste dans
  l'application.

## Implémentation

- `timepicker.rs` : passage à un `rebuild` ; options `hour12()` et `minute_step(n)` ;
  bascule AM/PM + grille 1–12 ; aperçu 12 h/24 h ; helper `digit12`.
- `datetimepicker.rs` (nouveau) : `DateTimePicker` combinant les deux sous-sélecteurs +
  récapitulatif.
- `lib.rs` : `mod datetimepicker;` + export `DateTimePicker`.
- `goldens.rs` : goldens `time_picker_12h` et `date_time_picker`.

## Vérification

- **Unitaire** : `minute_step(15)` → 4 minutes ; `hour12()` → grille de 12 + rangée AM/PM,
  aperçu `3:05 PM` pour 15 h 05 ; aperçu 24 h `09:30` ; clic → message ; le
  `DateTimePicker` n'affiche le récapitulatif que si un jour est choisi et rend
  « July 11, 2026  09:30 ».
- **Golden** : `time_picker_12h` (PM, heure 3, minute 05 allumées) et `date_time_picker`
  (récap + calendrier au 11 + heure 09:30) rendus et **inspectés**. Le golden 24 h existant
  (`time_picker`) reste identique. `cargo test --workspace` vert.

## Reste

- **Cadran horaire** optionnel et **saisie clavier** `HH:MM` (Material 3).
- **Validation d'un flux complet** (bouton « OK/Annuler », renvoi d'un `(date, heure)`
  unique) — ici les deux moitiés émettent indépendamment.

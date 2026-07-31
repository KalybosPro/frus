# Jalon 235 — Jours blackout / prédicat de sélection (DatePicker)

## Analyse

Les jalons 231/234 désactivent des **intervalles** (`[min, max]`). Mais un calendrier réel doit aussi
retirer des dates **isolées** : jours fériés, créneaux déjà réservés, week-ends. Plutôt que
multiplier les constructeurs (un par forme de contrainte), ce jalon expose l'**escape hatch général**
— un prédicat de disponibilité, façon `selectableDayPredicate` de Flutter (cf. « follow Flutter's
footsteps »).

## Décisions techniques

- **`filtered(is_enabled)`.** Un jour `(année, mois, jour)` est cliquable ssi `is_enabled(date)`. Ce
  seul constructeur couvre **tout** : blackout épars (`|d| !holidays.contains(&d)`), week-ends
  (`|(y,m,d)| weekday(y,m,d) not in {0,6}`), bornes (`min..=max`), ou n'importe quelle combinaison.
  `bounded`/`range_bounded` restent des raccourcis pratiques pour le cas fréquent min/max.

- **Réutilise `assemble(enabled)`.** Le prédicat public `Fn((i32,u32,u32)) -> bool` est adapté au
  prédicat interne `Fn(u32) -> bool` par `move |day| is_enabled((year, month, day))`. Aucune
  infrastructure nouvelle ; rendu désactivé identique (jalon 231).

## Implémentation

- `frus-widgets/src/datepicker.rs` : nouveau constructeur `filtered`.

## Vérification

- **Widget** `filtered_disables_days_by_predicate` : blackout `{12, 18}` juillet 2026 → 12 et 18 non
  cliquables, 1 et 13 cliquables.
- **Golden** `date_blackout` : jours 4, 5, 14, 15, 27 atténués/désactivés, le reste actif, 21
  sélectionné.
- Widgets 371 ; goldens 68 (calendriers existants inchangés).

## Reste

- Wirer un calendrier filtré dans la démo (p.ex. week-ends grisés).
- Prédicat en **mode plage** (aujourd'hui `filtered` couvre le mode simple ; `range` a `range_bounded`
  pour les bornes, pas encore un prédicat arbitraire).

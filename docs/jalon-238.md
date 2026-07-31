# Jalon 238 — Démo : calendrier filtré (week-ends grisés)

## Analyse

Le `DatePicker::filtered` (jalon 235) permet de désactiver des jours par prédicat, mais la démo
utilisait encore le calendrier simple. Ce jalon l'**ancre dans l'app** avec le cas le plus parlant :
une bascule « Weekdays only » qui grise les **week-ends**.

## Décisions techniques

- **Bascule `Switch` contrôlée.** Un état `weekdays_only` piloté par `Msg::SetWeekdaysOnly` ; la
  vitrine (route Settings) affiche l'interrupteur au-dessus du calendrier.

- **`demo_calendar(app)`.** Renvoie `DatePicker::filtered(..., |(y,m,d)| !is_weekend(y,m,d), ...)`
  quand la bascule est active, sinon `DatePicker::new(...)`. Démontre le prédicat sur des données
  réelles, sans toucher au widget.

- **Calcul de week-end maison.** `weekday` (Sakamoto, 0 = dimanche) + `is_weekend` (samedi/dimanche)
  dans la démo — aucune dépendance temporelle, cohérent avec l'esprit du `DatePicker`.

## Implémentation

- `frus-demo/src/lib.rs` : état `weekdays_only` + `Msg::SetWeekdaysOnly` + arm de reduce ; helpers
  `weekday`/`is_weekend` ; `demo_calendar` ; `Switch` + calendrier conditionnel dans `settings_screen`.

## Vérification

- **Démo** `calendar_weekdays_only_filters_weekends` : `is_weekend` correct (4–5 juillet 2026 =
  week-end, 6 = lundi) ; la bascule met `weekdays_only`, la vitrine se rend filtrée puis non filtrée.
- Démo 34 ; workspace (shell) compile ; widgets/goldens inchangés.

## Reste

- Un état « ligne sélectionnée » sur l'écran data (`on_select_row`).
- Clé de tri **personnalisée** par colonne du `DataTable` (dates, montants formatés).

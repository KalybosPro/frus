# Jalon 146 — Sélecteur d'heure (`TimePicker`)

## Analyse

Le `DatePicker` (calendrier mensuel) existait déjà, mais rien pour choisir une **heure**.
Flutter a `showTimePicker` (cadran horaire) ; il manquait son pendant frus. On complète la
famille « date/heure » par un sélecteur d'heure cohérent avec le calendrier.

## Décisions techniques

- **Grilles plutôt que cadran.** Le cadran Material (aiguille, arc) est lourd à peindre et
  peu lisible au clavier. On retient deux **grilles de cases** — heures `0–23`, minutes par
  **pas de 5** — dans le même esprit visuel que les cases-jour du `DatePicker` (même
  `TimeCell` surlignée `primary`, survol par couche d'état, coins arrondis). Simple,
  lisible, cliquable, et déjà accessible au pointeur.

- **Contrôlé, comme le reste.** `TimePicker::new(hour, minute, on_hour, on_minute)` : l'heure
  affichée vient de l'état applicatif ; le widget **émet** `on_hour(h)` / `on_minute(m)` au
  clic et ne décide de rien. L'aperçu `HH:MM` reflète **exactement** `hour`/`minute`, même
  quand la minute n'est pas un multiple de 5 (aucune case n'est alors allumée, mais l'aperçu
  reste juste).

- **Composite, sans état.** Comme le `DatePicker`, le picker n'est qu'un assemblage
  `[aperçu, section heures, section minutes]` (chaque section = libellé + `Grid`), bâti au
  constructeur. Aucune logique temporelle : le pas des minutes est une simple constante
  `MINUTE_STEP`.

## Implémentation

- `timepicker.rs` (nouveau) : `TimeCell<Msg>` (case cliquable surlignée) ; `TimePicker<Msg>`
  assemblant l'aperçu et les deux grilles.
- `lib.rs` : `mod timepicker;` + `pub use timepicker::TimePicker;`.
- `goldens.rs` : golden `time_picker` (9 h 30 → heure `09` et minute `30` surlignées).

## Vérification

- **Unitaire** : structure `[aperçu, heures(24 cases), minutes(12 cases)]` ; l'aperçu
  `09:30` est peint et une case sélectionnée est surlignée `primary` ; un clic sur une case
  émet bien `Hour`/`Minute` (via `ui.hit` + `ui.msg_for`).
- **Golden** `time_picker` rendu et **inspecté** : aperçu `09:30`, `09` et `30` allumés.
  `cargo test --workspace` vert, aucun golden existant déplacé.

## Reste

- **Minutes à la minute près** (aujourd'hui pas de 5) et **format 12 h (AM/PM)**.
- **Cadran** optionnel et **saisie clavier** directe (`HH:MM`), façon Material 3.
- **Combinaison date + heure** dans un même flux (`showDateTimePicker`).

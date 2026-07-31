# Jalon 234 — DatePicker plage bornée (range + fenêtre [min, max])

## Analyse

Le jalon 231 a borné le calendrier **simple** (`bounded`). Le mode **plage** (`range`), lui, laissait
encore tous les jours cliquables — impossible d'imposer une fenêtre de saisie à une sélection
d'intervalle (réserver une plage à l'intérieur d'une période ouverte, par exemple). Ce jalon comble
la parité : un mode plage **borné**.

## Décisions techniques

- **`range_bounded(start, end, min, max)`.** Superset de `range` : mêmes marques de plage
  (bornes en pastille, jours entre en bande douce) **plus** le prédicat `enabled` du jalon 231 —
  un jour est cliquable ssi `min <= date <= max` (bornes optionnelles, incluses). Aucune
  infrastructure nouvelle : réutilise `assemble(enabled)` et `range_mark`.

- **Combinaison propre.** La mise en avant (plage sélectionnée) et l'activation (fenêtre autorisée)
  sont **orthogonales** : un jour peut être « entre » les bornes de la plage tout en étant hors
  fenêtre (donc désactivé), et inversement. Chacune est calculée indépendamment.

## Implémentation

- `frus-widgets/src/datepicker.rs` : nouveau constructeur `range_bounded` (mark = `range_mark`,
  enabled = test `[min, max]`).

## Vérification

- **Widget** `range_bounded_disables_days_outside_the_window` : plage 10–15 dans une fenêtre `[8, 20]`
  → 7 et 21 non cliquables, 8/12/20 cliquables (dont un jour « entre », le 12, bien actif).
- **Golden** `date_range_bounded` : fenêtre 8–20 active, plage 10–15 mise en avant, hors-fenêtre
  atténué.
- Widgets 370 ; goldens 67 (`date_range`/`date_bounded` inchangés).

## Reste

- Jours **blackout** arbitraires (prédicat/ensemble de dates isolées, pas seulement un intervalle).
- Wirer les calendriers bornés dans la démo (écran date).

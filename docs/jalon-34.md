# Jalon 34 — Nouveaux widgets : Avatar, Stepper, Rating

Trois widgets de plus (affichage / saisie numérique / note).

## Widgets

- **`Avatar::new("Ada Lovelace")`** — pastille ronde d'**initiales** (2 premières
  lettres des mots, en majuscules), fond d'accent (`.color()` pour surcharger,
  `.size()`). Rendu pur.
- **`Stepper::new(valeur, on_change).range(min, max).step(n)`** — sélecteur
  numérique **−/valeur/+**, contrôlé. Composite `[−, texte, +]` ; les boutons
  émettent la **valeur bornée** (le stepper clamp lui-même à `[min, max]`).
- **`Rating::new(valeur, max, on_rate)`** — note en **étoiles cliquables** ;
  cliquer la i-ᵉ émet `on_rate(i + 1)`. Étoiles pleines `primary` / vides `muted`,
  focusables (accessibles au clavier).

## Démo (intégration)

- **`Avatar`** (initiales) à gauche de chaque ligne de tâche.
- **`Rating`** (« Votre avis ») et **`Stepper`** (« Quantité ») dans la carte de
  contrôles des Réglages.

## Tests

- `Avatar` : 2 initiales majuscules ; peint cercle + initiale.
- `Stepper` : `+`/`−` émettent la valeur ±pas, **bornée** à la plage.
- `Rating` : `max` étoiles ; clic i-ᵉ → `on_rate(i+1)` ; pleines ≠ vides (couleur).
- 68 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- `Stepper` : largeur du texte variable (léger décalage des boutons selon la valeur).
- `Rating` : demi-étoiles non gérées ; pas de survol « prévisualisation ».
- `Avatar` : initiales ou couleur unie (pas d'image — pas encore de widget image).

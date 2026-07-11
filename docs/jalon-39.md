# Jalon 39 — Correctif clic + nouveaux widgets : DatePicker, Carousel, Alert

## Correctif critique (commité à part : `a82eeae`)

Depuis J29 (boutons focusables), le handler d'appui posait un `Drag::TextSelect`
sur **tout** widget focalisé (code prévu pour les seuls champs texte) ; au
relâchement, ce faux glissement était consommé **avant** le dispatch → **les
clics souris étaient avalés** (le clavier marchait). Correctif : ne démarrer la
sélection que si `cursor_at()` renvoie `Some` (invariant : `TextInput` → `Some`,
boutons → `None`). Signalé par l'utilisateur en testant l'app réelle.

## Widgets

- **`DatePicker::new(année, mois, jour, on_select, on_nav)`** — calendrier mensuel
  contrôlé, bâti sur `Grid` (7 colonnes). En-tête « ‹ Mois Année › », jours de
  semaine, grille des jours (case sélectionnée en avant, cases vides avant le 1er).
  **Calcul de date maison** : bissextile, jours/mois, jour de semaine (Sakamoto) —
  aucune dépendance temporelle.
- **`Carousel::new(index, total, on_change, slide_courant)`** — flèches ‹ ›
  (désactivées aux bornes) autour du slide courant **fourni par l'app** (un seul
  réalisé). `on_change(index∓1)`.
- **`Alert::new("texte").title("...").warning()`** — encadré de message
  **persistant** (Info/Succès/Alerte/Erreur : fond teinté + barre d'accent +
  glyphe), à distinguer du `Toast` transitoire.

## Démo

- `Alert` (« Astuce ») en tête de la carte todo.
- `DatePicker` dans la carte de contrôles des Réglages (état année/mois/jour +
  navigation de mois).
- `Carousel` (3 slides) dans l'onglet « À propos ».

## Tests

- Correctif : `only_text_inputs_place_a_cursor` (Button `cursor_at` = None, TextInput = Some).
- `DatePicker` : math de date (bissextile, jour de semaine), 3 enfants (en-tête /
  jours / grille), nombre de cellules = cases vides + jours du mois.
- `Carousel` : flèches bornées ; ‹/› → `on_change`.
- `Alert` : variante → barre d'accent + titre + texte peints.
- 85 tests frus-widgets.

## Limites (v1)

- `DatePicker` : sélection d'un seul jour (pas de plage) ; pas de saisie clavier
  de date.
- `Alert` : texte mono-ligne (pas de retour à la ligne automatique).

# Jalon 202 — Icône œil + révélation du mot de passe dans le champ

## Analyse

Le jalon 198 a rendu l'icône **suffixe** d'un `TextInput` cliquable (`on_suffix`), et notait qu'il
manquait une **icône œil** (contour) pour révéler un mot de passe *dans* le champ — le geste
attendu partout. L'assistant d'inscription révélait bien les mots de passe, mais via un bouton
« Show / Hide password » **à côté** des champs. On rapatrie l'action dans le champ, avec l'icône
idoine.

## Décisions techniques

- **Deux icônes, `Eye` et `EyeOff`** (façon Material `visibility` / `visibility_off`). Le jeu
  d'icônes est **rempli** (règle non-zero) ; un œil, lui, est un **anneau** creux avec une pupille.
  On l'obtient sans changer le moteur : contour externe (amande) + contour interne **parcouru à
  l'envers** (winding opposé → l'ouverture s'annule à 0 = transparent) + pupille pleine (winding non
  nul au centre). L'ouverture est ainsi garantie **quel que soit** le sens absolu de tracé.
  `EyeOff` ajoute une barre diagonale (masqué).

- **Révélation dans le champ.** `wizard_input` prend un paramètre `eye: Option<bool>` : `Some(révélé)`
  pose l'icône suffixe (`EyeOff` si révélé, sinon `Eye`) et `on_suffix(WizardToggleReveal)`. Les deux
  champs mot de passe de l'assistant l'utilisent ; le bouton externe « Show / Hide » disparaît. Le
  masquage (`obscure`) reste piloté par `wizard_reveal`, l'icône ne fait que **basculer** cet état.

## Implémentation

- `frus-widgets/src/icons.rs` : variantes `IconName::{Eye, EyeOff}` + `eye(off)` (anneau opposé +
  pupille, barre optionnelle) ; helper `push_verb` pour recopier le cercle de la pupille.
- `frus-demo/src/lib.rs` : `wizard_input` gagne `eye: Option<bool>` (icône suffixe + `on_suffix`) ;
  l'étape « Security » passe `Some(app.wizard_reveal)` aux deux champs et perd le bouton externe.
- `frus-test/tests/goldens.rs` : golden `password_eye` (champ masqué + œil suffixe).

## Vérification

- **Unitaire** (`eye_is_a_ring_with_a_pupil_and_off_adds_a_slash`) : `Eye` = 3 sous-chemins fermés
  (deux amandes + pupille) ; `EyeOff` = 4 (avec la diagonale) ; les deux figurent dans le test
  « toute icône produit un chemin non vide ».
- **Golden** `password_eye` : champ « Password » masqué (points) avec l'icône œil à droite.
- **Manuel** : à l'étape Security, l'œil dans le champ révèle / masque les deux mots de passe.

## Reste

- **Survol du suffixe** (curseur main, surbrillance de l'œil) ; icône œil dans les champs mot de
  passe hors assistant (connexion, réglages).

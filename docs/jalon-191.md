# Jalon 191 — Button : état désactivé

## Analyse

L'assistant (jalon 190) voulait bloquer « Next » tant qu'une étape est invalide, mais `Button`
n'avait **que** deux états utiles : présent (cliquable) ou absent. Il manquait le **contrôle
indisponible** de Material — visible mais grisé et inerte — qui *montre* qu'une action existe sans
la permettre encore. C'est un pré-requis de la validation par étape (jalon 192).

## Décisions techniques

- **Un drapeau `enabled`, inerte de bout en bout.** `Button::enabled(false)` grise le bouton
  (aplat neutre `surface.lerp(muted)`, texte discret, **sans ombre**), et surtout le rend
  **inerte partout** : `on_click` rend `None`, `focusable` est `false` (hors tabulation clavier),
  et la sémantique n'annonce **pas** d'action cliquable (lecteurs d'écran). Un bouton désactivé
  n'est donc jamais actionnable, quelle que soit la voie (souris, clavier, a11y) — pas seulement
  grisé à l'écran.

- **Défaut inchangé.** `enabled` vaut `true` par défaut ; tous les boutons existants gardent leur
  comportement. Aucun rendu modifié tant qu'on n'appelle pas `enabled(false)`.

## Implémentation

- `button.rs` : champ `enabled` (+ builder) ; branche « désactivé » en tête de `paint` (aplat +
  texte discret, `return` avant l'ombre) ; `on_click`/`focusable`/`semantics` conditionnés.

## Vérification

- **Unitaire** : `disabled_button_is_inert_and_unfocusable` (pas de message, non focalisable,
  sémantique non cliquable ; réactivé → le clic repasse) ; `disabled_button_paints_no_shadow`
  (ombre présente si actif, absente si désactivé). `on_click_returns_message` reste **vert**.
- **Golden** `button_disabled` **inspecté** : « Next » actif (accent + ombre) à côté du désactivé
  (grisé, bord fin, sans ombre).
- `cargo test -p frus-widgets button::` **vert**.

## Reste

- **Infobulle « pourquoi désactivé »** au survol — composition avec le `Popover`/`Tooltip`
  existant.
- **Variante « chargement »** (spinner à la place du libellé) — état distinct, autre jalon.

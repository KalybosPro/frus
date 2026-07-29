# Jalon 185 — Snackbar : action + file d'attente

## Analyse

`Toast` (notification transitoire) n'était qu'une **carte statique** : pas d'**action** (le
« UNDO » du Snackbar Material qui permet d'annuler l'opération), et l'application devait gérer
seule l'**empilement** et l'**auto-fermeture**. Deux manques pour un vrai système de
notifications : une action optionnelle, et une file « une à la fois » qui expire toute seule.

## Décisions techniques

- **`Toast<Msg>` générique + action.** Porter un message d'action impose de généraliser `Toast`
  (auparavant `impl<Msg> Widget for Toast` non générique). `action(label, msg)` ajoute un
  **bouton texte en capitales** (widget privé `ActionButton`) à droite, émettant `msg` au clic
  (focalisable, sémantique `Role::Button`). Sans action, `children` est vide et le rendu reste
  **identique** (le démo, qui infère `Msg`, compile sans changement). La carte se place alors en
  rangée (`justify: End`, `align: Center`) pour poser l'action à droite ; le texte reste peint
  par `Toast` à gauche.

- **File d'attente pure `SnackbarQueue<T>`.** Dans l'esprit de [`Form`](../crates/frus-widgets/src/form.rs) :
  aucune peinture, juste de l'état. Une `VecDeque<(T, secondes)>` dont l'**avant** est la
  notification visible. `push(item, seconds)` empile ; `tick(dt)` décompte **la tête** et la
  retire à expiration (rendant `true` si la notification visible a changé) — l'auto-fermeture
  Material **sans minuterie côté widget**, pilotée par la boucle de l'application ; `dismiss()`
  ferme la courante (clic sur l'action) et rend sa charge ; `current()` donne l'affichée. `T` est
  la charge applicative (texte, type, message d'action).

- **Séparation nette.** Le widget dessine, la file ordonnance. L'application relie les deux :
  `queue.current()` → un `Toast`, `tick(dt)` chaque frame, `dismiss()` sur l'action.

## Implémentation

- `toast.rs` : `Toast<Msg>` (+ `action`, `action_w`, `children`) ; widget privé `ActionButton` ;
  `accent` déplacé dans un `impl<Msg>` sans borne `'static` (appelé depuis `paint`) ;
  `SnackbarQueue<T>` (`new`/`push`/`current`/`tick`/`dismiss`/`is_empty`/`len`).
- `lib.rs` : `pub use toast::SnackbarQueue`.
- `goldens.rs` : `snackbar_action`.

## Vérification

- **Unitaire** : `action_is_clickable_and_uppercased` (sans action → aucun enfant ; avec →
  bouton « UNDO » cliquable et focalisable) ; `queue_shows_one_at_a_time_and_expires` (une seule
  visible, décompte de la tête, relais à l'expiration, fermeture manuelle, file vide inerte).
  Le test de peinture existant reste **vert**.
- **Golden** `snackbar_action` **inspecté** : carte à barre d'accent, texte, action « UNDO » à
  droite en couleur d'accent.
- `cargo test -p frus-widgets toast::` **vert**.

## Reste

- **Bouton de fermeture (croix)** intégré au widget, en plus de l'action — variante Material.
- **Transitions d'entrée/sortie** (glissement/fondu) pilotées par la couche d'animation existante.
- **Positionnement/pile** (haut, bas, coins) : déjà à la main de l'application via un overlay.

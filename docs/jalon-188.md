# Jalon 188 — ToastHost : positionnement, empilement, transition

## Analyse

`Toast` (jalon 185) sait se dessiner et porter une action ; `SnackbarQueue` ordonnance. Restait
le **placement** : chaque écran refaisait à la main « une colonne alignée dans un coin avec une
marge » (le démo : `column![Toast].justify(End).align(Center).padding(20)`). Et aucune
**transition** d'apparition. Il manquait la couche qui ancre, empile et anime les notifications.

## Décisions techniques

- **Une couche plein écran ancrée dans un coin.** `ToastHost` remplit la surface disponible
  (`width/height: Percent(1.0)`) et aligne ses toasts via une colonne dont le `justify` (haut/bas)
  et l'`align` (gauche/centre/droite) découlent de [`ToastPosition`] (six coins). On la pose en
  **dernière couche d'un `Stack`** au-dessus de l'interface ; elle laisse tout passer et ne fait
  que placer.

- **Empilement natif.** Plusieurs `toast(...)` s'empilent en colonne (gap fixe) dans le coin —
  plus besoin d'agencement ad hoc côté application.

- **Transition d'entrée via la couche existante.** `fade_in(duration)` enveloppe **chaque** toast
  d'un [`AnimatedOpacity`](../crates/frus-widgets/src/animated.rs) (opacité animée implicite) —
  apparition en fondu sans nouveau mécanisme. Optionnel : le rendu par défaut (et le golden)
  reste à pleine opacité, déterministe.

- **Le contenu reste applicatif.** `ToastHost` ne décide **pas** quoi afficher : l'application
  passe le(s) toast(s) courant(s) (typiquement `SnackbarQueue::current`) et gère file/auto-
  fermeture. Séparation nette placement / ordonnancement / dessin.

## Implémentation

- `toasthost.rs` : `enum ToastPosition` (+ `justify`/`align`) ; `ToastHost<Msg>`
  (`new`/`padding`/`toast`/`fade_in`) ; `impl Widget` (colonne pleine surface, sans peinture).
- `lib.rs` : `mod toasthost` + `pub use toasthost::{ToastHost, ToastPosition}`.
- `goldens.rs` : `toast_host` (deux toasts empilés en bas-droite).

## Vérification

- **Unitaire** : `empty_host_has_no_children` ; `position_maps_to_justify_and_align`
  (`BottomEnd` → justify End/align End ; `TopCenter` → Start/Center) ;
  `stacks_multiple_and_fade_in_preserves_count` (deux toasts, `fade_in` conserve le compte).
- **Golden** `toast_host` **inspecté** : « File uploaded » (succès) au-dessus de « Message
  archived » + « UNDO », alignés en bas-droite, empilés.
- `cargo test -p frus-widgets toasthost::` **vert**.

## Reste

- **Transition de sortie** (fondu/glissement avant retrait) : nécessite de garder le toast une
  frame de plus en s'appuyant sur l'état de la file — extension côté `SnackbarQueue`.
- **Décalage clavier/insets** (remonter les toasts au-dessus du clavier mobile) — via
  `WindowInsets` existant.

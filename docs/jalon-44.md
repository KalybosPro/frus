# Jalon 44 — Échelle & taille dynamiques

Trois ajouts côté **shell/plateforme** pour que la responsivité réagisse à la
taille et à la densité **en direct**.

## Lot A — Densité / échelle utilisateur

`Application::density(&self) -> f32` (défaut `1.0`) : un facteur de zoom
**applicatif** appliqué par-dessus l'échelle DPI système. Le shell calcule
`échelle_totale = scale_système × densité` et l'utilise partout :

- taille **logique** passée à `view` = physique / échelle_totale (l'UI grandit /
  se resserre) ;
- scène mise à l'échelle totale au rendu (net en HiDPI) ;
- curseur, molette (PixelDelta), largeur du geste retour divisés par la totale.

L'app change `density` par message → toute l'UI zoome (façon zoom navigateur),
sans qu'aucun widget ne s'en préoccupe.

## Lot B — Breakpoints pilotés par la vraie taille

`Application::on_resize(&mut self, width, height)` (défaut no-op) : le shell suit
la taille **logique** courante et, à **chaque changement** (redimensionnement de
la fenêtre *ou* de la densité), appelle `on_resize` **avant** `view`. L'app peut
alors réagir au **changement de palier** dans sa logique (fermer un tiroir en
rétrécissant, réinitialiser une sélection…), pas seulement au rendu.

`SizeClass` est ré-exporté depuis `frus-shell` pour l'usage côté app.

## Lot C — Redimensionnement fluide

`Resized` et `ScaleFactorChanged` reconfigurent la surface et redemandent une
frame ; `RedrawRequested` reconstruit toujours la `view` à la taille logique
**vivante** et déclenche `on_resize` au moindre écart — donc le reflow responsive
suit le drag sans latence ni surface obsolète. Un changement de densité (via un
message) force lui aussi un redraw, donc le même chemin.

## Démo

Boutons **A− / A+** dans l'en-tête : zooment toute l'UI (densité `0.8..=1.4`).
`on_resize` mémorise le palier courant (`size_class`), ferme le détail Stats en
passant en Compact, et journalise chaque changement de palier.

## Tests

- `frus-demo` : densité bornée (`0.8..=1.4`, garde-fou `0.0 → 1.0`) ; `on_resize`
  met à jour le palier et ferme le détail en Compact.
- Le câblage winit (échelle du curseur/scène) n'est pas testable unitairement
  (comme le jalon fenêtre) — validé par compilation + démo sans régression.

## Limites (v1)

- Pas de transition animée du zoom (changement de densité instantané).
- `on_resize` est appelé au fil du redraw (pas d'événement dédié hors frame).

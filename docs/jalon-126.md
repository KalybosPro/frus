# Jalon 126 — `InteractiveViewer` : inertie (fling) + bornage du pan

## Analyse

L'`InteractiveViewer` (J122) déplaçait et zoomait, mais deux finitions manquaient : le
pan pouvait **sortir le contenu du cadre** (rien ne le retenait), et un relâchement en
mouvement s'**arrêtait net** (pas d'élan). Ce jalon ajoute le **bornage** et l'**inertie**
(fling), façon Flutter.

## Décisions techniques

- **Bornage pur et testable.** `InteractiveView::clamped(viewport)` contraint la
  translation pour que le contenu (à l'échelle courante) **couvre** toujours la
  fenêtre : on ne peut pas tirer un bord du contenu à l'intérieur. À l'échelle 1 le pan
  est nul (le contenu remplit exactement) ; sous 1 (dézoom) le contenu, plus petit, est
  **centré**. Appliqué après chaque pan, chaque zoom, et chaque frame de fling.

- **Inertie décélérée dans le runtime.** Une carte `interactive_velocity` (px/s) porte
  l'élan du pan relâché ; `Runtime::advance_interactive(viewports, dt)` déplace la
  translation, la **borne** (toucher un bord annule la vitesse de cet axe — pas de
  rebond), applique une **friction exponentielle** et s'arrête sous un seuil. Piloté
  frame par frame comme l'inertie de défilement, avec les fenêtres de la frame courante.

- **Gestes shell.** `Drag::Pan` suit désormais une **vitesse lissée** (moyenne
  exponentielle) et la **fenêtre** (bornage) ; le relâchement en mouvement amorce le
  fling (au-delà d'un seuil). Un nouvel appui **ou** un zoom **coupe** le fling en cours
  (on reprend la main). Le zoom molette est lui aussi borné.

## Implémentation

- `frus-widgets` : `InteractiveView::clamped` (+ constantes `PAN_FRICTION` /
  `PAN_MIN_VELOCITY`) ; `Runtime::interactive_velocity` + `advance_interactive` ;
  `Ui::interactive_bounds`.
- `frus-shell` : `Drag::Pan` enrichi (vitesse lissée, `last_t`, `viewport`) ; bornage du
  pan et du zoom ; amorçage du fling au relâchement ; appel per-frame
  `advance_interactive` (agrégé à l'inertie de défilement) ; l'appui/zoom coupe le fling.

## Tests

- `clamped` (purs) : le pan est **annulé** à l'échelle 1 ; borné au bord quand zoomé
  (le contenu couvre toujours) ; **centré** quand dézoomé.
- `advance_interactive` (runtime) : un fling **décélère, s'arrête** et reste **borné**
  (la vitesse est nettoyée au repos).
- Workspace complet vert : frus-widgets 231 (+4 : 3 bornage + 1 fling), frus-core 91.

## Reste

- **`boundaryMargin`** configurable (slack au-delà du cadre, dépassement élastique) —
  ici le bornage est **strict** (marge 0, défaut de Flutter).
- **Pincement 2 doigts** (tactile), une fois le multi-touch en place.
- Double-tap pour zoomer/réinitialiser (raccourci usuel).

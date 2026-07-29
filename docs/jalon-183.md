# Jalon 183 — Indicateur `Steps` : marqueurs cliquables

## Analyse

`Steps` (jalon 182) affichait la progression d'un assistant mais restait **passif** : on ne
pouvait pas cliquer un marqueur pour **revenir** à une étape déjà visitée (revoir/corriger). Le
`Stepper` de Material rend ses en-têtes d'étape cliquables (`onStepTapped`) — c'était le manque.

## Décisions techniques

- **Superposition de zones cliquables, rendu intact.** `Steps` s'auto-peint (connecteurs,
  marqueurs, libellés) sans enfants — je ne voulais pas casser ce rendu au pixel. `on_tap` ajoute
  **une** rangée de « hotspots » **transparents** (un par étape, taille d'un marqueur) posée par
  dessus : elle ne dessine rien mais capte clic et focus clavier. Le golden `form_wizard` reste
  donc **identique** (vérifié sans régénération).

- **Alignement exact par `SpaceBetween`.** Les hotspots sont une `Flex::row().justify(SpaceBetween)`
  de boîtes de diamètre `MARKER_D`. Sur toute la largeur, `SpaceBetween` place le centre de la
  boîte `i` en `R + i·(W − 2R)/(n − 1)` — **exactement** la formule `center_x` des marqueurs
  peints. Les zones cliquables coïncident donc pile avec les ronds, sans coordonnées codées en
  dur ni second calcul à maintenir.

- **`Steps` devient générique.** Porter un `on_tap(|usize| Msg)` impose un `Steps<Msg>` (au lieu
  du `impl<Msg> Widget for Steps` non générique du jalon 182). Chaque hotspot est un widget privé
  `Hotspot { label, message }` : `on_click` émet l'index, `focusable`, sémantique `Role::Button`
  (le libellé de l'étape). Sans `on_tap`, `children` est **vide** → aucun surcoût, comportement
  du jalon 182 conservé.

## Implémentation

- `steps.rs` : `Steps<Msg>` (+ champ `children`), builder `on_tap` (construit la rangée de
  hotspots), `center_x` déplacé dans un `impl<Msg>` sans borne `'static` (appelé depuis `paint`),
  widget privé `Hotspot`.
- Rendu (`paint`) et `center_x` inchangés : la géométrie est partagée entre marqueurs peints et
  hotspots.

## Vérification

- **Unitaire** : `on_tap_overlays_clickable_hotspots` — sans `on_tap`, aucun enfant ; avec, une
  rangée de trois hotspots dont chacun émet `Msg::Go(i)` et est focalisable. Tests du jalon 182
  (`current_is_clamped_to_last`, `markers_reflect_progress`) **verts**.
- **Golden** `form_wizard` **inchangé** (test repassé sans `FRUS_UPDATE_GOLDENS`) : la
  superposition n'altère pas le rendu.
- Doctest `Steps` (annoté `Steps<()>`) **vert**.

## Reste

- **Verrouiller les étapes futures** : n'autoriser le saut que vers les étapes déjà atteintes
  (l'application filtre déjà en choisissant les `Msg` émis, mais un mode intégré serait pratique).
- **Orientation verticale** (étapes empilées, contenu sous l'étape courante) — toujours en
  extension (cf. jalon 182).

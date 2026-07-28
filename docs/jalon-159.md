# Jalon 159 — Réordonnancement : coulissement des colonnes voisines

## Analyse

L'aperçu (jalons 155/158) soulevait un fantôme fidèle, mais les colonnes **restaient
figées** : rien ne montrait la place s'ouvrir. Il manquait l'effet « `ReorderableListView` »
— les voisines qui **s'écartent** pour ménager le dépôt et **referment** le trou de la
colonne saisie.

L'obstacle (déjà noté aux jalons précédents) : le shell **ne connaît pas** l'appartenance
colonne → widgets (une colonne = un en-tête + N cellules, chacune un `WidgetId` distinct).
Faire coulisser « la colonne 2 » depuis le shell semblait exiger cette cartographie.

## Décisions techniques

- **Réagencement purement géométrique.** Plutôt qu'une cartographie, on **reclasse les
  primitives** de la scène par leur **centre en x** : la colonne **source** (soulevée) est
  retirée, les colonnes entre source et cible sont **translatées d'un cran** (largeur de la
  source) pour combler le trou et ouvrir la place. En-têtes **et** cellules de données
  coulissent ensemble (même bande en x) — l'effet complet, sans structure supplémentaire.

- **Garde-fou anti-fond.** Une primitive plus large que ~1,5 colonne (fond de page, surlignage
  de ligne) est **laissée en place** : on ne déplace pas un arrière-plan entier. Le texte
  (non mesuré dans frus-core) est repéré par sa **position** (`bounds()` ponctuel), suffisant
  pour le classement en x.

- **Utilitaire partagé et pur.** `frus_widgets::reflow_reorder_columns(prims, src, target,
  to_right, lifted_owner)` : **fonction pure** sur des primitives, appelée par le shell **et**
  par le golden (aucune duplication), testable sans GPU. Le fantôme reste peint par le shell
  (`draw_ghost_card`) par-dessus la scène réagencée ; l'indicateur/estompe deviennent inutiles
  (le trou réel les remplace).

- **Briques frus-core.** `Primitive::bounds()` (boîte englobante, `Path` via ses points) et
  `Rect::union`.

## Implémentation

- `scene.rs` / `geometry.rs` (frus-core) : `Primitive::bounds()`, `Rect::union`.
- `reorder.rs` (frus-widgets) : `reflow_reorder_columns` (+ tests) ; export.
- `app.rs` (shell) : `paint_reorder_preview` réagence la scène (`reflow_reorder_columns`)
  puis peint la carte fantôme ; `draw_reorder_overlay` → `draw_ghost_card` (ombre + face
  fidèle + bord).
- `goldens.rs` : `table_reorder_preview` reconstruit le réagencement (source retirée, « Score »
  coulissé, fantôme « Role »).

## Vérification

- **Unitaire** (`reflow_reorder_columns`, sans GPU) : glissé à **droite** → colonne source
  retirée, voisines coulissées de **−1 cran** (col 1 → 0, col 2 → 100), fond large **conservé** ;
  glissé à **gauche** → coulissement de **+1 cran**. `draw_ghost_card` : repli plein = 2 primitives.
- **Golden** `table_reorder_preview` **inspecté** : « Role » soulevé (retiré, ses données
  disparues), « Score » (5 / 3) **coulissé** à la place de « Role », **trou** ouvert à droite,
  **carte « Role »** flottante au curseur. Effet de coulissement complet.
- `cargo test --workspace` **vert**, sans avertissement.

## Reste

- **Interpolation temporelle** (easing) du coulissement : aujourd'hui le réagencement **suit
  le curseur** (il bascule d'une colonne cible à l'autre) sans transition douce ; un tween
  demanderait un état d'animation par colonne.
- **Opacité du fantôme** (< 1) via `Primitive::Layer { opacity }`.

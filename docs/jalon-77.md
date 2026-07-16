# Jalon 77 — `frus-test` : rendu headless, snapshots et goldens (ouverture du §13)

## Analyse

Le §13 du cahier d'idées identifie la **DX de test** comme facteur d'adoption.
L'infra existait déjà en pièces détachées : chaque test GPU recopiait ~90
lignes de harnais offscreen (device headless, texture, readback). Flutter
packagé ça en `flutter_test` (`matchesGoldenFile`) — on suit ce pas.

## Architecture

- **`frus-gpu::render_offscreen(scene, w, h, clear) -> Option<OffscreenFrame>`**
  (nouveau module public `offscreen.rs`) : LE pipeline de la fenêtre — quads +
  quads de décoration + glyphes, cible **sRGB** (les octets relus = ce qu'une
  capture d'écran donnerait), relecture avec rembourrage 256 octets (largeurs
  arbitraires). `None` sans adaptateur GPU. Le harnais dupliqué des tests de
  `text.rs` est remplacé par un appel (−90 lignes).
- **`frus-test`** (nouveau crate, hors pyramide de prod : ← gpu + widgets) :
  - [`Snapshot`] : `pixel(x,y)`, `lit_pixels(seuil)`, `diff_count(other, tol)` ;
  - [`render_scene`] et **[`render_widget`]** — ce dernier fait ce que ferait
    le shell : `build_ui` (layout taffy + peinture au thème) dans une fenêtre
    virtuelle, état retenu neutre ;
  - **goldens** : `assert_golden(path)` compare à un PNG de référence.
    Absent → créé (à relire puis committer) ; `FRUS_UPDATE_GOLDENS=1` →
    régénéré ; écart → panique en écrivant `<nom>.actual.png` à côté.

## Décisions

- Les goldens dépendent du rasteriseur (AA du texte) : générer et comparer
  **dans le même environnement** (ici llvmpipe/WSL, déterministe — vérifié :
  deux exécutions successives, 0 diff). Tolérance paramétrable
  (`assert_golden_with(path, tol_canal, max_pixels)`).
- Dépendance `png` (pure Rust) **dans le crate de test uniquement** — la
  pyramide de prod reste inchangée.
- L'étage 1 du §13 (« `update` pur = tests triviaux ») ne demande aucun outil :
  c'est l'architecture Elm elle-même ; documenté en tête de crate.
- `*.actual.png` gitignorés (artefacts d'échec).

## Tests (266 → 269)

- `renders_rect_and_reads_back_srgb` (gpu) : chemin offscreen public, largeur
  non alignée (70 px) → rembourrage exercé, pixels exacts.
- `scene_matches_golden` : rect arrondi + texte **souligné** → golden commité
  (double preuve visuelle du jalon 75).
- `widget_tree_matches_golden` : arbre Container/Flex/Text (dont un
  `strikethrough`) rendu via `build_ui` + thème sombre → golden commité.
- `diff_count_is_exact` : 0 sur rendus identiques, 1 sur un pixel corrompu,
  absorbé par la tolérance max.

## Suite du chantier §13 (dans l'ordre de valeur)

1. Inspector runtime (dump diagnostique en overlay) ;
2. hot-reload préservant l'état (`subsecond`/`hot-lib-reloader`, l'état Elm
   étant une struct unique sérialisable) ;
3. template `cargo new` (`cargo generate`) pour démarrer une app frus.

# Jalon 123 — Vitrine enrichie : Clip + InteractiveViewer

## Analyse

Trois jalons de rendu (J120 tests au pixel, J121 découpe en forme, J122
`InteractiveViewer`) se sont empilés **sans jamais être affichés**. Ce jalon les rend
*tangibles* : la vitrine `frus-transforms` gagne une galerie **découpe en forme** et
une **fenêtre interactive** — de quoi *voir* (et manipuler) la découpe et le
pan/zoom, au-delà des tests headless.

## Décisions techniques

- **Découpe visible par contraste.** Un carré **dégradé à coins nets** est rogné en
  `ClipRRect(24)` puis en `ClipOval` : la différence avec les coins d'origine rend la
  découpe évidente d'un coup d'œil.

- **Fenêtre interactive détaillée.** Une grille de pastilles sur fond dégradé remplit
  une fenêtre `260×180` bornée `0.5×`–`4×`. Glisser la déplace, la molette y zoome
  (ancrée au curseur) ; à fort zoom le contenu déborde et est **découpé au cadre**.
  Encadrée d'un `Container` arrondi pour lire le cadre.

- **`view` toujours pure.** Les ajouts ne touchent pas le modèle Elm : mêmes `update`
  déterministe, souscription en pause à l'arrêt, `view` fonction pure de l'état. La
  transformation de la fenêtre vit dans le `Runtime` (état retenu), pas dans l'app.

- **Conventions.** Constructeurs de structs (`ClipRRect::new`, `ClipOval::new`,
  `InteractiveViewer::new`) ; textes d'interface en **anglais**.

## Implémentation

- `crates/frus-transforms/src/lib.rs` : imports `ClipRRect` / `ClipOval` /
  `InteractiveViewer` ; `gallery3` (deux tuiles de découpe) ; `viewer` (fenêtre
  interactive + contenu grille) ; câblage dans la colonne défilante avec en-têtes ; le
  titre passe à « Transform · Clip · InteractiveViewer · AspectRatio ».

## Correctif découvert au rendu

Le rendu hors écran de la vitrine a révélé un **bug de mise en page** de
`InteractiveViewer` (J122) : tout **frère placé après** la fenêtre se superposait.
Cause : la fenêtre n'était pas déclarée **feuille de layout** dans `build_layout`
(contrairement à `Scroll`), donc son sous-arbre restait dans les rectangles de la
colonne — et comme la marche pose ce sous-arbre **à part** (index séparé), l'index
principal se désynchronisait pour tous les frères suivants. Corrigé en ajoutant
`interactive()` à la liste des feuilles ; régression verrouillée par
`sibling_after_viewer_keeps_its_layout_position` (le frère suit bien la fenêtre de
150 px, sans superposition).

## Tests

- `renders_clip_shapes` : la `view` émet bien un `ClipShape::RRect(24)` **et** un
  `ClipShape::Oval` (collecte récursive des calques).
- `sibling_after_viewer_keeps_its_layout_position` (frus-widgets) : verrouille le
  correctif de mise en page ci-dessus.
- Les garde-fous existants tiennent : un calque transformé est émis, et le contenu est
  **posé dans la fenêtre** (anti-page-blanche). Suites vertes : `frus-transforms` 7,
  `frus-widgets` 222.

## Voir / lancer

- Bureau : `cargo run -p frus-transforms` — puis **glisser** dans la fenêtre
  interactive et **molette** pour zoomer ; observer les tuiles de découpe.
- Android : APK via `cargo-apk` (mêmes métadonnées que `frus-hello`).

## Reste

- Vérification **sur device réel** (desktop + Android) — l'objectif *voir* : découpe
  nette, pan/zoom fluides, hit-test qui suit.
- Pincement 2 doigts (tactile) une fois le multi-touch en place.

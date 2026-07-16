# Jalon 62 — `TextSpan` : texte riche, de l'arbre stylé au GPU

Suite du fil typographique (§5). Le jalon 60 a posé `TextStyle` et le rendu de la
graisse/l'italique ; celui-ci apporte le **texte riche** : plusieurs styles mêlés
dans un même paragraphe, mis en forme d'un seul tenant (une seule ligne de base).

## `TextSpan` : l'arbre stylé à héritage en cascade (frus-core)

`TextSpan` = un fragment de texte + des surcharges **partielles** + des enfants.
Le point clé : un enfant `.bold()` **hérite** la taille et la couleur de son parent
— ce qu'une fusion de `TextStyle` complets ne sait pas exprimer (elle écraserait la
taille). Les surcharges partielles vivent dans un type interne (`Overrides`, chaque
champ optionnel) composé via les builders : `.bold()`, `.weight()`, `.italic()`,
`.size()`, `.color()`, `.style(TextStyle)` (surcharge complète).

`flatten(base)` aplati l'arbre en **runs résolus** `(texte, TextStyle)`, en ordre de
lecture, en cascadant depuis le style de base du paragraphe. Les nœuds sans texte
propre (« groupes » de style) ne produisent pas de run.

## Le pipeline, de bout en bout

- **`TextRun`** (frus-core) : run prêt à rendre — texte + taille/graisse/italique/
  couleur **résolus**.
- **`Primitive::RichText { position, runs, … }`** + `Scene::rich_text` ;
  `scaled` met les tailles des runs à l'échelle, `push_faded` fond leurs couleurs.
- **`frus-gpu`** : un seul buffer cosmic-text par paragraphe, via
  **`set_rich_text`** — chaque run porte ses `Attrs` (graisse, italique,
  **métriques par-span** pour les tailles mêlées, **couleur par-span** que glyphon
  applique par glyphe). Métriques de base = le plus grand run.
- **`frus-text::measure_runs`** : mesure du texte riche shapé (largeur de la plus
  longue ligne, hauteur réelle `line_top + line_height` — les tailles mêlées
  comptent).

## `RichText` : le widget paragraphe (frus-widgets)

`RichText::new(span).base_style(theme.text.body_large)` — le style de base est la
racine de la cascade ; les couleurs héritées sont tranchées contre le thème **au
paint** (et modulées par l'opacité). Taille naturelle mesurée par `measure_runs`
(pas de retour à la ligne automatique pour l'instant, comme `Text`).

Démo : la ligne d'accroche de l'écran About mêle gras, italique et un segment
coloré (`no GC` en `theme.primary`) dans une seule phrase.

## Validation

- **Preuve GPU de bout en bout** : `renders_rich_text_to_non_background_pixels` —
  rendu offscreen + readback d'un paragraphe à runs mêlés (40 px normal + 24 px
  gras) sur un vrai device wgpu ; le harnais de readback est factorisé et partagé
  avec le test de texte simple.
- Cascade : 3 tests frus-core (héritage des attributs non précisés, cascade en
  profondeur, nœuds-groupes sans run) + doctest.
- Mesure riche : `rich_runs_measure_mixed_styles` (plus large avec un segment
  gras 24 px ; hauteur pilotée par le plus grand run ; vide → zéro).
- Widget : runs résolus contre le thème, gras hérité, couleur explicite ; la
  hauteur de layout suit le plus grand run.
- **226 tests** au total, tout vert (core 52, widgets 138, gpu 5, text 4, demo 15,
  shell 7, layout 3) ; build sans avertissement ; démo sans panique.

## Suite (§5 texte)

- **`TextLayout`** sur cosmic-text : `hit_test`/`caret_rect`/`selection_rects` et
  intrinsèques min/max → la brique pour migrer `TextInput` et, à terme, le
  paragraphe à retour à la ligne (mesure sous contrainte, closures de mesure taffy).
- Décorations (souligné/barré), `letter_spacing`/`line_height` dans `TextStyle`.

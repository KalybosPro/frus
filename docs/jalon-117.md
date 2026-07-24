# Jalon 117 — `Transform` : matrice affine unifiée

## Analyse

Unification des transformations de `Transform`. Jusqu'ici l'échelle passait par un
post-traitement **par primitive** (aligné sur les axes) et la rotation par un
**calque composité** ; les composer approximait (pivots hors-centre) et l'échelle non
uniforme fudgeait rayons/texte/chemins. On fond désormais échelle **et** rotation en
**une seule matrice affine 2×3** (`Affine`), portée par le calque composité — la
transformation exacte de tout un sous-arbre, sans approximation de composition.

## Décisions techniques

- **`Affine` dans `frus-core`** : matrice 2×3 (`[a, b, c, d, e, f]`) avec
  `translation` / `scale` / `rotation`, composition `then`, `about(pivot)`, `apply`
  et `inverse`. Le type unifié des transformations de peinture.

- **`LayerTransform` porte une `Affine`** (au lieu d'un couple angle/pivot). Le
  compositeur calcule l'**inverse** et le fragment échantillonne la texture à la
  position contre-transformée `M⁻¹(p)` — une seule passe pour n'importe quelle affine
  (échelle par axe, rotation, cisaillement, composition).

- **Une seule passe dans le walk.** Le sous-arbre est peint **à plat** ; échelle
  (autour de son pivot) et rotation (autour du sien) sont composées en `M`, et le tout
  est enveloppé dans un calque `transform = M`. Le post-traitement par primitive de
  l'échelle (J113/J115) disparaît.

- **Hit-test par matrice inverse.** Les cibles de clic portent `M⁻¹` (au lieu d'un
  couple rotation) ; le point de test lui est appliqué. Exact pour échelle **et**
  rotation composées, dans le bon ordre.

- **Ce que ça lève** : composition exacte des pivots hors-centre ; échelle non
  uniforme correcte (le contenu à plat est étiré par la texture au compositing, plus
  de fudge sur rayons/texte/chemins).

## Compromis

- **L'échelle passe maintenant par le GPU** (comme la rotation) : son rendu **n'est
  plus vérifiable sans GPU** (les primitives restent à plat dans le calque). Les tests
  vérifient donc la **matrice** du calque et le **hit-test** (matrice inverse), tous
  deux sans GPU. La correction du fragment reste validée par construction.
- **Focus / défilement / glisser / accessibilité** dans un sous-arbre transformé : ces
  rectangles restent **non transformés** (une matrice générale ne peut pas les garder
  alignés sur les axes). Les **clics** restent exacts (matrice inverse) ; l'anneau de
  focus et les bornes d'accessibilité apparaissent à la position non transformée.

## Implémentation

- `frus-core/geometry.rs` : type `Affine` (+ export). `frus-core/scene.rs` :
  `LayerTransform` enveloppe une `Affine` (`rotation`, `scaled`/`translated` par
  conjugaison).
- `frus-gpu` : `LayerComposite`/`CompInstance` portent l'inverse affine (6 flottants) ;
  `composite.wgsl` applique `M⁻¹` à `frag_px`.
- `frus-widgets/ui.rs` : le bloc de transformation compose `M` et enveloppe le
  sous-arbre dans un calque `M` ; `Hit::xform` devient `Option<Affine>` (inverse),
  `contains` l'applique.

## Tests

- `frus-core` : `affine_composes_scale_then_rotate_about_a_pivot`,
  `affine_inverse_round_trips` (aller-retour `M⁻¹∘M`).
- `frus-widgets` : les tests d'échelle/rotation vérifient la **matrice** du calque
  (partie linéaire, point fixe) ; `rotate_hit_test_counter_rotates_the_point` valide le
  hit-test par matrice inverse ; `scale_and_rotate_compose` vérifie la fusion en une
  matrice `rotation ∘ échelle`.
- Suites vertes : frus-core 90, frus-gpu 16, frus-widgets 211 ; workspace complet vert.

## Reste

- Transformer aussi les rectangles de **focus / a11y** sous une affine **alignée sur
  les axes** (échelle/translation pure) pour lever ce compromis dans le cas courant.
- Une démo animée rassemblant l'arsenal (`Tween` pilotant un `Transform` composé).

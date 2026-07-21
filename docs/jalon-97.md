# Jalon 97 — `AnimatedContainer` : couleur de fond animée

## Analyse

J95 a rendu les animations implicites **courbées & configurables** ; J96 a livré
l'opacité de groupe animée. Suite annoncée : des `Animated*` interpolant **d'autres
propriétés** (couleur/taille/padding), façon `AnimatedContainer` de Flutter.

La couleur de fond est la propriété **phare** — et, contrairement à taille/padding,
elle est **layout-free** : l'interpoler n'exige aucune intégration dans la mise en
page (qui, elle, se calcule *avant* l'animation). C'est donc le premier `Animated*`
multi-canal, propre et borné.

## Décisions techniques

- **Retenue par nœud, tweenée par canal.** Le runtime garde une [`ColorAnim`]
  `{ current, from, to, elapsed }` par widget et la fait tendre vers la cible via
  `advance_colors`, sur le **même modèle** que la valeur scalaire de J95 (rebase
  au changement de cible, snap au montage, courbe/durée du widget). L'interpolation
  se fait **canal par canal** (`Color::lerp`).

- **Livrée au paint via `Status`, en gardant `Status: Copy`.** Le statut transporte
  désormais `anim_color: Option<Color>` (`Color` est `Copy` — pas de `Vec`, donc
  `Status` reste `Copy`, aucune rupture aux sites d'appel de `paint`). La marche y
  place la couleur interpolée (`Runtime::anim_color(id)`).

- **`Container` API** (idiome J96) : `.animated_color(color, duration, curve)`.
  Le trait gagne `Widget::anim_color() -> Option<Color>` (cible), tweené par le
  runtime. Au paint, un fond animé **prime** sur l'interpolation survol/pressé
  (une couleur animée est la couleur). Opacité et couleur d'une même boîte
  **partagent** une `(durée, courbe)` (simplicité ; on anime rarement les deux).

- **Câblage shell** : `advance_colors(tree, dt)` rejoint la chaîne d'avancement
  par frame (aux côtés de `advance_values`), donc le fondu progresse et
  redemande des frames tant qu'il bouge.

## Pourquoi pas taille/padding (encore)

Animer une propriété **de disposition** exige la valeur interpolée **au moment du
layout** (taffy lit `style()`), donc d'injecter l'animation *avant* la peinture —
une intégration plus profonde dans `build_ui`. La couleur, purement picturale,
s'anime sans y toucher. Taille/padding : jalon dédié.

## Implémentation

- `frus-widgets` : `Runtime` (`ColorAnim`, `colors`, `anim_color`,
  `advance_colors`) ; trait `anim_color()` + forwarders (`Box`/`Keyed`/
  `Responsive`) ; `Status::anim_color` ; `Container.animated_color` + paint ;
  `ui::full_status` livre la couleur.
- `frus-shell` : `advance_colors` dans la boucle d'animation.

## Tests

- `animated_color_tweens_between_frames` (runtime) : snap au montage (rouge), tween
  linéaire rouge→bleu (mi-parcours ≈ `(0.5, 0, 0.5)`), fin au bleu, oubli du widget
  disparu.
- `animated_color_paints_the_interpolated_color` (scène) : après avancement à
  mi-parcours, le **rectangle de fond peint** porte la couleur interpolée (chaîne
  runtime → `Status` → paint → scène).
- Suites existantes vertes : le chemin est inerte sans `animated_color`.

## Reste

- Propriétés de **disposition** animées (taille/padding/radius) via injection au
  layout ; `Tween` typés génériques.
- Widgets `Opacity`/`AnimatedOpacity`/`AnimatedContainer` **nommés** (sucre au-
  dessus de `Container`).

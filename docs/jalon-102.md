# Jalon 102 — `AnimatedContainer` : marge (padding) animée

## Analyse

Dernière propriété « signature » d'`AnimatedContainer` à couvrir : la **marge
intérieure**. Comme la taille (J98), c'est une propriété de **disposition** : la
marge interpolée doit entrer **au layout** (pour replacer le contenu), pas au
paint. Elle emprunte donc le même point d'injection — [`effective_style`], déjà
partagé par `build_layout` et l'empreinte du cache de relayout, garantissant leur
cohérence (le cache s'invalide tant que la marge bouge).

## Décisions techniques

- **Timeline par côté.** Le runtime garde une [`PaddingAnim`]
  `{ current, from, to, elapsed }` par nœud, tweenée par `advance_paddings` sur le
  **même modèle** que taille/couleur/rayon. Interpolation **côté par côté**
  (`Insets` = 4 marges).

- **Cible = marge *effective* (contenu + bordure).** `Container::style()` réserve
  déjà la place de la bordure dans le padding de mise en page. Pour ne pas
  **perdre** cette réserve quand `effective_style` remplace le padding par la
  valeur animée, la cible (`Widget::anim_padding`) est la marge **effective** —
  extraite dans un `Container::effective_padding()` unique, source à la fois de
  `style()` et de la cible animée. (La bordure étant constante, interpoler la marge
  effective revient à interpoler la marge de contenu, réserve incluse.)

- **`Container::animated_padding(padding, duration, curve)`** (uniforme). Au paint,
  rien ne change ; c'est le layout qui bouge. Toutes les animations d'une boîte
  (opacité/couleur/taille/rayon/marge) partagent une `(durée, courbe)`.

## Implémentation

- `frus-widgets` : `Runtime` (`PaddingAnim`, `paddings`, `anim_padding`,
  `advance_paddings`, `lerp_insets`) ; trait `anim_padding()` + forwarders ;
  `Container` (`padding_anim`, `effective_padding()`, `.animated_padding`,
  `anim_padding()` + chaîne durée/courbe) ; `ui::effective_style` injecte aussi
  `style.padding`.
- `frus-shell` : `advance_paddings` dans la boucle d'animation.

## Tests

- `animated_padding_tweens_between_frames` (runtime) : snap au montage (0), tween
  linéaire 0→20 (mi-parcours ≈ 10 par côté), oubli du widget disparu.
- `animated_padding_insets_the_child_at_layout` (layout) : à mi-parcours, le fond
  de l'enfant est **décalé de ~10** — preuve que la marge interpolée entre bien au
  layout (chaîne runtime → `effective_style` → taffy → rects).
- `visible_border_reserves_layout_padding` (existant) reste vert : le refactor
  `effective_padding()` préserve la réserve de bordure.
- Suite complète verte (widgets 193).

## Bilan `AnimatedContainer` (complet)

| Propriété | Chemin        | Jalon |
|-----------|---------------|-------|
| opacité   | calque (GPU)  | J96   |
| couleur   | paint/`Status`| J97   |
| taille    | layout/`effective_style` | J98 |
| rayon     | paint/`Status`| J99   |
| **marge** | **layout/`effective_style`** | **J102** |

Toutes portées par la même timeline courbée (J95), exposées aussi via le widget
nommé `AnimatedContainer` (J100).

## Reste

- `alignment`/`margin` externes, `decoration` composite (parité Flutter fine).
- `Tween` typés génériques ; démo dédiée d'animations.

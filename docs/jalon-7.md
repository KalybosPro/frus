# Jalon 7 — Style : coins arrondis, bordures, alignements, marges par côté

Enrichit le rendu et la mise en page pour des UI réalistes.

## Ce qui est livré

- **Coins arrondis + bordures** via un **SDF** dans le fragment shader
  (anti-aliasing du bord, anneau de bordure). Une seule passe, aucune géométrie
  supplémentaire.
- **Alignements flex** : `Justify` (axe principal) et `Align` (axe croisé),
  mappés vers taffy.
- **Marges par côté** : type `Insets` (haut/droite/bas/gauche) ;
  `Style.padding` est désormais un `Insets`.
- **Primitive enrichie** : `Primitive::Rect { rect, color, radius, border_width,
  border_color }` + `Scene::draw_rect` (`fill_rect` conservé pour le cas simple).
- **API widgets** : `Container::radius/.border/.padding_each`,
  `Flex::justify/.align/.padding_each`.

## Le shader (SDF)

```
d = sdf_round_box(local_px, half_size, radius)   // distance signée (négatif à l'intérieur)
alpha = 1 - smoothstep(-0.5, 0.5, d)             // couverture anti-aliasée ~1px
color = mix(fill, border, smoothstep(-bw-0.5, -bw+0.5, d))  // anneau de bordure
→ vec4(color.rgb, color.a * alpha)
```

Le vertex transmet au fragment la position locale (px depuis le centre) et, en
`flat`, la demi-taille, le rayon, l'épaisseur et les couleurs.

## Décisions

- **SDF plutôt que géométrie** : arrondis et bordures « gratuits » côté
  géométrie (toujours un quad), tout se joue dans le fragment.
- `Style.padding: Insets` : le padding par côté sans casser `.padding(f32)`
  (uniforme).
- Défauts inchangés : `justify = Start`, `align = Stretch` (comportement
  identique aux jalons précédents).

## Démo

En-tête (compteur) **centré**, bouton **arrondi et bordé** avec padding par côté,
**centré** horizontalement ; les carrés deviennent des **cartes arrondies**.

## Tests

- `frus-core` : `fill_rect` reste radius 0 / sans bordure ; `draw_rect` stocke
  radius/bordure.
- `frus-gpu` : non-régression (rect plein rouge, radius 0 → centre rouge) **et**
  `rounded_rect_leaves_corner_transparent` (un fort rayon découpe le coin).
- `frus-layout` : `justify_center_centers_child` (enfant centré sur l'axe
  principal).

## Limites (prochains jalons)

- Pas d'ombres, ni de dégradés, ni de découpage (clipping) des enfants.
- Toujours pas de saisie/focus clavier ni de diff de sous-arbres.

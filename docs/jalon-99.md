# Jalon 99 — `AnimatedContainer` : rayon de coin animé

## Analyse

Après la taille (J98, layout) et la couleur (J97, paint), la dernière propriété
« signature » d'`AnimatedContainer` : le **rayon des coins**. Comme la couleur,
c'est une propriété **picturale** (elle ne touche pas la disposition), donc elle
suit le même chemin léger que la couleur — livrée au paint via `Status`, sans
toucher au layout ni à son cache.

## Décisions techniques

- **Timeline par coin.** Le runtime garde une [`RadiusAnim`]
  `{ current, from, to, elapsed }` par nœud, tweenée par `advance_radii` sur le
  **même modèle** que couleur/taille (rebase au changement, snap au montage,
  courbe/durée du widget). Interpolation **coin par coin** (`BorderRadius` = 4
  rayons).

- **Livraison au paint, `Status` reste `Copy`.** `Status::anim_radius:
  Option<BorderRadius>` (`BorderRadius` est `Copy`) — comme `anim_color`, aucun
  `Vec`, donc `Status` demeure `Copy` et aucun site d'appel de `paint` ne casse.
  La marche y place le rayon interpolé (`Runtime::anim_radius(id)`).

- **`Container` API** : `.animated_radius(radius, duration, curve)` — uniforme via
  `f32` ou par coin via [`BorderRadius`] (comme `.radius`). Au paint, un rayon
  animé **prime** sur le rayon fixe. Toutes les animations d'une même boîte
  (opacité/couleur/taille/rayon) partagent une `(durée, courbe)`.

## Implémentation

- `frus-widgets` : `Runtime` (`RadiusAnim`, `radii`, `anim_radius`,
  `advance_radii`, `lerp_radius`) ; trait `anim_radius()` + forwarders
  (`Box`/`Keyed`/`Responsive`) ; `Status::anim_radius` ; `Container.animated_radius`
  + paint ; `ui::full_status` livre le rayon.
- `frus-shell` : `advance_radii` dans la boucle d'animation.

## Tests

- `animated_radius_tweens_between_frames` (runtime) : snap au montage (0), tween
  linéaire 0→20 (mi-parcours ≈ 10 par coin), oubli du widget disparu.
- `animated_radius_paints_the_interpolated_radius` (scène) : à mi-parcours, le
  **rectangle de fond peint** porte le rayon interpolé (~10) — chaîne runtime →
  `Status` → paint → scène.
- Suites existantes vertes : chemin inerte sans `animated_radius`.

## Bilan `AnimatedContainer`

Les quatre propriétés « signature » de Flutter sont désormais animables sur
`Container`, par la même infrastructure de timeline courbée (J95) :

| Propriété | Chemin        | Jalon |
|-----------|---------------|-------|
| opacité   | calque (GPU)  | J96   |
| couleur   | paint/`Status`| J97   |
| taille    | layout/`effective_style` | J98 |
| rayon     | paint/`Status`| J99   |

## Reste

- Padding/marge animés (injection au layout, comme la taille).
- Widgets **nommés** `AnimatedContainer`/`Opacity`/`AnimatedOpacity` (sucre au-
  dessus de `Container`).
- `Tween` typés génériques ; animations pilotées explicitement (contrôleur).

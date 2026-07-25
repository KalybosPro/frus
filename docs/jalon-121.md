# Jalon 121 — Découpe en forme : `ClipRRect` / `ClipOval`

## Analyse

Jusqu'ici la découpe était **rectangulaire** : chaque primitive porte un `clip: Rect`
testé au fragment (`quad.wgsl`, `composite.wgsl`, …). Il manquait la brique
structurante de Flutter `ClipRRect` / `ClipOval` : rogner un sous-arbre à une **forme**
(coins arrondis, ellipse) — un avatar rond, une vignette à coins doux dont l'image
épouse exactement l'arrondi, une pastille circulaire.

## Décisions techniques

- **Réutiliser le calque, pas de nouvelle passe.** Un `Primitive::Layer` isole déjà un
  sous-arbre sur une texture puis le composite (le mécanisme `saveLayer` / opacité /
  transformation). On lui ajoute une **forme de découpe** [`ClipShape`] (`Rect` /
  `RRect(radius)` / `Oval`), inscrite dans le rectangle `clip` du calque. Seul le
  **shader de compositing** change — `quad`/`image`/`path`/`text` sont intacts.

- **Couverture par distance signee, anticrénelée.** Le fragment de `composite.wgsl`
  calcule la couverture de la forme : rectangle net (kind 0, comportement d'origine
  inchangé), rectangle arrondi via `sd_rounded_box` (kind 1), ellipse inscrite via une
  distance approchee gradient-normalisee (kind 2). Les formes courbes sont adoucies sur
  ~1 px (`smoothstep`), donc les bords sont propres à toute échelle.

- **Passe-plat en mise en page.** `ClipRRect` / `ClipOval` prennent la taille que le
  parent leur donne (comme leur enfant) ; la forme est **inscrite** dans cette boîte,
  le rayon borné à la demi-plus-petite dimension. Aucun impact sur les frères.

- **La forme suit les transformations.** [`ClipShape::scaled_xy`] met le rayon à
  l'échelle (DPI, `Primitive::scaled`) ; la translation laisse rayon/ellipse
  invariants (seul le rectangle `clip` bouge). L'arrondi reste donc correct sous un
  changement de densité.

- **Coût nul quand inutile.** `ClipShape::Rect` est le défaut : les calques existants
  (opacité, transformation) l'émettent et retombent exactement sur l'ancien test
  rectangulaire.

## Implémentation

- `frus-core` : enum [`ClipShape`] (+ `Default`, `scaled_xy`) ; champ `clip_shape` sur
  `Primitive::Layer` (propagé par `scaled_xy` / `translated` / `push_faded`) ; export.
- `frus-gpu` : `LayerComposite` / `CompInstance` portent `shape: [kind, radius, _, _]`
  (4e attribut d'instance) ; `composite.wgsl` calcule la couverture SDF.
- `frus-widgets` : nouveau module `clip` — `ClipRRect<Msg>` (rayon) et `ClipOval<Msg>` ;
  méthode `Widget::clip_shape()` (défaut `None`) forwardée par `Box<dyn>`, `Keyed`,
  `Responsive`, les wrappers animés ; la marche (`ui.rs`) enveloppe le sous-arbre dans
  un calque porteur de la forme (comme le groupe d'opacité). Ré-export de `ClipShape`.

## Tests

- `frus-widgets` (`clip`) : le bon `ClipShape` est émis, l'enfant est peint **dans** le
  calque, la découpe est **passe-plat** (le frère garde sa position).
- `frus-test` (`clip.rs`, **au pixel**, non ignorés) : `RRect(16)` **gomme les coins**
  du carré en gardant centre et milieux de bord ; `Oval` **garde le disque inscrit** et
  gomme les coins ; `RRect(0)` **dégénère** en rectangle plein (garde-fou). Rendus sur
  le rasteriseur logiciel — le shader applique bien la forme.
- Workspace complet vert : frus-core 90, frus-gpu 16, frus-widgets 215, frus-test
  clip 3 + transforms 4.

## Reste

- Découpe **arrondie par coin** (`BorderRadius` non uniforme) et `ClipPath` (chemin
  arbitraire) — même mécanisme, forme plus riche.
- `InteractiveViewer` (pan + pinch-zoom) au-dessus de la pile de transformation.

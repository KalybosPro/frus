# Jalon 127 — `ClipPath` : découpe à un chemin arbitraire (pipeline de masque)

## Analyse

Le clipping ne couvrait que des formes **analytiques** (rect / rrect / ellipse), testées
par SDF au fragment. La découpe à un **chemin quelconque** (étoile, pointe, bulle,
forme libre) n'est pas exprimable ainsi : c'est le chantier de masque annoncé en J125.
Ce jalon l'ajoute — la famille clipping est complète.

## Décisions techniques

- **Masque de couverture, pas de stencil.** Pour un `ClipShape::Path`, le compositor
  **rend le chemin en blanc** dans une texture pleine surface (réutilise le pipeline de
  chemins existant, `render_group`) : c'est le **masque**. Le fragment de compositing
  échantillonne son alpha et le **multiplie** à la couverture du calque. Bords
  anticrénelés gratuits (le remplissage du chemin est déjà MSAA). Aucun stencil, aucune
  duplication de pipeline.

- **Un seul point de branchement.** `composite.wgsl` gagne une 2ᵉ texture (le masque) ;
  hors `ClipPath`, on lie un masque **neutre 1×1 blanc** → multiplication par 1, sans
  effet, sans branche. Les formes analytiques (rect/rrect/oval) sont inchangées.

- **Chemin en coordonnées locales, décalé à l'écran.** Le widget `ClipPath::new(path)`
  reçoit le chemin en coordonnées **locales** (origine au coin de la boîte) ; la marche
  le translate à la position écran (comme un `ClipRRect`, passe-plat en mise en page).
  Prioritaire sur `clip_shape` via une méthode dédiée `Widget::clip_path()`.

- **`ClipShape` n'est plus `Copy`** (il porte un `Path`, `Vec`-adossé). Ripple contenu :
  quelques `*clip_shape` → `.clone()`. `scaled_xy` met le chemin à l'échelle (DPI).

## Implémentation

- `frus-core` : variante `ClipShape::Path(Path)` (+ `scaled_xy`), `Copy` retiré.
- `frus-gpu` : `render_mask` (chemin blanc → texture) ; `LayerComposite`/bind-group
  gagnent le masque (binding 2) ; masque neutre 1×1 blanc pour les calques sans chemin ;
  `composite.wgsl` échantillonne et multiplie ; kind 3 = chemin.
- `frus-widgets` : widget `ClipPath<Msg>` ; méthode `Widget::clip_path()` forwardée
  (`Box<dyn>`, `Keyed`, `Responsive`, animés) ; branche de découpe de la marche unifiée
  (chemin prioritaire, sinon forme analytique). Ré-exports `Path` / `PathVerb`.

## Tests

- `frus-test` (au pixel, GPU réel) : `path_clip_masks_to_the_shape` — un losange
  découpe le carré (centre et sommets peints, **coins gommés**). Les formes analytiques
  (rrect par coin, oval, rect) tiennent toujours (masque neutre).
- `frus-widgets` : `ClipPath` émet un calque `ClipShape::Path` **décalé à l'écran**.
- Rendu visuel (hors commit) : étoile 5 branches + triangle découpant un dégradé, bords
  nets. Workspace complet vert : frus-core 92, frus-gpu 16, frus-widgets 233, frus-test
  clip 5.

## Reste

- **Clipper dépendant de la taille** (façon `CustomClipper` de Flutter : une closure
  `Size → Path`) — ici le chemin est fixe en coordonnées locales.
- **Cache du masque** (re-rendu chaque frame pour l'instant ; à indexer comme les
  textures de calque si un `ClipPath` s'avère coûteux et statique).

# Jalon 90 — Images & textures

## Analyse

Après les chemins vectoriels (jalon 89), il manquait la seconde brique
fondatrice du moteur graphique : les **images bitmap**. Aucune vraie app ne s'en
passe (avatars, vignettes, illustrations, héros). Jusqu'ici `Avatar` ne savait
afficher que des initiales, faute de pipeline de textures.

Ce jalon ajoute la **gestion de textures** de bout en bout : téléversement GPU
mis en cache, échantillonnage, ajustement (`BoxFit`) et un widget `Image`. Le
**décodage** (PNG/JPEG) est volontairement laissé à une couche fine ultérieure —
la partie difficile et structurante est la gestion mémoire GPU des textures, pas
le format de fichier.

## Architecture

```
frus-core                        frus-gpu                      frus-widgets
─────────                        ────────                      ────────────
ImageData (id, rgba)  ── Primitive::Image ──► ImagePainter     Image
ImageHandle = Arc                            (cache textures    (BoxFit, tint)
BoxFit::apply → (dst, uv)                     par id, sampler,       │
Scene::draw_image / image                     quads texturés)   Scene::draw_image
                                             shaders/image.wgsl
```

### `frus-core` — le modèle (`image.rs`)
- `ImageData { id, width, height, rgba }` : pixels **RGBA sRGB** bruts, immuables.
  Chaque instance reçoit un **id unique** (compteur atomique) = clé de cache GPU.
  `PartialEq` compare **par identité** (pas les pixels) → égalité de scène et
  cache bon marché.
- `ImageHandle = Arc<ImageData>` : handle **partagé** (clone = incrément de
  compteur), stocké tel quel dans la primitive.
- `BoxFit` (`Fill · Contain · Cover · FitWidth · FitHeight · None · ScaleDown`) :
  `apply(src, dst) -> (rect, uv)`. **Letterbox** → rect rétréci + UV plein ;
  **rognage** (`Cover`) → rect plein + UV réduit et centré.
- `Primitive::Image { image, rect, uv, tint, clip, owner }` intégré aux passes
  transverses : `owner()`, `scaled()` (met le `rect`/`clip` à l'échelle, l'UV
  reste en `0..1`), `push_faded()` (fond de sortie : alpha de la teinte).
- `Scene::draw_image` (bas niveau : rect + uv + teinte) et `Scene::image`
  (ajustement automatique par `BoxFit`).

### `frus-gpu` — le rendu (`image.rs` + `shaders/image.wgsl`)
- **Cache de textures** `HashMap<id, texture>` : chaque image est téléversée
  **une seule fois** (format `Rgba8UnormSrgb`, `write_texture`), réutilisée
  d'une frame à l'autre. Les textures **non employées** lors d'une frame sont
  **évincées** (marquage `used` + `retain`), bornant la mémoire.
- **Pipeline texturé** : quad instancié, deux groupes de liaison — viewport
  (uniforme) et texture+échantillonneur. Un dessin par image (la texture liée au
  dessin), UV/teinte/découpe portés par l'instance. Échantillonneur **linéaire**
  (clamp).
- Le shader projette px→NDC, échantillonne, multiplie par la teinte (linéarisée)
  et découpe au fragment — mêmes conventions sRGB que `quad.wgsl`.
- Câblé dans **le renderer fenêtré et le hors-écran**, ordre
  `rectangles → images → chemins → texte`.

### `frus-widgets` — le widget `Image`
Boîte de taille fixe, ajustée par `BoxFit` (défaut `Contain`), **teinte**
optionnelle (icônes bitmap, fondu par opacité). Re-exporte `ImageData`,
`ImageHandle`, `BoxFit` pour les applications.

## Décisions techniques

- **Handle partagé + cache par id**, plutôt que pixels dans la primitive :
  clone bon marché, zéro re-téléversement, égalité de scène en O(1). L'id (et non
  le pointeur `Arc`) sert de clé — robuste à la réutilisation d'adresses.
- **`BoxFit` en `frus-core`, pas côté GPU** : l'ajustement est de la géométrie
  pure (testable sans GPU) ; le shader ne fait qu'échantillonner un `(rect, uv)`.
- **Un dessin par image** (pas d'atlas) pour cette fondation : simple et correct.
  Le batch par texture / atlas viendra si le profil le justifie.
- **Décodage différé** : la brique dure est la gestion des textures GPU ; les
  décodeurs (PNG/JPEG via `image`) sont une couche fine à ajouter ensuite, sans
  alourdir `frus-core` (zéro-dép) ni le temps de compilation ici.

## Explications & limites

- **Pas de décodage de fichiers** ce jalon : on part de `ImageData` bruts
  (pixels générés ou fournis). PNG/JPEG = prochain incrément.
- **Pas de mipmaps** (filtrage linéaire simple) : suffisant à l'échelle 1:1 ;
  un downscale fort pourra scintiller. Mips + `BoxFit::Cover` fin plus tard.

## Tests

- `frus-core` : identités uniques/stables & égalité par identité ; `BoxFit`
  (`Fill`/`Contain`/`Cover` : rect + uv calculés).
- `frus-gpu` (readback GPU, preuve pixel) : `samples_a_texture_by_quadrant` — une
  image 2×2 (R/G/B/W) étirée sur la surface ; chaque quadrant relit sa couleur
  (téléversement + échantillonnage + UV + aller-retour sRGB validés).
- `frus-widgets` : `Image` émet une primitive `Image` (letterbox `Contain`
  correct), la teinte surchargée est appliquée, la taille pilote la boîte.
- Aucune régression : les widgets existants n'émettent pas d'image → goldens et
  suites inchangés.

## Démo

La carte principale affiche une **image bitmap générée** (dégradé 64×64, créée
une fois via `OnceLock` et mise en cache par le renderer) à côté de la rangée
d'icônes, ajustée en `BoxFit::Cover`.

## Note — réparation d'un merge cassé

Un commit de merge externe (`bbea003 « Conflicts resolved »`, fusionnant une
branche divergente) avait **dupliqué** du code dans 4 fichiers (résolution de
conflit gardant les deux côtés), cassant la compilation : `Cargo.lock`
(dépendance en double), `widget.rs` (impl `Box` dupliquée), `textinput.rs`
(bloc de défilement dupliqué appelant un `prefix_width` inexistant) et
`app.rs` (module `clip` dupliqué). Les quatre ont été rétablis sur la version de
la lignée jalon 89 (celle qui compile), et le lockfile régénéré.

## Reste

- Décodage PNG/JPEG (couche fine sur `image`).
- Mipmaps & filtrage anisotrope ; atlas / batch par texture.
- `Avatar` sur image réelle ; `BoxDecoration` avec image de fond.

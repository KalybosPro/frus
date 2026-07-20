# Jalon 93 — Anti-aliasing (MSAA)

## Analyse

Depuis J89 (chemins vectoriels) et J90 (images), la géométrie oblique — bords de
triangles, arcs de cercles, icônes, bords d'images tournées — était **crénelée** :
un pixel appartenait au tracé ou pas, sans demi-teinte. C'est la dette
d'anti-aliasing explicitement différée « faisable sur la base du compositing »
(cf. *Reste* de [jalon-92.md](jalon-92.md)). Le compositing par calques ayant
posé une architecture de rendu **unifiée** ([`Painters::render`]), tout le
pipeline passe par un point unique où brancher le multi-échantillon.

Approche retenue : **MSAA** (multisample anti-aliasing) matériel — le GPU
échantillonne la couverture de chaque primitive à N sous-positions par pixel puis
**résout** (moyenne) vers l'image finale. C'est ce que fait un moteur 2D avant de
recourir à des techniques analytiques (SDF, coverage) plus lourdes ; c'est le
choix par défaut, correct et bon marché sur tout GPU.

## Décisions techniques

- **4× si supporté, sinon 1 (désactivé).** [`preferred_sample_count`] interroge
  `adapter.get_texture_format_features(format)` : si `sample_count_supported(4)`,
  on prend 4× ; sinon 1 (pas de MSAA, comportement inchangé). 4× est le meilleur
  compromis qualité/coût et le plus universellement disponible — **y compris le
  rasteriseur logiciel llvmpipe** de l'environnement de test/CI (confirmé : les
  readbacks montrent bien des bords lissés).

- **`sample_count` propagé à *tous* les pipelines.** Un render pass et les
  pipelines qui y dessinent doivent partager le **même** nombre d'échantillons.
  Le compte est donc passé à la construction de chaque painter (rectangles,
  images, chemins, texte via glyphon, composite) et injecté dans leur
  `MultisampleState`. Un seul point de vérité : [`Painters::new`].

- **Texture MSAA intermédiaire + résolution.** On rend dans une texture
  multi-échantillon ([`MsaaScratch`], `RENDER_ATTACHMENT` seul) puis on **résout**
  vers la cible mono-échantillon via `resolve_target` de l'attachement couleur —
  pour la passe principale (→ surface / texture de relecture) **comme** pour
  chaque pré-passe de calque (→ sa texture échantillonnée par le compositeur).
  Une **seule** texture MSAA est réutilisée : toutes les passes sont pleine
  surface et s'exécutent en *submits* séquentiels, jamais simultanément.

- **Cache de la texture MSAA.** Recréée uniquement quand la taille ou le format
  change (resize) ; sinon réutilisée frame après frame. La *vue* est créée à la
  volée et renvoyée **par valeur** (les `TextureView` de wgpu ne sont pas
  `Clone`), ce qui libère l'emprunt de `self` avant l'ouverture du render pass.

## Architecture

```
Pour chaque calque :  contenu ─▶ MSAA scratch (4×) ──resolve──▶ texture calque (1×)
Passe principale :     contenu + composite ─▶ MSAA scratch (4×) ──resolve──▶ cible (1×)
```

Sans support MSAA (`sample_count == 1`), les deux passes peignent directement
dans leur cible mono-échantillon (`resolve_target: None`) — chemin identique à
avant ce jalon.

## Limites assumées

- **`clear == None` (peindre par-dessus) non pris en charge sous MSAA** : la cible
  multi-échantillon ne contient pas le contenu existant de la cible finale. Tous
  les appelants actuels effacent (`Some(_)`), donc sans effet pratique ; à
  traiter si un mode « surimpression » apparaît.
- 4× fixe (pas encore réglable ni 2×/8× selon le GPU) ; suffisant et sûr.

## Implémentation

- `frus-gpu` :
  - `painter.rs`, `image.rs`, `path.rs`, `text.rs` : `new(..., sample_count)` →
    `MultisampleState { count, .. }` (glyphon reçoit le même compte).
  - `compositor.rs` : `MSAA_SAMPLES = 4`, [`preferred_sample_count`],
    [`MsaaScratch`] + `Painters::ensure_msaa`, câblage `resolve_target` dans
    `render` et `render_group`.
  - `renderer.rs` : compte choisi depuis l'adaptateur (log `MSAA : N×`).
  - `offscreen.rs` : `headless_device` renvoie aussi le compte ; `OffscreenFrame`
    expose `samples` (informe les tests).

## Tests

- `frus-gpu` (readback GPU, preuve pixel) : nouveau
  `msaa_smooths_a_diagonal_edge` — le bord **oblique** d'un triangle vert produit
  des pixels de vert **intermédiaire** (ni 0, ni 255), impossibles avec un rendu
  net ; l'intérieur reste plein vert, l'extérieur au fond. S'ignore proprement si
  `samples == 1` (GPU sans MSAA).
- Les readbacks existants (rect, triangle, contour, texture, calque) restent
  verts : ils échantillonnent **loin** des bords, insensibles au lissage.
- **Goldens** : `scene_rect_text` (rect arrondi + texte) et `widget_column_text`
  (texte) régénérés — leurs bords courbes sont désormais lissés (deltas d'octets
  minimes). Les goldens à **bords droits** (`rtl_row`, `rtl_drawer`,
  `inspector_overlay`) sont **inchangés** : une arête axis-alignée sur la grille
  n'a pas de couverture partielle — preuve que le changement est localisé au
  lissage et non une régression.

## Reste

- MSAA réglable (2×/4×/8× selon le GPU et un budget qualité).
- Anti-aliasing **analytique** pour le texte/les chemins fins (SDF, coverage) là
  où le MSAA 4× reste insuffisant.
- Réutilisation des textures de calque entre frames (le gain « perf » toujours en
  attente, cf. [jalon-92.md](jalon-92.md)).

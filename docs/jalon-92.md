# Jalon 92 — Compositing par calques & précompilation des pipelines

## Analyse

Deux manques, tous deux différés de jalons précédents :

1. **Compositing par calques** (différé de J88). Jusqu'ici, fondre un sous-arbre
   se faisait **primitive par primitive** (`push_faded` multiplie l'alpha de
   chacune). Là où des primitives se **chevauchent**, l'alpha se cumule → le
   chevauchement fonce (double-superposition). C'est incorrect pour une opacité
   **de groupe** (un panneau, un dialogue, une transition qui s'estompe d'un
   bloc). Flutter résout ça avec `saveLayer`/`Opacity` : rendre le groupe à part
   sur une couche, puis composer la couche entière à l'opacité voulue.

2. **Précompilation des shaders** (le pari « anti-jank » de J89). Skia/Flutter
   compilent des variantes de shader **à la première utilisation** → micro-gels
   (le motif qui a motivé Impeller).

## Décisions techniques

- **Jeu de pipelines fixe, créé au démarrage.** frus n'a qu'une poignée de
  pipelines (rect, image, chemin, texte, composite), tous créés à
  [`Painters::new`]. Il n'y a **pas** de variantes compilées à la volée. Pour
  garantir que la première vraie frame ne paie **rien**, [`Painters::warm_up`]
  rend au démarrage une petite scène qui exerce **chaque** chemin (rect, image,
  chemin, texte **et** un calque → composite), forçant la finalisation pilote.
  → Zéro « shader jank » au premier rendu, par construction.

- **Calque = rendu-vers-texture + recomposition.** Un [`Primitive::Layer`] porte
  sa propre liste de primitives. Le compositeur le rend **d'abord** sur une
  texture pleine surface (fond transparent), dans une **passe séparée avec son
  propre *submit*** — indispensable pour ne pas aliaser les buffers d'instances
  partagés entre passes (une écriture de buffer ne s'applique qu'au *submit*
  suivant ; réutiliser les mêmes painters entre *submits* distincts est correct,
  entre passes d'un même *submit* ne l'est pas). Le [`CompositePainter`] recompose
  ensuite la texture d'un bloc à l'opacité de groupe (quad plein écran, découpe
  au fragment, alpha `= échantillon.a × opacité`). L'échantillon d'une texture
  sRGB étant déjà linéaire, aucune reconversion.

- **Regroupement des painters** (`compositor.rs`). Les quatre painters de contenu
  + le composite sont réunis dans un [`Painters`] avec une méthode `render`
  unique (calques compris), désormais partagée par le renderer fenêtré **et** le
  rendu hors-écran — la duplication entre les deux a disparu.

## Architecture

```
Scene (contient des Primitive::Layer)
   │
   ├─ pour chaque Layer : render_group → texture pleine surface  (submit séparé)
   │                        (rect+image+chemin+texte, fond transparent)
   ▼
Passe principale (1 submit) :
   rect → image → chemin → texte → composite(chaque texture de calque @ opacité)
```

Ordre/limites assumés :
- Les calques sont **composités au-dessus** du contenu principal (comme le texte
  est toujours au-dessus des rectangles) : un calque ne peut passer *sous* une
  primitive émise après lui. Suffisant pour les cas d'usage (groupes de premier
  plan) ; un tri en passe unique viendra si besoin.
- **Calques imbriqués** non recompositionnés (un `Layer` dans un `Layer` est
  ignoré à ce niveau) — fondation d'abord.
- Texture de calque **pleine surface** (coordonnées absolues, alignement
  trivial) : simple et correct ; un cadrage/pooling optimisera plus tard.

## Implémentation

- `frus-core` : `Primitive::Layer { primitives, opacity, clip, owner }` intégré
  aux passes transverses — `owner()`, `scaled()` (recurse dans les enfants),
  `push_faded()` (multiplie l'opacité de groupe). Constructeur `Scene::layer(op,
  |inner| …)` qui bâtit une sous-scène.
- `frus-gpu` : `compositor.rs` (`Painters` + `CompositePainter`) +
  `shaders/composite.wgsl` ; `renderer.rs` et `offscreen.rs` délèguent à
  `Painters::render` ; `warm_up` appelé à la construction du `Renderer`.

## Tests

- `frus-core` : un calque capture ses sous-primitives + opacité + découpe ;
  fondre un calque **multiplie** son opacité ; `scaled` transforme les enfants.
- `frus-gpu` (readback GPU, preuve pixel) :
  `layer_group_opacity_is_uniform_over_overlap` — deux rectangles **opaques** qui
  se chevauchent, en calque à 0.5 : le chevauchement a **exactement** la même
  couleur qu'une simple couverture (alpha de groupe uniforme, pas de
  double-superposition), et c'est bien ~50 % rouge sur fond noir.
- Toutes les suites existantes passent **par le nouveau `Painters::render`** (les
  readbacks rect/chemin/image/texte transitent par le compositeur) — aucune
  régression, goldens inchangés.

## Démo

Une tuile `CustomPaint` ajoute deux carrés d'accent qui se chevauchent, groupés
en **calque à 0.55** — le chevauchement ne fonce pas, illustrant l'opacité de
groupe correcte.

## Reste

- **Réutilisation** des textures de calque entre frames (cache GPU keyé par
  contenu, façon frontière de repaint côté GPU) — le gain « perf » du pari.
- Anti-aliasing (MSAA) — orthogonal, désormais faisable sur cette base.
- Calques `transform`/`clip` généraux ; tri en passe unique ; calques imbriqués ;
  intégration d'un widget `Opacity` dans le walk (comme `RepaintBoundary`).

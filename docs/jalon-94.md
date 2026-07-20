# Jalon 94 — Réutilisation GPU des textures de calque

## Analyse

J92 a introduit les calques ([`Primitive::Layer`]) : chaque calque est rendu sur
une texture pleine surface (pré-passe : *submit* + tessellation + dessin) puis
composité. Jusqu'ici cette pré-passe était **refaite à chaque frame**, même pour
un calque **statique** — gaspillage direct, le pari « perf » explicitement
différé (cf. *Reste* de [jalon-92.md](jalon-92.md) et [jalon-93.md](jalon-93.md)).

C'est l'équivalent GPU de la **frontière de repaint** (J88, cache de peinture
côté CPU) : tant que le contenu d'un calque ne change pas, sa texture peut être
**réutilisée telle quelle**.

## Décisions techniques

- **Clé = rang du calque + égalité de contenu.** L'`owner` des primitives vaut 0
  par défaut (peu fiable), on indexe donc les calques par leur **rang** dans la
  scène. `Primitive` dérive `PartialEq`, d'où une comparaison **exacte** du
  contenu (`Vec<Primitive>`) frame à frame. Point clé de sûreté : une clé qui
  « glisse » (calques réordonnés/insérés) ne fait que **rater** le cache → une
  pré-passe correcte est refaite, **jamais un pixel faux**.

- **Cache = texture conservée entre frames.** [`CachedLayer`] garde la texture
  (mono-échantillon, résolue depuis le MSAA — elle est déjà échantillonnable),
  l'instantané de son contenu et ses dimensions. Réutilisation si contenu **et**
  taille (resize) inchangés ; sinon (re)rendue. Les calques disparus sont purgés
  (`truncate` au nombre de calques de la frame).

- **Économie réelle.** Un hit saute **toute** la pré-passe : pas de nouveau
  *submit*, pas de tessellation, pas d'écriture de buffers, pas de dessin — on ne
  refait qu'une `TextureView` (négligeable) sur la texture déjà en VRAM.

## Implémentation

`frus-gpu/compositor.rs` : [`CachedLayer`] + champs `layer_cache` / `layer_renders`
dans [`Painters`] ; la boucle de calques de `render` passe par un nouveau
`layer_texture(index, primitives, w, h)` (hit → réutilise, miss → `render_group`
+ mémorise) ; `truncate` des calques disparus. Aucun changement de pixel : le
cache renvoie exactement la texture qu'aurait produite un re-render.

## Tests

- `frus-gpu` : `static_layer_texture_is_reused_across_frames` — via un compteur
  de pré-passes rendues (`layer_render_count`), à travers **un même** `Painters` :
  1ʳᵉ frame → 1 rendu ; 2ᵉ frame (calque inchangé) → **toujours 1** (réutilisé) ;
  contenu changé → 2 (re-render) ; calque retiré → cache **purgé**.
- Le reste de la suite est **inchangé** (le cache ne modifie aucun pixel) —
  goldens compris, confirmant l'absence de régression visuelle.

## Reste

- **Rendre dans la texture cachée existante** lors d'un re-render à dimensions
  identiques (éviter la réallocation pour un calque *animé*).
- Invalidation plus fine qu'un `Vec<Primitive>::eq` (hash de contenu) si le coût
  de comparaison devient sensible sur de gros calques.
- Cible Web (wasm + WebGPU) ; MSAA réglable / AA analytique (cf. J93).

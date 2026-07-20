# Jalon 91 — Décodage d'images (PNG/JPEG)

## Analyse

Le jalon 90 a posé la gestion de textures GPU, mais ne savait partir que de
**pixels bruts** (`ImageData::from_rgba`). Une vraie app charge ses images depuis
des **fichiers** (`logo.png`, `photo.jpg`). Il manquait donc le décodeur — la
couche fine qui transforme des octets de fichier en `ImageData`.

Le choix d'architecture du jalon 90 était délibéré : `frus-core` reste
**zéro-dép** (il ne détient que des pixels). Le décodeur (crate `image`, avec ses
dépendances : `png`, `jpeg-decoder`…) est isolé dans un crate **optionnel**, pour
que `frus-core`/`frus-widgets` n'en héritent pas.

## Architecture

```
             octets (PNG/JPEG)
                   │
   frus-image::decode  (crate `image`, formats png+jpeg)
                   │  détection au format, → RGBA8
                   ▼
   frus_core::ImageData ──► (jalon 90) texture GPU en cache
```

Nouveau crate **`frus-image`** (dépend de `frus-core` + `image`), au même niveau
que `frus-text` dans le graphe de dépendances. Une seule fonction publique :

```rust
pub fn decode(bytes: &[u8]) -> Result<ImageData, DecodeError>;
```

- **Format détecté aux octets magiques** (pas d'extension requise).
- Toute image est convertie en **RGBA8 sRGB** (le format qu'attend le renderer).
- `DecodeError` masque le type d'erreur du crate `image` (API stable, découplée).

Le crate `image` est configuré `default-features = false, features =
["png", "jpeg"]` pour **limiter l'arbre de dépendances** aux deux formats visés.

## Décisions techniques

- **Crate séparé plutôt que dans `frus-core`/`frus-widgets`.** Le décodeur est
  lourd (formats, zlib…) et n'est pas requis partout : une app qui ne fait que du
  dessin procédural ne doit pas le payer. Les apps qui chargent des ressources
  dépendent explicitement de `frus-image`. `frus-core` garde son invariant
  zéro-dép.
- **`decode(bytes)` seul**, pas de lecture de fichier ni de réseau : le *quoi*
  (octets) est fourni par l'app (`include_bytes!`, `std::fs::read`, un
  téléchargement…). Le crate ne fait qu'une chose. Un `ImageProvider` (asset /
  réseau / mémoire, façon Flutter) pourra se poser au-dessus plus tard.
- **Asset de démo à provenance reproductible.** Le PNG committé n'est pas un
  binaire opaque : l'exemple `frus-image/examples/gen_logo.rs` le régénère
  (`cargo run -p frus-image --example gen_logo > crates/frus-demo/assets/logo.png`).

## Tests

- `png_round_trips_pixels_exactly` : encode une image 4×3 connue → décode →
  dimensions et pixels **exacts** (coins rouge/vert).
- `jpeg_decodes_with_correct_dimensions` : JPEG (avec perte) → dimensions et
  taille de tampon correctes (format détecté).
- `format_is_detected_from_magic_bytes` : en-tête PNG reconnu sans indice.
- `garbage_bytes_error_cleanly` : octets invalides → `Err` avec message.
- Doctest : aller-retour encode→décode d'une 2×2.

## Démo

`demo_image()` charge désormais un **PNG embarqué décodé**
(`decode(include_bytes!("../assets/logo.png"))`), au lieu du dégradé généré du
jalon 90 (conservé en **repli** si le décodage échoue). L'image est décodée une
fois (`OnceLock`) puis mise en cache par identité côté renderer.

## Reste

- Autres formats (WebP, GIF, …) : ajouter des features du crate `image`.
- `ImageProvider` : abstraction asset/réseau/mémoire + chargement **asynchrone**
  (via `Command`) pour ne pas décoder sur le thread UI.
- Mipmaps (jalon 90) pour un downscale propre des grandes photos.

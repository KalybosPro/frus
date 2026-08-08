# Milestone 90 — Images & textures

## Analysis

After the vector paths (milestone 89), the graphics engine's second founding
brick was missing: **bitmap images**. No real app does without them (avatars,
thumbnails, illustrations, heroes). Until now `Avatar` could only show initials,
for want of a texture pipeline.

This milestone adds **texture management** end to end: cached GPU upload,
sampling, fitting (`BoxFit`) and an `Image` widget. **Decoding** (PNG/JPEG) is
deliberately left to a thin later layer — the hard, structuring part is the GPU
memory management of the textures, not the file format.

## Architecture

```
frus-core                        frus-gpu                      frus-widgets
─────────                        ────────                      ────────────
ImageData (id, rgba)  ── Primitive::Image ──► ImagePainter     Image
ImageHandle = Arc                            (texture cache     (BoxFit, tint)
BoxFit::apply → (dst, uv)                     by id, sampler,        │
Scene::draw_image / image                     textured quads)   Scene::draw_image
                                             shaders/image.wgsl
```

### `frus-core` — the model (`image.rs`)
- `ImageData { id, width, height, rgba }`: raw **RGBA sRGB** pixels, immutable.
  Each instance gets a **unique id** (an atomic counter) = the GPU cache key.
  `PartialEq` compares **by identity** (not the pixels) → cheap scene equality and
  caching.
- `ImageHandle = Arc<ImageData>`: a **shared** handle (a clone is a refcount
  bump), stored as-is in the primitive.
- `BoxFit` (`Fill · Contain · Cover · FitWidth · FitHeight · None · ScaleDown`):
  `apply(src, dst) -> (rect, uv)`. **Letterboxing** → a shrunk rect + full UV;
  **cropping** (`Cover`) → a full rect + a reduced, centred UV.
- `Primitive::Image { image, rect, uv, tint, clip, owner }`, integrated into the
  cross-cutting passes: `owner()`, `scaled()` (scales the `rect`/`clip`, the UV
  staying in `0..1`), `push_faded()` (exit fade: the tint's alpha).
- `Scene::draw_image` (low level: rect + uv + tint) and `Scene::image` (automatic
  fitting through `BoxFit`).

### `frus-gpu` — the rendering (`image.rs` + `shaders/image.wgsl`)
- **A texture cache** `HashMap<id, texture>`: each image is uploaded **once**
  (format `Rgba8UnormSrgb`, `write_texture`) and reused from frame to frame.
  Textures **not used** during a frame are **evicted** (a `used` mark + `retain`),
  bounding the memory.
- **A textured pipeline**: an instanced quad, two bind groups — viewport (a
  uniform) and texture+sampler. One draw per image (the texture bound to the
  draw), with UV/tint/clip carried by the instance. A **linear** sampler (clamp).
- The shader projects px→NDC, samples, multiplies by the (linearised) tint and
  clips in the fragment — the same sRGB conventions as `quad.wgsl`.
- Wired into **the windowed renderer and the offscreen path**, in the order
  `rectangles → images → paths → text`.

### `frus-widgets` — the `Image` widget
A fixed-size box, fitted by `BoxFit` (default `Contain`), with an optional
**tint** (bitmap icons, opacity fading). It re-exports `ImageData`, `ImageHandle`
and `BoxFit` for applications.

## Technical decisions

- **A shared handle + a cache by id**, rather than pixels in the primitive: a
  cheap clone, zero re-uploading, O(1) scene equality. The id (not the `Arc`
  pointer) is the key — robust against address reuse.
- **`BoxFit` in `frus-core`, not on the GPU side**: fitting is pure geometry
  (testable without a GPU); the shader only samples a `(rect, uv)`.
- **One draw per image** (no atlas) for this foundation: simple and correct.
  Batching by texture / an atlas will come if profiling justifies it.
- **Decoding deferred**: the hard brick is GPU texture management; the decoders
  (PNG/JPEG through `image`) are a thin layer to add next, without weighing down
  `frus-core` (zero-dependency) or the compile time here.

## Explanations & limits

- **No file decoding** in this milestone: we start from raw `ImageData`
  (generated or supplied pixels). PNG/JPEG = the next increment.
- **No mipmaps** (plain linear filtering): sufficient at 1:1 scale; a heavy
  downscale may shimmer. Mips + a refined `BoxFit::Cover` later.

## Tests

- `frus-core`: unique and stable identities & equality by identity; `BoxFit`
  (`Fill`/`Contain`/`Cover`: the computed rect + uv).
- `frus-gpu` (GPU readback, the pixel proof): `samples_a_texture_by_quadrant` — a
  2×2 image (R/G/B/W) stretched over the surface; each quadrant reads back its own
  colour (upload + sampling + UV + the sRGB round trip all validated).
- `frus-widgets`: `Image` emits an `Image` primitive (correct `Contain`
  letterboxing), an overridden tint is applied, the size drives the box.
- No regression: the existing widgets emit no image → goldens and suites
  unchanged.

## Demo

The main card shows a **generated bitmap image** (a 64×64 gradient, created once
through a `OnceLock` and cached by the renderer) next to the row of icons, fitted
with `BoxFit::Cover`.

## Note — repairing a broken merge

An external merge commit (`bbea003 "Conflicts resolved"`, merging a divergent
branch) had **duplicated** code in 4 files (a conflict resolution keeping both
sides), breaking the build: `Cargo.lock` (a duplicated dependency), `widget.rs`
(a duplicated `Box` impl), `textinput.rs` (a duplicated scrolling block calling a
`prefix_width` that no longer exists) and `app.rs` (a duplicated `clip` module).
All four were restored to the milestone-89 lineage's version (the one that
compiles), and the lockfile regenerated.

## What's left

- PNG/JPEG decoding (a thin layer over `image`).
- Mipmaps & anisotropic filtering; an atlas / batching by texture.
- `Avatar` on a real image; `BoxDecoration` with a background image.

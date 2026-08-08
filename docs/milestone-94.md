# Milestone 94 — GPU reuse of layer textures

## Analysis

J92 introduced layers ([`Primitive::Layer`]): each layer is rendered onto a
full-surface texture (a pre-pass: *submit* + tessellation + drawing) and then
composited. Until now that pre-pass was **redone every frame**, even for a
**static** layer — direct waste, and the "perf" bet explicitly deferred (see
*What's left* in [milestone-92.md](milestone-92.md) and [milestone-93.md](milestone-93.md)).

It is the GPU equivalent of the **repaint boundary** (J88, the CPU-side paint
cache): as long as a layer's content does not change, its texture can be **reused
as it is**.

## Technical decisions

- **The key = the layer's rank + content equality.** The primitives' `owner` is 0
  by default (unreliable), so layers are indexed by their **rank** in the scene.
  `Primitive` derives `PartialEq`, giving an **exact** content comparison
  (`Vec<Primitive>`) from frame to frame. The key safety point: a key that
  "slips" (layers reordered or inserted) can only **miss** the cache → a correct
  pre-pass is redone, **never a wrong pixel**.

- **The cache = a texture kept between frames.** [`CachedLayer`] keeps the texture
  (single-sample, resolved from the MSAA — so it is already sampleable), the
  snapshot of its content and its dimensions. It is reused if both the content
  **and** the size (a resize) are unchanged; otherwise it is (re)rendered. Layers
  that have gone are purged (`truncate` to the frame's layer count).

- **A real saving.** A hit skips the **whole** pre-pass: no new *submit*, no
  tessellation, no buffer writes, no drawing — only a `TextureView` is recreated
  (negligible) over the texture already in VRAM.

## Implementation

`frus-gpu/compositor.rs`: [`CachedLayer`] + the `layer_cache` / `layer_renders`
fields in [`Painters`]; `render`'s layer loop goes through a new
`layer_texture(index, primitives, w, h)` (hit → reuse, miss → `render_group` +
remember); `truncate` for layers that have gone. No pixel change: the cache
returns exactly the texture a re-render would have produced.

## Tests

- `frus-gpu`: `static_layer_texture_is_reused_across_frames` — through a counter
  of rendered pre-passes (`layer_render_count`), across **the same** `Painters`:
  1st frame → 1 render; 2nd frame (the layer unchanged) → **still 1** (reused);
  content changed → 2 (a re-render); the layer removed → the cache **purged**.
- The rest of the suite is **unchanged** (the cache modifies no pixel) — goldens
  included, confirming there is no visual regression.

## What's left

- **Rendering into the existing cached texture** on a re-render at identical
  dimensions (avoiding a reallocation for an *animated* layer).
- Finer invalidation than a `Vec<Primitive>::eq` (a content hash) if the
  comparison cost becomes noticeable on large layers.
- The Web target (wasm + WebGPU); adjustable MSAA / analytic AA (see J93).

# Milestone 93 — Anti-aliasing (MSAA)

## Analysis

Since J89 (vector paths) and J90 (images), oblique geometry — triangle edges,
circle arcs, icons, the edges of rotated images — was **jagged**: a pixel either
belonged to the path or it did not, with no half-tone. This is the anti-aliasing
debt explicitly deferred as "feasible on the compositing base" (see *What's left*
in [milestone-92.md](milestone-92.md)). Layer compositing having established a **unified**
render architecture ([`Painters::render`]), the whole pipeline now goes through a
single point where multisampling can be plugged in.

The approach chosen: hardware **MSAA** (multisample anti-aliasing) — the GPU
samples each primitive's coverage at N sub-positions per pixel and then
**resolves** (averages) into the final image. It is what a 2D engine does before
reaching for heavier analytic techniques (SDF, coverage); it is the default
choice, correct and cheap on any GPU.

## Technical decisions

- **4× if supported, otherwise 1 (disabled).** [`preferred_sample_count`] queries
  `adapter.get_texture_format_features(format)`: if `sample_count_supported(4)`,
  we take 4×; otherwise 1 (no MSAA, unchanged behaviour). 4× is the best
  quality/cost compromise and the most universally available — **including the
  llvmpipe software rasteriser** of the test/CI environment (confirmed: the
  readbacks do show smoothed edges).

- **`sample_count` propagated to *all* the pipelines.** A render pass and the
  pipelines that draw into it must share the **same** sample count. So the count
  is passed when each painter is constructed (rectangles, images, paths, text
  through glyphon, composite) and injected into their `MultisampleState`. A single
  source of truth: [`Painters::new`].

- **An intermediate MSAA texture + resolve.** We render into a multisample
  texture ([`MsaaScratch`], `RENDER_ATTACHMENT` only) and then **resolve** into the
  single-sample target through the colour attachment's `resolve_target` — for the
  main pass (→ surface / readback texture) **and** for each layer pre-pass (→ its
  texture, sampled by the compositor). A **single** MSAA texture is reused: all
  the passes are full-surface and run in sequential *submits*, never
  simultaneously.

- **Caching the MSAA texture.** Recreated only when the size or the format
  changes (a resize); otherwise reused frame after frame. The *view* is created on
  the fly and returned **by value** (wgpu's `TextureView`s are not `Clone`), which
  releases the borrow of `self` before the render pass opens.

## Architecture

```
For each layer:  content ─▶ MSAA scratch (4×) ──resolve──▶ layer texture (1×)
Main pass:       content + composite ─▶ MSAA scratch (4×) ──resolve──▶ target (1×)
```

With no MSAA support (`sample_count == 1`), both passes paint directly into their
single-sample target (`resolve_target: None`) — an identical path to before this
milestone.

## Accepted limits

- **`clear == None` (painting over) is not supported under MSAA**: the multisample
  target does not contain the final target's existing content. Every current
  caller clears (`Some(_)`), so there is no practical effect; to be handled if an
  "overlay" mode appears.
- 4× fixed (not yet adjustable, and no 2×/8× depending on the GPU); sufficient and
  safe.

## Implementation

- `frus-gpu`:
  - `painter.rs`, `image.rs`, `path.rs`, `text.rs`: `new(..., sample_count)` →
    `MultisampleState { count, .. }` (glyphon receives the same count).
  - `compositor.rs`: `MSAA_SAMPLES = 4`, [`preferred_sample_count`],
    [`MsaaScratch`] + `Painters::ensure_msaa`, `resolve_target` wired into
    `render` and `render_group`.
  - `renderer.rs`: the count chosen from the adapter (logging `MSAA: N×`).
  - `offscreen.rs`: `headless_device` also returns the count; `OffscreenFrame`
    exposes `samples` (informing the tests).

## Tests

- `frus-gpu` (GPU readback, the pixel proof): a new `msaa_smooths_a_diagonal_edge`
  — a triangle's **oblique** edge produces **intermediate** green pixels (neither 0
  nor 255), impossible with crisp rendering; the interior stays solid green, the
  exterior stays at the background. It skips cleanly if `samples == 1` (a GPU
  without MSAA).
- The existing readbacks (rect, triangle, outline, texture, layer) stay green:
  they sample **far** from the edges, so they are insensitive to the smoothing.
- **Goldens**: `scene_rect_text` (a rounded rect + text) and `widget_column_text`
  (text) regenerated — their curved edges are now smoothed (tiny byte deltas). The
  goldens with **straight edges** (`rtl_row`, `rtl_drawer`, `inspector_overlay`)
  are **unchanged**: an axis-aligned edge on the grid has no partial coverage —
  proof that the change is localised to smoothing and is not a regression.

## What's left

- Adjustable MSAA (2×/4×/8× depending on the GPU and a quality budget).
- **Analytic** anti-aliasing for text and thin paths (SDF, coverage) where 4× MSAA
  is not enough.
- Reuse of layer textures between frames (the "perf" gain still pending, see
  [milestone-92.md](milestone-92.md)).

# Milestone 89 — Vector paths & icons

## Analysis

The `Scene` knew only **three primitives**: `Rect` (rounded corners, border,
gradient, shadow — through an SDF), `Text` and `RichText`. There was no
**arbitrary path**. The direct consequences: no real icons (they would have been
text/emoji glyphs), no custom painter, no charts, no free-form shapes. For a
framework of this ambition, this is the lowest founding brick: icons, custom
drawing and (later) charts all rest on it.

This milestone adds the **path** primitive end to end — model, GPU, widgets — and
exposes it through an `Icon` widget (a bundled set) and a `CustomPaint` widget (a
free canvas).

## Architecture

```
frus-core                       frus-gpu                     frus-widgets
─────────                       ────────                     ────────────
Path (verbs)  ── Primitive::Path ──► PathPainter             Icon ─┐
Stroke                              (lyon → indexed          icons │→ Scene::fill_path
Scene::fill_path/stroke_path/        triangles + clip)        Custom┘  (Primitive::Path)
          paint_path                 shaders/path.wgsl        Paint
```

### `frus-core` — the model (`path.rs`)
- `PathVerb`: `MoveTo · LineTo · QuadTo · CubicTo · Close` (straight segments +
  quadratic/cubic Bézier).
- `Path`: a sequence of verbs, a **chainable builder** (`move_to().line_to()…`),
  plus constructors (`rect`, `circle` — four cubic arcs, constant `0.5523`) and
  the `scaled` / `translated` transforms (for fitting a `24×24` icon into its box,
  and for the logical→physical DPI step).
- `Stroke { color, width }`.
- `Primitive::Path { path, fill: Option<Color>, stroke: Option<Stroke>, clip,
  owner }` — integrated into the three existing cross-cutting passes: `owner()`,
  `scaled()` (scales the geometry **and** the stroke width), `push_faded()` (exit
  fade: fading both the fill and the stroke).
- `Scene::fill_path` / `stroke_path` / `paint_path`.

### `frus-gpu` — the rendering (`path.rs` + `shaders/path.wgsl`)
- **CPU tessellation through lyon**: `FillTessellator` (the *non-zero* rule) and
  `StrokeTessellator` (width), each through a `Ctor` that injects **colour + clip**
  into every vertex produced. All of a frame's paths are merged into a single
  `VertexBuffers<PathVertex, u32>` (lyon offsets the indices automatically), then
  uploaded as one vertex buffer + one index buffer (grown by powers of two as
  needed).
- **An indexed pipeline** (`TriangleList`), vertex = `pos(px) · color(sRGB) ·
  clip`. The shader projects px→NDC and **clips in the fragment** (the same
  convention as `quad.wgsl`), sRGB→linear on write. The tessellators and the
  geometry are **retained** from frame to frame (zero reallocation in steady
  state).
- Wired into **the windowed renderer and the offscreen rendering** in the order
  `rectangles → paths → text` (so icons go above backgrounds, under text).

### `frus-widgets`
- **`Icon`**: renders an icon from the set, scaled (`size/24`) and centred in its
  box; the colour is the theme's `on_surface` by default and **overridable**
  (`.color(...)`) — in line with the "everything must be customisable" rule.
- **`icons.rs`**: `IconName` (Check, Close, Add, Menu, Star, Heart, Circle,
  Square, Play, ChevronLeft/Right) — solid silhouettes on a `24×24` grid
  (polygons, a procedural star and plus, a Bézier heart, a cross and menu made of
  sub-paths).
- **`CustomPaint`**: a fixed-size canvas that delegates its painting to a
  `Fn(&mut Scene, Rect, &Theme)` closure — the custom-painter counterpart, themed
  at paint time.

## Technical decisions

- **lyon vs a hand-rolled tessellator.** lyon 1.0 is the Rust reference (robust,
  with curves + strokes + fill rules). Writing a correct tessellator
  (self-intersections, stroke joins) would be a project in itself. We adopt it.
- **CPU tessellation, no compute.** Simple, portable (including the future Web
  target), and sufficient at this scale; the geometry is cached per frame.
- **Separate passes** (rect/path/text) rather than one sorted pass: we **extend
  the layered model already in place**. An accepted limit: a path cannot go
  *under* a rectangle emitted after it (just as text is always on top). A unified
  sorted pass will come with compositing.

## Explanations & limits

- **Anti-aliasing.** The tessellated geometry is **crisp but not smoothed** (no
  MSAA here, for a deterministic readback under WSL's software GPU). So an icon's
  oblique edges are slightly jagged; smoothing (MSAA or lyon's geometric AA) will
  arrive with compositing.
- **Solid fill.** `fill` is a solid colour; gradients and textures on a path will
  come later (gradients already exist for rectangles).

## Tests

- `frus-core`: the builder and verb order, `rect`/`circle`, `scaled`/`translated`;
  a builder doctest.
- `frus-gpu` (GPU readback, the pixel proof): `fills_a_vector_triangle` (the
  interior painted, the exterior at the clear colour);
  `strokes_a_path_outline_only` (the stroke is painted, the centre stays empty).
- `frus-widgets`: `Icon` emits **one** filled path, the colour override beats the
  theme, the size drives the box; every `IconName` produces a non-empty path
  (star = 10 vertices, menu = 3 sub-paths); `CustomPaint` invokes the closure with
  its resolved box.
- No regression: the existing widgets emit no path → so the goldens and every
  suite stay identical.

## Demo

The main card shows a **row of vector icons** (a check in the accent colour, a
star, a heart, a menu, a chevron) — tessellated paths rendered by the new
pipeline.

## What's left

- Anti-aliasing (MSAA / geometric AA).
- Gradients & patterns on paths; a unified sorted pass (with compositing).
- A wider icon set; loading paths from SVG.

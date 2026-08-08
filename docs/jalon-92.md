# Jalon 92 — Layer compositing & pipeline precompilation

## Analysis

Two gaps, both deferred from earlier milestones:

1. **Layer compositing** (deferred from J88). Until now, fading a subtree was
   done **primitive by primitive** (`push_faded` multiplies each one's alpha).
   Where primitives **overlap**, the alpha accumulates → the overlap darkens
   (double-blending). That is wrong for a **group** opacity (a panel, a dialogue,
   a transition fading out as one piece). The established solution is a
   save-layer: render the group separately onto a layer, then compose the whole
   layer at the wanted opacity.

2. **Shader precompilation** (the "anti-jank" bet of J89). Traditional 2D engines
   compile shader variants **on first use** → micro-freezes.

## Technical decisions

- **A fixed pipeline set, created at start-up.** frus has only a handful of
  pipelines (rect, image, path, text, composite), all created in
  [`Painters::new`]. There are **no** variants compiled on the fly. To guarantee
  that the first real frame pays **nothing**, [`Painters::warm_up`] renders a
  small scene at start-up that exercises **every** path (rect, image, path, text
  **and** a layer → composite), forcing driver finalisation. → Zero "shader jank"
  on the first render, by construction.

- **A layer = render-to-texture + recomposition.** A [`Primitive::Layer`] carries
  its own list of primitives. The compositor renders it **first** onto a
  full-surface texture (transparent background), in a **separate pass with its
  own *submit*** — indispensable so as not to alias the instance buffers shared
  between passes (a buffer write only applies at the next *submit*; reusing the
  same painters between distinct *submits* is fine, between passes of the same
  *submit* it is not). The [`CompositePainter`] then recomposes the texture as one
  piece at the group opacity (a full-screen quad, clipped in the fragment, alpha
  `= sample.a × opacity`). Since the sample of an sRGB texture is already linear,
  there is no reconversion.

- **Grouping the painters** (`compositor.rs`). The four content painters plus the
  composite one are gathered into a [`Painters`] with a single `render` method
  (layers included), now shared by the windowed renderer **and** the offscreen
  path — the duplication between the two has gone.

## Architecture

```
Scene (containing Primitive::Layers)
   │
   ├─ for each Layer: render_group → a full-surface texture  (separate submit)
   │                        (rect+image+path+text, transparent background)
   ▼
Main pass (1 submit):
   rect → image → path → text → composite(each layer texture @ its opacity)
```

Accepted ordering and limits:
- Layers are **composited above** the main content (just as text is always above
  rectangles): a layer cannot go *under* a primitive emitted after it. Sufficient
  for the use cases (foreground groups); a single sorted pass will come if
  needed.
- **Nested layers** are not recomposited (a `Layer` inside a `Layer` is ignored at
  this level) — foundation first.
- A **full-surface** layer texture (absolute coordinates, trivial alignment):
  simple and correct; cropping/pooling will optimise it later.

## Implementation

- `frus-core`: `Primitive::Layer { primitives, opacity, clip, owner }` integrated
  into the cross-cutting passes — `owner()`, `scaled()` (recursing into the
  children), `push_faded()` (multiplying the group opacity). A `Scene::layer(op,
  |inner| …)` constructor that builds a sub-scene.
- `frus-gpu`: `compositor.rs` (`Painters` + `CompositePainter`) +
  `shaders/composite.wgsl`; `renderer.rs` and `offscreen.rs` delegate to
  `Painters::render`; `warm_up` called when the `Renderer` is constructed.

## Tests

- `frus-core`: a layer captures its sub-primitives + opacity + clip; fading a
  layer **multiplies** its opacity; `scaled` transforms the children.
- `frus-gpu` (GPU readback, the pixel proof):
  `layer_group_opacity_is_uniform_over_overlap` — two **opaque** overlapping
  rectangles, in a layer at 0.5: the overlap has **exactly** the same colour as a
  single coverage (a uniform group alpha, no double-blending), and it is indeed
  ~50% red on a black background.
- All the existing suites pass **through the new `Painters::render`** (the
  rect/path/image/text readbacks go through the compositor) — no regression,
  goldens unchanged.

## Demo

A `CustomPaint` tile adds two overlapping accent squares, grouped in a **layer at
0.55** — the overlap does not darken, illustrating correct group opacity.

## What's left

- **Reuse** of layer textures between frames (a GPU cache keyed by content, the
  GPU-side counterpart of a repaint boundary) — the "perf" side of the bet.
- Anti-aliasing (MSAA) — orthogonal, and now feasible on this base.
- General `transform`/`clip` layers; a single sorted pass; nested layers;
  integrating an `Opacity` widget into the walk (like `RepaintBoundary`).

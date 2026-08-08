# Jalon 127 — `ClipPath`: clipping to an arbitrary path (mask pipeline)

## Analysis

Clipping only covered **analytic** shapes (rect / rrect / ellipse), tested by SDF in the
fragment. Clipping to an **arbitrary path** (a star, a spike, a bubble, a free form) is
not expressible that way: it is the mask work flagged in J125. This milestone adds it —
the clipping family is complete.

## Technical decisions

- **A coverage mask, not a stencil.** For a `ClipShape::Path`, the compositor **renders
  the path in white** into a full-surface texture (reusing the existing path pipeline,
  `render_group`): that is the **mask**. The compositing fragment samples its alpha and
  **multiplies** it into the layer's coverage. Anti-aliased edges for free (path fills
  are already MSAA). No stencil, no duplicated pipeline.

- **A single branch point.** `composite.wgsl` gains a 2nd texture (the mask); outside
  `ClipPath` we bind a **neutral 1×1 white** mask → a multiply by 1, no effect, no
  branch. The analytic shapes (rect/rrect/oval) are unchanged.

- **A path in local coordinates, offset to the screen.** The `ClipPath::new(path)` widget
  receives the path in **local** coordinates (the origin at the box's corner); the walk
  translates it to the screen position (like a `ClipRRect`, pass-through in layout). It
  takes priority over `clip_shape` through a dedicated `Widget::clip_path()` method.

- **`ClipShape` is no longer `Copy`** (it carries a `Path`, `Vec`-backed). The ripple was
  contained: a few `*clip_shape` → `.clone()`. `scaled_xy` scales the path (DPI).

## Implementation

- `frus-core`: the `ClipShape::Path(Path)` variant (+ `scaled_xy`), `Copy` dropped.
- `frus-gpu`: `render_mask` (a white path → a texture); `LayerComposite`/the bind group
  gain the mask (binding 2); a neutral 1×1 white mask for layers with no path;
  `composite.wgsl` samples and multiplies; kind 3 = path.
- `frus-widgets`: the `ClipPath<Msg>` widget; the `Widget::clip_path()` method, forwarded
  (`Box<dyn>`, `Keyed`, `Responsive`, animated); the walk's clipping branch unified (path
  first, otherwise the analytic shape). `Path` / `PathVerb` re-exported.

## Tests

- `frus-test` (at the pixel level, on a real GPU): `path_clip_masks_to_the_shape` — a
  diamond clips the square (the centre and the vertices painted, the **corners erased**).
  The analytic shapes (per-corner rrect, oval, rect) still hold (a neutral mask).
- `frus-widgets`: `ClipPath` emits a `ClipShape::Path` layer **offset to the screen**.
- Visual rendering (outside the commit): a 5-point star + a triangle clipping a gradient,
  crisp edges. The whole workspace green: frus-core 92, frus-gpu 16, frus-widgets 233,
  frus-test clip 5.

## What's left

- A **size-dependent clipper** (a `Size → Path` closure) — here the path is fixed in
  local coordinates.
- **Mask caching** (re-rendered every frame for now; to be indexed like the layer
  textures should a `ClipPath` prove expensive and static).

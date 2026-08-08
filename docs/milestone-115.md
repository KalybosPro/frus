# Milestone 115 — `Transform`: non-uniform scale (`scale_xy`)

## Analysis

The first half of completing `Transform`: **non-uniform scaling**
(`scale_xy(sx, sy)`) — stretching or flattening a subtree with different factors
in X and Y (a bar that lengthens, a thumbnail that squashes). Until now scaling
was necessarily uniform (a single `factor`).

## Technical decisions

- **Generalising primitive scaling per axis.** `Primitive::scaled(factor)` (also
  used for DPI) becomes a special case of `Primitive::scaled_xy(sx, sy)`:
  rectangles and images stretch **exactly** per axis; the **scalar** quantities
  with no axis (corner radius, border, blur, path) follow the mean of the two
  factors, the font size follows `sy` and the wrapping width follows `sx`. When
  `sx == sy` (uniform scaling, DPI), everything becomes exact again — so the
  existing behaviour is preserved.

- **It reuses J113's post-processing path** (no layer, no GPU): the emitted range
  of primitives and the interaction rectangles are scaled per axis about the pivot
  (`scaled_about_xy`, `scale_about_xy`). So rendering and hit-testing stay
  consistent and **testable without a GPU**.

- **Helpers added**: `Point::scale_xy`, `Rect::scale_xy` / `scale_about_xy`,
  `Primitive::scaled_xy` / `scaled_about_xy`, `LayerTransform::scaled_xy`.

- **API.** `Transform::scale_xy(sx, sy)` (about the centre) and
  `scale_xy_from(sx, sy, pivot)`. `scale(factor)` / `scale_from` become uniform
  shorthands.

## Implementation

- `frus-core/geometry.rs`: `Point::scale_xy`, `Rect::scale_xy` /
  `scale_about_xy`.
- `frus-core/scene.rs`: `Primitive::scaled` delegates to `scaled_xy`;
  `scaled_about` delegates to `scaled_about_xy`; `LayerTransform::scaled_xy`.
- `frus-widgets/widget.rs`: `transform_scale() -> Option<(f32, f32, Alignment)>`
  (per-axis factors) + forwards.
- `frus-widgets/transform.rs`: `scale_xy` / `scale_xy_from`.
- `frus-widgets/ui.rs`: the scaling block applies `scaled_about_xy` /
  `scale_about_xy`.

## Tests

- `scale_xy_stretches_per_axis`: `scale_xy(3.0, 1.0)` → a ~60×20 background
  (stretched in X only), still centred.
- The uniform scaling tests (J113) stay green (the `sx == sy` case).
- Suites green: frus-core 88, frus-widgets 210; the whole workspace green.

## What's left

- **Composing** several transformations within one `Transform` (translate + scale
  + rotation applied together) — the second half of the completion.
- Non-uniform correction of the **corner radii** (they become elliptical) and of
  **paths** — approximated here by the mean of the factors.

# Milestone 128 — Showcase: ClipPath + RotatedBox + FittedBox

## Analysis

J125–J127 completed the family (per-corner clipping, `RotatedBox`/`FittedBox`,
`ClipPath`) without showing any of it. This milestone makes them **tangible** in
`frus-transforms` — and *seeing* pays off again: the rendering confirms at a glance the
path clipping (a star), the rotation that **changes the box**, and the `Contain` fit,
with no overlap of the neighbours.

## Technical decisions

- **`ClipPath` as a star.** A 5-point star path (`star_path`, local coordinates) clips a
  gradient square — edges anti-aliased by the GPU mask, alongside `ClipRRect` and
  `ClipOval` (gallery 3).

- **`RotatedBox` made visible by text.** "ROTATED" turned by 3 quarters becomes
  **vertical** (its box goes tall and narrow) — *visible* proof that the rotation affects
  layout, unlike `Transform`.

- **`FittedBox·Contain`.** A large "Fit" (48 px) is scaled to **fit** a 120×80 frame —
  the scale follows from the box (gallery 4).

- **The `view` stays pure**, conventions respected (struct constructors, interface text
  in English).

## Implementation

- `crates/frus-transforms/src/lib.rs`: importing `ClipPath` / `RotatedBox` / `FittedBox`
  / `BoxFit` / `Path` / `Point`; the `star_path` helper; gallery 3 extended (the star
  tile); gallery 4 (`RotatedBox` + `FittedBox`); headings and title updated.

## Tests

- `renders_clip_shapes`: the `view` also emits a `ClipShape::Path` (the star) on top of
  `RRect` and `Oval`.
- The existing guards green (a transformed layer emitted, content placed inside the
  viewport).
- Visual rendering (outside the commit) confirmed: a crisp star, vertical text, "Fit"
  fitted, **no overlap** under the `RotatedBox` gallery. The `frus-transforms` suite: 7.

## Seeing it / running it

- Desktop: `cargo run -p frus-transforms` — scroll; drag/zoom the interactive viewport;
  look at the star, the rotation, the fit.
- Android: an APK through `cargo-apk`.

## What's left

- Verification **on a real device** (desktop + Android): the final *seeing*.
- A tile animating a `ClipPath` (a pulsing path) would illustrate dynamic clipping.

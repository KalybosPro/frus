# Jalon 110 — `AspectRatio`: a box with a width/height ratio

## Analysis

The first of the missing layout widgets: **`AspectRatio`**. Until now there was
no way to hold a constant width/height ratio (a 16:9 video thumbnail, a square
avatar, a 4:3 card) without hard-freezing *both* dimensions — and therefore
without adapting to the available width.

## Technical decisions

- **`aspect_ratio: Option<f32>` in `frus_layout::Style`.** taffy 0.7 handles the
  ratio natively (`width / height`, the usual convention). Added: the field,
  hashed into `layout_hash` (it changes the geometry → it invalidates the relayout
  cache) and passed through in `to_taffy`.

- **The box takes the width and derives the height.** A width that is merely
  *stretched* (`align: stretch`) is **not** enough for taffy to derive the other
  axis: verified empirically (a `probe_aspect_ratio` probe) — a stretched box with
  a ratio stayed at height 0. A **known** dimension is required. So `AspectRatio`
  sets `width: Percent(1.0)` (the parent's full width); taffy then derives the
  height. The most common case: an `AspectRatio` inside a column or a full-width
  context.

- **A pure layout widget.** `AspectRatio::new(ratio).child(...)` paints nothing;
  the child inherits the box (stretched in height through `align: stretch`,
  filling the width if it grows — `flex`, an image, a solid background).

## Implementation

- `frus-layout/style.rs`: the `aspect_ratio` field, defaulting to `None`, plus
  `layout_hash` and `to_taffy`.
- `frus-widgets/aspectratio.rs`: the `AspectRatio` widget (`new` clamps the ratio
  to `> 0`, `child`, `style()` = `width: Percent(1.0)` + `aspect_ratio`).
- `frus-widgets/flex.rs`: passes `aspect_ratio: None` (its `Style` constructor is
  enumerated — only `Flex` enumerates it; the other widgets use
  `..Default::default()`).
- `AspectRatio` exported in `lib.rs`.

## Tests

- `derives_free_dimension_from_ratio`: inside a column 100 wide, an
  `AspectRatio(2.0)` gives a 100×50 box (the filling child paints ~100×50).
- `ratio_below_one_is_taller_than_wide`: `AspectRatio(0.5)` → 100×200 (taller than
  it is wide).
- Suites green: frus-layout 16, frus-widgets 201; the whole workspace green.

## What's left

- `FractionallySizedBox` (size as a fraction of the parent), `Transform`
  (rotating/scaling/translating a child) — the other layout widgets.
- An `AspectRatio` derived from a constrained **height** (the `Row` case) — not
  covered: the brick targets the full-width case, which is the most common.

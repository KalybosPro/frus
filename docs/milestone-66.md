# Milestone 66 — `BorderRadius`: **per-corner** radii (SDF)

The last gap in the §5 box model: corners could only be rounded uniformly (a
single `f32` travelled from the scene to the GPU). There was no way to express a
sheet rising with only its top corners rounded, a tab, or a segment in a group.

## `BorderRadius` (frus-core, `Copy`)

`{ top_left, top_right, bottom_right, bottom_left }` with `uniform`, `top`,
`bottom`, `inflate` (the shadow envelope), `scale` (DPI), and **`clamped`**
(negative radii clamped to zero before rendering, as the brief recommends).

**`impl From<f32>`** is the key to the migration: every entry point
(`Scene::draw_rect`/`gradient_rect`/`shadow`, `BoxDecoration::radius`,
`Container::radius`) now takes `impl Into<BorderRadius>` — **every existing call
passing an `f32` compiles and renders identically**, and a call passing a
`BorderRadius` gets per-corner behaviour. In line with the "everything must be
customisable" rule: `Container::new().radius(BorderRadius::top(12.0))`.

## The pipeline

- `Primitive::Rect.radius` becomes a `BorderRadius`; `scaled` scales all 4 radii
  for DPI.
- **GPU instance**: a new `radii: vec4` attribute (tl, tr, br, bl), clamped to
  zero on the painter side; the old `params.x` slot is freed.
- **Shader**: `corner_radius(p, radii)` picks the radius of the fragment's
  **quadrant** (centred coordinates, y downwards), and then the classic SDF is
  unchanged — border, shadow blur and gradient all work as they are with a
  per-corner radius.

## Validation

- **GPU proof by readback**: `per_corner_radius_rounds_only_selected_corners` —
  a rectangle with only its top-left corner rounded (30 px): pixel (0,0) cut
  away, the other three corners **square**, the centre solid. On a real wgpu
  device.
- Backward compatibility: `rounded_rect_leaves_corner_transparent` (a uniform
  radius through `f32`) passes unchanged — the `From<f32>` path is
  pixel-identical.
- **238 tests** in total, all green; a warning-free build; the demo did not
  panic.

## What's left (remaining §5)

Text decorations (underline/strikethrough), `letter_spacing`/`line_height`,
consolidating `ColorScheme` (+ HCT `from_seed`), `content_padding` → taffy,
`Alignment`, RTL (§14). Opportunistic adoption of per-corner radii (BottomSheet's
top corners, tabs, segments) as we go.

# Jalon 7 — Style: rounded corners, borders, alignment, per-side padding

Enriches rendering and layout enough for realistic UIs.

## What ships

- **Rounded corners + borders** through an **SDF** in the fragment shader
  (anti-aliased edge, border ring). A single pass, no extra geometry.
- **Flex alignment**: `Justify` (main axis) and `Align` (cross axis), mapped onto
  taffy.
- **Per-side padding**: the `Insets` type (top/right/bottom/left);
  `Style.padding` is now an `Insets`.
- **Enriched primitive**: `Primitive::Rect { rect, color, radius, border_width,
  border_color }` + `Scene::draw_rect` (`fill_rect` kept for the simple case).
- **Widget API**: `Container::radius/.border/.padding_each`,
  `Flex::justify/.align/.padding_each`.

## The shader (SDF)

```
d = sdf_round_box(local_px, half_size, radius)   // signed distance (negative inside)
alpha = 1 - smoothstep(-0.5, 0.5, d)             // anti-aliased coverage ~1px
color = mix(fill, border, smoothstep(-bw-0.5, -bw+0.5, d))  // border ring
→ vec4(color.rgb, color.a * alpha)
```

The vertex stage passes the fragment stage the local position (px from the
centre) and, as `flat`, the half-size, the radius, the thickness and the
colours.

## Decisions

- **SDF rather than geometry**: rounded corners and borders come "for free" on
  the geometry side (still a single quad); everything happens in the fragment.
- `Style.padding: Insets`: per-side padding without breaking `.padding(f32)`
  (the uniform case).
- Defaults unchanged: `justify = Start`, `align = Stretch` (identical behaviour
  to the previous milestones).

## Demo

A **centred** header (the counter), a **rounded and bordered** button with
per-side padding, horizontally **centred**; the squares become **rounded
cards**.

## Tests

- `frus-core`: `fill_rect` stays radius 0 / borderless; `draw_rect` stores the
  radius and border.
- `frus-gpu`: non-regression (solid red rect, radius 0 → red centre) **and**
  `rounded_rect_leaves_corner_transparent` (a large radius cuts the corner out).
- `frus-layout`: `justify_center_centers_child` (child centred on the main axis).

## Limits (next milestones)

- No shadows, gradients, or clipping of children.
- Still no keyboard input/focus and no subtree diffing.

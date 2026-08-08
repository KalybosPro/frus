# Milestone 112 — `Transform`: paint offset (`translate`)

## Analysis

The last of the missing layout widgets: **`Transform`**. It offsets its child **at
paint time**, without touching layout — the child can overflow its box and the
siblings do not move. It is the brick behind effects that *slide* (a badge in a
corner, an entry sliding in, an error shake) and, combined with a `Tween` read in
`view()`, behind animated movement.

This milestone covers **translation** only. Scaling and rotation need an **affine
matrix** in the pipeline — GPU rendering today only knows axis-aligned quads (a
scaled rect is still a rect, but a rotated rect is not) — so they are deferred to
a dedicated milestone.

## Technical decisions

- **It reuses the walk's translation cascade.** The offset is added to the
  translation propagated to the children (like `Container::alignment`'s
  anchoring). And since the primitives **and** every interaction surface (click,
  long press, focus, scrolling, dragging, accessibility) all derive from that same
  translation, the offset is **automatically correct everywhere**, hit-testing
  included — no post-processing, and no risk of paint/click inconsistency.

- **`align_offset` → `child_offset`.** The old function (the fractional
  anchoring offset) becomes `child_offset`: it **accumulates** the anchoring and
  the `Transform::translate` offset. Called at both points in the walk (the main
  tree + virtualised list items / `layout_builder`).

- **RTL correction.** The world's x axis being flipped in RTL, a positive logical
  `dx` ("towards the end") would point left: so its sign is inverted to stay
  consistent with the reading direction.

- **A `transform_translate()` trait method**, forwarded by the transparent
  wrappers (`Box`, `Keyed`, `Responsive`, the `animated` wrappers) in the usual
  pattern.

## Implementation

- `frus-widgets/transform.rs`: the `Transform` widget (`translate(dx, dy)`,
  `child`, a pass-through `style()`, `transform_translate() = Some((dx, dy))`).
- `frus-widgets/widget.rs`: the `transform_translate` trait method + the `Box`
  forward.
- `keyed.rs` / `responsive.rs` / `animated.rs`: forwards.
- `ui.rs`: `align_offset` → `child_offset` (accumulating anchoring + translate),
  at both call sites.
- `Transform` exported in `lib.rs`.

## Tests

- `translate_offsets_the_child_at_paint`: `translate(30, 10)` paints the child
  (20×20) at ~(30, 10).
- `translate_does_not_affect_layout`: a sibling placed after a child offset by 50
  stays at its layout position (`y = 20`) — the offset is purely visual.
- The frus-widgets suite green (205); the whole workspace green.

## What's left

- `Transform` **scaling** (`scale`): affine post-processing over the subtree's
  range of primitives (through `split_off`, like the opacity layer) **plus**
  transforming the interaction rectangles — it stays axis-aligned, without
  touching the GPU.
- `Transform` **rotation**: an affine matrix passed to the shaders (vertex + SDF),
  plus inverse-transform hit-testing — a render-infrastructure milestone.

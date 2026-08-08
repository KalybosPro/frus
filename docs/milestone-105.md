# Milestone 105 — Container: `alignment` + composite `decoration`

## Analysis

Two gaps remained on `Container` compared with the conventional container API:

1. **Anchoring the child.** With no setting, the child stretches to fill the box
   (the flex `Start`/`Stretch` default). The established API exposes an
   `alignment` to centre it, stick it to a corner, an edge…
2. **Decorating as one piece.** The box was decorated field by field (`.color`,
   `.border`, `.radius`, `.gradient`, `.shadow`). The established API gathers all
   of that into a reusable `BoxDecoration` passed as `decoration`.

## Technical decisions

- **`Alignment` (frus-core).** The nine named anchors
  (`TopLeft`…`Center`…`BottomRight`), each projected onto two independent edges
  through `horizontal()` / `vertical()` → [`AlignEdge`] (`Start`/`Center`/`End`).
  A pure geometric type, with no layout dependency (frus-core does not know about
  `Justify`).

- **Anchoring = the existing flex levers.** The box stays a **flex row** (the
  horizontal main axis → `justify`; the vertical cross axis → `align`).
  `Container` translates `alignment.horizontal()` → `Justify` and
  `alignment.vertical()` → `Align` inside `style()`. No new positioning
  primitive: taffy is reused. And since `style()` is the source shared by
  `build_layout` **and** the relayout cache's signature (`layout_hash` covers
  `justify`/`align`), the cache stays consistent for free.

- **`decoration(BoxDecoration)` = decomposition.** The builder breaks the
  composite `BoxDecoration` out into the container's existing fields (background,
  gradient, border, radius, shadow). Zero new state, zero new paint path: the
  animations (colour/radius…) remain applicable on top. Only the shadow's
  `spread` is not preserved — the container's shadow model has none (already the
  case for `.shadow`).

## Implementation

- `frus-core/geometry.rs`: `enum Alignment` (9 variants, `Default = Center`),
  `enum AlignEdge`, `Alignment::{horizontal, vertical}`. Re-exported by `lib.rs`.
- `frus-widgets/container.rs`: an `alignment: Option<Alignment>` field; the
  `.alignment(Alignment)` and `.decoration(BoxDecoration)` builders; `style()`
  maps the anchor onto `Justify`/`Align`. Importing `Align` and `Justify` from
  frus-layout.

## Tests

- `alignment_centers_the_child`: a 20×20 child inside 100×100 → the background at
  ~(40, 40).
- `alignment_anchors_child_to_a_corner`: `BottomRight` → the background at
  ~(80, 80).
- `decoration_applies_composite_fields`: a `BoxDecoration` (green + radius 8 +
  border 2) → a green background of radius 8 painted, the border reserved at
  layout time (padding = 2).
- Suites green: frus-core 85, frus-widgets 196.

## What's left

- A **fractional** `Alignment` (continuous `{ x, y }`) + `Lerp` → an animatable
  `Tween<Alignment>` (which requires placing the child manually, outside discrete
  flex).
- Exposing `.alignment` / `.decoration` on the named `AnimatedContainer` widget.

# Jalon 159 — Reordering: neighbouring columns slide

## Analysis

The preview (milestones 155/158) lifted a faithful ghost, but the columns **stayed frozen**:
nothing showed the slot opening. The reorderable-list effect was missing — the neighbours
**parting** to make room for the drop and **closing** the gap left by the grabbed column.

The obstacle (already noted in the previous milestones): the shell **does not know** the
column → widgets mapping (a column = one header + N cells, each a distinct `WidgetId`). Making
"column 2" slide from the shell seemed to require that mapping.

## Technical decisions

- **A purely geometric reflow.** Rather than a mapping, we **reclassify the scene's
  primitives** by their **centre in x**: the **source** column (lifted) is removed, the columns
  between source and target are **translated by one notch** (the source's width) to close the
  gap and open the slot. Headers **and** data cells slide together (the same band in x) — the
  full effect, with no extra structure.

- **An anti-background guard.** A primitive wider than ~1.5 columns (a page background, a row
  highlight) is **left in place**: we do not move a whole backdrop. Text (unmeasured in
  frus-core) is located by its **position** (a point `bounds()`), enough to classify it in x.

- **A shared, pure utility.**
  `frus_widgets::reflow_reorder_columns(prims, src, target, to_right, lifted_owner)`: a **pure
  function** over primitives, called by the shell **and** by the golden (no duplication),
  testable without a GPU. The ghost is still painted by the shell (`draw_ghost_card`) over the
  reflowed scene; the indicator/dim become unnecessary (the real gap replaces them).

- **frus-core bricks.** `Primitive::bounds()` (a bounding box, `Path` through its points) and
  `Rect::union`.

## Implementation

- `scene.rs` / `geometry.rs` (frus-core): `Primitive::bounds()`, `Rect::union`.
- `reorder.rs` (frus-widgets): `reflow_reorder_columns` (+ tests); export.
- `app.rs` (shell): `paint_reorder_preview` reflows the scene (`reflow_reorder_columns`) then
  paints the ghost card; `draw_reorder_overlay` → `draw_ghost_card` (shadow + faithful face +
  edge).
- `goldens.rs`: `table_reorder_preview` rebuilds the reflow (the source removed, "Score" slid,
  the "Role" ghost).

## Verification

- **Unit** (`reflow_reorder_columns`, no GPU): dragged **right** → the source column removed,
  the neighbours slid **−1 notch** (col 1 → 0, col 2 → 100), a wide background **kept**;
  dragged **left** → a **+1 notch** slide. `draw_ghost_card`: the solid fallback = 2
  primitives.
- **Golden** `table_reorder_preview` **inspected**: "Role" lifted (removed, its data gone),
  "Score" (5 / 3) **slid** into "Role"'s place, a **gap** open on the right, the floating
  **"Role" card** at the cursor. The full sliding effect.
- `cargo test --workspace` **green**, with no warning.

## What's left

- **Temporal interpolation** (easing) of the slide: today the reflow **follows the cursor** (it
  flips from one target column to the next) with no smooth transition; a tween would need a
  per-column animation state.
- **Ghost opacity** (< 1) through `Primitive::Layer { opacity }`.

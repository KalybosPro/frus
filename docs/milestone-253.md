# Milestone 253 — Neighbouring cards shift on vertical insertion (the "gap")

## Analysis

Horizontally (`Table` columns), the reorder preview **reflows** the neighbours
(`reflow_reorder_columns`, an earlier milestone): the lifted column's gap closes, the drop slot opens,
all of it **following the cursor**. **Vertically** (Kanban cards), the preview only laid down an
**insertion line** (milestones 248, 252): the cards stayed frozen, with no "gap" under the bar. This
milestone brings the **vertical counterpart** of the reflow.

## Technical decisions

- **`reflow_reorder_cards`** in `frus-widgets::reorder`, the vertical twin of
  `reflow_reorder_columns`, **purely geometric** (with no knowledge of the tree):
  - the **source column** (the lifted card's x band) has the cards **below** the lifted one **move up**
    one notch → the gap closes;
  - the **target column** (the insertion line's x band) has whatever is **at/below** the line **move
    down** one notch → the slot opens.
- **A notch = the card's height** (`src.height`). A block **taller** than `1.5×` that notch is a
  column/page background (not a card): left in place — the strict counterpart of the horizontal
  `max_cell` guard.
- **No shearing.** Each primitive slides according to the **centre** of its bounds. Since the insertion
  lines sit at the cards' **edges** (never right at a centre), all of a card's primitives fall on the
  same side → the card moves as a block (including a **rich**, multi-primitive card).
- **Reuses `owners`** (milestone 251): the lifted card's subtree is removed from the preview (it is
  already floating as the ghost).

## Implementation

- `frus-widgets/src/reorder.rs`: `pub fn reflow_reorder_cards(prims, src, line, lifted)` (+ the export).
  Tests: `lifting_a_card_closes_the_source_gap`, `insertion_line_opens_a_hole_in_the_target_column`
  (the source **and** the target, distinct columns), `tall_backgrounds_stay_put`.
- `frus-shell/src/app.rs`: the **vertical** branch of `paint_reorder_preview` reflows the scene through
  `reflow_reorder_cards` (like the horizontal branch), then lays the insertion line on top. The `owners`
  computation moves above the `match` (shared by the ghost + the reflow).

## Verification

- **Widgets 391** (+3): the reflow is covered by **pure** tests — the lift in the source column, the gap
  opening in the target column, tall backgrounds staying put.
- **No regression**: the preview only exists during a drag (outside the goldens) — **goldens 77
  unchanged**; demo 36; shell 27. The **horizontal** axis is intact (a separate branch).

## Notes

- The slide is **immediate** (proportional to the position), without the horizontal spring smoothing
  (`reorder_x`) — a possible refinement (vertical inertia).
- As in milestones 250–252, the **live** drag rendering remains runtime state not inspected on a GPU
  here; the verification covers the reflow's pure geometry.

## What's left

- Vertical inertia/spring for the slide (parity with the horizontal).
- A cross-cutting consolidation/review of the drag-and-drop domain (Table + Kanban).

# Milestone 252 — Insertion indicator between cards (hovered half)

## Analysis

The **vertical** drag preview (milestone 248) always put the insertion line on the hovered slot's
**top** edge, and the drop inserted **before** that target. So there was no way to drop a card
**between** two cards by aiming at the second: aiming at a card always meant "just above". As in any
reorderable list, the intent must depend on the hovered **half**: the **upper** half → insert
**before**; the **lower** half → insert **after**.

## Technical decisions

- **A midpoint split.** For a **vertically** reorderable slot, a cursor above the midpoint inserts
  **before** (the top edge), below it **after** (the bottom edge, index +1). On the **horizontal** axis
  (`Table` columns), nothing changes: the drop logic stays identical.

- **The visual and the routing agree.** The same predicate (`reorder_insert_after`) drives **both** the
  painted insertion line **and** the effective drop index — the bar shows exactly where the card will
  land. `to_pos` being an **insertion index** (the reducer inserts at that index, with the −1
  adjustment for a downstream move within the **same** column), "after" simply translates to `+1`.

- **The final drop zone.** Its lower half yields `slot(col, len) + 1`, clamped by the reducer to `len`:
  the insertion stays at the end of the column (harmless).

## Implementation

- `frus-shell/src/app.rs`:
  - `drop_insertion_line(target, thickness, after)` — the target's **top** (`after = false`) or
    **bottom** (`after = true`) edge.
  - `TodoApp::reorder_insert_after(target, rect)` — true if the target is **vertical** and the cursor is
    in its lower half (false when horizontal or off-target).
  - `reorder_drop_line` paints the line at the chosen edge; the **drop** shifts the effective index by
    `+1` when `reorder_insert_after`.

## Verification

- **Shell 27**: `insertion_line_sits_on_the_target_top_edge` (before → the top edge) **and**
  `insertion_line_sits_on_the_target_bottom_edge_when_inserting_after` (after → the bottom edge,
  `y = bottom − thickness/2`).
- The insertion semantics (`to_pos` = an index, the same-column adjustment) are already covered
  reducer-side (`kanban_move_relocates_a_card`).
- **No regression**: the static rendering unchanged (the indicator only appears during a drag) —
  goldens 77 unchanged; demo 36; widgets 388. The horizontal axis (Table): `reorder_insert_after` always
  returns `false`, the drop behaviour strictly identical.

## Notes

- The insertion line and the grab remain **runtime** state, not inspected on a GPU in this environment;
  the bar's geometry (both edges) and the index semantics are covered by pure tests.

## What's left

- Shifting the neighbouring cards on a vertical insertion (opening a "gap" under the bar), like the
  horizontal `reflow` of columns.
- A new widget domain, or a cross-cutting consolidation/review.

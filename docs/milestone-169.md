# Milestone 169 — Accessibility: announced row selection

## Analysis

Milestone 167 announced **sorting** and the **checkboxes**, but not **row selection by click**
(`on_select_row`) — clicking a row changed the state silently for a screen-reader user. It had
to be announced, with the **row number** to locate the action.

## Technical decisions

- **The cell knows its row.** `Cell` (data) and `WidgetCell` gain the row index; their
  `announce()` (milestone 167's hook, already read by the shell on click) announces the
  **resulting** state: "Row N selected" / "Row N deselected" (toggling the current `selected`
  state). Every cell in the row carries the announcement — clicking anywhere in the row selects
  it, so it announces it.

- **Focus navigation is *not* duplicated.** Announcing "button, Save" on Tab would be
  **redundant**: AccessKit already publishes the **focused** node (the tree's `focus`), which
  the screen reader announces natively. Adding a live region there would make it **speak
  twice**. So we rely on the existing AccessKit focus — a decision, not an oversight.

## Implementation

- `table.rs`: the `row` field on `Cell` (`Option`, `None` for a header) and `WidgetCell`
  (`usize`); `Cell::announce` (the data branch) and `WidgetCell::announce` announce "Row N
  selected/deselected". Filled in at rebuild time.

## Verification

- **Unit**: `row_click_selection_is_announced` — an unselected text row → "Row 1 selected"; a
  selected widget row → "Row 2 deselected"; a non-selectable table → no announcement.
- `cargo test --workspace` **green**.

## What's left

- **Number vs label**: we announce "Row N"; some apps would prefer the row's content ("Ada
  selected"). The app could supply it through a future announcement override.
- Extending the **count** to the checkboxes ("3 rows selected") rather than row by row.

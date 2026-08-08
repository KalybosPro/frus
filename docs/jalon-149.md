# Jalon 149 — Table: indeterminate "check all" & keyboard sorting

## Analysis

Multiple selection (milestone 148) had two gaps against Material:

- **"Check all"** only showed checked / unchecked — never the **indeterminate** state (when
  *only some* rows are checked).
- Sorting was reachable by **mouse** only; from the keyboard you could neither reach nor
  activate a header.

## Technical decisions

- **A tri-state "check all" box.** `CheckCell` gains an `indeterminate` flag; the table
  computes it (`some_selected` = at least one row checked but not all). Rendered Material
  style: a full `primary` box struck through with a **dash** (instead of the tick). Display
  order: checked > indeterminate > unchecked.

- **Keyboard sorting "for free".** The shell already activates any **focusable** widget
  carrying an `on_click` on Enter/Space (the buttons milestone). So it is enough to make the
  right cells **focusable**: the **sortable headers** (`header && message`) and the
  **checkboxes** — not the data cells (they stay mouse-clickable without cluttering the tab
  order). Keyboard focus sorts / checks with no new logic, and the focus ring is drawn
  automatically.

- **A layout reminder.** Flexible columns only have a width if the table has a **width**
  (`width`): with no constraint, a `Flex` row at automatic width shrinks its flexible cells
  to zero (and then nothing is focusable/clickable). Documented by a test that sets the
  width.

## Implementation

- `table.rs`: `CheckCell` gains `indeterminate` (the dash rendering) + `focusable`; `Cell`
  gains `focusable` (sortable headers); the `some_selected` helper; the header passes the
  indeterminate state through.
- `goldens.rs`: `data_table_multiselect` regenerated (an indeterminate "check all").

## Verification

- **Unit**: `all_selected`/`some_selected` — nothing checked `(false,false)`, partial
  `(false,true)`, all `(true,false)`; only the **2 headers** enter the Tab cycle (data cells
  excluded); the click/sort/selection tests stay green.
- **Golden**: `data_table_multiselect` **inspected** — the "check all" box struck through (a
  dash) under a partial selection. `cargo test --workspace` green.

## What's left

- **Column resizing** with the mouse (handles between headers): needs a dedicated drag state
  in the shell (scrollbar style) + an `on_resize(column, width)` callback — a milestone in
  its own right.
- **Widget cells** (beyond text).

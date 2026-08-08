# Jalon 148 — Table: multiple selection & variable-width columns

## Analysis

The `Table` (milestone 145) rested on `Grid` → **strictly equal columns** and no multiple
selection. For a real business table, it needed:

- **Multiple selection**: a column of **checkboxes** per row, topped by a **select all** in
  the header.
- **Variable column widths**: fixed (px) or flexible (a share of the remaining space).

## Technical decisions

- **`Flex` rows instead of the `Grid`.** A grid with equal tracks allows neither a narrow
  column (checkboxes) nor mixed widths. The table is now a **column of `Flex` rows**, each
  cell carrying its width: `Length(px)` fixed (`flex_grow = 0`) or `Auto` flexible
  (`flex_grow = 1`). Since every row applies the **same widths in the same order**, the
  columns stay aligned; the fixed total width (`width`) is distributed by the layout engine.

- **Multiple selection driven by the app.** `checkboxes(on_check, on_check_all)` adds the
  checkbox column (on the left). Each box reflects `selected`; the header is checked when
  **all** the rows are. `on_check(row)` toggles one row, `on_check_all` toggles everything —
  the table still has **no state** of its own. Row clicking (`on_select_row`) and the boxes
  coexist.

- **A drawn box, the tick = the `Check` icon.** The `CheckCell` paints a square (a border
  when unchecked, a `primary` fill + a tick when checked); the tick reuses
  `IconName::Check`'s vector path — consistent with the rest, crisp at any size.

- **Shared factors.** The cell background (tinted header / highlighted row / hover) and the
  cell style (width + row height) are two functions common to text cells and checkboxes, to
  avoid duplication.

- **A compatible API.** `header`/`row`/`width`/`on_sort`/`sorted`/`on_select_row`/`selected`
  unchanged; `column_widths(&[f32])` and `checkboxes(..)` added.

## Implementation

- `table.rs`: rewritten as `Flex` rows; `Cell` gains a width; the new `CheckCell`;
  `column_widths`, `checkboxes`; the `cell_background`, `cell_style`, `col_width`,
  `all_selected` helpers.
- `goldens.rs`: `data_table` regenerated (the same rendering, a `Flex` layout); the new
  `data_table_multiselect`.

## Verification

- **Unit**: the row structure (header + data); a header click → `Sort`, a row click →
  `Select`, a row box click → `Check(r)`, a header box click → `CheckAll`; a fixed 80 px
  column does place the next column past it.
- **Golden**: `data_table` (visually unchanged) and `data_table_multiselect` (boxes, a
  partial "select all" unchecked, checked rows highlighted, a fixed 1st column) rendered and
  **inspected**. `cargo test --workspace` green.

## What's left

- An **indeterminate** state for "select all" (when *some* rows are checked).
- **Widget cells** (not just text): the rebuild (`rebuild`) regenerates from `String`s;
  hosting arbitrary widgets would require not rebuilding them.
- **Column resizing** with the mouse (handles between headers).

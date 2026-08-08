# Jalon 204 — Grid: header sorting + per-cell validation

## Analysis

Milestone 201 turned the grid into a real keyboard spreadsheet (always-editable cells, Tab/Enter,
adding/removing rows). Two gestures every spreadsheet is expected to have were missing: **sorting**
by clicking a header, and **flagging** invalid input. `Table` already knows how to emit `on_sort`
and show a sort arrow (`sorted`); all that was left was wiring the demo.

## Technical decisions

- **Sorting driven by the application.** `Table::on_sort(Msg::GridSort)` makes the headers
  clickable; `reduce` flips ascending/descending on the clicked column and **sorts the rows** (a
  case-insensitive comparison). The header arrow follows the state through `.sorted(col, asc)`.
  `Table` never sorts by itself — it only emits the column (milestone 199).

- **Pure per-cell validation.** `grid_cell_error(col, value) -> Option<&str>`: `Name` (col 0) is
  required, `Email` (col 2) must contain `@` and `.` once filled in. An invalid cell goes through
  `TextInput::error(...)` (a border + a message, already in the widget). A pure function, testable
  without rendering.

- **Enter on the last row creates a row.** Extending milestone 201: `GridEnter` on the last row
  pushes an empty row and moves the focus down into it, instead of staying put — typing continues
  from the keyboard without touching the mouse.

## Implementation

- `frus-demo/src/lib.rs`: `Msg::GridSort(usize)`; the `grid_sort: Option<(usize, bool)>` field; the
  `GridSort` (sorting) and `GridEnter` (creation at the end) arms; `grid_cell_error`; `grid_screen`
  wires `on_sort` + `sorted` + a per-cell `error`; the hint updated.

## Verification

- `grid_edit_navigate_and_resize`: updated — Enter on the last row **creates** a row.
- `grid_sort_toggles_and_validates`: sorting col 0 ascending then descending (the order checked);
  `grid_cell_error` on an empty Name, a malformed email, and the valid cases (an empty email
  tolerated).

## What's left

- **Numeric** sorting for numeric columns (everything is text here), stable multi-column sorting,
  cross-row validation (email uniqueness), and blocking submission while a cell is in error.

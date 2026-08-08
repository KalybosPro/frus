# Milestone 232 — Self-sorting DataTable (reusable widget)

## Analysis

`Table` is purely controlled: it emits the clicked column (`on_sort`), shows the `sorted(...)`
indicator, but **does not sort** — the application reorders its rows itself. As a result, the sorting
logic (comparison, direction, case) is copied by hand into every reducer (see the demo's grid). This
milestone opens the **DataTable** domain: a table that **sorts its own data** for display, while
staying controlled.

## Technical decisions

- **`sort_rows` / `compare_cells` — pure, public logic.** `compare_cells(a, b)` compares
  **numerically** if both cells read as numbers, otherwise lexically and **case-insensitively**.
  `sort_rows(rows, col, asc)` returns a sorted copy. Exported free functions: reusable outside the
  widget too (a reducer can sort its data the same way).

- **`DataTable` encapsulates display sorting.** You pass it the raw rows and the `sorted(col,
  direction)` state; it rebuilds an internal `Table` with the rows already sorted + the indicator, and
  forwards `on_sort` to it. The sort state stays **in the app** (the controlled model) — only the
  display transformation is encapsulated.

- **Composition, not inheritance.** `DataTable` **delegates** the five `Widget` methods `Table`
  overrides (`style`, `children`, an empty `paint`, `on_click`, `stack`) to an internal `Table`
  rebuilt at each builder — the same `rebuild()` pattern as `Table`. A `Msg = ()` default type for
  ergonomics.

## Implementation

- `frus-widgets/src/datatable.rs` (new): `compare_cells`, `sort_rows`, `DataTable` (the
  `column_widths`, `sorted`, `on_sort` builders; the internal `rebuild`).
- `frus-widgets/src/lib.rs`: `mod datatable;` +
  `pub use datatable::{compare_cells, sort_rows, DataTable};`.

## Verification

- **Widget** `sort_rows_is_numeric_aware_and_case_insensitive`: a numeric column 2 < 9 < 10 (and not
  "10" < "2" < "9"), a text column alice < Bob < Carol, the descending direction reversed.
- **Widget** `compare_cells_prefers_numbers_then_text`, `data_table_builds_a_non_empty_tree`.
- The `DataTable` **doctest**.
- **Golden** `data_table_sorted`: sorted by "Score" descending (12, 10, 9, 2) + the "▼" indicator.
- Widgets 367; goldens 65.

## What's left

- Internal **pagination** (a page slice + a `Pagination` under the table).
- Wiring `DataTable` into the demo (replacing the copied sort in the grid's reducer).
- A **custom** sort key per column (dates, formatted amounts).

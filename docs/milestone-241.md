# Milestone 241 — DataTable: multiple selection (checkboxes)

## Analysis

The base `Table` already offers **multiple selection**: a checkbox column topped by a "check all"
(`checkboxes(on_check, on_check_all)`), the checked state reflecting `selected`. But, as with single
selection (milestone 239), `Table` reasons in **displayed positions**: under `DataTable`'s sorting +
pagination, the 2nd displayed row's box does not check the 2nd source row.

This milestone exposes multiple selection at the `DataTable` level, with the **same** displayed
position ↔ source index mapping already in place for sorting, paging and single selection.

## Technical decisions

- **`checkboxes(on_check, on_check_all)`.** `on_check(source_row)` receives the **source row**'s index
  (mapped through `page_indices`, like `on_select_row`); `on_check_all` is a message passed through as
  is — the application decides what "all" covers (every source row, or the page). The checked state
  **reuses** [`selected`](DataTable::selected): the same source indices, the same mapping to visible
  positions.

- **Coexists with `on_select_row` (the mail-client pattern).** The box handles **group selection**
  (highlight + tick), while a click on the row's **body** stays a row click (focus/detail). The two
  target different cells (the box vs the text) — the deepest hit-test separates them.

- **Controlled.** The checked set lives in the app (`data_checked: Vec<usize>` of source indices); the
  widget only maps and displays.

## Implementation

- `frus-widgets/src/datatable.rs`: the `on_check`/`on_check_all` fields + the `checkboxes` builder;
  `rebuild` wires `Table::checkboxes` with the position → source mapping; the
  `checkbox_click_reports_the_source_row_through_sort_and_page` test (page 2 of an ascending sort → the
  box returns the source index, the header box's `999` sentinel filtered out).
- `frus-demo/src/lib.rs`: the `data_checked` state + `Msg::{DataCheck, DataCheckAll}` (toggle / check
  all-uncheck all); `data_screen` wires `.checkboxes(...).selected(&data_checked)` alongside the row
  click, with an "N checked" summary.

## Verification

- **Widgets** `checkbox_click…`: page 2 (size 2) of an ascending `[1,2,0]` sort → the box returns the
  source index `0`.
- **Golden** `data_table_checkboxes`: sorted by "Score" descending `[Bob, Dan, Ada, Carol]`, the
  **source** rows 0 (Ada) and 3 (Dan) checked → two boxes checked at their sorted positions (2nd/3rd)
  and the header box **indeterminate** (2 of 4) — visually inspected.
- **Demo** `data_table_screen_…` extended: toggling a row (check/uncheck), "check all" = 12 rows,
  "check all" again = uncheck everything.
- Widgets 376; goldens 71; demo 34; the shell compiles.

## What's left

- A **filter/search** above `DataTable` (the app filters the source rows; the widget
  sorts/paginates/selects the subset) — milestone 242.
- **Bulk** actions (an action bar when rows are checked).

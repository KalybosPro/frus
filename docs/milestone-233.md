# Milestone 233 — The DataTable's internal pagination

## Analysis

`DataTable` (milestone 232) sorts its rows but shows them **all**. For a real data table, they must
be **paginated**: show only a slice and offer a page selector. `Pagination` already exists as a pure
control (page numbers); this milestone **composes** it with `DataTable`, which slices the page
itself.

## Technical decisions

- **Pure `page_count` / `page_rows` helpers, public.** `page_count(len, per)` = the number of pages
  (at least 1); `page_rows(rows, current, per)` = the page's slice (1-indexed, brought back into range
  if it overflows). Reusable outside the widget, like `sort_rows`.

- **`.paginated(current, per_page, on_page)`.** Slices from the **already sorted** rows (sort first,
  then page) and places a [`Pagination`](crate::Pagination) under the table; the page count is
  computed on the sorted **total**. `on_page(page)` reports the click — the app holds the current page
  (the controlled model).

- **`inner` becomes a `Box<dyn Widget>`.** To top the `Table` with a `Pagination`, the internal
  rendering moves from a `Table` to a `Box<dyn Widget>`: either the `Table` alone, or a `Flex` column
  `[table, pager]`. The `Widget` delegation (style/children/paint/on_click/stack) targets that `Box`.

## Implementation

- `frus-widgets/src/datatable.rs`: `page_count`, `page_rows`; the `page`/`on_page` fields; the
  `paginated` builder; `rebuild` slices the page and composes `Table` + `Pagination`;
  `inner: Box<dyn Widget>`.
- `frus-widgets/src/lib.rs`: `page_count`, `page_rows` added to the `pub use`.

## Verification

- **Widget** `pagination_slices_rows_and_counts_pages`: 7 rows / 3 = 3 pages; page 1 = `[1,2,3]`,
  page 3 = `[7]`, an out-of-range page brought back to the last.
- **Widget** `data_table_with_pagination_builds_table_and_pager`: the tree has **2** children (the
  table + the selector).
- **Golden** `data_table_paginated`: the top 3 by Score descending (15, 12, 10) + a ‹ 1 2 3 ›
  selector.
- Widgets 369; goldens 66.

## What's left

- Wiring `DataTable` (sorting + pagination) into the demo, removing the copied sort from the grid's
  reducer.
- A **page size** selector; an "N–M of T" label.

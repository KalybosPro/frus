# Milestone 239 — DataTable: selected row (source index ↔ displayed position mapping)

## Analysis

The base `Table` already knows how to **highlight** a row (`selected`) and emit on click
(`on_select_row`), but it reasons in **displayed positions** (0..n of what it shows). `DataTable`,
though, **sorts** and **paginates** its display: the row the user sees in 2nd position is not
necessarily the 2nd row of the source data. Without a mapping, the application would receive a
displayed index — useless for identifying the data, and the highlight would shift on the slightest
sort.

This milestone adds **row selection** to `DataTable`, keeping the source data's identity across sorting
+ pagination — exactly the service the widget already provides for sorting and paging.

## Technical decisions

- **Source row indices throughout.** `rebuild` no longer sorts/slices `Vec<String>`s but a list of
  **indices** `0..rows.len()` (`sorted_order`, a **stable** sort), then takes its page slice. That
  `page_indices` list keeps, for each displayed position, the row's original index.

- **`on_select_row(f)`.** A click on the displayed position `d` returns `f(page_indices[d])` — so the
  **source row**'s index, whatever the current sort or page.

- **`selected(&[source…])`.** The application marks rows by their **source** index; `DataTable` only
  highlights those present in the current slice (a source → displayed position mapping).

- **The controlled model unchanged.** The selection state lives in the app
  (`data_selected: Option<usize>`); the widget only does the display mapping, as it does for
  `sort`/`page`.

- **Pagination helpers made public.** `page_count`, `page_rows`, `page_range_label` (already documented
  as "reusable outside the widget") are now re-exported — `rebuild` inlines its own index slicing, so
  they remain the reusable API they were advertised as.

## Implementation

- `frus-widgets/src/datatable.rs`: the `on_select`/`selected` fields + the `on_select_row`/`selected`
  builders; `sorted_order()` (a stable index sort); `rebuild` rewritten around `page_indices` (the
  mapping in both directions); the `selection_click_reports_the_source_row_through_sort_and_page` test
  (collecting the tree's `on_click` messages → checking the click returns the source index across
  sorting **and** pagination).
- `frus-widgets/src/lib.rs`: re-exporting `page_count`, `page_rows`, `page_range_label`.
- `frus-demo/src/lib.rs`: the `data_selected` state + `Msg::DataSelectRow` (toggling on re-click);
  `data_screen` wires `.on_select_row(...).selected(&[i])` and shows a **detail** of the selected row
  (read from the source data by its index).

## Verification

- **Widgets** `selection_click…`: an ascending sort on the key → the source order `[1,2,0]`; page 1
  (size 2) → the click returns `[1,2]`, page 2 → `[0]`. The mapping survives sorting and pagination.
- **Golden** `data_table_selected`: sorted by "Score" descending `[Bob 12, Dan 10, Ada 9, Carol 2]`;
  the **source** row 3 (Dan) appears highlighted in **2nd** position — visually inspected.
- **Demo** `data_table_screen_…` extended: a click = selecting the source row, a click elsewhere =
  moves it, a re-click = deselects.
- Widgets 374; goldens 69; demo 34; the shell compiles.

## What's left

- A **custom** sort key per `DataTable` column (dates, formatted amounts) — milestone 240.
- **Multiple** selection in `DataTable` (checkboxes, like `Table`).

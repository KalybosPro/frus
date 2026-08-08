# Jalon 242 — DataTable: search/filter

## Analysis

`DataTable` already encapsulates **display transforms** — sorting (milestone 232), pagination
(233/236), and the selection mapping across them (239/241). Search is one more: filtering the source
rows down to those matching a query, **before** sorting and pagination. Placing it upstream of the same
source-index pipeline means selection (single or multiple) keeps working on the visible subset, with no
extra code.

## Technical decisions

- **`searchable(query, on_query)`.** A search field (a [`TextInput`]) tops the table; `on_query(text)`
  reports each keystroke to the application (which updates `query` and, generally, returns to page 1).
  The widget does not store the query: it comes from the app at each render (the controlled model).

- **The filter at the head of `sorted_order`.** The index pipeline starts by keeping only the matching
  rows (`row_matches`), then sorts, then slices the page. `page_indices` stays a list of **source
  indices** (a subset) → the displayed position ↔ source mapping (a click, a box, the highlight) and the
  footer's total ("N–M of <filtered>") follow automatically.

- **`row_matches(row, query)`.** A **case-insensitive** substring over **every** column; an
  empty/blank query lets everything through. A public, reusable function (a reducer can filter the same
  way).

## Implementation

- `frus-widgets/src/datatable.rs`: the `row_matches` helper; the `query`/`on_query` fields + the
  `searchable` builder; the filter at the head of `sorted_order`; `rebuild` tops the block with a
  `TextInput` when `on_query` is set. The `row_matches_is_case_insensitive_substring_over_all_cells` and
  `search_filters_rows_before_sort_and_keeps_source_indices` tests (the query "a" → filtered then sorted
  into the source indices `[2, 0]`).
- `frus-widgets/src/lib.rs`: re-exporting `row_matches`.
- `frus-demo/src/lib.rs`: the `data_query` state + `Msg::DataSearch` (updates the filter, page → 1);
  `data_screen` wires `.searchable(app.data_query, Msg::DataSearch)`.

## Verification

- **Widgets**: `row_matches` (case, substring, columns, empty); `search_filters…` (the filter upstream
  of the sort, the source indices preserved).
- **Golden** `data_table_search`: an "ar" field + only `Bob (Paris)` and `Carol (Berlin)` out of four —
  visually inspected.
- **Demo** `data_table_screen_…` extended: typing updates `data_query` and returns to page 1.
- Widgets 378; goldens 72; demo 34; the shell compiles.

## What's left

- **Bulk** actions (an action bar when rows are checked).
- A **"no results"** message when the filter empties the table.
- A new widget domain (`Tabs`/`Tree`/`Kanban`).

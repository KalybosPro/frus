# Milestone 236 — DataTable: page size + "N–M of T" label

## Analysis

Pagination (milestone 233) placed a page-number selector, but two customary data-table cues were
missing: **how many** rows are shown ("1–3 of 7") and a way to **change the page size**. This
milestone enriches the footer.

## Technical decisions

- **A pure, public `page_range_label` slice label.** `N–M of T` (an en dash), or `0 of 0` when empty,
  with the page brought back into range. Reusable outside the widget, like `sort_rows`/`page_rows`.
  Always shown on the left of the footer when the table is paginated.

- **`.page_sizes(sizes, on_page_size)`.** Optional: a `SegmentedControl` of the offered sizes, on the
  right of the footer, with the current size preselected. `on_page_size(size)` on change — the app
  updates the size (and generally returns to page 1). No effect if not paginated.

- **The footer = a flex row.** `[label] [flex spacer] [Pagination] [size selector?]`. The internal
  `Box<dyn Widget>` (milestone 233) moves from `[table, pager]` to `[table, footer]`.

## Implementation

- `frus-widgets/src/datatable.rs`: `page_range_label`; the `page_sizes`/`on_page_size` fields; the
  `page_sizes` builder; `rebuild` composes the footer (the label + the pager + the
  `SegmentedControl`).
- `frus-widgets/src/lib.rs`: `page_range_label` added to the `pub use`.

## Verification

- **Widget** `page_range_label_describes_the_slice`: `1–3 of 7`, `4–6 of 7`, a partial last page
  `7–7 of 7`, `0 of 0` when empty, an out-of-range page brought back.
- **Widget** `page_size_selector_appears_in_the_footer`: a footer with **3** children
  (label+spacer+pager), **4** with the size selector.
- **Golden** `data_table_paginated` (enriched): "1–3 of 7" · ‹ 1 2 3 › · 3|5|10 (3 active).
- Widgets 373; goldens 68; the doctest OK.

## What's left

- Wiring `DataTable` (sorting + pagination + size) into the demo, removing the copied sort from the
  reducer.
- A **custom** sort key per column (dates, formatted amounts).

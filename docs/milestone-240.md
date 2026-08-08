# Milestone 240 — DataTable: custom sort key per column

## Analysis

`DataTable` sorts its rows with [`compare_cells`]: **numerically** if both cells read as numbers,
otherwise **case-insensitive text**. That is enough for "Name" or "Score", but it fails as soon as a
column carries values the default classifies badly:

- **priorities** (`High`/`Medium`/`Low`) → sorted alphabetically (`High, Low, Medium`), not
  semantically;
- **formatted dates** (`Mar 2024`) → a lexical sort ≠ a chronological one;
- **formatted amounts** (`$1.2M`, `$950k`) → do not parse as numbers, so an incorrect text sort.

This milestone lets the application supply a **per-column comparator**, while keeping the controlled
model (the sort state stays `(column, direction)` in the app).

## Technical decisions

- **`sort_with(col, cmp)`.** A `Fn(&str, &str) -> Ordering` comparator per column, stored in an indexed
  `Vec<Option<…>>` (like `Table`'s header actions). It defines the **ascending** order; the direction
  (`sorted(_, ascending)`) applies on top (reversed when descending).

- **Integrated into `sorted_order` (milestone 239).** The index sort consults the sorted column's
  comparator if there is one, otherwise falls back on `compare_cells`. The source index ↔ displayed
  position mapping (hence selection and pagination) works identically — it is the **same** index sort.

- **Local to the widget.** The comparator only affects `DataTable`'s display sort; the reusable
  [`sort_rows`] helper (the default sort) stays unchanged for the reducers that use it.

## Implementation

- `frus-widgets/src/datatable.rs`: the `comparators` field + the `sort_with` builder; `sorted_order`
  picks the custom comparator or the default; the `custom_comparator_orders_a_column_semantically` test
  (`Low < Medium < High`, through collecting the `on_click` messages → the displayed order is semantic,
  not alphabetical).
- `frus-demo/src/lib.rs`: a **"Level"** column added to `DATA_PEOPLE` (`High`/`Medium`/`Low`) + the
  `level_rank` helper; `data_screen` wires
  `.sort_with(3, |a,b| level_rank(a).cmp(&level_rank(b)))`; the row detail shows the priority.

## Verification

- **Widgets** `custom_comparator…`: three `High/Low/Medium` rows sorted ascending → the source indices
  `[1,2,0]` (Low, Medium, High), and not `[0,1,2]` from the text sort.
- **Golden** `data_table_custom_sort`: the "Priority" column sorted ascending → the display reads
  `Low, Medium, High` (and not `High, Low, Medium`) — visually inspected.
- **Demo** `data_table_screen_…` extended: a semantic `level_rank`; sorting the Level column renders.
- Widgets 375; goldens 70; demo 34; the shell compiles.

## What's left

- **Multiple** selection in `DataTable` (checkboxes, like `Table`).
- A **filter**/search above `DataTable` (the app filters the source rows).

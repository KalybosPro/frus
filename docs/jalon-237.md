# Jalon 237 — Demo: Data table screen (DataTable wired in)

## Analysis

`DataTable` (milestones 232/233/236) was tested in isolation but absent from the application. This
milestone **anchors it in the demo**: a new read-only screen that sorts and paginates a real data
set, wired to the state — end-to-end proof of ergonomics.

A deliberate contrast with the **editable grid** (the `Grid` route): that one sorts reducer-side
(`app.grid.sort_by`) because its cells are `TextInput`s bound to the row index. `DataTable`, being
read-only, **sorts its own display** — the app copies no sorting at all.

## Technical decisions

- **A new `Data` route** (index 6): added to the `enum`, to the `screen` dispatch, to
  `save_state`/`restore_state` (live reload) and to the drawer ("Data table →").

- **Minimal state: `(data_sort, data_page, data_page_size)`.** The reducer only flips the sort
  direction, changes page, changes size — it **never** reorders the data. `DataSort` and
  `DataPageSize` return to page 1. The `0`s (derived defaults) are coerced to starting values (page 1,
  size 5) in the screen.

- **`data_screen`.**
  `DataTable::new(headers, rows).on_sort(DataSort).paginated(page, per, DataPage).page_sizes([5,10], DataPageSize)`,
  plus `.sorted(col, asc)` if a sort is active. A set of 12 rows (name, role, score) → real
  pagination.

## Implementation

- `frus-demo/src/lib.rs`: `Route::Data` + the plumbing; the state fields +
  `Msg::{DataSort, DataPage, DataPageSize}` + the reduce arms; `DATA_PEOPLE` + `data_screen`; the
  drawer entry; the `DataTable` import.

## Verification

- **Demo** `data_table_screen_sorts_and_paginates_without_touching_data`: the screen renders; a first
  header click = ascending, a re-click = descending, sorting returns to page 1; changing the size
  returns to page 1. The source data is never reordered (the widget sorts the display).
- Demo 33; the widgets/goldens unchanged.

## What's left

- Wiring a **filtered/bounded** `DatePicker` into the demo (milestone 238).
- A "selected row" state (`on_select_row`) on the data screen.

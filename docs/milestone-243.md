# Milestone 243 — DataTable: bulk action bar

## Analysis

Once multiple selection is in place (milestone 241), the expected use is to **act** on the checked
rows: delete, export, move… Material tables then show a **contextual action bar** — "N selected" and
the action buttons — which only appears when a selection exists. This milestone adds it to `DataTable`,
leaving the application to supply the buttons (a slot).

## Technical decisions

- **`bulk_actions(make)`.** A **factory** of action widgets (called back at rebuild time, like
  `Table`'s header actions): the application builds its [`Button`](crate::Button)s with the variants and
  messages it wants. The widget freezes no action style — it supplies only the **slot** and the counter.

- **Visible only with a selection.** The bar is rendered **above** the table (below the search field if
  there is one) only if [`selected`](DataTable::selected) is non-empty; otherwise, nothing. The "N
  selected" label counts the selected rows (across all pages).

- **A controlled model, honest actions.** The messages the buttons emit are handled by the app. In the
  demo, `Delete` **really deletes** the checked rows: the table's data moves into the state
  (`data_rows`, `None` = the starting set) and is modified, with the selection and the focus reset.

## Implementation

- `frus-widgets/src/datatable.rs`: the `bulk_actions` field + the builder; `rebuild` prefixes the block
  with a `Flex` bar ("N selected" + a spacer + the action widgets) when a selection exists; the
  `bulk_actions_bar_shows_only_with_a_selection` test (a sentinel action appears with a selection,
  disappears without).
- `frus-demo/src/lib.rs`: `data_rows: Option<Vec<…>>` + the `TodoApp::data_rows` helper;
  `Msg::{DataClearChecked, DataDeleteChecked}` (Clear empties the selection; Delete removes the checked
  rows, in descending index order, then resets the selection/focus); `DataCheckAll` counts on the
  current rows; `data_screen` wires `.bulk_actions(|| [Clear, Delete])`.

## Verification

- **Widgets** `bulk_actions_bar…`: the bar absent with no selection, present (with an emittable action)
  as soon as a row is selected.
- **Golden** `data_table_bulk_actions`: two rows checked → "2 selected" + `Clear` (secondary) and
  `Delete` (danger) above the table — visually inspected.
- **Demo** `data_table_screen_…` extended: Clear empties the selection without touching the focus;
  Delete removes the checked row (12 → 11) and resets the selection/focus.
- Widgets 379; goldens 73; demo 34; the shell compiles.

## What's left

- An **empty state**: a "No results" message when the filter/the data empties the table — milestone 244.
- A confirmation before `Delete` (a dialog) in the demo.

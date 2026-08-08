# Milestone 171 — Table: fully widget header

## Analysis

The header could carry a text label (+ an icon, + an action widget on the right), but remained
**structurally** a text cell. Some grids want a **completely free** header: a bespoke sort
button, a built-in filter, a chip, a two-line title… It had to be possible to **replace** the
header row with **arbitrary widgets** (as `widget_row` did for data in milestone 164).

## Technical decisions

- **`widget_header`, mirroring `widget_row`.** `Table::widget_header(cells)` takes a
  **factory** per column (`Fn() -> Box<dyn Widget>`, in an `Rc`), called back at each rebuild.
  The header row is then built from widget cells with a **header background**, instead of text
  cells.

- **Sorting/reordering left to the application.** The table cannot guess how to sort an
  arbitrary header widget: **automatic** sorting and reordering do not apply here. The
  application **wires** the behaviour into its widgets (e.g. a header button emitting its own
  sort message). That is the accepted trade-off of total customisation — consistent with
  "sorting decided by the application" (the table only displays the state it is given).

- **`WidgetCell` reused, with a header background.** The existing widget cell gains a `header`
  flag (a header background + systematic background painting); widget headers are
  `WidgetCell { header: true, .. }`. No new kind of cell.

## Implementation

- `table.rs`: the `header_widgets: Vec<CellFactory>` field; the `widget_header` builder (clears
  `headers`, last call wins); the `WidgetCell.header` flag (the header background); the widget
  header branch in `rebuild`; `header_present` includes widget headers.
- `goldens.rs`: `table_widget_header` (a "User" chip + a "Sort" button).

## Verification

- **Unit**: `widget_header_hosts_arbitrary_header_widgets` — the header row hosts the supplied
  widgets ("Name", "Sort" painted); the bespoke header button emits **its** message (`Sort(1)`),
  proof that the app wires the sorting.
- **Golden** `table_widget_header` **inspected**: a chip + a button in the header, on a header
  background, data below — no regression on the other goldens.
- `cargo test --workspace` **green**.

## What's left

- **Mixing text + widget per column**: `widget_header` replaces the whole row; a "widget for
  some columns only" mode (the rest sortable text) would be an extension — not required here
  (the app can put a plain label widget in the others).
- **Reordering widget headers**: possible in future by exposing the `reorder_index`/`on_reorder`
  hooks from the supplied widgets.

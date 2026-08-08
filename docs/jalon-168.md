# Jalon 168 — Table: icon headers (+ sorting widget columns)

## Analysis

A table header could only show a **text label**. Real grids often top a column with an **icon**
(a category pictogram, a star for a rating…) — the icon **then** the label. And, since widget
cells (milestone 164), **sorting a widget column** deserved to be **documented**: the table does
not compare widgets, the application supplies the key.

## Technical decisions

- **A leading icon, decorative, without breaking sorting.** The header stays a `Cell` (so still
  **sortable** and **reorderable**); it gains an `icon: Option<IconName>` field, painted
  **before** the label. The label — and the sort indicator that follows it — shift by one icon
  width: icon + text + (▲/▼), coexisting cleanly.

- **An icon per column, on demand.** `Table::header_icons(&[Option<IconName>])`: `None` leaves
  the column without an icon. The icon is purely visual (no added semantics: the label already
  carries the screen-reader announcement).

- **Sorting widget columns: documented.** The table only emits the **clicked column**
  (`on_sort`); it is the **application** that orders its data by the corresponding field (the
  name behind an avatar, say), then passes the sorted rows back — as with text columns.
  Documented on `widget_row`.

## Implementation

- `table.rs`: the `ICON` / `ICON_GAP` constants; the `Cell.icon` field painted before the label
  (the text and the sort indicator shifted); the `Table.header_icons` field, the `header_icons`
  builder; a doc note on sorting widget columns (`widget_row`).
- `goldens.rs`: `table_header_icons` (a Menu icon + "Name", a Star icon + "Rating ▼").

## Verification

- **Unit**: `header_icon_shifts_label_and_paints` — the label of a header with an icon moves
  back by at least one icon width; a column with no icon is not shifted.
- **Golden** `table_header_icons` **inspected**: leading icons in front of the labels, the sort
  indicator preserved, the data aligned — no regression on the text goldens.
- `cargo test --workspace` **green**.

## What's left

- A **fully widget header** (beyond icon + label: a filter button, a menu): would require a
  header built from a factory while keeping sorting/reordering — heavier work, not required
  here.
- An icon **on the right** (after the label) or **clickable independently** of the sort: a
  possible extension should a concrete case demand it.

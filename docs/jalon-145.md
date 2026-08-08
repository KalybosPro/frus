# Jalon 145 — Table: sortable header & selectable rows

## Analysis

The existing `Table` (built on `Grid`) showed a styled header and rows of text, but was
**static**: no sorting, no selection, no interaction. To make it the basis of a business
application, it needed:

- A **sortable header**: clicking a column emits a message; an **indicator** (a ▲/▼
  triangle) marks the sorted column and its direction.
- **Selectable rows**: clicking a row emits a message; selected rows are **highlighted**.

## Technical decisions

- **The table orders nothing.** Faithful to frus's Elm architecture, it **emits** on click
  (`on_sort(column)`, `on_select_row(row)`) and only **displays** the state it is given
  (`sorted(col, asc)`, `selected(&rows)`). The application sorts the data and passes the
  state back — the widget stays a pure function of its inputs. The API is compatible:
  `header`/`row`/`width` unchanged.

- **Data first, the grid rebuilt.** Since `children()` must return an already-built
  subtree, the `Table` now stores its **data** (`headers`, `rows`) and its **state** (sort,
  selection, callbacks), and **regenerates** the `Grid` (`rebuild`) after each setting. So
  the builder call order does not matter: the final state is consistent (e.g.
  `on_select_row` set after the `row`s).

- **Interactivity at cell level.** A cell that returns `on_click(msg)` becomes a click
  target (the painted rect = the click area) — with no keyboard focus required. Each header
  cell carries its column's sort message, each data cell its row's selection message; all
  the cells of a selected row share the highlighted background, giving a row highlighted end
  to end.

- **A vector sort indicator.** Lacking an up/down arrow icon, the triangle is a small `Path`
  (3 segments) filled after the sorted header's label — crisp at any scale, with no
  dependency on the font.

## Implementation

- `table.rs`: `Cell<Msg>` gains `selected`, `sort`, `message` (click → sort/selection, hover
  through the state layer); `Table<Msg>` stores data + state + callbacks and `rebuild`s the
  grid; the new `on_sort`, `sorted`, `on_select_row`, `selected` constructors.
- `goldens.rs`: the `data_table` golden (a sorted header + a highlighted row).

## Verification

- **Unit**: a click on the column 1 header → `Sort(1)`, a click on the 2nd row → `Select(1)`
  (through `ui.hit` + `ui.msg_for`); the sort indicator paints a `Path`, the selected row
  paints a `primary`-tinted rect. The old test (6 cells) stays green.
- **Golden** `data_table` rendered and **inspected**: "Name ▲" with the triangle, the "Bob"
  row highlighted. `cargo test --workspace` green, no existing golden moved.

## What's left

- **Multiple selection / select all** (a header checkbox), **widget cells** (not just text)
  and **variable column widths** (today equal, through `Grid`).
- **Sorting from the keyboard** (Enter on a focused header).

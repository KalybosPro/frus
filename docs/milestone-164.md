# Milestone 164 — Table: widget cells (beyond text)

## Analysis

The table could only display **text** (`.row(&[&str])`). A real grid mixes status dots,
avatars, inline action buttons… It needed **widget cells**.

The crux: the table **rebuilds itself** after each setting (a rebuild per builder call, for
order independence). But a `Box<dyn Widget>` is **not clonable** — impossible to store one and
replay it at each rebuild.

## Technical decisions

- **A cell by factory.** A widget cell is supplied by a **factory**
  `Fn() -> Box<dyn Widget<Msg>>` (stored in an `Rc`, so shareable): `rebuild` **calls it back**
  to produce a **fresh** widget at each rebuild — compatible with the existing architecture,
  with no need to forward an `Rc<dyn Widget>` (≈ 50 methods). Data rows become a
  `RowKind::{Text, Widgets}`; **text stays unchanged** (no regression on existing tables).

- **A cell = a themed container.** `WidgetCell` takes the column width × the row height,
  centres its content (horizontal padding), paints the **cell background** (hover / selection)
  and stays **clickable** for row selection — the content (a button, a chip…) paints **on
  top**, and a clickable inner widget catches the click where it is (the topmost hit-test),
  the free area selecting the row.

## Implementation

- `table.rs`: the public `CellFactory<Msg>` type; `enum RowKind`; `WidgetCell` (background +
  centred content + a selection click); `rows: Vec<RowKind>`; `.widget_row(cells)`; `rebuild`
  handles both variants.
- `goldens.rs`: `table_widget_cells` (an avatar column + a `Chip` column).

## Verification

- **Unit**: a `widget_row` produces a row of cells **each containing a widget**; the content
  ("admin") is **painted**; the widget row stays **selectable** (`on_select_row`). Sorting /
  selection / resizing / reordering of text tables: unchanged.
- **Golden** `table_widget_cells` **inspected**: an **avatar** column ("A", "B") and a **chip**
  column ("admin", "editor"), centred in their cells.
- `cargo test --workspace` **green**.

## What's left

- **Sorting widget columns**: the app supplies the sort key (the table cannot compare widgets)
  — already possible app-side, to be documented.
- **Widget header cells** (an icon + a label) and a **row height adaptive** to the content
  (today a fixed `ROW_H`: taller content is cropped).

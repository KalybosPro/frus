# Jalon 197 — Editable grid: interactive wiring

## Analysis

Milestone 196 proved (by golden) that `Table` can show a `TextInput` per cell; what remained was
**wiring it up** for real: click a cell to edit it, type, commit — a mini spreadsheet in the demo.
That is the concrete application of the inline editing pattern.

## Technical decisions

- **A dedicated route, minimal state.** `Route::Grid` (reachable from the drawer) shows a grid whose
  state fits in two `TodoApp` fields: `grid: Vec<Vec<String>>` (the data) and
  `grid_edit: Option<(row, column)>` (the cell being edited). Demo data seeded in `init`.

- **Swapping the widget per cell.** `grid_screen` builds a `Table::widget_row` per row; each cell is
  a **factory** that returns, depending on `grid_edit`:
  - at rest, a clickable `Container` (`on_click` → `GridEdit(r, c)`) showing the value;
  - while editing, a bound `TextInput` (`on_input` → `GridInput`, `on_submit` → `GridCommit`).
  No `Table` code changed: this is milestone 196's composition, driven by state.

- **Focusing the cell immediately (milestone 198).** `GridEdit` wraps the future `TextInput` in
  `keyed(("grid", r, c))` and returns `Command::focus(("grid", r, c))`: on the next build, the caret
  lands **inside** the clicked cell — the click opens and focuses in one go.

- **A pure editing cycle.** `reduce` handles `GridEdit` (open + focus), `GridInput` (update the
  targeted cell), `GridCommit` (close). Everything derives from `grid` / `grid_edit`.

## Implementation

- `frus-demo/src/lib.rs`: `Route::Grid` (+ `save_state`/`restore_state`); the `grid` / `grid_edit`
  fields (seeded in `init`); `Msg::{GridEdit, GridInput, GridCommit}` (+ the `reduce` arms);
  `grid_screen`; the drawer entry.

## Verification

- **Integration** (`grid_click_edit_commit`): the grid renders; clicking a cell opens it for editing
  **and** requests its focus (`!cmd.is_empty()`); typing updates the right cell; the commit closes
  it; the other cells stay intact. The 19 demo tests stay **green** (20 in total).
- **Visual**: identical to the `table_editable` golden (milestone 196) — one `TextInput` cell among
  clickable text cells; this milestone makes its behaviour interactive.
- `cargo build -p frus-demo` **clean**.

## What's left

- **Keyboard navigation** (Tab → the next cell, Enter → the next row) — chaining the
  `("grid", r, c)` focus keys.
- **Adding / removing rows**, sorting, per-cell validation (`TextInput::error` + `Form`).

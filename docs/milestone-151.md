# Milestone 151 — Table: mouse column resizing

## Analysis

The table (milestones 145–149) sorted, selected and checked, but its columns were
**frozen**: no handle to adjust a width with the mouse, though that is an expected gesture
in any data grid (data tables, spreadsheets).

The blocker identified back in milestone 149: it requires an **absolute drag** in the shell,
whereas the only existing mechanism (`on_drag(fraction)`, used by `Slider`) gives a
**fraction clamped** to the widget's bounds — on a thin handle, the fraction saturates
immediately and cannot **grow** a column.

## Technical decisions

- **A generic delta drag.** A new `Widget::on_drag_delta(dx)`: `dx` is the horizontal
  movement (px) **since the last event**. The shell tries it **before** `on_drag`; a widget
  implements only one of the two (`Slider` = fraction, a handle = delta). `Drag::Widget`
  remembers `last_x` to deliver the incremental delta. That **incremental** choice (rather
  than absolute from the start) composes with tree rebuilding: each small message is
  **accumulated** by the application, with no double counting when the view is rebuilt
  mid-drag.

- **Handles in a floating layer.** The table overlays (through `Stack`) a **handle layer**
  on top of the grid: a thin vertical bar at each column's right edge (except the last),
  positioned by **transparent shims** (`Spacer`) on the columns' exact geometry. The shims
  are **inert** (neither clickable nor focusable): sort / selection clicks **pass through**
  the layer to the grid (only *clickable* widgets populate the hit-test table), while
  `draggable_at` only catches the handles. A drag suppresses the release click (already the
  case for every `Drag::Widget`), so grabbing a handle does not trigger a sort.

- **Controlled, like everything else.** `on_resize(column, delta)`: the application
  **accumulates** the width (`widths[col] = (widths[col] + delta).max(MIN)`) and passes it
  back through `column_widths`. The table stores no "live" width.

- **Only with fixed columns.** The handles only appear if **every** column has a fixed width
  (known edges); one flexible column disables the layer (its rendered width is unknown to
  the widget).

## Implementation

- `widget.rs`: `on_drag_delta` (default `None`) + the `Box` forwarder; `keyed.rs`,
  `responsive.rs`: forwarders.
- `app.rs` (shell): `Drag::Widget` gains `last_x`; `apply_widget_drag(id, rect, dx)` tries
  `on_drag_delta(dx)` then `on_drag(fraction)`; the drag computes the incremental delta.
- `table.rs`: `Spacer` (an inert shim) + `ResizeHandle` (a draggable handle,
  `on_drag_delta` → `on_resize(col, dx)`); the `on_resize` field (`Rc`) + `.on_resize()`;
  `resize_overlay` builds the layer; `rebuild` wraps the grid in a `Stack` and forwards
  `stack()`.
- `goldens.rs`: the `data_table_resizable` golden (3 fixed columns, the handles visible).

## Verification

- **Unit**: the handle emits `Resize(0, 12.0)` for a 12 px delta and `None` for a null
  delta; it is **grabbable** (`draggable_at`) at the 1st column's edge (x≈100); **flexible**
  columns produce **no** handle. Sorting / selection / checkboxes unchanged (tests green).
- **Golden** `data_table_resizable` **inspected**: thin vertical bars at the right edge of
  "Name" and "Role", none after "Score" (the last column).
- `cargo test --workspace` **green** (the slider included — the `on_drag` fraction path is
  preserved).

## What's left

- An **`ew-resize` cursor** when hovering a handle (a Material visual cue) — requires a
  per-widget cursor notion in the shell.
- **Resizing flexible columns** (measuring the rendered width to seed the delta) and
  **reordering** columns (dragging a header).

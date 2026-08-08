# Jalon 153 — Table: column reordering (dragging a header)

## Analysis

After resizing (milestone 151), the table was still missing column **reordering** — moving
a column by dragging its header, a gesture expected of any data grid.

The crux: a header must stay **sortable on click** *and* become **reorderable on drag**.
Unlike the resize handles (a separate widget), it is the **same** widget that has to tell
the two gestures apart. The shell already knew how to do that for pan / touch scrolling (the
`TOUCH_SLOP` threshold, the `moved` flag, `was_tap`) but not for a widget drag.

## Technical decisions

- **Tap-vs-drag by threshold, reused.** A new `Drag::Reorder { from, start, moved }`:
  pressing on a reorderable header **arms** it without engaging the drag; below the
  `TOUCH_SLOP` threshold, releasing is still a **sort** (`was_tap`); beyond it, it is a
  **reorder** and the click is suppressed (as with any drag). No new logic: we mirror the
  pan's `moved` model exactly.

- **The target column = the hit-test table, no new registry.** Two trait methods:
  `reorder_index()` (this header is reorderable → its column) and `on_reorder(to)` (the
  **source** header knows its index and the callback). On drop, the shell resolves the
  **target** column by re-reading `reorder_index()` from the header under the cursor —
  through the **existing** hit-test table (sortable headers are already clickable). Zero new
  collection when building the UI.

- **Controlled.** `on_reorder(from, to)`: the application permutes its column order and
  rebuilds. The table stores no "live" order.

- **No ghost rendering (MVP).** The column drops "dry"; the sliding preview (a
  semi-transparent proxy, neighbours shifting) is left to what's next — the gesture and the
  routing are in place.

## Implementation

- `widget.rs`: `reorder_index` / `on_reorder` (default `None`) + the `Box` forwarder;
  `keyed.rs`, `responsive.rs`: forwarders.
- `app.rs` (shell): `Drag::Reorder`; `reorderable_at` (hit-test → header → column); arming on
  press (without a `return`, to keep tap = sort); `moved` tracking at the threshold; on
  release, the target column under the cursor → `on_reorder(from, to)` routed, unless
  `to == from`.
- `table.rs`: `Cell` gains `reorder: Option<(usize, Rc<…>)>` + `reorder_index`/`on_reorder`
  (headers only); the `on_reorder` field (`Rc`) + `.on_reorder()`.

## Verification

- **Unit**: each header exposes its column (`reorder_index` = 0, 2…) and produces
  `Reorder(from, to)`; **clicking still sorts** (`on_click` = `Sort`); **data** cells are not
  reorderable. Sorting / selection / resizing unchanged.
- **Not covered by a unit test**: the end-to-end gesture (press → threshold → drop) lives in
  the shell, with no pointer-event harness (a real window would be required); it faithfully
  replicates the pan's already-proven `moved` / `was_tap` model.
- `cargo test --workspace` **green**.

## What's left

- A **sliding preview**: a semi-transparent proxy of the grabbed header + an animated shift
  of the neighbouring columns + a highlighted drop zone.
- **Keyboard reordering** (Ctrl+Arrows on a focused header).

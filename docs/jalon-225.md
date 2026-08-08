# Jalon 225 — Unpinning on a second click

## Analysis

Since milestones 221–223, clicking a point/bar pins it (the detail in a `Chip`) and highlights it (a
halo/ring). But there was no way to **remove** the selection: once pinned, the dashboard stayed marked
until another element was clicked. The missing gesture, standard for a selector: **re-click** the
already-selected element to deselect it.

## Technical decisions

- **The toggle app-side, not widget-side.** The charts report a click through `on_point`; it is up to
  the application to decide that a second click on the **same** `(category, series)` cancels the
  selection. `reduce(ChartPoint)` compares the target to `chart_sel`: if it is already selected, we
  reset `chart_sel` and `chart_pin` to `None`; otherwise we pin as before. The widget did not change —
  the highlight disappears on its own as soon as `selected` goes back to `None`.

- **The hint updated.** The help text becomes "click a point to pin it, or again to unpin".

## Implementation

- `frus-demo/src/lib.rs`: `reduce(Msg::ChartPoint)` toggles (unpins if
  `chart_sel == Some((cat, s))`, pins otherwise); the hint text updated.

## Verification

- **Demo** `re_clicking_a_selected_point_unpins_it`: `ChartPoint(2, 1)` pins
  (`chart_sel = Some((2, 1))`, the detail present); a second `ChartPoint(2, 1)` **unpins**
  (`chart_sel = None`, the detail cleared); `ChartPoint(0, 0)` pins another point again.
- The existing tests (`clicking_a_point_pins_its_detail`, `clicking_a_point_marks_it_selected`) click
  **distinct** points: unchanged. Demo 32; the widgets/goldens untouched.

## What's left

- **Percentage** labels in the tooltip in 100% mode (today: raw values).
- Moving out of the charts domain (a new widget: an advanced `Calendar`/`DataTable`).

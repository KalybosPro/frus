# Jalon 223 — Pinned point/bar highlighted (persistent halo + ring)

## Analysis

Milestones 221/222 make points and bars **clickable** and pin the detail in a `Chip`. But nothing in
the chart showed **which** element the pin came from: the hover accent (milestones 211/217) vanishes
as soon as the pointer leaves the area. A **persistent** highlight of the current selection was
missing.

## Technical decisions

- **`.selected(Option<(category, series)>)` on both charts.** An `Option` signature so the app's state
  (`Option<(usize, usize)>`) plugs straight in. `None` = nothing highlighted. A purely additive field:
  `None` by default, the paint **unchanged** (the goldens safe).

- **`LineChart`: a halo + a ring on the marker.** After plotting the series (so on top), if a point is
  selected — outside stacked mode, outside a hidden series — we lay down a translucent halo
  (`MARKER_R + 6`, `α·0.22`) then a solid ring (`MARKER_R + 3`, 2 px) in the series' colour.
  Independent of hover: the highlight stays as long as the selection holds.

- **`BarChart`: a contrasting ring around the bar.** The selected bar/stratum's rectangle is
  **captured** during the paint loop (the same geometry as the plot), then a ring dilated by 2.5 px,
  with a 2 px `on_surface` border (a contrasting colour, readable over any coloured bar), is drawn
  afterwards. It works both grouped **and** stacked.

- **The app retains the selection.** `Msg::ChartPoint(cat, series)` now also sets
  `chart_sel = Some((cat, series))`; `dashboard_chart` passes `.selected(app.chart_sel)` to the
  **main** chart (lines or bars). Clicking a point/bar rings it immediately.

## Implementation

- `frus-widgets/src/chart.rs`: the `selected` field + the `.selected` builder on `BarChart` and
  `LineChart`; `BarChart::paint` captures `sel_rect` and draws the ring; `LineChart::paint` draws the
  halo + ring on the selected marker.
- `frus-demo/src/lib.rs`: the `chart_sel` state; `reduce(ChartPoint)` fills it; both branches of
  `dashboard_chart` wire `.selected(app.chart_sel)`.

## Verification

- **Widget** `selected_bar_draws_a_persistent_ring`: the pinned bar adds a bordered rectangle (0
  without a selection, 1 with); a pinned hidden series adds none.
- **Widget** `selected_point_draws_a_persistent_ring`: the pinned point adds a **stroked** circle (0
  without a selection, 1 with); a pinned hidden series: none.
- **Demo** `clicking_a_point_marks_it_selected`: `ChartPoint(3, 0)` → `chart_sel = Some((3, 0))`,
  following the last click.
- **Goldens** `line_chart_selected` + `bar_chart_selected` (61 in total): the halo/ring visible.
- Widgets 355, demo 30, shell 25; the suite green.

## What's left

- Normalising stacking to **100%** (proportions rather than absolute values).
- **Unpinning** (a re-click on the selected element to clear `chart_sel`/`chart_pin`).

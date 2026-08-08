# Milestone 222 — Clicking a bar: pinned detail (BarChart::on_point)

## Analysis

Milestone 221 made a `LineChart`'s **points** clickable (`on_point(cat, series)`), but `BarChart`s
stayed inert: a dashboard switching to "grouped bars" or "stacked bars" lost the interaction. This
milestone brings bars to **parity** with line points.

## Technical decisions

- **`BarChart::on_point(cat, series)`.** A mirror of `LineChart::on_point`. After the legend test,
  `positional_click` rebuilds the paint's geometry and looks for the **rectangle** (a grouped bar or a
  stacked stratum) containing the local point; it returns `on_point(category, series)`.

- **Hit-testing both layouts.** When **grouped**, each series `j` occupies a sub-bar
  `[bx, bx + draw_w]` (with the `inner = 0.86` factor and the `(bar_w - draw_w)/2` offset, identical
  to the paint) of height `(value/max)·plot_h`. When **stacked**, each stratum is a full-width segment
  `[sbx, sbx + group_w]` between its lower and upper cumulative totals. A **hidden** series does not
  count (an unplotted bar = not clickable), exactly as in the paint.

- **The app reuses `Msg::ChartPoint`.** The message and the `series · category = value` formatting
  (milestone 221) are family-independent: wiring `.on_point(Msg::ChartPoint)` onto the dashboard's
  `BarChart` is enough to pin the detail on a bar click.

## Implementation

- `frus-widgets/src/chart.rs`: `BarChart` gains the `on_point` field + the `.on_point` builder;
  `positional_click` moves from "legend only" to "legend **then** bars" (the same structure as
  `LineChart`), with the grouped/stacked hit-test and respect for `hidden`.
- `frus-demo/src/lib.rs`: `dashboard_chart`'s `BarChart` branch wires `.on_point(Msg::ChartPoint)`
  when the legend is on (the main chart).

## Verification

- **Widget** `clicking_a_bar_emits_category_and_series`: a click at the centre of category A's 2nd bar
  (the additional series) → `(0, 1)`; above the bar → `None`; when stacked, a click at the bottom of
  the column → stratum `0`; a bar of a **hidden** series → `None`.
- **Demo** `grouped_bars_are_clickable_in_dashboard`: the main chart in grouped bars emits
  `ChartPoint` on at least one point of its area (a sweep, independent of the internal constants).
- Widgets 353, demo 29; the goldens with no regression (an additive change, the paint unchanged).

## What's left

- A **highlighted pinned** bar/point (a persistent ring) in the chart — milestone 223.
- Normalising stacking to **100%** (proportions rather than absolute values).

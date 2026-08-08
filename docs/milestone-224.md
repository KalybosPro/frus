# Milestone 224 — 100% stacking (normalised proportions)

## Analysis

Stacking (milestones 213/216) shows cumulative **absolute values**: a column's height depends on its
total, which flattens the reading of **proportions** when the totals vary a lot. The "100%" chart
(bars or areas) answers a different question — *what share does each series take in each category?* —
by normalising each category to its own total.

## Technical decisions

- **`.normalized(bool)` on both charts.** It only has an effect in multi-series stacked mode
  (`normalized = self.normalized && stacked`). Additive: `false` by default, the paint **unchanged**
  (the goldens safe).

- **A per-category denominator.** `category_total(i)` sums the **visible** series of category `i`
  (respecting `hidden`, clamped to `1e-6` to avoid a division by zero). In 100% mode, each stratum is
  plotted over `value / category_total(i)` instead of `value / global_scale`: each column (bars) or
  each category (areas) then fills **the whole height**.

- **A percentage axis.** `draw_grid` gains a `percent` parameter: in 100% mode, the ticks show
  `0%..100%` instead of the values. Shared by both charts.

- **A toggle in the app.** A "100% stacking" `Switch` (visible only for the stacked types) drives
  `chart_normalized`; `dashboard_chart` passes `.normalized(app.chart_normalized)` to the stacked
  branches (stacked areas, stacked bars).

## Implementation

- `frus-widgets/src/chart.rs`: the `normalized` field + the `.normalized` builder + `category_total`
  on `BarChart` and `LineChart`; `draw_grid` gains `percent`; each paint's stacked branch uses the
  per-category denominator (`denom` for the bars, the `spt` closure for the areas).
- `frus-demo/src/lib.rs`: the `chart_normalized` state, `Msg::SetChartNormalized`, `reduce`, the
  `Switch` in `charts_screen`, `.normalized(...)` on both stacked branches of `dashboard_chart`.

## Verification

- **Widget** `normalized_stacked_bars_fill_each_column`: column A (total 5) is **full** at 100% but
  partial in absolute mode (the max, 8, is in B); the axis shows `100%`.
- **Widget** `normalized_stacked_areas_fill_to_the_top`: the upper edge's stroke is **flat** at 100%
  (plot_top) everywhere when normalised, but follows the totals in absolute mode.
- **Demo** `normalized_toggle_applies_to_stacked_kinds`: the toggle sets `chart_normalized`, both
  stacked types render.
- **Goldens** `bar_chart_normalized` + `line_chart_normalized` (63 in total): full columns/areas, a
  percentage axis.
- Widgets 357, demo 31, shell 25; the suite green.

## What's left

- **Unpinning** (a re-click on the selected element to clear `chart_sel`/`chart_pin`).
- **Percentage** labels in the tooltip in 100% mode (today: raw values).

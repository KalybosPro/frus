# Jalon 209 — Charts: multiple series + legend

## Analysis

`LineChart` only drew one series. Comparing (sales vs costs, this year vs last) calls for
**several series** sharing the same categories and the same scale, plus a **legend** to tell them
apart. It is the brick that takes the chart from a demo to a real tool.

## Technical decisions

- **A main series + additional ones, aligned by index.** `new(...)` supplies the categories and the
  first series; `.series(name, color, values)` adds others (values aligned by index). `.name(...)`
  names the main one. Backwards compatible: without `.series`, the rendering is milestone 200/206's.

- **An explicit colour per additional series** (the customisable line): no hidden palette imposed —
  the caller supplies the colour, the main one keeps `color`/`theme.primary`.

- **A shared scale and axis.** `max_value` spans **all** the series; the grid and ticks (milestone
  203) are shared, so the curves are directly comparable.

- **Less noise with multiple series.** The per-point value labels (milestone 200) and the area
  (milestone 206) only show with a **single series**; with several, we rely on the axis and the
  legend.

- **The legend.** `.legend(bool)` draws a band at the top (a colour swatch + a name per series), and
  reserves `LEGEND_H` above the plot area. It only shows if enabled **and** at least one series is
  named.

## Implementation

- `frus-widgets/src/chart.rs`: the `name` / `extra` / `legend` fields on `LineChart`; the `.name` /
  `.series` / `.legend` builders; `max_value` across every series; `has_legend`; the paint
  restructured (a loop per series + the legend band); the `LEGEND_*` constants.
- `frus-test/tests/goldens.rs`: the `line_chart_multi` golden (2 series + the axis + the legend).

## Verification

- **Unit** `multi_series_draws_each_line_and_a_legend`: two polylines, two `~10x10` swatches, and
  the `Sales`/`Costs` names in the legend. `max_value_spans_all_series`: the scale does take the
  additional series' maximum.
- **Golden** `line_chart_multi`: two coloured curves, the legend at the top, a shared axis.

## What's left

- **Stacked** series (cumulative areas), a grouped/stacked multi-series **BarChart**, and a
  clickable legend to hide/show a series.

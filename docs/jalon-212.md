# Jalon 212 — BarChart brought up to LineChart's level: grouped + legend + tooltip

## Analysis

`LineChart` gained multiple series (209), a legend (209) and a hover tooltip (211). `BarChart` had
stayed single-series. This milestone gives it the same capabilities — **grouped** bars, a legend, a
tooltip — by **factoring out** what the two charts share.

## Technical decisions

- **The same multi-series model as LineChart (milestone 209).**
  `.name` / `.series(name, colour, values)` / `.legend(bool)`; `max_value` across every series.
  Backwards compatible: without `.series`, milestone 199's rendering.

- **Grouped bars.** Per category, a centred **group** of `s` bars side by side (the group's width =
  `slot * BAR_FILL`, divided into `s`). Value labels only show with a single series (avoiding
  clutter); category labels once.

- **Three shared helpers, zero duplication.** Adding the legend and the tooltip to BarChart was the
  moment to pull `draw_legend`, `draw_tooltip` and `chart_plot_hit` out as free functions, used by
  **both** charts. `LineChart` was rewired onto them (behaviour unchanged, its goldens and tests
  confirm it).

- **The tooltip and tracking reuse milestones 208/211.** `BarChart::cursor_icon` turns on
  `hover_cursor` over the plot area through `chart_plot_hit`; `paint` lists each series' value at the
  hovered category through `draw_tooltip`.

## Implementation

- `frus-widgets/src/chart.rs`: the `draw_legend` / `draw_tooltip` / `chart_plot_hit` helpers;
  `LineChart` rewired onto them; `BarChart` gains `name` / `extra` / `legend`, `max_value` /
  `has_legend`, a grouped paint + the legend + the tooltip, and `cursor_icon`.
- `frus-test/tests/goldens.rs`: the `bar_chart_grouped` golden (2 series + the axis + the legend).

## Verification

- **Unit** `grouped_series_draw_a_bar_per_series_and_a_legend` (6 bars = 3×2, 2 swatches, the names
  in the legend); `hovering_bars_shows_a_tooltip_guide` (a guide on hover, `cursor_icon` Some over
  the area). The existing LineChart tests pass (a regression-free refactor).
- **Golden** `bar_chart_grouped`; `line_chart_multi` and the others unchanged.

## What's left

- **Stacked** bars (a per-category total), a tooltip tracking the **exact bar** under the pointer
  (not just the category), and an additional series' colour drawn from a default palette when
  omitted.

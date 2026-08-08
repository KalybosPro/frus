# Milestone 203 — Charts: y-axis + grid (shared)

## Analysis

Milestones 199/200 gave us bars and a curve, but with **no reading reference**: there was no way to
estimate a value without its label. A y-axis (ticks + horizontal grid lines) answers that, and it
must be **common** to both charts — they already share their geometry.

## Technical decisions

- **A shared, opt-in axis.** A free `draw_grid(...)` function draws `divisions` horizontal lines
  spread between the baseline and the top of the plot area, each labelled with its value (`0..max`)
  right-aligned in a left margin. `BarChart` and `LineChart` both gain the same `.grid(divisions)`
  (default `0` = no axis) and call `draw_grid` before painting.

- **Non-breaking.** Without `.grid(...)`, `axis_width` returns `0`, the plot area stays full width
  and the rendering is **identical** to milestones 199/200 (their goldens are unchanged). With an
  axis, a `Y_AXIS_W` margin shifts the bars and points right to make room for the ticks.

- **The grid reads behind.** Grid lines in a muted `theme.border`, ticks in `theme.muted`: present
  without masking the data.

## Implementation

- `frus-widgets/src/chart.rs`: the `Y_AXIS_W`, `AXIS_SIZE` constants; the free `axis_width` and
  `draw_grid` functions; the `grid: usize` field + `.grid(n)` on `BarChart` **and** `LineChart`;
  paints shifted by the axis margin (`plot_left`, `plot_w`).
- `frus-test/tests/goldens.rs`: the `line_chart_axis` golden (`grid(4)`).

## Verification

- **Unit** (`grid_draws_horizontal_lines_and_axis_labels`, `no_grid_by_default_keeps_full_width`):
  with `grid(4)`, at least 5 thin lines (4 grid + the baseline) and the `0` and `8` ticks are drawn;
  with no grid, no ticks at all.
- **Golden** `line_chart_axis`: the `Mon..Fri` series with a horizontal grid and the scale on the
  left.

## What's left

- A **filled area** under the curve, **multiple series** + a legend, "round" ticks (a nice-multiples
  scale), an x-axis labelled independently of the number of points.

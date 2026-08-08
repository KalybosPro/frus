# Jalon 199 — Charts: bar chart

## Analysis

The **charts** domain was untouched: there was no way to visualise numeric data (statistics, time
series…). The first brick: a **bar** chart, simple and themed.

## Technical decisions

- **A self-painted, data-driven view.** `BarChart` takes a `(label, value)` series and paints it
  itself (bars, values, labels, baseline) — no children, not generic over `Msg` (like
  [`Icon`](../crates/frus-widgets/src/icon.rs)): it is a **view**, not a control. The bars are
  scaled to the **maximum value** (clamped to 1 for a stable scale even with zero values).

- **Themed and customisable.** Bars in the theme's `primary` (overridden by `color`), values in
  `on_surface`, labels in `muted`, the baseline in `border`. `height` sets the height.

- **Fills the width.** `width: Percent(1.0)` (the chart takes the width offered): so the parent must
  have a **defined width** (otherwise the `Percent` width collapses to 0 — a layout trap hit and
  fixed in the golden by setting the container's width).

- **Value formatting**: an integer if the value is one, otherwise one decimal.

## Implementation

- `chart.rs`: `BarChart` (`new` / `color` / `height`); `impl<Msg> Widget` (self-painted); the
  `max_value` / `format_value` helpers.
- `lib.rs`: `mod chart` + `pub use chart::BarChart`.
- `goldens.rs`: `bar_chart` (a Mon–Fri series).

## Verification

- **Unit**: `value_formatting` (3.0 → "3", 2.5 → "2.5"); `empty_series_paints_nothing`;
  `bars_scale_to_the_max_value` (one bar per value, the largest value → the tallest bar,
  proportional; values and labels drawn).
- **Golden** `bar_chart` **inspected**: five proportional bars (max = Thu 8), the values above, the
  labels below, the baseline.
- `cargo test -p frus-widgets chart::` **green**.

## What's left

- A **line chart** (a polyline) — requires stroking a line (segments); rects are enough for bars,
  not for lines.
- A **y-axis / grid** (ticks, reference values) and **stacked / grouped bars**.
- **Interaction** (hovering a bar → a value tooltip) — through the existing hover state.

# Jalon 221 — Clicking a chart point → pinned detail

## Analysis

The clickable legend (milestone 215) already routes a sub-region click to a message. The next gesture
a dashboard is expected to have: **clicking a point** on the curve to pin its value. Hit-testing a
point requires the box's **height** — which `positional_click(local_x, local_y, width)` did not
supply.

## Technical decisions

- **`positional_click` gains `height`.** The trait's signature becomes
  `(local_x, local_y, width, height)` — the shell already passes `rect`, so it passes `rect.height`
  too. A mechanical change propagated to every widget (`TextInput`, `keyed`, `responsive`, the
  default); only the charts use it. It unblocks any sub-region hit-test that depends on vertical
  geometry.

- **`LineChart::on_point(cat, series)`.** After the legend test, `positional_click` rebuilds the
  paint's geometry and looks for a **marker** (of the **visible** series) within a `POINT_HIT_R`
  radius. Outside stacked mode, where individual markers do not exist. It returns
  `on_point(category, series)`.

- **The app pins the detail.** `Msg::ChartPoint(cat, series)` formats `series · category = value`
  from the shared data (`CHART_SERIES` / `CHART_CATS`) into `chart_pin`, shown as a `Chip` under the
  chart. Bars stay non-clickable (segments: see What's left).

## Implementation

- `frus-widgets`: `positional_click` gains `height` (the trait + `TextInput` / `keyed` / `responsive`
  / `Box`); `LineChart` gains `on_point` + the point hit-test; `POINT_HIT_R`.
- `frus-shell/src/app.rs`: passes `rect.height` to `positional_click`.
- `frus-demo/src/lib.rs`: `Msg::ChartPoint` + the `chart_pin` state + `reduce` (the formatting);
  `.on_point(Msg::ChartPoint)` wired onto the main chart; the pin `Chip` in `charts_screen`.

## Verification

- **Widget** `clicking_a_point_emits_category_and_series`: a click on the main series' point A →
  `(0, 0)`; far from a marker → `None`; a point of a **hidden** series → `None`.
- **Demo** `clicking_a_point_pins_its_detail`: `ChartPoint(3, 0)` → `Sales · Thu = 8`, replaced by
  `ChartPoint(1, 1)` → `Costs · Tue = 4`. Widgets 352, demo 28, shell 25; the goldens with no
  regression.

## What's left

- Clicking a bar **segment** (`BarChart::on_point`, hit-testing the rectangles) and a stacked
  stratum, and a **highlighted pinned** point (a persistent halo) in the chart.

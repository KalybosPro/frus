# Milestone 200 — Charts: line chart (LineChart)

## Analysis

Milestone 199 opened the "charts" domain with [`BarChart`]: ideal for **comparing** magnitudes. To
read a **trend** (a series over time), the natural form is the **polyline** — points joined by
segments. It is the domain's second widget, and the first widget-side consumer of
`Scene::stroke_path` (a path outline, with no fill).

## Technical decisions

- **The same geometry as the BarChart.** `LineChart` reuses the layout identically: the value band
  at the top, the category labels under the baseline, a `0..max` scale. One point per category,
  centred in its "slot", at a height proportional to the value. So a BarChart and a LineChart of the
  same series read **in the same place**.

- **A vector stroke rather than rectangles.** The curve is a `Path` (a `move_to` then `line_to`s)
  rendered by `scene.stroke_path` — the first widget-side use of a path **outline**. Each point
  carries a round **marker** (a filled `Path::circle`) so it stays readable even when flat.

- **Self-painted, non-generic, themed (like BarChart / Icon).** No children, no `Msg`: it is a data
  **view**. `color` overrides the stroke (default `primary`), `height` the height (default 200);
  `width: Percent(1.0)` — so the parent must be **sized**.

## Implementation

- `frus-widgets/src/chart.rs`: `LineChart` (`new`, `color`, `height`); `paint` computes the points,
  strokes the polyline (`stroke_path`), places the markers (a `fill_path` of circles), the values
  and the labels. The `MARKER_R`, `LINE_W` constants; reuses `format_value` and the BarChart's
  geometry.
- `frus-widgets/src/lib.rs`: the `LineChart` export.
- `frus-test/tests/goldens.rs`: the `line_chart` golden (the same series as `bar_chart`).

## Verification

- **Unit** (`line_empty_series_paints_nothing`, `line_connects_all_points`): an empty series →
  nothing; three points → a stroked polyline (a `stroke: Some, fill: None` path) of **two**
  segments, one filled marker per point, values and labels drawn.
- **Golden** `line_chart`: the `Mon..Fri` series as a curve, markers, values, baseline.

## What's left

- A **y-axis** (ticks + a horizontal grid), a filled area under the curve, multiple series (a
  legend), hovering a point → a tooltip.

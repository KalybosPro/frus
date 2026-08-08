# Jalon 206 — Charts: filled area under the curve

## Analysis

`LineChart` (milestone 200) draws the **trend**; to emphasise **volume** (a cumulative total, a
share), chart libraries fill the area between the curve and the baseline. It is a natural addition,
reusing the non-zero path fill already in place.

## Technical decisions

- **Opt-in, a closed polygon.** `.area(bool)` (default `false`). When on, we build a `Path`: from
  the baseline below the first point, we join every point of the curve, then come back down to the
  baseline below the last point. The non-zero fill closes the contour automatically.

- **Painted underneath.** The area is filled **before** the polyline and the markers (the stroke
  colour heavily muted, `AREA_ALPHA = 0.16`), so the line stays crisp on top.

- **Composes with the axis.** The area and the y-axis (milestone 203) are independent: the golden
  combines `.grid(4).area(true)`.

## Implementation

- `frus-widgets/src/chart.rs`: the `AREA_ALPHA` constant; the `fill: bool` field + `.area(bool)`;
  filling the polygon before the stroke in `LineChart`'s paint.
- `frus-test/tests/goldens.rs`: the `line_chart_area` golden (`grid(4)` + `area(true)`).

## Verification

- **Unit** `area_fills_a_polygon_under_the_curve`: with `.area(true)`, exactly **one** filled path
  made of straight segments (`LineTo`) — the area; without it, **zero** (only the markers, circles,
  are filled).
- **Golden** `line_chart_area`: a translucent area under the curve, the stroke and markers above.

## What's left

- **Multiple series** + a legend (stacked or overlaid), a vertical gradient for the area (rather
  than a flat fill), and smoothed interpolation (Bézier curves) between points.

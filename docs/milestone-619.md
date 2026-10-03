# Milestone 619 — The reference's motion curves

## Objective

The reference's page transitions are drawn with curves frus did not have. Its Android and
desktop pages move on `easeInOutCubicEmphasized`, Material 3's emphasized easing; its iOS
page slides in on `fastEaseInToSlowEaseOut` and the page under it leaves on
`linearToEaseOut`, coming back on `easeInToLinear`. The first two are not cubics: they are
`ThreePointCubic`s, two cubic Béziers joined at a midpoint (`curves.dart:453`), the only
way to draw a slow start, a fast middle and a long settle with Béziers. This milestone adds
the shape and the four curves, ahead of the page transitions that use them.

## What was done

- **`Curve::ThreePointCubic { a1, b1, midpoint, a2, b2 }`**: the first segment runs from
  `(0, 0)` to `midpoint` with control points `a1` and `b1`, the second from `midpoint` to
  `(1, 1)` with `a2` and `b2`, each a unit cubic stretched over its own part of the square,
  as the reference scales them (`curves.dart:496`). The ends are returned as they are, as
  every curve's are in the reference.
- **`Curve::ease_in_out_cubic_emphasized()`** (`curves.dart:1781`),
  **`Curve::fast_ease_in_to_slow_ease_out()`** (`:1509`),
  **`Curve::linear_to_ease_out()`** (`:1639`) and **`Curve::ease_in_to_linear()`**
  (`:1536`), with the reference's numbers.

## Tests

- `the_three_point_curves_meet_at_their_midpoints`: both three-point curves keep their
  ends, pass through their midpoint, never run backwards, and give the reference's values
  at 0.1, 0.25, 0.5 and 0.75. Those were computed by an independent port of the
  reference's own evaluation, a bisection that stops within a thousandth; the test allows
  five.
- `the_ios_cubics_lead_and_lag`: `linearToEaseOut` stays ahead of linear,
  `easeInToLinear` behind it.

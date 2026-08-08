# Milestone 111 — `FractionallySizedBox`: size as a fraction of the parent

## Analysis

The second missing layout widget: **`FractionallySizedBox`**. It lets you say
"take half the available width" or "a quarter of the height" without knowing the
parent's absolute size — indispensable for fluid layouts (a bar at 70%, a panel at
30%…).

## Technical decisions

- **It reuses `Dimension::Percent`** (already present in `Style`): nothing new in
  `frus-layout`. A factor that is set → `Percent(f)` on that axis; an axis that is
  not set → `Auto` (it follows the content). A thin brick.

- **The box sizes itself**, rather than constraining the child. In our **flex**
  model (rather than a downward-constraint model), setting its own size as a
  percentage of the parent gives the same visual result in the common case (a
  child that fills). The child then fills the box (stretching on the cross axis,
  `flex` on the main axis).

- **Factors clamped to `>= 0`.** `width_factor` / `height_factor` are
  independent; one can be set without the other.

## Implementation

- `frus-widgets/fractional.rs`: the `FractionallySizedBox` widget
  (`width_factor`, `height_factor`, `child`, and a `style()` that maps each factor
  onto `Percent` or `Auto`).
- `FractionallySizedBox` exported in `lib.rs`. No change in `frus-layout` (the
  `Percent` field already existed).

## Tests

- `width_factor_takes_a_fraction_of_the_parent`: `width_factor(0.5)` inside a
  column 100 wide → the filling child is 50 wide.
- `height_factor_takes_a_fraction_of_the_parent`: `height_factor(0.25)` inside a
  column 200 tall → a box 50 tall.
- The frus-widgets suite green (202); the whole workspace green.

## What's left

- `Transform` (rotating / scaling / translating a child) — the last layout widget
  of this series, and the heaviest (a scene/paint matrix).
- An `alignment` on `FractionallySizedBox` (positioning the fractional box within
  the remaining space) — it would reuse the anchoring machinery (J106–J108).

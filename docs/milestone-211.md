# Milestone 211 — Charts: sub-region tooltip on hover

## Analysis

Milestone 208 plumbed the pointer's position through (`Status::hover_cursor`) and the suffix halo.
The same infrastructure enables the gesture a chart is expected to have: **hover** to read the exact
value under the pointer. That was the stated end goal — reusing `hover_cursor` for a tooltip.

## Technical decisions

- **Reuses `cursor_icon` (milestone 205) as the trigger.** `LineChart::cursor_icon` returns
  `Some(Cursor::Default)` when the pointer is inside the **plot area** — without changing the cursor
  shape (a chart is not clickable), but leading the shell to set `hover_cursor`. Outside the area:
  `None`. No new machinery.

- **A multi-series tooltip.** On hover, `paint` finds the **nearest category** in x, draws a vertical
  guide, accents each series' marker at that category, and draws a box listing the category then,
  per series, its swatch + its value. With a single series, the line reduces to the value.

- **A self-placing box.** Sized to the longest label, placed to the right of the guide and folded
  left if it would overflow, anchored at the top of the area — never out of frame.

- **No cost at rest.** `hover_cursor` stays `None` while the pointer is off the plot area: no
  repaint, the goldens (rendered without hover) unchanged.

## Implementation

- `frus-widgets/src/chart.rs`: `LineChart::cursor_icon` (tracking over the plot area); the tooltip
  block at the end of `paint` (the guide + accented markers + the box); the `TOOLTIP_SIZE` constant.

## Verification

- `hovering_the_plot_shows_a_tooltip_guide`: a vertical guide appears when `hover_cursor` is over
  the area, none without hover; `cursor_icon` answers `Some(Default)` inside the area, `None` above
  it. (Since the tooltip only exists on hover, it is not *goldenable* through `render_widget`;
  covered by this unit test.)

## What's left

- The same tooltip for **BarChart** (the bar under the pointer), tracking the nearest point in 2D
  distance (not just in x), and an appearance/disappearance **transition** on hover.

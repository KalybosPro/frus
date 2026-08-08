# Jalon 67 — Adopting per-corner (sheet, segments) + the border reserves its space

A finishing milestone: milestone 66's per-corner radii go into the widgets where
they are the **correct** look, and the brief's `content_padding` rule (§5) is
wired into layout.

## `Button::radius(…)` (the "everything must be customisable" rule)

The button's radius was taken straight from the theme, with no override. A new
`radius(impl Into<BorderRadius>)` builder (default: the theme's radius) — the
shadow follows (`inflate`). This is the brick for connected button groups.

## `SegmentedControl`: genuinely "connected" segments

Each segment is no longer a uniform pill: only the group's **outer** corners are
rounded (the first on the left, the last on the right, straight joins, a lone
segment uniform). The outer radius is overridable (`.radius(f32)`, default 10).
Pinned by `segments_round_only_the_outer_corners` (the three segments' fills
carry exactly left-rounded / straight / right-rounded).

## `BottomSheet`: rounded top corners

The sheet's panel painted a square surface; it now has rounded **top** corners
(`BorderRadius::top(theme.radius + 6)`, with the bottom edge still stuck to the
window), and the top edge line inset from the rounding. The sheet's pinned tests
(mid-slide, geometry) pass unchanged.

## The border reserves its space (`content_padding` → taffy)

A bordered `Container` now **reserves the stroke's thickness** in its layout
padding: the content is no longer eaten by the border (the rule
`BoxDecoration::content_padding` documented without it being wired). An invisible
border (zero thickness or zero alpha) changes nothing. Pinned by
`visible_border_reserves_layout_padding`. The impact: the demo's three bordered
containers see their content move in by 1 px — the correct behaviour.

## Validation

- **240 tests**, all green — including BottomSheet/Drawer's pinned tests
  unchanged, and the 2 new ones (segments' outer corners, the border reserve). A
  warning-free build; the demo did not panic.

## What's left (remaining §5)

Consolidating `ColorScheme` (+ HCT `from_seed`), text decorations,
`letter_spacing`/`line_height`, `Alignment`, RTL (§14).

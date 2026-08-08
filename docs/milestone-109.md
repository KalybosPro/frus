# Milestone 109 — Container: outer margin (`margin`)

## Analysis

The last piece of the conventional container API: the **outer margin**. The
container could space its content **inside** (padding, J102) but could not reserve
space **around** itself — there was no way to separate two cards without inserting
a spacer widget.

Container scorecard: padding ✓ (J102), composite decoration ✓ (J105), alignment ✓
(J105–J108), **margin ✓ (this milestone)**.

## Technical decisions

- **`margin` in `frus_layout::Style`.** taffy handles margins natively
  (`LengthPercentageAuto`); the field was simply missing from our thin `Style`.
  Added: a `margin: Insets` field, mixed into `layout_hash` (it changes the
  geometry → so it must invalidate the relayout cache) and mapped onto a
  `taffy::Rect` in `to_taffy`.

- **The margin is outer, independent of the decoration.** taffy places the box
  **inset** by its margin; the background, border and shadow are painted inside
  that reduced box (no painting change — `paint` already receives inset `bounds`).
  The margin **pushes the siblings** without enlarging the decoration.

- **`Container::margin(f32)` / `margin_each(...)`**, parallel to `padding` /
  `padding_each`. `Flex` (Row/Column) does **not** expose a margin; it passes
  `Insets::ZERO`.

## Implementation

- `frus-layout/style.rs`: the `margin` field, defaulting to `ZERO`, plus
  `layout_hash` and `to_taffy`.
- `frus-widgets`: `Container` (the `margin` field, the `.margin`/`.margin_each`
  builders, `style()` filling in `margin`); `flex.rs` passes `Insets::ZERO` (its
  `Style` constructor is enumerated).

## Tests

- `margin_pushes_siblings_and_insets`: in a column, a 2nd child (height 20) with a
  margin of 10 starts at `y = 30` (sibling 20 + margin 10) and is inset at
  `x = 10`, without its box growing (height 20).
- Suites green: frus-layout 4, frus-widgets 199; the whole workspace green.

## What's left

- `Transform` (rotating/scaling/translating a child), `AspectRatio`,
  `FractionallySizedBox` — the other layout widgets.
- A shell idiom / demo bringing the arsenal together (animations + alignment +
  margin).

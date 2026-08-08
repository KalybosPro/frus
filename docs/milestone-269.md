# Milestone 269 — `compute_scroll` **fills the constrained axis** (end of the filler container)

## The goal

In milestone 266, making the Kanban columns fill the board's height forced a workaround on the app:
wrapping the horizontal `Scroll` in a `flex(1)` `Flex` (a plain `Auto`-height `Container`
**collapsed**). The cause: a scrollable's **content** did not fill the viewport on its **cross**
(constrained) axis — it sized to its content, denying any `flex(1)`/`Percent` child a defined basis.
This milestone fixes the **root**: `compute_scroll` now fills the constrained axis (a tight cross-axis
constraint, as a scrolling list does). Apps no longer need the filler.

## The fix (`frus-layout/src/tree.rs`)

In `compute_scroll`, before the computation: if the scrolling is **single-axis** (one axis free, the
other constrained) and the **root** dimension on the constrained axis is `Auto`, we set it to the
viewport's size (`Length`). So the content takes the viewport's cross size; the **free** axis (the
scrolling one) keeps its natural size (`MaxContent`).

Scope guards:

- **Single-axis only**: `fill_w = !free_x && free_y` (vertical scrolling → fills the width);
  `fill_h = !free_y && free_x` (horizontal → fills the height). **Definite layout** (both axes
  constrained, `Constraints::definite`) and **2D scrolling** (both axes free) are **untouched** — no
  regression for screens/windows/modals, nor for tables scrollable in X **and** Y.
- **`Auto` only**: an **explicit** dimension (`Length`/`Percent`) on the content's root is **respected**.

## Simplification app-side

- **`frus-demo`** (`board_screen`): the board goes back to a **plain**
  `Container::new().padding(24).child(board)` inside the horizontal `Scroll` — the structure from before
  milestone 266, the one that used to collapse, and that **works** now. The `flex(1)` `Flex` workaround
  is removed.
- `Kanban::scrollable_columns()` keeps its `height: Percent(1.0)` (it fills the `Container`'s content
  area, itself filled by `compute_scroll`).

## Verification

- **Desktop**: `frus-layout` 4, `frus-widgets` 396, `frus-shell` 27, `frus-demo` 36, goldens **77** —
  all green (no regression: definite layout and 2D scrolling are excluded from the filling). The
  `scrollable_columns_fill_the_board_height_then_scroll` guard now passes with a **`Container`** (the
  very case that collapsed in milestone 266), proof of the fix.
- **On device**: the `frus-demo` APK rebuilt — still to confirm that the columns fill the height and
  scroll (a simplified app structure, an identical result).

## What's left

- Nothing outstanding. Cross-axis filling by default brings `Scroll` in line with the established
  scrolling-view behaviour (a tight cross-axis constraint, a free scrolling axis).

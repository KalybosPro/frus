# Jalon 266 — Fill-then-scroll: per-column vertical scrolling **without an explicit height**

## The goal

To replace milestone 264's **stopgap** (the app computes and passes a column height through
`Kanban::card_area_height`) with genuine **filling**: the column takes the board's available height,
then its cards **scroll**. The app has no height left to compute.

## The root cause of the blocker (milestone 263), finally understood

A `Scroll` is a taffy **leaf node** (crates/frus-widgets/src/ui.rs, `build_layout`:
`scroll_content().is_some()` → `layout.leaf(...)`): its content is laid out **separately**. In the main
layout, the `Scroll` is therefore a leaf **with no measured content** — its flex basis is 0. But
`Scroll::new()` set `height: Length(200)` by default: in `flex(1)` mode **without an explicit height**,
that height stayed a **flexible basis of 200 px** — the viewport did not "fill", it demanded 200 px of
free space to grow into. And above all: `flex_grow` only distributes space **if the direct parent has a
defined main-axis size**. As soon as one link in the chain (the column, the row, an enclosing
`Container`) was at `Auto` height (hence sized to its content), there was **no free space** to
distribute → the `Scroll` collapsed to 0. This was **not** an engine limitation: it is the same
constraint everywhere (a "fill the remaining space" child only makes sense inside a flex with a bounded
extent).

Empirical proof (throwaway tests, later turned into a guard): a chain **entirely at a defined, filling
height** gives the `flex(1)` `Scroll` a viewport equal to the remainder (e.g. 300 − title − footer = 260)
that **scrolls** the overflow (`max_y` > 0). Interposing an `Auto`-height `Container` **breaks it again**
(a viewport of 0).

## The fixes

- **`frus-widgets/src/scroll.rs`** — the primitive: `Scroll` remembers whether its size was **set**
  (`width_explicit` / `height_explicit`). In `flex` mode (`flex_grow > 0`), an unset dimension on the
  **scrolling axis** goes to `Auto` (a basis of 0) instead of the default value, so `flex_grow`
  **fills** instead of reserving 200. (`.width()` / `.height()` mark the dimension as set.)
- **`frus-widgets/src/kanban.rs`** — `Kanban::scrollable_columns()` (new) enables fill mode: the `Row`
  takes `height: Percent(1.0)` (the ancestor is at a defined height) and **stretches** its columns
  (`Align::Stretch`); each column's card area becomes a `flex(1)` vertical `Scroll` (a basis of 0) — a
  fixed title above, a fixed "+ Add card" button below. It takes precedence over `card_area_height`
  (milestone 264), kept as a fallback when no ancestor is at a defined height. The **default mode is
  unchanged** (bare cards, columns aligned at the top): the Kanban golden does not move.
- **`frus-demo/src/lib.rs`** (`board_screen`) — moves to `.scrollable_columns()` (no more height
  computation). The visual margin comes from a **`Flex` at `flex(1)` + padding** wrapping the horizontal
  `Scroll` (and no longer a `Container`: an `Auto`-height box would **break** the chain — a `flex(1)`
  `Flex`, by contrast, fills the screen's defined height).

## Verification

- **Desktop**: compiles; widgets **396** (including the
  `scrollable_columns_fill_the_board_height_then_scroll` guard: a column's `Scroll` fills the board's
  height — a viewport > 300, well beyond the 200 default — **and** scrolls, `max_y` > 0); goldens **77**
  **unchanged** (the default mode preserved); shell **27**.
- **On device**: to be confirmed by finger (full-height columns reaching the bottom of the board; each
  column scrolling its cards; dragging still working).

## What's left

- Possibly making a `Scroll`'s content **fill** the **constrained** axis (a tight cross-axis constraint)
  directly in `compute_scroll`, which would avoid having to wrap in a `flex(1)` `Flex` app-side. Not
  necessary for now.

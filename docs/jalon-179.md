# Jalon 179 — Frozen columns: separator shadow & freezing on the right

## Analysis

Column freezing (milestone 178) froze the **first** columns on the left, with no visual cue of
the freeze edge. Two gaps: (1) a **separator shadow** to signal that content is scrolling behind
the frozen columns; (2) freezing on the **right** (action columns, totals) — common when you want
to keep row buttons in view.

## Technical decisions

- **Freezing at both edges.** The frozen-column count becomes a `(left, right)` pair:
  `frozen_columns(n)` freezes on the left, `frozen_columns_right(m)` on the right. The layout
  becomes a `Flex` row `[left frozen block?, horizontal Scroll(middle), right frozen block?]`;
  each block is built by a shared `frozen_block(cols, w)` helper. The scrolling middle (with its
  header) carries the central columns.

- **A separator shadow as a layer.** A `FrozenShadow` (an **inert** stack layer — it catches no
  clicks) paints a `scrim → transparent` gradient (through `gradient_rect`) at the **inner edge**
  of each frozen block, **over** the scrolling area. The frozen root becomes a `Stack`
  `[the row of blocks, the shadow]`. Since the shadow has no `on_click`, the cells beneath stay
  clickable (sorting / selection).

- **A trap avoided.** A stack layer with a default `Style` (`Auto`) shrinks to `0×0` (no
  children) and paints nothing; so `FrozenShadow` is given an **explicit size** (the total
  width/height) to fill the stack.

## Implementation

- `table.rs`: `frozen` becomes `(usize, usize)`; the `frozen_columns` / `frozen_columns_right`
  builders; the `frozen_block` helper; `build_frozen` handles left/middle/right + the
  `FrozenShadow` layer (a `gradient_rect` gradient, an explicit size).
- `goldens.rs`: `table_frozen_columns` (regenerated, the shadow visible);
  `table_frozen_both_edges`.

## Verification

- **Unit**: `freezing_both_edges_pins_left_and_right_columns` — 1 frozen left + 1 right, a
  scrolling middle (max_x > 0); a header frozen **on the right** ("Act") and **on the left**
  ("Name"), both sortable. `frozen_columns_split…` (left freeze only) stays green, the shadow
  not blocking clicks.
- **Golden**: `table_frozen_both_edges` **inspected** (Name frozen, Q1/Q2 scrolling, Act frozen
  on the right, shadows at both edges); `table_frozen_columns` regenerated (a shadow at the
  freeze edge) — no regression on the other 33 goldens.
- `cargo test --workspace` **green**.

## What's left

- A **themable shadow thickness/opacity**: today `scrim` at 0.28 — it could follow the theme's
  elevation.
- **Freezing + virtualisation**: still mutually exclusive (the virtualised vertical and frozen
  horizontal scrolls would have to be nested) — a dedicated milestone.

# Milestone 265 — Vertical drag inertia (spring-loaded insertion line)

## The goal

To give the cards' **vertical** reordering (Kanban) the same **inertia** as the columns' **horizontal**
reordering (`Table`). On the horizontal side (earlier milestones), the smoothed abscissa `reorder_x`
chases the cursor through a spring and feeds the columns' reflow (`reflow_reorder_columns`): the
neighbours *slide* instead of jumping. On the vertical side, the **insertion line** and the **gap**
*jumped* from one card notch to the next. This milestone gives them the counterpart: a smoothed
ordinate `reorder_y`.

## The approach

A `reorder_y` spring (the same time constant and `spring_toward` function as `reorder_x`) chases not the
raw cursor but the **chosen slot edge** — the ordinate `reorder_drop_line` already computes (the
top/bottom edge depending on the hovered half, milestone 252). So the line always **snaps** to a valid
notch, but **glides** between notches instead of jumping.

That smoothed ordinate feeds **both** the painted indicator **and** the cards' reflow
(`reflow_reorder_cards`): the line and the gap move together; as `reorder_y` sweeps the interval, the
intermediate cards flip one by one (a cascade), the exact vertical analogue of the columns' slide. The
**drop routing** is unchanged (based on the cursor's real position through `reorder_insert_after`): the
smoothing only animates the **approach**, and at rest `reorder_y == target`.

## Implementation (`frus-shell/src/app.rs`)

- The `reorder_y: f32` field (initialised to 0), set to `cursor.y` when a drag starts (no initial jerk).
- The frame loop: the reorder spring computation moves from an `if horizontal` to a `match` on the axis
  — **Horizontal** → `reorder_x` towards `cursor.x` (unchanged); **Vertical** → `reorder_y` towards
  `reorder_drop_line(...).y` (the chosen edge), animating while the gap exceeds 0.5 px.
- `paint_reorder_preview` (the vertical branch): the insertion line is the chosen slot with its
  **ordinate replaced** by `reorder_y` (`Rect { y: self.reorder_y, ..target }`), then passed to
  `reflow_reorder_cards` **and** painted — the line and the gap glide in concert.

## Verification

- **Desktop**: compiles; shell **27** tests OK, including `insertion_line_sits_on_the_target_top_edge`
  (the snap logic **unchanged**: `reorder_drop_line` is not modified) and
  `spring_approaches_target_monotonically_and_settles` (the spring converges, with no overshoot).
- **On device**: the actual inertia (the line's glide + the cards' cascade) is **runtime/GPU** — to be
  confirmed by finger on the board.

## What's left

- Nothing blocking. The time constant (0.07 s) could be fine-tuned after real use.

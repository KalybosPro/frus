# Jalon 163 — Reordering: gentle inertia & announced headers

## Analysis

The column slide (milestone 161) **stuck to the cursor**: responsive, but a little dry (no
give when the finger stops or jumps). Two items from "What's left": **inertia** (a spring) and
the **accessibility** of reordering.

## Technical decisions

- **A single-state spring, not one per column.** Rather than an animated offset per column
  (heavy), we **smooth the cursor's abscissa**: a `reorder_x` state chases the real position
  through an **exponential spring** (a ~70 ms time constant), and it is **that** which feeds
  the geometric reflow. The columns therefore slide with **gentle inertia**, while the **ghost
  sticks to the real cursor** (it "leads", the background "catches up") — a Material feel, at a
  minimal cost. The spring is **frame-rate independent** (`1 − e^{−dt/τ}`) and does not
  overshoot; the frame stays "animated" until it settles.

- **Announced headers.** Each header now carries **semantics** (a button role + a label); if it
  is reorderable, its **value** states "column N of M". A screen reader therefore announces the
  column **and its position**: walking back through them after a move (mouse or Ctrl+Arrows),
  the user **perceives** the new order. Data cells stay silent (no noise).

## Implementation

- `app.rs` (shell): the `reorder_x` field; initialised at the cursor when the drag starts; the
  **spring** advanced in the animation loop (`spring_toward`, a pure function) and injected into
  `reflow_reorder_columns`; the frame stays animated until it settles.
- `table.rs`: `Cell::semantics` (headers: role + label + "column N of M" if reorderable).

## Verification

- **Unit**: `spring_toward` — a **monotonic**, **bounded** approach (no overshoot), all but
  reached after ~0.5 s. `Cell::semantics` — the "B" header announced as `label="B"`,
  `value="column 2 of 3"`; a data cell **silent**.
- The inertia is an **interactive temporal** effect (the render loop), not goldenable; its law
  is isolated and tested. The `table_reorder_preview` golden (a direct reflow) stays unchanged.
- `cargo test --workspace` **green**, with no warning.

## What's left

- A **"live" spoken announcement** of the move ("moved to position 3"): requires a dedicated
  AccessKit **live region** (beyond the current passive semantics tree).
- **Settle on drop**: a small spring of the ghost card towards its final position.

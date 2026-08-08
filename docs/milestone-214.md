# Milestone 214 — Grid: cycling through the errors

## Analysis

Milestone 210 led to the **first** faulty cell; on a grid with several, the user then wanted to move
to the **next**, and so on. The button becomes a **cycle** through every fault.

## Technical decisions

- **A cycle that wraps.** `Next error` (formerly `Go to first error`) focuses the fault **after** the
  last one targeted, in row-by-row order, and **wraps** to the first after the last. The targeted
  position is remembered in `grid_error_cursor`.

- **A single enumeration of the faults.** `grid_faults` lists every invalid cell (in row-by-row
  order) through `grid_cell_error`; `grid_first_error` (milestone 210) and `grid_next_error` derive
  from it — the validity rule stays single.

- **Visual feedback on arrival.** `Command::focus` puts the keyboard focus on the targeted cell: the
  existing focus ring highlights it. (A dedicated *brief halo* would require a transient animation —
  left in What's left.)

## Implementation

- `frus-demo/src/lib.rs`: the `grid_error_cursor` field; `grid_faults`; `grid_next_error` (with
  wrapping); `grid_first_error` delegates; `GridFocusError` cycles; the button renamed `Next error`.

## Verification

- `grid_next_error_cycles_through_all_faults`: over three faults `(0,0)`, `(0,2)`, `(1,2)`, four
  successive calls give `(0,0) → (0,2) → (1,2) → (0,0)` (wrapping). Milestone 210's test stays green
  (`grid_first_error` functionally unchanged).

## What's left

- A **brief halo** (an animated pulse) on the cell upon arrival, **scrolling** to it if the grid
  overflows the viewport, and skipping the **current** fault if it is corrected before cycling on.

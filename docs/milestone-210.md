# Milestone 210 — Grid: Save disabled + jump to the first error

## Analysis

Milestone 207 guarded submission inside `reduce` (a "Fix N errors" toast), but the `Save` button
stayed clickable — you only learnt of the failure afterwards, without knowing **where** to correct.
Two improvements: make the invalid state **unreachable**, and offer a **shortcut** to the fault.

## Technical decisions

- **`Save` disabled when invalid.** `button(...).enabled(errors == 0)`: the button is greyed and
  emits nothing while a cell is faulty. Invalidity becomes visible *before* the click, not after.
  `reduce`'s guard (milestone 207) stays as a defence.

- **A shortcut to the first fault.** When `errors > 0`, a `Go to first error` button appears;
  `Msg::GridFocusError` focuses the first invalid cell through `Command::focus(("grid", r, c))` —
  the user is taken straight to the correction.

- **A single fault rule.** `grid_first_error` reuses `grid_cell_error` (milestones 204/207), walked
  row by row: the same definition of validity everywhere.

## Implementation

- `frus-demo/src/lib.rs`: `Msg::GridFocusError`; `grid_first_error`; the `GridFocusError` arm (the
  focus); `grid_screen`: `Save` conditionally enabled + a `Go to first error` button inserted
  dynamically (`Flex::child`) when there are errors.

## Verification

- `grid_focus_error_targets_the_first_faulty_cell`: `grid_first_error` points at the first fault
  `(1, 0)` (an empty Name), `GridFocusError` emits a focus; once everything is corrected, no target
  and no command.

## What's left

- **Scrolling** to the focused cell if the grid overflows the viewport, cycling through *all* the
  faults (not just the first), and a brief halo on the targeted cell on arrival.

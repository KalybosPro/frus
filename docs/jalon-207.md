# Jalon 207 — Grid: submission guarded by validation

## Analysis

Milestone 204 flags invalid cells individually, but nothing stopped you from "submitting" an
inconsistent grid, and the user had no overall view. The validation state had to be **aggregated**
at table level and the submission **guarded**.

## Technical decisions

- **A pure counter.** `grid_error_count(grid)` sums the invalid cells through milestone 204's
  `grid_cell_error` — a single source of truth for validation, reused by the status bar and by the
  submission.

- **A live status bar.** Under the table, `All cells valid` (accent) or `N error(s)` (the error
  colour), recomputed each frame from the state — it follows typing with no machinery.

- **Guarded submission.** `Msg::GridSave` only goes through (a `Grid saved` toast) if the counter is
  zero; otherwise a `Fix N error(s) before saving` toast reports the count. The validation lives in
  `reduce` (testable), not in the view.

## Implementation

- `frus-demo/src/lib.rs`: `Msg::GridSave`; `grid_error_count`; the `GridSave` arm (a toast per the
  counter); `grid_screen` gains an `Add row` / `Save` action row + the status bar.

## Verification

- `grid_save_is_gated_on_cell_errors`: two errors → `GridSave` blocks and counts
  (`Fix 2 errors before saving`); once corrected, `grid_error_count == 0` and `GridSave` goes through
  (`Grid saved`).

## What's left

- Visually **disabling** the `Save` button while there are errors (rather than leaving it clickable
  and reporting), **Escape** to cancel editing a row (needs a per-row snapshot), and highlighting the
  **first** faulty cell on a `Save` click.

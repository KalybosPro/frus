# Milestone 726 — A screen reader's view of a simulated device

## Objective

Milestone 724 left this open: while a desktop window simulates a device (milestone 721), the
accessibility tree it hands the system (AccessKit) was laid out as if the interface started at
the window's corner, at the window's scale. The interface is in the phone's screen, zoomed, so a
screen reader's highlight or a magnifier would have been off the widget they name.

## What was done

- `build_tree_update_at` takes the interface's **origin** in the window, in physical pixels, as
  well as the scale. The root carries `translate(origin) × scale(scale)`, still left unset when
  it is the identity. `build_tree_update`, the tree drawn from the window's corner, is kept for
  the tests.
- The shell hands it the **total scale** (the window's × the application's density, or × the
  zoom while simulating) and, while simulating, the screen's corner. Before, it handed the
  window's scale alone, so an application with a density of its own was also described at the
  wrong size. That was outside simulation, and it is fixed by the same line.

## Checked

Through Windows UI Automation, which reads what a screen reader reads, on the demo as an
`iphone-15`. Nothing was clicked. UIA reports *Add* at 345, 411, 79 × 41 window pixels, and the
frus screenshot of the same window draws it at about 346–425 × 416–449. That is the same place,
inside the phone's screen; the extra height is the semantics' touch target. Without the origin,
the same rectangle would be about 64 px to the left of and 74 px above the button. That was
worked out from the placement, not observed.

## Tests

- `a11y::the_root_moves_the_tree_to_a_simulated_screen`: a button at (20, 40), scale 2, origin
  (607, 148) lands at (647, 228)–(759, 340); a move alone is a transform.
- The existing `the_root_scales_logical_bounds_to_physical_ones` is unchanged and passes.

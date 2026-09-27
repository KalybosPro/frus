# Milestone 589 — Start and end pins on a stack's layers

Milestone 588 settled that a left inset stays on the left in a right-to-left script. Stack
layers already worked that way: a probe showed `Positioned::left(10)` at 10 px from the left in
Arabic too. What was missing is the other half, the reference's `PositionedDirectional`: a way
to pin a layer to the **start** of the stack, the corner a badge or a close button belongs in
whichever way the script runs.

## What it does

- **`Positioned::start(px)`** and **`Positioned::end(px)`** pin a layer to the stack's start and
  end edges: the left and the right in a left-to-right script, the right and the left in a
  right-to-left one.
- **`AnimatedPositioned::start` / `end`** likewise, and a layer moving between two sets of pins
  carries them.
- `left` and `right` stay physical, and a `left` or a `right` given as well wins on its own
  side, where the reference forbids giving both kinds at once.

## How

- `Positioning` has two more pins, `start` and `end`. `Positioning::for_direction(rtl)` turns
  them into a left and a right. The stack calls it at the one place it places a layer, where the
  direction is known. A stack's layers are placed on the screen as they are, and not through the
  frame's mirror, which is why `left` was already physical there.
- The runtime's animated pins go from six values to eight. The slot family is generic over its
  size, so this is the only change there.

## Verification

- A 20-px square in a 100-px stack:
  - start 10: at 10 in left to right, at 70 (10 from the right) in right to left;
  - end 10: the other way round;
  - left 10 in right to left: at 10;
  - start 30 with left 10: at 10.
- An animated layer at start 10 in right to left: at 70. The runtime's pins for it carry the
  start and resolve it to the right.
- Mutations, each failing a test:
  - the stack ignoring the direction;
  - start and end not swapped in right to left;
  - the animated start dropped;
  - a start winning over a left.

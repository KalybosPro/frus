# Milestone 70 — Focus: keyboard-only ring + arrow navigation (geometric)

Opening **§6** with its first item ("the focus tree + key routing, a prerequisite
for everything"), through its two most rewarding and testable pieces.

## `FocusHighlightMode`: the ring no longer flashes on click

The brief: *"only paint the focus ring if the last interaction was from the
keyboard"*. A new `Runtime::focus_visible` bit:

- **pointer pressed** → `false` (focus stays active — a field keeps its caret —
  only the generic ring disappears);
- **any key pressed** → `true` (a redraw if the bit flips).

`draw_focus_ring` is gated on it; widgets that draw their own focus (`TextInput`:
an animated border) are unchanged — that is an editing affordance, not a
navigation ring.

## **Arrow** focus navigation (a geometric policy)

`Ui::focus_directional(current, FocusDirection)`: among the focusables, it picks
the nearest one **within a cone** around the direction (not a simple half-plane —
a candidate that is roughly aligned across but only just "ahead", because of
slightly different widths, is not a directional target: exactly the bug the test
caught). Score = advance + 3 × lateral deviation.

On the shell side: the arrows navigate from any focusable — **except**
left/right inside a text field (there they move the caret; up/down navigate even
from a single-line field). Tab/Shift+Tab (tree order) unchanged.

## Validation

- **247 tests**, all green:
  - the ring present from the keyboard, **absent** after a pointer focus, never
    for a widget with its own focus;
  - a 2×2 grid: right/down correct, **the diagonal controlled** (from b, down → d
    aligned, not c), nothing to the left of the edge — the degenerate case of
    unequal widths is pinned.
- A warning-free build; the demo did not panic.

## What's next (§6)

- Key propagation **leaf→root** with a 3-state result
  (handled/ignored/skip) — the payoff: Escape closes the dialogue from anywhere.
- A regularised keyboard model (physical + logical + character), scrolling in 4
  pieces, the `padding`/`viewInsets` split, focus scopes (trapping inside a
  modal).

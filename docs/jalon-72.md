# Jalon 72 — Focus scopes: the modal traps Tab, arrows and clicks

A continuation of §6: **focus scopes** (the brief listed them as "can wait" —
they could not any longer: Escape had been closing the topmost modal since
milestone 71, but **Tab escaped from it** into the scrimmed background
interface).

## The mechanism

While the overlays are rendered, each **modal** overlay (scrimmed: `Center`, the
`Left`/`Right` drawers, the `Bottom` sheet) marks the index of its first
focusable — `focus_scope_start`. The last rendered (the topmost, nested portals
included) wins. Anchored overlays (`Below`, `Tooltip`) do not trap.

The **pool of participating focusables** (`focus_pool`) then becomes the scope's
slice of focusables, and the three ways into focus all respect it:

- **Tab** (`focus_next`): cycles **inside** the modal; a current focus outside the
  scope (taken before it opened) is treated as "none" — so Tab **enters** the
  trap.
- **Arrows** (`focus_directional`): the candidates are trapped; the starting
  point may be outside the scope (the first arrow press moves in).
- **Focus on click** (`focus_hit`): a click on the scrim no longer focuses a
  background field.

With no modal, `focus_scope_start` is `None`: every focusable participates — the
historical behaviour, strictly unchanged (the existing Tab tests pass as they
are).

## Validation

- **249 tests**, all green — the new test pins: Tab enters the modal, moves
  around inside it and **cycles** there; the arrows do not leave the scope; a
  click on a background button's area focuses nothing; with no modal, Tab starts
  at the back as before.
- A warning-free build; the demo did not panic. The demo's confirmation modal,
  drawer and sheet now trap the keyboard — combined with Escape (J71) and the
  keyboard-only rings (J70), the keyboard story for modals is complete.

## What's next (§6)

A regularised keyboard model (physical + logical + character), scrolling in 4
pieces (`Position/Controller/Physics/Activity` — where it meets milestone 53's
`Simulation` layer), the `padding`/`viewInsets` split.

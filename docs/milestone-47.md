# Milestone 47 — Right drawer & permanent drawer

Two complements to `Drawer`: docking to the **right** edge, and a **permanent**
mode (docked in the flow, always visible).

## Right drawer — `Placement::Right`

Symmetrical to `Left`: a full-height panel stuck to the right edge, with a scrim
and an animated slide (the same runtime-driven mechanism, milestone 46).

- `process_overlays` places a `Right` panel at
  `x = window_width − progress · panel_width` (the right edge stays stuck, the
  panel comes in from the right), with its height constrained to the window.
- The scrim applies here too (`Center | Left | Right`).
- API: `Drawer::new(open).right()`.

The `DrawerPanel`'s edge line moves to the **inner** edge (left for a right-hand
drawer, right for a left-hand one).

## Permanent drawer — `Drawer::permanent(bool)`

When `permanent` is true (typically at the `Expanded` tier), the panel is no
longer a modal overlay: it is **docked in the flow**, always visible next to the
body, **with no scrim and no animation**.

- The `Drawer` becomes a **row**: `[panel, body]` (left) or `[body, panel]`
  (right); the panel keeps its fixed width and the body takes the rest
  (`flex(1)`).
- `overlay()` returns `None` and `anim_target()` returns `None` (nothing to
  animate).
- `open` / `on_dismiss` are ignored in this mode.

It is the "docked" counterpart of the rail: a drawer that, on a large screen,
stops being collapsible.

## Demo

The application's drawer is now **docked on the right**:

- **Compact / Medium**: modal, sliding, opened by the "☰" button;
- **Expanded**: **permanent** — the hamburger disappears and the panel docks on
  the right. The result is a **3-zone** layout: rail (`NavScaffold`) · body ·
  drawer panel.

## Tests

- `frus-widgets`: `right()` → `Placement::Right`; permanent → no overlay, no
  `anim_target`, a 2-child row, **no scrim**, a full-height panel docked on the
  left (x ≈ 0); permanent + `right()` → the panel stuck to the right edge
  (x + width ≈ window width).

## Limits (v1)

- In permanent mode there is no collapse/expand toggle (the panel is fixed) —
  that is the modal mode's role at the narrower tiers.

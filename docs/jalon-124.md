# Jalon 124 — `FittedBox` + `RotatedBox`: transforms that affect layout

## Analysis

`Transform` (J112–J117) transforms **at paint time** only: the box does not move.
The two transforms that **take part in layout** were missing:

- **`RotatedBox(quarter_turns)`** — rotates the child by whole quarter turns **and**
  swaps the box's width/height for an odd quarter (a vertical label in a sidebar, a
  rotated axis caption).
- **`FittedBox(fit)`** — scales the child to **fit** the box according to a
  [`BoxFit`] (`Contain`/`Cover`/`Fill`/…), the scale following from the box's size.

## Technical decisions

- **Layout leaves, the child placed separately.** Like `Scroll` / `InteractiveViewer`,
  both are **leaves** in the taffy tree: the child is measured and rendered separately,
  otherwise it would be stretched to the box instead of taken at its **natural** size.
  `build_layout` computes the `RotatedBox`'s box from the child's natural size (swapped
  for an odd quarter); `FittedBox` takes its own box (`width`/`height`/`flex`).

- **One shared rendering factor.** `RotatedBox`, `FittedBox` and `InteractiveViewer`
  now share `emit_transformed_child`: paint the child flat, wrap it in a composited
  layer transformed by `M`, put `M⁻¹` on the hits, and — if `M` stays **axis-aligned**
  (scale/translation, but not an odd quarter rotation) — transform the focus / scroll /
  drag / accessibility bounds.

- **Fit maths in `frus-core`.** `BoxFit::scale(src, dst) -> (sx, sy)` (pure, tested):
  `Fill` per axis, all the others uniform (aspect preserved), `ScaleDown` never shrinks
  past 1, a degenerate source → neutral.

- **A reusable natural size.** `natural_size` lays a subtree out under free axes
  (`MaxContent`) — the common brick behind the `RotatedBox`'s box (layout) and the
  `FittedBox`'s factor (rendering).

- **Cache consistency.** `layout_signature` (the relayout fingerprint) follows
  `build_layout` to the letter: `RotatedBox` **hashes its child** (its box depends on
  it); `FittedBox` and `InteractiveViewer` are leaves (fingerprint = style). While in
  there, `interactive()` was missing from that list (introduced in J122) — fixed.

## Implementation

- `frus-core`: `BoxFit::scale` (+ the `scale_fits_content_per_mode` test).
- `frus-widgets`: the `fittedbox` (`FittedBox<Msg>`) and `rotatedbox`
  (`RotatedBox<Msg>`) modules; the `Widget::fitted()` / `rotated_quarter_turns()`
  methods, forwarded (`Box<dyn>`, `Keyed`, `Responsive`, animated); `build_layout` (a
  custom leaf for the rotation, a leaf for the fit) + `natural_size`; walk branches
  through `emit_transformed_child` (the `InteractiveViewer` branch refactored to use
  it); the `plain_subtree_len` guard; `layout_signature` aligned.

## Tests

- `frus-core`: `BoxFit::scale` per mode.
- `rotatedbox`: a quarter **swaps** the box (the sibling follows at `y=80`); a half turn
  **keeps** it; a rotated layer is emitted.
- `fittedbox`: `Fill` carries the scale **per axis**; `Contain` **preserves the aspect**
  (a uniform factor).
- Visual rendering (outside the commit) confirmed: rotation 1/2/3, fit
  Contain/Cover/Fill, **no overlap** between siblings. The whole workspace green:
  frus-widgets 227, frus-core 91.

## What's left

- **Free-angle rotation** affecting layout (beyond quarters) and exact rotated
  constraints (v1 measures the child **unconstrained**).
- A showcase: a `RotatedBox` tile (vertical text) + `FittedBox` in `frus-transforms`.

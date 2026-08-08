# Jalon 255 — Drag-and-drop painting moved onto the theme / named constants

## Analysis

Milestone 254's review flagged, in the shell's drag-and-drop painting, **literals** at odds with the
customisability rule: a hardcoded ghost shadow colour (`Color::BLACK.fade(0.28)`), and a handful of
geometric **magic numbers** (offset 4, blur 12, lift −2, insertion thickness/radius). Meanwhile,
`Button` already takes its shadow from the theme (`theme.scheme.shadow.with_alpha(…)`) — that is the
established convention.

## Technical decisions

- **The shadow colour from the theme.** The ghost uses `theme.scheme.shadow` (overridable through the
  theme), like `Button`, instead of `Color::BLACK`. Since `scheme.shadow` **is** black in the light and
  dark themes we ship, the rendering is **strictly identical** — this is a **de-hardcoding**, not a
  visual change.
- **Geometry in named constants.** The shadow's offset/blur/opacity, the border's opacity/thickness, the
  horizontal lift and the insertion line's thickness live in a small documented `drag_preview` module —
  as `Button`/`Card` keep their shadow geometry locally. The insertion line's radius now derives from the
  theme, clamped to half the thickness (`theme.radius.min(line.height * 0.5)` = 1.5 at the current
  values → identical).
- **Scope deliberately limited to DnD.** `Card`/`Toast` carry the same shadow literal
  (`rgba(0,0,0,0.3)`); left for a dedicated consolidation pass (see What's left) to keep this milestone
  focused.

## Implementation

- `frus-shell/src/app.rs`: the `drag_preview` module (the geometry constants); `draw_ghost_card` takes
  the shadow colour from the theme and the named constants; the insertion line takes its thickness
  (`INSERT_THICKNESS`) and a theme-derived radius; the horizontal lift goes through `LIFT_Y`.

## Verification

- **Shell 27** (including `ghost_card_shape`); **goldens 77 unchanged** (a pixel-identical
  de-hardcoding).
- The DnD painting is runtime state (a drag), not inspected on a GPU here; the change is a
  value-for-value substitution (the theme shadow = black, the clamped radius = 1.5), with no visual
  difference.

## Notes

- The convention chosen, consistent with `Button`: the **colour** comes from the theme, the **geometry**
  stays in named local constants. A genuine themed "elevation spec" (colour + offset + blur, Material
  style) is still possible but would touch `lerp` and every shadow painter — out of scope here.

## What's left

- Unifying `Card`/`Toast`'s shadow (the same hardcoded `rgba(0,0,0,0.3)`) onto `theme.scheme.shadow`, or
  even a shared elevation helper.
- Consolidating `ui.rs` (the walk loops) and unifying the two `reflow_*`.
- Same-column reflow coverage; vertical inertia/spring.

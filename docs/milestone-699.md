# Milestone 699 — The accessibility tree in physical pixels

## Objective

Found while checking milestone 698 on a real window: on a Windows screen at 150 %, the demo's
"✓" button — 56 logical pixels, 84 on the screen — was reported to UI Automation as 56 × 56,
and away from where it is drawn. Every node of the tree was wrong the same way, which is what a
screen reader's highlight, a magnifier following the focus and a "click this" from an assistive
tool all go by.

The layout hands AccessKit its bounds in **logical** pixels. AccessKit expects the final
coordinates "relative to the origin of the tree's container (e.g. window), in **physical**
pixels". At 100 % the two are the same, which is why it went unseen.

## What was done

- **The window's scale factor goes on the root node as a transform** (`Affine::scale`), which
  AccessKit applies to every node under it; the nodes keep the layout's own bounds. One number
  in one place rather than every rectangle rewritten, and the tree stays in the layout's units
  for anything that reads it.
- At a scale of 1 the root has **no** transform, as AccessKit asks of an identity. A scale not
  yet known — the shared snapshot starts at 0, and a screen reader may ask for the tree before
  the first frame — is taken as 1 rather than collapsing every node to a point.
- `A11y::update` takes the scale; the shell passes the one it keeps up to date from
  `ScaleFactorChanged`, so a window dragged to a screen of another density is right from its
  next frame.
- The web's bridge is unchanged: a page's coordinates are CSS pixels, the layout's own.

## Verification

- `the_root_scales_logical_bounds_to_physical_ones`: a 56 px button at (461, 307.5) comes out at
  (691.5, 461.25)–(775.5, 545.25) through the root's transform at 1.5; no transform at 1, nor
  before the scale is known.
- On the real window (Windows 11, 150 %): the demo opened on a task's screen, its "✓" button read back
  through UI Automation: **84 × 84** at the place the layout draws it (the glyph's position,
  mapped from the layout, falls inside it). Before the change, the same check read **56 × 56**,
  away from the button.

## Left

- A `Container` with `on_click` has no node at all, and plain text is not in the tree: the
  demo's bottom-bar actions cannot be reached with a screen reader. The reference gives a
  tappable area a node carrying its tap action and its words. A design question of its own —
  which widgets make nodes, and how text merges into them.

## Mutation testing

Four hand mutants, each killed by `the_root_scales_logical_bounds_to_physical_ones`:

| mutant | |
|---|---|
| a scale of 0 put on the root | collapses the tree before the first frame |
| an identity transform set at 1 | |
| no transform at all | the bug itself |
| the snapshot handing the tree 1 instead of its scale | survived at first; the test now builds the tree through the snapshot too |

Not covered by a unit test: the shell passing the window's scale to `A11y::update` (the bridge
needs a window). The UI Automation check above is what covers it.

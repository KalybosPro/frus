# Milestone 107 — Anchoring: virtualised lists + `AlignmentDirectional`

## Analysis

J106 delivered fractional anchoring, but left two holes listed in its "what's
left":

1. **Virtualised lists / `layout_builder`.** Their rendering goes through
   `render_item` (a separate path, without the walk's special branches), which
   **did not apply** the anchoring offset: a container anchored as a list item or
   as a `LayoutBuilder` child stayed stuck to the top left.
2. **Directional anchoring.** A physical `Alignment` does not follow the reading
   direction; the start/end equivalent (resolved in RTL) was missing.

## Technical decisions

- **`render_item` reuses `align_offset`.** The same computation as the main walk
  (free space × fraction, the offset cascaded through the translation). One source
  of truth for anchoring, two rendering paths.

- **`AlignmentDirectional` (frus-core).** A `{ x_start, y }` struct, with
  `x_start` expressed **start → end** (`-1` = start, `+1` = end), nine constants
  (`CENTER_START`, `TOP_END`…). `resolve(direction) -> Alignment` flips `x_start`
  in RTL (start ↔ right); the `y` is direction-invariant. A pure geometric type,
  on the `InsetsDirectional` model.

- **Resolution where the direction is known.** `Container` stores the directional
  anchor as it is; it is `Builder::align_offset` (which holds `self.rtl`) that
  **resolves** it into a physical `Alignment` and then treats it like any anchor —
  the existing RTL correction does the rest (a consistent double pass: a resolved
  directional anchor follows exactly the physical mechanics). The directional
  anchor **wins** over the physical one.

- **A `Widget::alignment_directional()` trait method** (default `None`) +
  forwarders (`Box`/`Keyed`/`Responsive`/named). `Container::alignment_directional(...)`
  + `style()` sets `Start`/`Start` if **either** anchor is set.

## Implementation

- `frus-core`: `AlignmentDirectional` + constants + `resolve` (geometry.rs),
  re-export.
- `frus-widgets`: the `alignment_directional()` trait method + forwarders;
  `Container` (an `alignment_dir` field, the builder, `style()`, the accessor);
  `align_offset` resolves the directional anchor by `self.rtl`; `render_item`
  applies the anchoring.

## Tests

- `directional_alignment_resolves_by_direction` (core): `CENTER_START` → the left
  in LTR, the right in RTL; `TOP_CENTER` invariant.
- `directional_alignment_flips_the_child_in_rtl` (widgets): the same tree, a child
  anchored `CENTER_START` is at x≈0 in LTR and x≈80 in RTL.
- Suites green: frus-core 87, frus-widgets 198; the whole workspace green.

## What's left

- A shell idiom / demo animating `align_tween.animate(&ctrl).value()`.
- Anchoring with **multiple children** (today: a single child).

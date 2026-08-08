# Jalon 106 — Fractional `Alignment` + `Tween<Alignment>` (manual placement)

## Analysis

J105 introduced `Container::alignment` over nine **discrete** anchors, translated
into flex `justify`/`align`. Discrete means non-interpolable: a `Tween<Alignment>`
would jump from position to position. For an **animatable** anchor (sliding a
child smoothly from one corner to another, an align transition), a **continuous**
`Alignment` and **manual** placement are needed (flex does not do fractions).

## Technical decisions

- **A continuous `Alignment` (frus-core).** A `{ x, y }` struct, fractions in
  `[-1, 1]` (`-1` = the bottom/left edge, `+1` = the top/right, per axis), with
  the nine usual anchors as constants (`Alignment::CENTER`, `TOP_LEFT`…). Being
  continuous, it implements **`Lerp`** → so a `Tween<Alignment>` slides from one
  anchor to another for free. `fraction_x/​y()` bring each axis back into `[0, 1]`
  (the share of free space to leave before the child).

- **Manual placement in the walk.** `Container::style()` lets taffy place the
  single child at the **top left** of the content box (`Start`/`Start`, natural
  size, no stretching). The walk (`Builder::align_offset`) then computes the free
  space `content_box − child` and **offsets** the child by
  `free × fraction` through its translation — which cascades over its whole
  subtree (hit-testing included). It only applies to a **single child** (anchoring
  targets one child).

- **RTL.** taffy computes in LTR and then `mirror` sends the child to the right:
  the baseline is then the right-hand anchor. So `1` is subtracted from the x
  fraction, keeping the anchor **physical** (`x = +1` ⇒ the right in both reading
  directions; `Alignment` is not directional, unlike `AlignmentDirectional`).

- **A `Widget::alignment()` trait method** (default `None`), forwarded by `Box`,
  `Keyed`, `Responsive` and the named widgets. Relayout-cache consistency is free:
  `style()` (which carries `Start`/`Start`) is the source shared by `build_layout`
  and the signature (`layout_hash`).

## Implementation

- `frus-core`: the `Alignment` struct + constants + `fraction_x/y`
  (geometry.rs); `impl Lerp for Alignment` (tween.rs); re-export (removing
  `AlignEdge`, which was short-lived from J105).
- `frus-widgets`: the `alignment()` trait method + forwarders; `Container`
  (`style()` sets Start/Start when anchored, `alignment()`),
  `Builder::align_offset` + its application in the walk's children branch.

## Tests

- `alignment_tween_slides_between_anchors` (core): `TOP_LEFT → BOTTOM_RIGHT`, the
  midpoint = `CENTER`.
- `fractional_alignment_places_child_proportionally`: an anchor of `(0.5, -0.5)`
  → the child at ~(60, 20) inside 100×100 (which the discrete version could not
  do).
- `alignment_centers_the_child` / `alignment_anchors_child_to_a_corner` (J105,
  updated to the constants) stay green under manual placement.
- Suites green: frus-core 86, frus-widgets 197; the whole workspace green.

## What's left

- Anchoring inside a **virtualised list item** (`render_item` does not apply the
  offset yet — a secondary path).
- `AlignmentDirectional` (start/end, resolved in RTL) if needed.
- A shell idiom / demo: `align_tween.animate(&ctrl).value()` passed to
  `.alignment()`.

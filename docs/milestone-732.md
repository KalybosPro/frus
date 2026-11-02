# Milestone 732 — Scaled and rotated layers are filtered

## Objective

Milestone 731 found that text under a scaling `Transform` loses strokes: the bottom navigation
bar's labels, scaled to 12/14, came out with letters damaged, and the bar was changed to animate
its font size instead. The fault was left in the renderer, where it touched every application
that scales or turns text.

## Cause

A transformed layer is rendered into a texture and composited through the inverse transform. The
compositor's sampler was `Nearest`. At 6/7, a nearest sample skips one source row and column in
seven, and a stroke one pixel wide that falls on a skipped one is gone. Turned shapes got stepped
edges for the same reason.

## What was done

- The compositor's sampler is `Linear`. The texture was already declared filterable and the
  sampler binding filtering, so nothing else changes. A layer that is neither scaled nor turned
  samples texel centres, where linear reads exactly the texel; no golden without a transform
  moved.
- `scaling_down_keeps_every_thin_line` in `transforms.rs`: seven one-pixel lines at 6/7, spaced so
  that one of them falls on every residue, and so a skipping sampler must lose one. It fails with
  the old sampler.
- Seven goldens were regenerated, with the mesa CI pins, after reading each one against the old
  picture: `constraint_boxes` and `overflow_band` (the turned text of their overflow labels),
  `turned_and_rotated`, `picker_wheel`, `navigator_mid_push_zoom`, `animated_switcher_halfway`
  and `explicit_transitions`.

## Not done

- **No mipmaps.** Below one half, a linear sample still misses texels, and text at 0.5 is
  uneven. It would take a mip chain per transformed layer, a cost for every such layer that no
  application has asked to pay yet.
- **Scaling up blurs**, as any enlarged texture does: text under a large scale is not laid out
  again at its final size.
- The bottom navigation bar keeps animating its font size: it stays sharper than a filtered
  texture.
- Not looked at on a phone. The sampler is the same on every backend, and the goldens run on
  lavapipe.

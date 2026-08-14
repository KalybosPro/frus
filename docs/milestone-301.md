# Milestone 301 — A glow that is light, not a dent

Reported from the device: *"I scrolled vertically, top to bottom. But it happens as
if I pulled the sides too."*

The screenshot shows the overscroll glow at the top of the demo's list — a wide, flat
wash spanning the whole width, ending in a crisp curved boundary across the middle of
the card. Nothing was pulled sideways. But a hard curve drawn across the full width is
read as a deformation of the page, because that is what a curved edge across a page
normally means.

## What it actually was

`OverscrollGlow::paint` ends in:

```rust
scene.fill_path(&Path::oval(map(oval)), color.fade(opacity));
```

A **flat** fill. `Primitive::Path` carried `fill: Option<Color>` and nothing else, so a
path could not fade. The arc therefore had full opacity right up to its boundary and
none past it, at up to `MAX_OPACITY = 0.5`. There is no opacity at which that reads as
light: a glow is a falloff, and this had none.

Reproduced in the harness first, at the phone's proportions — the existing
`overscroll_glow` golden used a 240 px viewport, narrow enough to hide it. The new
`overscroll_glow_wide` is 424 px wide, pulled 220 px, and shows exactly what the
photograph shows.

## Paths can carry a gradient now

`Primitive::Path` gains `gradient: Option<PathGradient>` — an end colour and **two
points**, in the path's own space:

```rust
scene.fill_path_gradient(&path, from_color, to_color, from, to);
```

Two points rather than a direction, deliberately. The glow's ellipse is mostly off
screen; a gradient across its bounding box would spend itself where nobody looks. The
fade has to be aimed at the edge the light comes from and the depth the arc reaches.

The GPU side cost almost nothing, because the path shader already carried a colour per
vertex — it just interpolated it `flat`, every vertex having the same one. Paths now
carry a start colour, an end colour and a parameter `t`, and the fragment mixes them.
That makes the feature general: charts, `CustomPaint` and `ClipPath` can all fade now.

**Every existing golden is byte-identical.** A flat fill sets `color2 == color` and
`t = 0`, so `mix` returns exactly what it did before.

## The mistake worth recording

The first attempt evaluated the gradient **per vertex** and clamped it there. It
looked right and was wrong, in a way the numbers showed immediately: the glow came out
at 4% of its intended strength.

`t` is affine in position, so it interpolates exactly — but `clamp(t)` is not affine.
The glow's ellipse has its top vertex 180 px above the sliver that shows through the
band. A triangle reaching from there to the arc's tip has `t = 0` at one end and
`t = 1` at the other, so the rasteriser ramps the colour across all 194 px of it, and
the 13 px on screen get the tail end of the ramp.

Clamping belongs in the fragment. `t` is passed through unclamped, the shader clamps
per pixel, and the geometry can extend as far past the gradient as it likes.

There was a second, smaller version of the same error: the fade was first aimed at the
*band* — the widest the arc could ever be — rather than at `band × size`, the depth the
arc actually reaches at the current pull. That spent five sixths of the gradient off
screen too.

## What it looks like now

A vertical slice through the middle of the arc, before and after:

| y | before | after |
|---|---|---|
| 0 | 54, 61, 74 | 54, 60, 73 |
| 4 | 54, 61, 74 | 50, 56, 68 |
| 8 | 54, 61, 74 | 46, 51, 63 |
| 12 | 54, 61, 74 | 41, 46, 57 |
| 13 | 51, 57, 70 | 40, 44, 56 |
| 14 | 40, 44, 55 | 40, 44, 55 |

Flat, then a cliff — against full strength at the edge, gone by the arc's tip.

## Verification

- `cargo test -p frus-test` — **128 pixel tests, 0 failures**, and exactly one existing
  golden changed: `overscroll_glow.png`, which is the fix. Everything else — icons,
  charts, the notched bottom bar, the three clips — byte-identical, which is the proof
  that a flat fill goes through the new pipeline untouched.
- 815 workspace tests, 24 in `frus-gpu`.
- fmt clean, clippy silent.

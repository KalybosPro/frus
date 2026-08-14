# Milestone 302 — A fade that follows the shape

Milestone 301 gave paths a gradient so the overscroll glow could stop being a flat
wash with a hard curved edge across the page. This is the check of that fix on the
device it was reported from — and the check found the same defect one layer down.

## What the device showed

The demo's task screen fits on the phone in portrait, so there is nothing to
overscroll and no glow to look at. Rotating to landscape shortens the viewport, the
content overflows, and a pull past the top produces the arc.

Down the middle of the arc it was right: full strength at the edge, faded to 4/255 by
the arc's tip. At the **flanks** it was not. Reading the pull against the frame at
rest, per column:

| x | depth | peak | left at the cut |
|---|---|---|---|
| 220 | 49 px | 34 | **14** |
| 400 | 62 px | 31 | 8 |
| 800 | 81 px | 34 | 4 |
| 1200 | 81 px | 34 | 4 |
| 2200 | 68 px | 34 | **7** |

Fourteen levels left over where the shape stops is an edge, and it looked like one: a
curve sweeping in from the corner, softer than before but still a line drawn across
the page.

## Why a straight fade could not have worked

The arc is the cap of a very wide, very shallow ellipse. Its boundary is deepest in
the middle and rises towards each end. A straight fade reaches zero **on a line** — so
it can be aimed at the deepest point, and then everywhere else the shape's boundary
comes up to meet it *before* the fade has finished. What is left over is the edge.

This is not a tuning problem. No choice of the two points fixes it, because the thing
that must go to zero is a curve and a linear ramp's zero set is straight.

So the fade has to be measured the way the shape is: `PathGradient` gains a `Radial`
variant, and the distance is measured **in radii**, so the far end of the fade is the
ellipse itself, whichever way its boundary turns.

```rust
scene.fill_path_radial(&path, from_color, to_color, center, radii, inner);
```

`inner` is where the fade starts, as a fraction of the radii — a rind rather than a
ramp from the centre. The glow needs it: the ellipse's centre is hundreds of pixels
off screen, and a fade from there would arrive nearly spent. It falls out of the
geometry with nothing to tune, the centre sitting `radius - band` from the edge with
radius `radius`, both scaled by the same `scale_y`, which therefore cancels:

```rust
let inner = ((radius - band) / radius).clamp(0.0, 0.999);
```

The flanks now fade *sideways* as well as inwards, which is not a side effect to
tolerate but the point: a glow is brightest where the finger is and thins away along
the edge. The peak at the far column drops from 34 to 20 while the middle stays at 33.

## The gradient moved into the fragment shader

Milestone 301 resolved the fade **per vertex**, which worked because a straight fade
is affine and therefore interpolates exactly. A radial one is not affine, and the
failure is not a small error — it is total. Lyon tessellates a filled ellipse into
triangles whose corners are all points **on the boundary**: every vertex at the far
end of the fade, so every pixel between them too, and the entire arc would come out at
the end colour, which is transparent. The shape would vanish.

Per-vertex evaluation makes the gradient a function of the tessellation. So the vertex
now carries the gradient's *description* — four floats of geometry and two of kind —
identical across the path, and `path.wgsl` resolves it from the pixel's own position.
That also retires 301's other subtlety: there is no longer a `t` that must be passed
unclamped because clamping it per vertex would be wrong.

It costs one more `vec4` and one `vec2` per vertex, and paths are the smallest of the
four primitive streams.

## Verification

- On the device — Huawei STK-L21, Android 10, the release APK. Same pull, same
  columns: what was left at the cut is **4/255 across the whole arc**, down from 14 at
  the flank, and the sweeping curve is gone from the corner.
- `cargo test -p frus-test` — 128 pixel tests, and **exactly the two glow goldens
  changed**. Icons, charts, the notched bottom bar, the three clips: byte-identical.
  A flat fill still goes through untouched, and so does a straight one.
- 968 workspace tests, fmt clean, clippy silent.

## Left

The gradient is a two-stop fade with no stop list, and strokes cannot take one. Both
are additions when something needs them rather than gaps in this.

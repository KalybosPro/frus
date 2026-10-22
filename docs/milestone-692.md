# Milestone 692 — Path triangles kept between frames

## Objective

Milestones 690 and 691 left the same note: every path is tessellated again on every frame. An
icon, an SVG illustration or an animation's unchanged shapes cost lyon's work 60 times a second
to produce the same triangles, and the cost grows with the artwork, not with what changed.

## What was done

- **`MeshCache`** (`frus-gpu/src/mesh.rs`) keeps a path's triangles — positions and indices,
  nothing else — keyed by what is done to it (filled by a rule, or stroked at a width with a
  cap) and by its outline **relative to its first point**, rounded to 1/256 px.
  - Relative, because the most common change is a move: a list scrolled, a layer of an
    animation sliding, a widget laid out a few pixels over. Those paths find their triangles
    again; one that turned or changed size does not.
  - Rounded, because a moved outline's relative coordinates differ in their last bits. Paths
    that round alike share triangles that are at most 1/512 px from either, far below the
    0.1 px curves are flattened to.
  - Only the shape is kept. Colour, gradient and clip go into the vertices as before, when the
    triangles are appended to the frame, so a path recoloured or reclipped still hits.
- **Ageing and a budget.** An entry not drawn for more than 120 frames (two seconds at 60 Hz)
  is dropped. When the entries hold more than 2¹⁸ vertices (~10 MB with their indices), the
  frame's end keeps only what that frame drew: an animation whose shapes differ on every frame
  must not grow the cache without end.
- **One frame, one end.** Layers prepare their paths before the frame's own scene, several
  times a frame, so the cache's frame ends once, from the compositor, after the main scene.

## Measured

lottie-android's 134 snapshot-test animations (milestone 691), 60 frames each at 400×400, the
path work of a frame (lookup plus tessellation on a miss) on the development laptop, release
build:

| | without | with |
|---|---|---|
| all 134 files, 60 frames | 1.03 s | 0.23 s |
| hits / misses | — | 19 187 / 4 425 |
| `LayerBlend_*` (shapes that move), per frame | 0.5–0.75 ms | 0.02–0.09 ms |
| `ReducedMotion`, the heaviest, per frame | 1.64 ms | 0.21 ms |

A static picture — icons, an SVG illustration — misses once and then hits on every frame.

## Not done yet

- A path that rotates or scales misses on every frame: the key is in screen space. Keying in
  the path's own space would need the scene to keep the transform apart from the points, which
  it does not.
- The triangles are still assembled into vertices and uploaded every frame; keeping them on the
  GPU is the next step if that shows up in a profile.
- Measured on the test phone.

## Verified

Six unit tests in `mesh.rs`: a moved path is not tessellated again and comes back at its new
place; fill rule, stroke width and cap and the outline each make a different entry; the
triangles cover the shape (a square's area) and are relative to its first point; an empty path
keeps nothing; ageing; the budget keeping only the last frame. On the GPU, every `frus-test`
target: path paint, SVG and Lottie pass, and the goldens fail exactly the 15 they fail on
master on this laptop (blessed under lavapipe).

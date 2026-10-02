# Milestone 610 — A layer costs its size, and the display sets the pace

Milestone 609's frame statistics left the phone's slow frames with the GPU. The scene held two
or three layers while scrolling, and each one cost the whole screen. This milestone makes a
layer cost what it covers, and makes the display set the pace of the frames.

## What a layer cost

A layer, which is a group drawn at one opacity, clipped to a rounded box, or under a
transform, is drawn into a texture of its own and then put back on the frame. That texture was
**the size of the surface**, allocated anew every frame. It was put back by a quad **covering
the whole surface**, so the compositing shader ran on every pixel of it, whatever the layer
held.

On the test phone, 1080 × 2340 with four samples a pixel, the home screen's group-opacity
showcase holds two squares, 56 × 32 px. It cost two passes over 2.5 million multisampled
pixels, every frame. The ink of a touch and the overscroll glow added more of the same.

## The region

A layer's **region** is now its clip, in whole pixels, held to the surface:

- its content is moved so that the region starts at the origin;
- it is drawn into a texture of the region's size;
- it is put back by a quad covering only the region, the texture read from the region's
  corner.

Nested layers do the same inside their parent's texture, relative to it. Since the
composite already discarded everything outside the clip, nothing it drew was outside it.

**A layer keeps the whole surface** when its content can go past its clip, or holds geometry
in the surface's own coordinates, or holds a layer that does:

- a transform;
- an image filter (a blur spreads);
- a backdrop;
- a path clip, whose mask is a picture of the surface.

So does a layer whose clip covers the whole surface anyway.

Three things follow from moving content into its own frame:

- **The cache keeps a moved layer.** It compares what a layer draws, and a layer carried
  across the screen whole, such as a row scrolling, now draws the same thing in its own frame.
  It is not drawn again.
- **The painters' viewport follows the size drawn at**: a group sets it to its own size before
  its pass, and the frame sets it back to the surface's after the layers.
- **The multisampled scratch target is kept per size**, a handful of them, rather than one
  that would have been reallocated each time two sizes alternated.

**A moved layer now moves its mask.** `Primitive::translated` left a layer's filter where it
was, on the grounds that a mask is written in fractions of its box. But a mask is resolved into
the scene's coordinates before it is drawn, so a moved masked layer was masked where it had
been. This was reachable before this milestone: a reorder moves rows this way. A layer drawn in
its region is moved too, and a masked layer inside one would have been masked in the wrong
place.

## The pace: Fifo, always

The surface took the first presentation mode it offered. On the test phone that was
**Mailbox**: the application draws as fast as it can, the display shows the latest image, and
the others are drawn for nothing. The frame statistics showed median intervals of 14 ms on a
16.7 ms display: images never seen, the battery spent on them, and a motion not paced by the
display. Now it is **Fifo**, one image per refresh, in order. Every surface supports it.

## On the phone

Flicking up and down the home screen, then a slow drag, measured with milestone 609's
statistics:

| | milestone 609 | layers bounded |
|---|---|---|
| flicking | 35–38 fps, 95th-percentile interval 57 ms | **41–45 fps**, 37–44 ms |
| rendering, 95th percentile | 45–50 ms | 31–39 ms |
| a slow drag | 49–52 fps, 95th percentile 58 ms | **55–58 fps**, 35 ms |

Some other things were measured and ruled out:

- **Multisampling** made no difference: one sample a pixel gave 43–46 fps.
- **The overscroll glow** made no difference either: drawing none gave 42.
- **A frame latency of one** made no difference.
- **Fifo** gives the same rate as Mailbox, with intervals at the display's 16.7 ms rather than
  under it.

## Not yet

Flicking, about a third of the frames still miss the display. The CPU's part of rendering, the
wait for an image and the drawing, stays under a few milliseconds. It is the presentation that
waits, on a GPU still busy with earlier frames. What the GPU spends its time on is not known
yet: the frame statistics time the CPU. The next step is the GPU's own clock, through
timestamp queries, where the device has them.

## Verification

- **Every golden, unchanged to the pixel**, at a tolerance of nought, with the layers bounded:
  clips, rounded and oval clips, nested layers, group opacity, transforms, filters and
  backdrops. The GPU tests and the widget tests pass too.
- The region:
  - is the clip in whole pixels;
  - is what is on the surface, for a layer partly off it;
  - is a single pixel, for one wholly off it;
  - is the whole surface, for a layer that covers it.
- The whole surface is kept:
  - for a transform, a blur, or a path clip;
  - for a path-clipped layer one level inside, and two levels inside.
- A moved layer moves its clip and its mask.
- Mutations, each failing a test:
  - the texture read without the region's corner;
  - the quad at the origin;
  - the frame's viewport not set back after the layers;
  - a group's viewport not set before its pass;
  - the nested rule one level deep only;
  - a layer covering the surface given a region of its own;
  - the mask left behind.

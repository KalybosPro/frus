# Milestone 600 — `AnimatedAlign` with factors, and the room `Aligned` takes

Milestone 597 gave `Aligned` its width and height factors and left `AnimatedAlign` without
them. Its anchor moved through the ordinary layout's offset, which a measured box does not
read.

## What it does

- `AnimatedAlign::width_factor` and `height_factor`, as on `Aligned`: on that axis, the box is a
  multiple of its child. The anchor still moves when it changes, on every axis. The factor
  itself does not animate, as in the reference.
- **`AnimatedAlign` now takes the room as `Aligned` does.** It was a container with an animated
  anchor, which asks for nothing: inside a box that stretches nothing it holds, it was as big as
  its child and had no free space to move it across. The reference's is an `Align`, and fills.

## How

`AnimatedAlign` is now an `Aligned` with a way to move:

- `Aligned` carries an optional duration and curve. With them, it declares its anchor to the
  runtime (`anim_offset`), which tweens it. The container that places the child moves its own
  anchor too.
- Without a factor, the `Aligned` is that container's node, and the ordinary layout reads the
  tweened anchor, as before.
- With a factor, the container is the measured box's child and places the child on the axes
  without a factor, moving its own anchor there. On the axes with a factor, the walk places the
  container: it now reads the runtime's anchor for the box, as the ordinary layout does.
- `AnimatedAlign` forwards to its `Aligned`: layout, anchor, animation, factors and the room it
  asks for.

## Verification

- With a width factor of 4 on a 20-px square, going from left to right: at rest at 0, half way
  at 30, at the end at 60, in an 80-px box.
- With a height factor, going from top left to top right on a 200-px page: half way at 90, and
  still at the top of its 40-px box.
- Without a factor, in a 100×60 box, a square anchored bottom right is in the box's corner.
- The existing anchor tests: mounted settled, moving, arriving, reached through a selector.
- Mutations, each failing a test:
  - the walk not reading the moving anchor;
  - the placing container not moving its own;
  - the anchor not declared to the runtime;
  - the measured box not passed on;
  - the room not asked for.

  The last survived at first. The test put the anchor in a column inside a scroll, and a column
  there stretches its children whatever they ask. It now sits in a box that does not.

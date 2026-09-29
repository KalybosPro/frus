# Milestone 597 — Width and height factors for `Aligned` and `Center`

Milestone 579 gave `Aligned` and `Center` the room they are given and left their factors out: a
box that is a multiple of its child has to measure the child first, which #52 was about.
Milestone 596 added that measure, as `Widget::size_factor`, for `SizeTransition`. This uses it
again.

## What it does

`Aligned::width_factor(f)` and `Aligned::height_factor(f)`, and the same two on `Center`:

- On an axis with a factor, the box is `f` times the child's size there, in whole pixels, instead
  of the room. The child sits at the anchor inside it.
- On an axis without one, the box still takes the room, as before.
- Below `1.0` the box is smaller than the child, and the child **spills**: it is not cut, as the
  reference's is not.
- `Center::new(child).width_factor(1.0).height_factor(1.0)` is a box exactly the size of its
  child: the usual way to stop a centre from taking the page.
- A directional anchor, such as `CENTER_START`, follows the script on an axis with a factor too.

## How

A factored `Aligned` is a measured box, as `SizeTransition` is:

- The container that already placed the child becomes the box's child, laid out apart. It is
  asked its own size on the axes with a factor, and given the room on the others, where it still
  places the child. Without it, a child with no size of its own would be stretched across the
  room rather than placed in it.
- `size_factor` gives the multiples, `1` on an axis without a factor. The fingerprint already
  hashes them (milestone 596).
- The box asks for the room only on the axes without a factor (`fill_axes`).
- Where the child sits on an axis with a factor is decided in the walk, which now resolves a
  widget's `alignment_geometry` against the reading direction for a measured box. The transform
  itself only holds a physical alignment.
- Without a factor nothing changes: the same container, the same tree, the same ids.

### Who cuts: `Widget::clips_to_box`

The walk used to cut the child of any box with a `size_factor`. That was right for a reveal and
wrong for an anchor. The cut is now its own hook, `Widget::clips_to_box`: `SizeTransition` says
yes, a factored `Aligned` says no, as the reference's `Align` does not clip. Wrappers forward it.

## Differences from the reference

- **Wrapping under a width factor.** On an axis with a factor, the child is asked its size with
  no limit. The reference lays it out loosely, with the room as the most it may take. A paragraph
  in an `Aligned` with a width factor is therefore one line here, where it wraps at the room in
  the reference. A measured box's rules (`AxisConstraint`) have no "at most the room" yet.
- **`AnimatedAlign`** has no factors yet. Its anchor moves through the ordinary layout's offset,
  which a measured box does not read.

## Verification

- A height factor of 3 on a 20-px child: the box is 60 px, the next thing starts at 60, and the
  child, with no width of its own around a 20-px square, is centred across the page.
- A factor of 2 with `BOTTOM_RIGHT`: the child is at the bottom right of a 40-px box.
- Alone on a page, where the height is on offer, a height factor of 2 is still a 40-px box at the
  top, not the page's height.
- A width factor of 3 in a row: a 60-px box, centred child, next thing at 60.
- Both factors at 1: the box is the child, and the next thing starts at 20.
- A factor of 0.5: a 10-px box, the next thing at 10, the child not cut.
- `CENTER_START` with a width factor, in a 200-px row: at the left, and at the right in
  right-to-left.
- Factors 2 then 3 on one runtime: the next thing moves from 40 to 60.
- Mutations, each failing a test:
  - the child given the room on an axis with a factor;
  - a factor left out of `size_factor`;
  - the room asked for on an axis with a factor;
  - `Center` not passing the measured box on;
  - the anchor not resolved against the reading direction;
  - the old rule, cutting any box with a factor;
  - `SizeTransition` not cutting.

  The first survived at first: every test put the axis with a factor where nothing was on
  offer, so being given the room and being asked came to the same thing. The page case above
  now tells them apart.

# Milestone 596 — SizeTransition

This is the explicit transition that issue #32 left out, because "it animates a size it has to
measure". Milestones 592 to 595 made measuring a child possible and used it twice.

## What it does

`SizeTransition::new(factor, child)` reveals its child along one axis by a value the caller
owns:

- The box is `factor` of the child's size, from `0.0` (nothing) to `1.0` (all of it).
- The child is laid out at its own size throughout and cut to the box: uncovered, not squeezed.
- What is below or beside the box moves with it.
- `axis` chooses down (the default) or across.
- `axis_alignment` chooses which part shows while the child is not all there: `-1` the start,
  `0` the middle (the default, as the reference's), and `1` the end.
- Across the axis, the box is as big as the room.

In the reference it is an `Align` with a height or width factor inside a `ClipRect`. This is the
same thing.

## How

It is the measured box of `ConstraintsTransformBox` again, with one new hook:

- **`Widget::size_factor`** gives the fraction of the child's size the box is, per axis. The
  measure closure multiplies the child's size by it, **rounded up to whole pixels** (milestone
  289), and then holds it to the offer as usual.
- The layout fingerprint hashes the factor. A box a different fraction of its child is a
  different box, and a factor that moves from frame to frame has to relayout.
- The walk cuts the child to the box, as it does an animated one.
- On the axis it reveals along, the child decides its own size. Across it, the child is given
  the room and the box asks for it (`fill_axes`, as `AnimatedSize` does).

The hook is general: it is how the `Aligned` width and height factors, which milestone 579 left
waiting on #52, can be done.

## Verification

- Half of a 20-px mark: the box is 10 px, the next thing starts at 10, and the mark is still 20 px
  tall and cut at 10.
- The alignment at the end shows the bottom half: the mark is moved up by the half that is hidden.
- 0 is no box and 1 is the whole mark.
- Across: a quarter of a 40-px mark is a 10-px box, and the next thing in the row starts there.
- The factor moving from 0.5 to 1.0 on one runtime moves what is below from 10 to 20: the cache
  saw it.
- In a 100-px box that stretches nothing, a mark with no width of its own is 100 px wide.
- Mutations, each failing a test:
  - the factor not applied;
  - no cut;
  - the factor left out of the fingerprint;
  - the alignment ignored;
  - no request for the room across;
  - the factor on the wrong axis.

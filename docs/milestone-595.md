# Milestone 595 — AnimatedCrossFade

This is the last of the eleven implicit animations issue #30 listed. It needed `AnimatedSize`
(milestone 594), which needed the measured children of #52 (milestones 592 and 593).

## What it does

`AnimatedCrossFade::new(first, second, show_first, duration)` shows one of two children. When
`show_first` changes:

- the other child fades in over the first;
- the box moves from the first child's size to the second's;
- whatever of the leaving child runs past the box is cut.

`first_curve`, `second_curve`, `size_curve` and `alignment` are the reference's. The alignment
is top-centre by default. The hidden child, and the one leaving, take no tap and are not read
out.

In the demo, a filter that leaves no task cross-fades the list to "Nothing to show for this
filter." and back. The list still grows and shrinks as tasks come and go, through the
cross-fade's own `AnimatedSize`.

## How it is built

It is composed, as the reference's is: an `AnimatedSize` around a cell holding both children.

- **The cell** is an `overlap` container (milestone 502's one-cell grid), stretched:
  - the **shown** child is laid out in it, and gives the cell its size;
  - the **hidden** child is in an `OverflowBox` covering the cell, laid out at the cell's width
    and **its own height**, and anchored by the alignment. It adds nothing to the cell's size,
    so the size the box moves to is the shown child's alone, as the reference's positioned
    bottom child adds nothing to its stack.
- `OverflowBox::natural_height()` is new: the box's width and the child's own height. Without
  it the leaving child would be squeezed to the box's height as the box shrank.
- **Each child's slot** is the same four wrappers whether it is shown or not, under a key of
  its own: `Keyed`, then `IgnorePointer` and `ExcludeSemantics` switched on while hidden, then
  an `AnimatedOpacity`. A child keeps its fade from one state to the other. The shown child is
  last in the cell, so it is painted over the one leaving.
- **`FillWidth`** sits inside each slot: a column that stretches its child across and asks for
  the width. The wrappers above it stretch nothing they hold. Without it, the children were
  laid out 0 px wide, painted but impossible to tap. The request travels up through them to the
  cell (milestone 590).

## Found on the way: a faded-out subtree is no longer painted

A group opacity of 1 already skipped its layer. A group opacity of **0** still painted the
whole subtree into a layer of opacity 0, so the frame held primitives nobody could see. A
demo test checks that everything painted inside a list row moves with the row, and it found
one of those: the hidden "Nothing to show" line lying over the rows.

The reference's `RenderAnimatedOpacity` does not paint a child at 0. Now neither does this one.
The subtree is still walked, so what it registers is still there: an `IgnorePointer` decides
input, not the fade, as in the reference. Its primitives are dropped.
`FadeTransition(0.0)`'s test said the mark was painted at 0. It now says nothing is painted.

## Differences from the reference

- **Width.** Both children fill the width the cross-fade is given. The reference's are loose,
  and a narrow child there is as narrow as it is. Filling the width is what a text or a column
  wants, and what `AnimatedSize` already does with its child.
- **Retained state across the switch.** The hidden child sits one box deeper than the shown one
  (the `OverflowBox`), so its identity changes when it is shown or hidden. What the runtime keeps
  for it, a scroll offset or a caret, does not survive the switch. The fade does, being above
  that box.

## Verification

- At rest, the box is the shown child's 40 px even though the hidden one is 100 px. Only the
  shown child is painted, and it takes the tap.
- A switch: half-way, both are painted below full opacity, the new one over the old one, the box
  is between 40 and 100 px and what is below it follows, and only the new one takes a tap. At the
  end, only the new one, at 100 px.
- Switching back from the 100-px child: half-way it is still 100 px tall, cut at the box's
  bottom, while the box is on its way down.
- A plain shown child over a tappable, labelled hidden one: a tap finds nothing, and the label is
  not read out.
- The demo's 79 tests, and the library's.
- Mutations, each failing a test:
  - the hidden child laid out in the cell;
  - its own height not asked for, in the builder or in the walk;
  - its semantics or its taps left on;
  - the layers in the other order;
  - `FillWidth` not asking for the width;
  - a zero opacity painted;
  - both children at full opacity.

  Three survived the first round, and each was the test's fault:
  - The blocks were told their height, which a box keeps however it is laid out, so a leaving
    child squeezed to the box looked the same. They are now as tall as what they hold.
  - The hidden tappable block sat in a `Semantics`, which stretches nothing it holds. It was
    0 px wide, and there was nothing to tap whether the taps were on or off. It now has a width
    of its own.

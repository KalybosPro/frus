# Milestone 375 — A list that runs the other way

`ListView` scrolled down the screen and only down the screen. A row of cards, a strip of
thumbnails, a shelf of covers — none of them was possible, and each of them is this same
list turned on its side.

The reference writes `ListView(scrollDirection: Axis.horizontal)`. Ours had no way to say
it: the widget carried no axis, and the walk that places its items was written in `y`.

## Not a `Flex` in a scroll

A horizontal row *can* be built today: put a `Flex::row` inside a
`SingleChildScrollView` with `Axis::Horizontal`. It works, and it builds every child every
frame.

That is fine for six chips and wrong for two hundred covers — and two hundred covers is
exactly what a shelf is. The whole reason `ListView` exists is that the per-frame cost
should follow what is *visible*, not what exists, and that argument does not become less
true when the axis changes.

## One axis and its cross

The walk's virtual-list branch was about seventy lines of `offset_y`, `item_height`,
`viewport.height` and `top`. The change makes it a **main** axis and a **cross** axis, and
only two `match across` at the very end have to know which is which.

The alternative was a second copy of the branch. It is tempting, because each copy reads
more plainly than the generalised one — and it is how the two would drift. The window
arithmetic below is where a reversed list gets its signs right, and a fix applied to the
vertical copy and not the horizontal one is not a hypothetical: milestone 361 already had
to go and correct that arithmetic once.

`item_height` is `item_extent` now, in `VirtualList` and in `ListView::new`'s
documentation. A field called `item_height` on a horizontal list is the same kind of lie
milestones 367 and 369 went round correcting; better not to add a new one.

## What follows the axis, and what does not

**Padding** follows it. The leading inset is the one at the end the items start from, so a
horizontal list's `left` leads and its `top`/`bottom` become cross insets that take height
off every card. Reversed, `right` leads instead.

**`reverse`** follows it. Item 0 sits at the end the axis finishes at — the bottom of a
column, the trailing edge of a row — which is the same conversation about which end an
*index* is, in a second direction.

**The cross axis is handed over whole**, exactly as milestone 351 established for a
vertical list: a card whose height nobody set is as tall as the shelf rather than hugging
whatever is inside it. Constrained-but-not-filled is what made a list of coloured rows
paint a column of chips down the left, found on a device; the same mistake across would
paint a row of chips along the top.

**`Axis::Both`** does not follow it, because it cannot. A list virtualises along one axis —
that is what lets it place item `n` without building the ones before it — so `Both` reads
as vertical. A surface that scrolls both ways is `SingleChildScrollView`, which does not
virtualise and does not need to.

## Scrolling came free, and is tested anyway

The list registers the same `Scrollable` every other scrolling surface registers, with
`max_x` where it used to put `max_y`. Everything downstream — the wheel, the drag, the
fling, the overscroll glow, the scrollbar — already knew how to handle a horizontal one,
because `SingleChildScrollView` has had that axis all along.

Seven of the eight new tests are layout at rest. The eighth drives the offset the shell
would set and watches the window move, because **a shelf that lays out correctly and does
not move is the failure worth guarding against** — every other assertion would still pass.

## Left

`itemExtentBuilder` and `prototypeItem` — a list whose items are not all the same size —
are a different mechanism, not a parameter: the window can no longer be found by division.

## A correction

This note first said that `GridView` *does not scroll at all*. That was written from
reading its builders, not from running it, and it is wrong. Put inside a
`SingleChildScrollView`, a grid scrolls correctly: thirty tiles of 60 px in a 200 px window
report `max_y = 400`, and scrolling by 120 moves the first tile to `y = -120`.

What is actually missing is narrower and worth stating accurately. `GridView` does not
scroll **by itself** — the reference's is a `ScrollView` subclass, so `GridView(...)`
scrolls where ours needs wrapping — and it does not **virtualise**: all thirty tiles are
built and painted when twelve are visible, and thirty is the number a test used, not the
number a photo grid has.

# Milestone 518 — A row carried past the last one, and the release that put it back

Milestone 517 put `ReorderableList` under a finger and left one thing it saw: a row carried
to the bottom of the demo's list, and let go there, went back where it came from. The list
is followed by more of the page, so a finger at the bottom edge is over that, not over a row
— and the release asked only *what is under the pointer*. Nothing was, so nothing moved.

The same question had a smaller hole in it. Rows with a gap between them leave strips that
belong to no row, and a release over one of those moved nothing either.

## What the reference does

It drops at the nearest end. Its list works out where a carried item goes from the item's
position along the list's axis, against every item in the list — never from what happens to
be under the finger — so a row carried below the last one lands after it, and one carried
above the first lands before it. A gap between two items is not a place of its own; it is on
one side of one of them.

## The fix

**Where a release lands is now asked once, and answered in two steps.**
`reorder_drop_target` is what the insertion line and the release both use, so the line
cannot promise a place the drop does not keep:

1. the target under the pointer, as before — a row, a card, a board's drop zone;
2. over none, and for a vertical reorderable, **the nearest slot of its own list**.

*Its own list* is `reorder_siblings`: the children of the parent of the row that moves — not
of the grip that was grabbed — that can be dropped on, with their boxes this frame. A list's
rows are one widget's children, and so are a board column's cards and the zone that ends it.
Nothing else on the page is.

*Nearest* is `nearest_reorder_slot`, pure: the slot closest to the pointer along the axis —
below the last is the last, above the first is the first, a gap belongs to the nearer of its
two rows. Which half of that slot the pointer is on then says before or after, exactly as it
does over a row, so a finger below the last row means *after the last row*.

**Only across the list.** The fallback holds while the pointer is within the horizontal
extent of the list's slots. Beside it is somewhere else — another column of a board, whose
own cards and drop zone answer for themselves — and a card let go in the empty space beside
its column must not be read as *the end of the column it came from*. The reference ignores
the cross axis because its list is all there is; a board is not.

A table's columns, which reorder horizontally, are untouched.

## Verification

**On the device** — the same Huawei STK-L21, the finger held with `motionevent`, two tasks in
the demo's list:

- Carried by its grip below the last row, over the progress bar that follows the list, the
  slot opened after the last row and the insertion line was drawn there, with the finger over
  no row at all; the release put the row at the end. Seen three times: released in the same
  command as the hold, released from a separate command after a capture of the held state, and
  as the first gesture after a cold start.
- Carried above the first row, over the buttons above the list, the line was drawn on the
  first row's top edge and the release put the row first.
- A trace on the release, added for the run and removed after it, showed each time the target
  the fallback found and the index it asked for: `to=2` from `from=0` below the list, `to=0`
  from `from=1` above it.

**The first attempt did not move the row**, and it is recorded here because it was not
explained. It was on the build before the trace: the insertion line showed the slot after the
last row, and the release put the row back. It did not happen again in four attempts on a build
that differed only by the trace, including the same sequence and a cold start. It behaves
exactly like the surviving mutation in which the release asks only what is under the pointer —
the line and the release disagreeing — which would fit that APK not having been built from the
sources as they are now; that could not be checked, because the rebuild overwrote it.

**Tests.** `nearest_reorder_slot`: below the last row however far and above the first; a gap
between two rows split at its middle; beside the list no slot, and its own edges still in it.
And on the demo's real page: below the last row nothing can be dropped on — the bug, stated —
the first row's own list is its three rows, the nearest to that point is the last, and the
drop after it moves the first task to the end.

**Mutations**, seven. Five fail a test: the cross-axis guard removed, the list's edges
excluded, the farthest slot taken for the nearest, the distance measured from the top edge
only, and a row's list taken to be its own children. **Two survive, and are the shell's
wiring**: the release asking only what is under the pointer, and the fallback turned off for a
vertical list. Both live where the frame loop meets the tree, which no harness drives; the
device run above is what checks them.

## What is left

- **Keyboard reordering**, `Kanban` on the list machinery, a **horizontal** list and a
  **proxy decorator** — as 490 and 517 left them.

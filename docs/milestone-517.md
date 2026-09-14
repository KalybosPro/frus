# Milestone 517 — A row carried to the edge, and the list that stopped coming

Issue #44's last open line was the device run: `ReorderableList` (milestone 490) had passed
every test it had and had never been under a finger. It went under one on a Huawei STK-L21,
and a short drag worked — a row lifted by its grip, the neighbour sliding aside, the drop
landing where it was aimed. The long drag, the one the issue names — *a row carried past the
bottom so the list scrolls under it* — did not, in two ways.

## What the phone showed

**The scroll started, and then stopped on its own.** A row held at the bottom edge scrolled
the page 95 px and no further, however long the finger stayed. The captures said why before
the code did: the lifted row was drawn 97 px above the finger — almost exactly the distance
scrolled.

**Things that were not rows moved.** With the row in the air, the floating action button's
`+` sat a row above its own disc, and with the finger a little lower the navigation bar's
items left the bar.

## Why

**The ghost rode up with the list.** The shell draws the carried row at *the row's box this
frame*, offset by *how far the finger has moved since the press*. Both are right until the
list scrolls: then the box moves up with the content while the finger does not, so the ghost
moves up too. Auto-scroll is measured against the ghost, not the pointer — the reference's rule,
and the right one — so the ghost climbing back inside the viewport is the scroll switching
itself off. The same arithmetic lived in the lifted-item path of `Draggable`, whose card was
anchored at the press but whose contents were copied from the scrolled frame: the two would
have come apart by the distance scrolled.

**The reflow was geometry and nothing else.** `reflow_reorder_cards` slides every primitive
whose centre lies in the source's band and below it. On a board that is the cards, and a
column's background is spared by its height. On a page it is also whatever floats over the
list or sits under it — a button, a bar — and neither is a card.

## The fix

**The press follows the content.** When auto-scroll moves an area's offset, the distance the
content actually moved on screen — the applied offset, clamped, turned back into screen terms
by the same `offset_delta` every other path uses, so a reversed list is right as well — is
added to the drag's press point. For a lifted item, the box recorded at the press moves with
it. The ghost is then exactly where it would have been had nothing scrolled, so it stays over
the edge for as long as the finger holds it there, and the list keeps coming. `follow_content`
is that step, pure, and the only place it is written.

**Only what can be reordered makes room.** `reflow_reorder_cards` takes the owners of what may
move, and leaves everything else where it is drawn. `reorderable_owners` answers that from the
tree: every reorderable that can be dropped on, and all it paints — a row with its label, its
avatar, its delete button and the grip inside it; a card with its content; a board's drop zone.
Geometry still decides *how far* each moves; the tree decides *whether*.

## Verification

**On the device** — the same phone, the same gesture, rebuilt:

- Carried by its grip to the bottom edge and held there, the row scrolled the page on to the
  end of the list and stopped at the end of the content, not 95 px in. Two seconds later,
  still held, nothing had moved: there was nothing left to reveal.
- The lifted row stayed under the finger the whole way, its grab point at the finger's height
  in every capture.
- While it was in the air the floating action button kept its `+` on its disc, and the
  navigation bar kept its items.
- Carried back up from the edge onto the last rows, the slot opened between *Five* and *Six*,
  and the release put the row exactly there — the drop landing where the gap had shown it would.

**Tests.** `follow_content` on both kinds of carried thing: after a 95 px scroll the reorder's
ghost is where it would have been had nothing scrolled, and a lifted item's recorded box went
with its content. `reflow_reorder_cards` leaves a button floating over a column alone while the
card below the lifted one still closes the gap. And in the demo, on the real page, what
`reorderable_owners` lets move is checked against what the page paints: everything painted
within a row moves, and nothing centred outside one does — a floating button, a bar, a field
and a header all around the list.

**Mutations**, seven. Five fail a test: the reorder's press left where it was, the lifted
item's box left where it was, the reflow ignoring what may move, what may move taken from what
cannot be dropped on, and a row making room without what it paints. **Two survive, and are
the shell's wiring**: the press moved by the offset rather than by the content (the sign), and
the preview handed every primitive to the reflow. Both live where the frame loop and the paint
meet, which no harness drives; the device run above is what they were checked by.

A first run of those mutations reported all seven killed, and was wrong: the demo was being
edited alongside, did not compile, and every mutation checked through the demo test failed on
the compile error. They were run again in a separate worktree holding only this milestone's
files, which is where the verdicts above come from.

## What is left

- **A drop past the list's last row moves nothing.** The demo's list is followed by a footer,
  so a finger held at the bottom edge is over the footer, not over a row, and a release there
  has no target and puts the row back. Carrying it back onto the last row works, as above. The
  reference's list drops at the nearest end instead, and that is the next step for this gesture.
- **Keyboard reordering**, which wants vertical arrows `Key` does not have (milestone 490).
- **`Kanban` on the same list machinery**, a **horizontal** list, and a **proxy decorator** for
  the row in the air — all as 490 left them.

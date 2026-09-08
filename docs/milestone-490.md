# Milestone 490 — A list whose rows can be dragged into a new order

Advances [#44](https://github.com/KalybosPro/frus/issues/44), and closes the auto-scroll
half of milestone 285's loose ends.

`ReorderableList`: rows a finger can pick up and put down somewhere else in the list. A
playlist, a set of favourites, the demo's task list.

## Most of it was already here, and that is the finding

Reordering existed as a **board**. `Kanban` moves cards between columns, `Table` moves
columns, and an application that wanted the ordinary case had either to take a board's
assumptions or to build the gesture out of `Draggable` and get the hard parts right itself.

But the *gesture* was never Kanban's. The shell has carried a vertical reordering drag
since milestone 252 — the ghost lifted out of the frame, the insertion line at the hovered
half, the neighbours sliding out of the way, the drop routed back as `on_reorder` — and it
carries it for **any** widget that answers `reorder_index` with `ReorderAxis::Vertical`. A
board is one caller of that machinery, not its owner.

So the list is a row wrapper, a grip, and the decisions the gesture leaves open. What was
missing underneath was two things, and both are the sort that only show up in a list:

- a list **long enough to scroll**, which needs the viewport to move while a row is carried
  past its edge;
- a row that is **also** somewhere a finger scrolls, which needs the press not to be taken
  by the drag.

## Who may be grabbed

A row that lifted on a press anywhere on it would stop its own list scrolling: the press
that starts the drag is the press that would have started the scroll. The reference splits
the answer into two widgets — `ReorderableDragStartListener` for a grip and
`ReorderableDelayedDragStartListener` for a hold — and leaves the choice to the
application, because it depends on what else the row is for.

`ReorderGrab` is that choice, and it defaults the way the reference defaults it, read from
the platform rather than from a flag: a **grip** on a desktop, where the pointer is precise
and a small target is easy, and a **hold** on a phone, where the finger is coarse and the
list scrolls under it.

The hold needed the shell. A press on a reorderable used to take the drag there and then;
now a row that asks for a hold takes **nothing** — a `pending_reorder` waits for the
long-press deadline while the scroll keeps the gesture, and the deadline hands it over. It
is `pending_lift` with a different destination, and it is deliberately its own field rather
than a second use of that one: what the two become is different, even though the reason
they wait is the same and it is the reason a list can still be scrolled.

**The demo uses the grip, on the phone too**, and that is the case the split exists for.
Its task row already answers to a hold — the hold lifts it towards the two state zones —
and it already answers to a sideways drag, which dismisses it. The grip is the one way in
that takes nothing away from the other two.

## What index comes back

Dropping a row on the lower half of the third row means *after the third row*, raw index 3.
But the row being carried is no longer in the list, so where it lands is index 2.

The reference passed the raw number and told applications to subtract one themselves. It
has since **deprecated that callback** in favour of `onReorderItem`, whose whole
documented difference is that it "adjusts the newIndex parameter for a removed item". This
framework starts where the reference ended up: `on_reorder(from, to)` gives the index the
row ends up at, and `settled_index` is the one line that makes it so, tested in both
directions and onto itself.

Onto itself matters more than it looks. A drop on a row's **own** lower half asks for the
slot after it, which is where it already is. The shell guards that case by identity — and
it cannot when the grip and the row are two widgets, because what was grabbed is the grip
and what was dropped on is the row, and they do not compare equal. The arithmetic knows: a
settled index equal to `from` sends no message at all.

## Two hooks, and a symmetry that was missing

`Widget::reorder_droppable`, the mirror of `reorder_draggable`. A grip is a **source that
is not a target**: it is a small box inside a much larger row, and the drop is aimed at
whatever is topmost under the pointer. Without it, carrying a row over another row's grip
aims at the grip — the insertion line would be drawn across a 40-pixel gutter instead of
across the row, which is a promise about where the row is going that is not true.

That hook then earns a second keep, and it is the one that would have been a device bug.
**What is grabbed is the grip; what moves is the row.** Everything the preview is made of
is the row's — the ghost lifted out of the frame, the height of the gap that closes behind
it, the band the neighbours are matched against — and the shell used to take all of it from
the box of whatever was grabbed. From a grip that is a forty-by-twenty gutter: an icon
floating instead of a row, a slot two centimetres too narrow opening for it, and the
neighbours never sliding at all, since their centres are nowhere near the gutter's band.
So the shell now resolves *what is moving* from *what was grabbed*, and `reorder_droppable`
is what tells the two apart: a grip is not droppable, so the row is the droppable
reorderable of the same index under the grip's own middle. Two questions, one flag.

`Widget::reorder_announcement`, what a screen reader is told once the row has landed. The
shell already spoke on a drop, and could only speak of the **axis**: a Kanban card's index
is a flat `column × stride + position` that means nothing read aloud, so vertical drops
said "Card moved" and nothing more. A widget whose index *is* a position can do better, and
now says so itself — "Moved to position 3 of 7". The old wording is the fallback, for the
widgets that still cannot say anything better.

## The list comes to meet the row

Carrying a row past the bottom of the screen has to scroll the list, or a list longer than
a screen cannot be reordered at all. The roadmap has listed this as a gap of `Draggable`
since milestone 285.

It is one small function, and it is the reference's law: the speed is the **overhang**
times fifty pixels per second — the further out the item is, the faster the list comes to
meet it — with the overhang capped at twenty, so carrying a row right off the window
settles at a fast but aimable speed instead of one nobody can steer.

Four decisions are worth naming, because each is a way it could have felt wrong:

- **The box that is measured is the ghost's, not the pointer's.** A row half off the bottom
  is already asking to go further, whatever the finger holding it happens to be over.
- **An exhausted edge stays still.** At the end of the content there is nothing left to
  reveal, and a list straining against its own end while a row hovers there is noise.
- **The leading edge wins.** An item taller than the viewport hangs over both edges at
  once; pulled two ways it would scroll nowhere, or jitter between them.
- **It moves the offset *and* the target.** The area's inertia rests at a target of its own,
  and leaving that behind would spring the list straight back out from under the row.

Because the helper takes the box being carried, `Draggable` gets it too: a lifted item now
auto-scrolls the area under it exactly as a row does. That was the entry on the roadmap,
and it cost nothing extra to close.

## The builder's order is not part of the API

A row wrapper needs to know things that belong to the list — which grab mode, what the grip
looks like, how many rows there are — and those are set by builder calls that may come
after the rows were added. Settling each row when it is added would make
`.grab(…)` silently do nothing if it came last, and a builder whose method order matters
without saying so is a bug waiting for somebody's afternoon (the roadmap has one already,
on `BottomSheet`).

So the rows share the list's spec, and read it when they are asked. The grip's *contents*
go one step further: they are built by a `ThemeBuilder`, which runs during layout, so the
glyph, its size, its colour, or a widget of the application's entirely can be set at any
point in the chain and still be the one that is drawn. A grip in a mode that does not use
one is a zero-sized box, so nothing is measured, drawn, or hit — the mode itself is live.

## In the demo, the indices lie

The demo's list is **filtered**. The rows on screen under *Active* are not the rows in the
model, so a `MoveTodo(0, 1)` that indexed the model would step over the done tasks between
them and put the task somewhere nobody pointed at — the failure that makes a reordering
list feel haunted.

The update turns both indices back into **identities** before anything moves: the row that
was carried, and the row it landed on. The destination is then found in the model *after*
the removal, for the same reason the widget hands over an index that already counts the row
as gone. The test drives it under the filter, because unfiltered the two happen to agree
and the bug is invisible.

`visible_todos` came out of it: the filter now lives in one place, because a screen that
decides which rows to draw and an update that decides which row moved have to agree on the
answer.

## What is left

- **Keyboard reordering.** `Table`'s columns move with Ctrl+Left/Right, which exist as
  `Key::Left { word: true }`. A list's rows want Ctrl+Up/Down, and `Key` has **no vertical
  arrows at all** — they belong to the focus walk in the shell and never reach a widget.
  Adding them is a variant plus a decision about what the focus walk keeps, which is a
  design question and not a line of code.
- **Migrating `Kanban` onto this.** The roadmap has asked for it since milestone 285. Now
  that a plain vertical list is a caller of the same machinery, the board is the second one
  rather than the only one — but a column is a two-dimensional index, and the flat
  `column × stride + position` is what stands between them.
- **A horizontal list.** The machinery has both axes; this widget declares the vertical one
  and would need the grip to move to the bottom edge and the insertion line to turn.
- **A proxy decorator**: what the reference lets an application do to the row while it is
  in the air. The ghost here is the row's own paint, lifted from the frame, and it is not
  yet anybody's to change.

## Not verified on a device, and the issue stays open for it

The issue's own condition for done is a phone: the demo's list reordered by dragging,
including a row carried past the bottom so the list scrolls under it. **No device was
attached this session**, so that has not been done, and this repository's own rule —
written down after a rubber band that every green test agreed existed and no finger could
ever see — is not to believe unit tests about a gesture that only exists while something is
held. Everything here is exactly the kind of thing that rule is about.

So the issue stays open on that one line. What stands in for it meanwhile is a test that
drives the demo's **real** tree through the registries in the order the shell reads them —
grab, resolve what moves, aim the drop, route the message — which is as close as this
repository gets without a phone, and which would have caught a grip that shadowed its row.

## Verification

Nine tests on the widget, six on the auto-scroll's arithmetic, two on the demo — the
mapping of screen indices to model identities, and the whole route a reorder takes through
its own view — and a golden of a list at rest — which is all a
picture can honestly say about a widget whose whole subject happens under a finger. What it
does show is what the widget **costs** a row that is not being dragged: the grip in its own
40 pixels at the trailing edge, and the row ending where the grip starts rather than
underneath it.

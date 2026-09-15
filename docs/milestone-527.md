# Milestone 527 — A list whose rows run across

Milestones 490, 517 and 518 left `ReorderableList` with one shape: rows that run down. A strip
of cards, a row of chips, the tabs of a bar read across, and the list had no way to be one. Every
*what is left* since 490 named it — *a horizontal list*.

The surprise was not the geometry. It was that **"horizontal" already meant something else**.
`ReorderAxis::Horizontal` is what `Table`'s columns declare, and it is the trait's default. To the
shell it named a whole gesture: the ghost rises a notch and follows the pointer along x, the
neighbours slide under a spring that tracks the pointer itself, and the drop takes the place of
the column under the finger. There is no insertion line, no *before or after*, and no nearest slot
past the end. Turning a list's rows to `Horizontal` would have handed them a table's gesture.

## What the reference does

Its list has one gesture and an axis. Everything that measures — where the carried item starts,
how far it extends, which half of a neighbour it is over, where the gap opens — reads the start
and the extent along the scroll direction and nothing else, so a horizontal list is the vertical
one with the coordinates exchanged. Its proxy is kept in its lane: a drag update is restricted to
the list's axis before it moves the item, whichever axis that is. Its auto-scroll is the same one
on both axes.

A reading direction enters only through the scroll direction. A horizontal list under a
right-to-left script scrolls from the right, which it treats as a reversed axis: the first item
is on the right, and the half of a neighbour that means *after* is the left one.

## The decision

**The axis and the gesture are two questions.** `Widget::reorder_inserts` asks the second: is a
horizontal reorderable dropped *between* its neighbours, like a list's row, or *into the place* of
the one under the pointer, like a table's column? `false` by default, which is the table, so a
table that answers nothing is exactly what it was. Every vertical reorderable already inserts and
is not asked. In the shell the two answers become one private `ReorderMotion` — `Columns`, or
`Slots(axis)` — and every branch that used to match on the axis matches on that instead, so the
vertical path is the same code it was with its axis spelled out. `Keyed` and the other transparent
wrappers forward the new hook, and the test that reads the trait's source makes sure of it.

**The list turns as a whole.** `ReorderableList::axis(ReorderAxis::Horizontal)` lays the rows out
along x, and each row puts its grip *across* the list: beside the row in a list that runs down,
under it in one that runs across, keeping its depth (`handle_width` is now documented as a depth).
The grip's middle stays inside its row, which is how the shell finds what a grab moves on either
axis.

**Turning the row was not enough, and the first strip was a line of grips.** A row's content
was wrapped in `Expanded` — a basis of nothing, grown into the width the grip leaves — which is
right beside a grip and wrong above one: stacked in a column nobody gave a height to, content
that starts at nothing grows into nothing. The layout test found it (every row 40 px tall, all
of it grip) and a held strip was worse, 0 px tall. The wrapper is now the list's own
`ReorderContent`: down a list it builds `Expanded`'s box exactly — the same function builds
both — and across one it starts the row at its own height and grows it only into a height the
list was given. Like the rest of the spec it reads the axis at layout time, so an `.axis()`
after the rows still turns them.

**The pure pieces take the axis.** `reflow_reorder_cards` and `nearest_reorder_slot` read *along*
and *across* through two small helpers instead of `y` and `x`; both gained an argument, which is
the one breaking change, and `ReorderAxis::Vertical` gives what they gave. The board's background
guard — a block more than one and a half cards tall is a column, not a card — stays vertical only:
there is no horizontal board, and in a row of chips a neighbour wider than one and a half of the
lifted chip is a chip, whose label would slide out of it if it stood still.

**Which half means after** is `reorder_drop_after`, and the insertion line and the release both
ask it, so the line cannot show one side of a row while the release keeps the other. Down a list,
the lower half, in either reading direction. Across one, the right half — **and the left one under
right-to-left**, where the layout has already mirrored the strip so that its first row is on the
right. *Past the end* needed nothing: the nearest slot is a question of distance, and past a
mirrored strip's left edge the nearest slot is still its last. Nothing is asked of the application;
the reading direction is the theme's, the same one the layout mirrored the strip by.

**The ghost follows the finger along x only**, the reference's lane. A vertical list's ghost keeps
following in both directions, because a vertical list and a board are one path in the shell and a
board carries a card across columns; that is unchanged.

**The line springs along x.** A vertical line's ordinate chases the chosen edge; a horizontal
line's abscissa does, starting from the pointer as the vertical one starts from it. Auto-scroll
needed nothing: it has measured the carried box against both edges of the area under the pointer
since milestone 490, and 517's *the press follows the content* is written for both axes.

## Verification

**Tests, the pure pieces along both axes.** `reflow_reorder_cards` across a strip: the gap
closing along x and nothing moving along y, a chip wider than one and a half of the lifted one
making room with the rest, a card below the strip left alone, the line opening the slot, and a
mirrored strip opening it on the left. `nearest_reorder_slot` across: past the right end however
far, before the left, a gap split at its middle, above and below the strip no slot and its edges
still in it — and the same point read down the list beside it, so the axis is what decides. The
vertical cases are the ones 518 wrote, with the axis spelled out. `reorder_drop_after`: the lower
half down a list in either reading direction, the right half across one, the left half right to
left, and no half in a slot with no extent. In the shell: the line standing on its end at the
edge the row goes to, both directions; which gesture a reorderable gets from its two answers,
with a table's defaults kept and the real rows answering down and across; and the ghost's offset
per gesture.

**Tests, on built frames.** A strip lays its rows out side by side with each grip under its row,
40 px deep and inside it; a held strip's rows keep their height and a strip given a height hands
what the grip leaves to them; every row and grip answers *across* and *inserts*, including after
an `.axis()` that came last. Then the vertical tests' horizontal twins, asked in the order the
shell asks: grab a grip, find the row under its middle, take the row's list, find nothing to drop
on past the end, fall back to the nearest slot along x, take the half, route the message — left to
right and right to left, where past the end is past the left edge — and the reflow of the real
frame, where everything the second and third rows paint moves along x by the lifted row's width
and nothing moves along y. The vertical list's golden still matches.

**Mutations**, nineteen, each applied alone, tested, and taken back, with the tree's diff hashed
before and after the run to show nothing was left behind. Fifteen fail a test. In
`frus-widgets`: the transparent wrappers no longer forwarding `reorder_inserts`, which the test
that reads the trait's source catches; *along* reading y across a strip; the horizontal reflow
moving along y; the board's background guard applied across a strip too; the nearest slot
reading the point as if the list ran down; *after* ignoring the reading direction; the row
stacking its content and grip side by side; the grip keeping its depth as a width; the list
laying its rows out in a column; the content keeping `Expanded`'s zero basis across; a row that
does not say it inserts; and a row that always says it runs down. Killed in the shell: a horizontal reorderable that never inserts,
a row's ghost following the finger in both directions, and the line ignoring the reading
direction. **Four survive, and all four are the shell's wiring**: the drop's fallback measured
down a horizontal list, the release ignoring the reading direction, the line springing towards
the pointer instead of the slot's edge, and the preview reflowing a strip along y. They live
where the frame loop meets the tree, which no harness drives — 517 and 518 left theirs in the same
place — and the device run below is what would check them; the built-frame tests above ask each
piece those lines call, with the arguments those lines pass.

**Commands**, in this worktree: `cargo test -p frus-widgets --lib` (1596 passed),
`cargo test -p frus-shell --lib` (107), `cargo test -p frus-demo --lib` (58), the vertical list's
golden in `frus-test`, `cargo clippy -p frus-widgets -p frus-shell -p frus-demo -p frus-test
--all-targets -- -D warnings`, `cargo fmt --all -- --check`, and
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p frus-widgets -p frus-shell`, all clean.

## What is left

- **The device run.** No phone was attached. This repository does not believe green tests about a
  gesture that only exists while something is held — 517 is the reason — so nothing here has been
  seen under a finger yet: a strip carried to its right and left edges and held, the line and the
  gap along x, a release past either end, and the same in a right-to-left locale. The demo now
  has a strip to try it on: ten labels across the top of the Kanban board screen (Menu, then
  *Kanban board*), wider than a phone and reordered in the model by `MoveLabel`.
- **A strip that reads against its theme.** The direction the release reads is the theme's. A
  `Row` or a `Column` can run against it with `text_direction`, but only for its own children, and
  a strip's rows are the list's own children, so today the layout and the release always agree. A
  `text_direction` on the list itself would have to reach both.
- **Keyboard reordering**, `Kanban` on the list machinery and a **proxy decorator** — as 490, 517
  and 518 left them. A horizontal list's keys would be Ctrl+Left/Right, which `Key` does have.

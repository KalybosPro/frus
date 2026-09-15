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

## Seen on the phone, and fixed

A release build of the demo on the Huawei (1080 × 2340 px, density about 2.75), driven with
`adb shell input motionevent`: a press on the strip's Feature label, a hold of 0.9 s, then moves
along x a tenth of a second apart, a screenshot, and the release.

- **During the drag** no label looked lifted — no ghost, no gap, no line — and the strip had
  scrolled. The board below looked scrolled too, and its Doing column showed Research above
  Build widget.
- **After the release** the labels were in their old order, the strip stayed scrolled, the board
  was back at its start, and Research was at the top of Doing.
- **A second attempt**, held on Design and moved further in one step, did nothing visible but
  bring the strip back to its start.

### What the shell did

**A carried row's box came from the frame, and the frame loses it at an edge.** Everything a
reorder draws and measures — the ghost, the auto-scroll, the gap that closes behind the row —
starts from the row's box, and the shell asked the frame for it every frame. The frame's
registry keeps a reorderable's box only as far as it shows, clipped to its scroll's viewport,
and keeps nothing at all for one scrolled out of sight. Since milestone 517 the *press* goes with
the content while a list auto-scrolls, so that the ghost stays under the finger. The box did
not: the moment a carried row reached the strip's edge, the content moving under it clipped its
box, the clipped box stood still while the press moved on, and the ghost — drawn at the box plus
the finger's travel — ran ahead of the finger. The ghost now hung further past the edge, so the
strip scrolled faster; within a fifth of a second the row was scrolled out of the frame, the
reorder had no box, and the preview drew nothing. The release then dropped the label where the
runaway strip had put a slot under the finger.

Measured on the demo's own screen at the phone's logical size, through the shell: Feature held
and carried to x = 370 took the strip to an offset of 100 in a tenth of a second, a ghost 7 px
wide and 62 px ahead of the finger six frames later, no source at all six frames after that
with the strip stopped at 233, and a release that put Feature fifth. **A list that runs down and
a board's cards have the same edge**: row 0 of a long list carried to the bottom is lost as soon
as its own place scrolls off the top, and a card carried to the board's right edge is lost as
soon as its column does. Milestone 517's test of the press following its content assumed the
row's box moved with the content, which is what the frame does not do.

That is the part of the report that reproduces, and it accounts for a label that was not drawn
lifted and a strip that scrolled away. **The rest did not reproduce**, and is left open rather
than explained away:

- Through the shell's own input path and a frame in the real frame's order, on the demo, at the
  phone's logical size, with the injected coordinates divided by 2.75 and the same timing — with
  and without phone-sized bars, straight after the push and after coming through the drawer —
  the hold lifts Feature, the carry moves it and the release sends `MoveLabel(1, 2)`: a label
  moves, no card does, and neither region scrolls. At those coordinates the ghost's right edge
  stops at 375, short of the strip's 392.7, so the runaway above is not reached either.
- The frame never has a card under a press on the strip: at every board offset tried, the only
  reorderables at the press and release points are the strip's rows, and the Kanban cards'
  flat indices sharing numbers with the rows' indices changes nothing, since each is read only
  from its own widget. No two widgets of the Board screen share an identity (135 of 135 distinct).
- Replaying the same pixel sequence at every total scale from 1.5 to 4.0 in steps of 0.05, with and
  without a 99-px status bar, never moved Research: below 1.6 the press lands on a card of the
  board and that card moves; from about 3.05 the carried label reaches the edge and runs away as
  described.
- The demo's density setting is not persisted, and the Kanban is not either, so a fresh launch
  starts from the seed and a scale of 2.75.

So either the phone's surface was not the one the coordinates were converted with, or something
happened on the device that no replay here performs. The device run below is what separates the
two.

### The fix

**What is carried keeps its own box.** `Drag::Reorder` holds the row it carries and that row's
box, taken from the frame once — the first frame the drag is carried in, before anything has
scrolled — and from then on moved with the content by the same step that moves the press, as a
lifted item's box already was. The ghost and the auto-scroll (`carried_rect`), the preview (the
row lifted out, the gap behind it, the ghost's content) and the release's fallback to the
nearest slot all read that box and that row. The frame is still asked for everything else: what
is under the finger, the slots of the list, the target's half. The reorder springs moved out of
the frame into one function the frame and the driver below both call.

### A driver for the shell

No test drove the shell's event loop, which is why three device runs in a row found what green
tests could not. The shell now has a windowless driver, `frus_shell::testing::Driver`, behind a
hidden `testing` feature the demo turns on for its tests: an application on a stated surface, fed
presses, moves, releases and the long-press deadline through the shell's own `pointer_event` and
`hold_deadline_reached` (split out of the loop's handlers unchanged), and frames that take the real
frame's steps in the real frame's order, less the GPU. The shell's message channel became
optional for it, since an event loop cannot be built on a machine with no display. The frame it
runs is still a copy of those steps, not the frame itself; what the two share is what the fix
touched, and the rest of the loop remains unexercised.

### Verification

**Tests**, through the driver, on the Kanban screen's shape (a bar, ten 96-px labels that lift on
a hold, a board of rich cards) and on a list of twenty 60-px rows:

- `a_label_carried_past_the_strips_edge_stays_under_the_finger` — **the reproduction**: held on
  Feature and carried to x = 370, then held for a second and a half, frame by frame the carried
  box keeps its place under the finger and its full size, the ghost is drawn there, the strip
  only scrolls on, reaching its end; the release past the last label sends `Label(1, 9)`. Failed
  before the fix at frame 1: *carried at x 348, 90.7 wide, the finger at 370*.
- `a_card_carried_to_the_boards_edge_stays_under_the_finger` — the same on the board. Failed
  before the fix: the card was lost before the carry ended.
- `a_row_carried_past_the_bottom_of_a_long_list_stays_under_the_finger` — the same down a list
  that runs down, and the release lands near the end. Failed before the fix at frame 0: *nothing
  is carried any more*.
- `a_hold_on_a_label_then_a_carry_along_x_reorders_the_labels_and_moves_no_card` — the phone's
  own sequence: `Label(1, 2)`, the cards as they were, neither region scrolled. Passed before.
- `a_card_carried_across_the_board_moves_the_card` — Design API onto Build widget's upper half:
  `Card(0, 0, 1, 0)`, no label moved. Passed before.
- In the demo, `the_board_strip_carried_through_the_shell_moves_a_label_and_no_card` — the demo
  itself through the driver, its strip gripped as a desktop grips it: Feature carried to Design's
  right half moves Feature and no card.
- 517's `a_carried_item_follows_its_content` now carries the box and checks it went with the
  content.

**Mutations**, five, each applied alone to the fixed tree and restored after, with the hash of the
whole diff checked equal before and after every run:

- `follow_content` no longer moving the carried box: killed by four tests.
- The box never recorded, so every frame asks the frame again: killed by the three edge tests.
- The release handing the drop no carried row: killed by the reproduction, whose release is past
  the last label with Feature long scrolled away.
- `carried_rect` read from the frame: killed by the three edge tests.
- The preview drawn at the frame's box: **survived** the first run — nothing checked where the
  ghost is drawn, which is the symptom the phone showed. The driver now records the ghost's
  outline and the held frames check it against the carried box; run again, killed by the three
  edge tests.

**Commands**, in this worktree, all clean: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --all -- --check`, `cargo test -p frus-widgets --lib` (1604 passed),
`cargo test -p frus-shell --lib` (121), `cargo test -p frus-demo --lib` (64), the reorderable
list's golden in `frus-test`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p frus-widgets -p
frus-shell`, and `cargo clippy -p frus-shell --target aarch64-linux-android --no-deps -- -D
warnings`.

### On the device, after the fix

The Huawei STK-L21, a release build of this fix with milestone 531 under it, the demo's Kanban
screen, injected the same way. **Feature** pressed and held 0.9 s was lifted, outlined. Carried
along x to between Design and Docs, it followed the finger and Design moved left into the gap it
left, the board below untouched. Carried on to the right edge and held a second, the strip
scrolled to its end — Ops, Tests, Release — with Feature still under the finger and a gap after
Release. Released there, Feature was last, and no card of the board had moved.

Not tried on the device: the left edge, a right-to-left locale, a row of the home list or a card
carried to an edge, and the insertion line looked for on its own.

## What is left

- **Research moving to Doing, and the board scrolling**, from a gesture on the strip, seen once on
  the build before this fix and 531's frame fix under it. Not reproduced here at any scale, and
  not seen again on the device after both fixes.
- **A row partly clipped when it is lifted** keeps that clipped box as its size: a label half off
  the strip's edge, held there, is carried at the width that showed.
- **The rest of the device run**: the strip's left edge, a release past the start, and a
  right-to-left locale. The demo's strip is ten labels across the top of the Kanban board screen
  (Menu, then *Kanban board*), reordered in the model by `MoveLabel`.
- **A strip that reads against its theme.** The direction the release reads is the theme's. A
  `Row` or a `Column` can run against it with `text_direction`, but only for its own children, and
  a strip's rows are the list's own children, so today the layout and the release always agree. A
  `text_direction` on the list itself would have to reach both.
- **Keyboard reordering**, `Kanban` on the list machinery and a **proxy decorator** — as 490, 517
  and 518 left them. A horizontal list's keys would be Ctrl+Left/Right, which `Key` does have.

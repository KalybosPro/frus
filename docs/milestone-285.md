# Milestone 285 — Drag and drop, and three gestures on one row

Reordering, which `Table` and `Kanban` already do, answers a narrow question: *where
in this list?* Its answer is an index, and both ends of the gesture belong to the same
widget. Drag and drop answers a wider one: *which thing, onto which other thing?* The
two ends are unrelated widgets that need not know of each other.

So what travels between them is a **payload** the application chooses — a `u64` it can
map back to whatever it means. `Draggable::payload(id)` on one end,
`DragTarget::on_drop(|payload| …)` on the other, and nothing in between has to know
what an `id` is.

## The gesture nobody else can have

A draggable inside a list is the interesting case, because the same finger on the same
row already means two other things: sideways dismisses it, up and down scrolls the
list.

The rule chosen is that **a draggable yields to a scrollable underneath it**. A widget
that took every drag inside a list would silently stop that list scrolling, and a list
that does not scroll is a worse bug than an item that does not lift — the gesture that
broke would be the one a user makes a hundred times an hour.

That leaves the hold. `Draggable::long_press()` lifts on a **long press**, which is the
one signal a scroll cannot claim: a finger that stays still is not scrolling. So the
demo's task rows now carry three gestures, told apart by *what the finger does* rather
than by what is on top:

| | |
|---|---|
| sideways | dismiss the row |
| up and down | scroll the list |
| hold, then anywhere | lift the row and carry it |

## One hold, one meaning

That hold was already spoken for. The demo's rows had `on_long_press(DeleteTodo)`, and
the shell dutifully served **both** claims: on the device, holding a row deleted it
*and* lifted a ghost of it. The first run of this feature deleted the task it was
supposed to be carrying.

Two claims on one gesture have to be arbitrated, not both honoured. **The lift wins**:
it changes what the rest of the gesture means, whereas the message is a discrete action
on something the finger is still holding. The message is dropped, and a widget that
wants both has to pick one. The demo picked the lift; deleting is the × or a swipe.

## What floats under the finger

The ghost is the item's **own primitives**, lifted out of the frame by owner and drawn
translated — not a second widget the application builds. A rebuilt "feedback" widget is
a second definition of the same thing, and two definitions is one too many; this one
cannot drift from what is on screen. The reorder ghost already worked this way, and
`draw_ghost_card` is now shared with it.

What is left behind is faded rather than removed, because the item is being *carried*,
not deleted.

The target's highlight is the **target's own paint**, driven by a new
`Status::drag_over` — the shell decides which target, the widget decides what that
looks like. A target that refuses the payload is never highlighted, so the answer is
visible before the finger lifts rather than after.

## The bug the device found, again — the mirror of milestone 282

Wrapping the demo's rows in `Draggable` made them **vanish**: the counter still said
two tasks, the progress bar still moved, and the list was empty.

`Dismissible` is a layout **leaf** — a stack, whose layers are laid out separately. A
leaf contributes no content size. Wrapped in an ordinary container, it became that
container's main-axis child with an `Auto` basis, so the basis resolved to its content
size, which is zero. The row was laid out 0 px wide and drew nothing. Silently, since
neither widget paints a box of its own.

This is milestone 282's lesson seen from the other side. There, `Keyed` — a
*transparent* wrapper — answered the structural questions for itself instead of
forwarding them. Here, `Draggable` — a *nesting* wrapper — did the same thing by
omission. Both fail the same way and for the same reason:

> A wrapper must forward the questions that decide **how its content is treated**,
> whether it replaces its child in the tree or wraps it. `stack`, `continuous`,
> `draws_own_focus`, `repaint_boundary`.

Fixed by forwarding `stack()` and `continuous()` from the child, with a regression test
that wraps a `Dismissible` and checks it still paints — and a second one checking that
a target around ordinary content still takes that content's height, which a hardcoded
`stack()` would have broken instead.

The same investigation turned up a related trap and fixed it: a **stack's layers are
now given their box** (`Constraints::filled`, milestone 284) rather than asked what size
they would like. "Each layer fills the box" is what the code's own comment already said;
an unsized layer used to hug its content and collapse to nothing, invisibly.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **779 tests, 0
  failures** (772 at milestone 284): 9 for the pair, including both wrapper regressions.
- `cargo build --workspace --all-targets` — OK, no new warning.
- `cargo build -p frus-hello --target wasm32-unknown-unknown` — OK.

**On a physical device** (Huawei, Android 10), the whole gesture end to end: hold a task
row → it lifts, content intact, the original left faded in place → drag it over "Mark
done" → release → the task is checked and the row returns to full opacity. The broken
layout above, and its repair, were both confirmed on screen.

**The debts from milestones 282 and 283 are cleared in the same session**, on the same
device: swipe-to-dismiss removes a row (282), and the paged view turns a panel on a
swipe, settles on exactly one, updates `on_page_changed`, and follows the picker back
the other way (283).

## What's left

- **No drag between scroll areas.** Carrying an item to a target that is off screen
  needs the scrollable under the pointer to auto-scroll at the edges. Nothing does that
  yet.
- **The payload is a `u64`.** It is enough to index anything an application holds, but
  a typed payload would catch the mistake of dropping a task on a column that expects a
  file. That wants a generic parameter the `Widget` trait cannot carry today.
- **No drop animation.** A refused drop should fly back to where it came from; today it
  simply stops being drawn.
- **`Kanban` and `Table` still have their own reorder machinery.** This pair could
  underlie both, but that is a migration, not a milestone.

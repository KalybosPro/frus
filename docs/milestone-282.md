# Milestone 282 — Swipe to dismiss, and who owns a gesture

## The interesting problem is not the swipe

Sliding a row aside is easy. The problem is that a dismissible row lives **inside a
list**, and both want the same finger.

The two gestures are indistinguishable at the moment of the press: a finger goes down on
a row, and nothing yet says whether it means to move the row sideways or the list
up. Resolving that by *who is on top* — the usual hit-test answer — gives the row every
gesture and silently kills scrolling for the whole list.

So the shell arbitrates by **direction**, once, at the moment the finger passes the drag
threshold:

```rust
if along_the_items_axis { hand the gesture to the swipe }
else                    { keep it as a scroll }
```

The loser never sees the gesture at all. That is the point: a swipe that also scrolled
the list a little would be worse than either.

Concretely, a press over a dismissible inside a scrollable prepares a `Drag::Scroll`
that *carries the candidate*. Nothing is decided until the threshold is crossed; on the
frame it is, the axis test either drops the candidate or hands the whole gesture over
— returning the scroll offset untouched, because the list never moved.

## The three acts

| | |
|---|---|
| **drag** | the item follows the finger, revealing a background behind it |
| **settle** | released: it flies out the way it was going, or slides back |
| **collapse** | once out, its height shrinks to nothing, and *then* the message goes |

The **collapse is what keeps a list from jumping**: the neighbours close the gap over
300 ms instead of teleporting into it. The message is dispatched at the end, so the
application removes the row only once the hole it leaves has already closed — the one
ordering that makes the animation possible at all, since an application that removed the
row on release would have nothing left to animate.

That is also why a collapsing item is the one thing the runtime keeps advancing after
its widget has left the tree. An application that removes the row early still gets its
message; otherwise the message that tells it to remove the row could never arrive.

## The fling test that matters

A release counts as a fling only if it is fast enough (700 px/s) **and** beats the other
axis by a clear margin (400 px/s). Without the second half, a hurried scroll — fast, and
never perfectly vertical — throws rows out of the list. The margin is what makes a
diagonal not a swipe.

A flick the other way wins over the drag so far: dragged 45 % towards one side and
flicked back, the row leaves the way it was flicked. The velocity is the more recent
statement of intent.

A one-way item (`DismissAxis::ToEnd`) **refuses** the wrong direction outright rather
than letting it move and snapping back — nothing moves, so nothing has to be undone.

## The bug the device found, again

Wrapping the demo's task rows in `Dismissible` broke the whole card: rows, progress bar
and buttons all piled onto one line.

`Dismissible` overlays its backgrounds under its child, which makes it a stack — and a
stack is a layout **leaf**, its layers laid out separately at its own box. But the rows
are wrapped in `Keyed`, and **`Keyed` never forwarded `stack()`**. So the layout asked
the wrapper, was told "not a stack", and laid the three layers out in flow.

`Keyed` calls itself a transparent wrapper, and it forwards some forty methods — but not
the four *structural* ones the layout and the walk ask **before** they look at a widget's
children: `stack`, `continuous`, `draws_own_focus`, `repaint_boundary`. `Responsive`,
the other transparent wrapper, forwards three of them. This has been wrong since `Keyed`
existed: any keyed stack has been laid out in flow, and any keyed continuously-animating
widget has been quietly dropping frames.

> The generalisation: a wrapper that claims to be transparent has to forward the
> questions that decide **how its content is treated**, not only the ones that describe
> it. The first kind fails silently.

Fixed, with a regression test that keys a stack and a spinner and checks both answers
survive the wrapper.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **743 tests, 0
  failures** (729 at milestone 281): 13 for the swipe state machine, 1 for the `Keyed`
  regression.
- `cargo build --workspace --all-targets` — OK, no new warning.

**On a physical device** (Huawei STK-L21, Android 10): the broken layout above, and its
repair after the `Keyed` fix, were both confirmed on screen. **The swipe gesture itself
was not device-verified** — the phone disconnected before that run — so what is claimed
here for the gesture rests on the unit tests, and the on-device check is still owed.

## What's left

- **No confirmation step.** A destructive swipe that asks first — "delete?", or an undo
  window — needs the message to be able to *refuse*, which the current one-way dispatch
  cannot express.
- **No cross-axis drift.** The item flies straight out; a small vertical offset as it
  leaves reads better and is one constant away.
- **The item does not resist at the ends.** A swipe past the item's own width simply
  clamps, where a little resistance would say "this is as far as it goes".

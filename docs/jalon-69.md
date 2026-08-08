# Jalon 69 — Gestures, stages 0+1: normalised input + long press

Opening the brief's **Block B** (§3, "the biggest structural gap on the input
side"), through its first two stages — delivering a new capability along the
way: **`on_long_press`**.

## Stage 0 — normalised pointer input

The four winit sources (mouse pressed/released, cursor, 4-phase touch) converge
on **one** input: `PointerEvent { kind: Down/Move/Up/Cancel, position (logical
px), touch }` → `App::pointer()`. **`Cancel` is first-class** (the brief insists
on it: app backgrounded, gesture stolen → give up with no success callback) — it
resets dragging, pressing and the recogniser. Not throwaway: it is the base the
arena (stage 2) will plug into. *(The full `Vec<HitEntry>` hit-test path and the
multi-pointer `PointerRouter` are deferred along with the arena — so as not to
ship a dead API.)*

## Stage 1 — the tap-or-long-press recogniser (arena vocabulary)

`PressRecognizer` (frus-shell, a **pure** machine — the instants are passed as
parameters, so it is testable tick by tick):

- The **long press accepts greedily** when the delay is crossed (500 ms without
  moving): the message is emitted immediately and the following release is
  **swallowed** (the long press evicts the tap) — exactly the arena semantics.
- The **tap accepts passively**: a release before the deadline → the existing
  click path, intact.
- Movement beyond the **slop** (8 px) → the long press is rejected and the
  gesture becomes a drag/scroll again. `Cancel` → give up.
- **Precise wake-up**: `ControlFlow::WaitUntil(deadline)` arms the winit loop at
  exactly the right moment (`new_events(ResumeTimeReached)` fires the
  recogniser) — zero polling frames, consistent with the "0 CPU at rest"
  discipline.

A press captured by a bar, handle or selection does not stand as a candidate; a
touch scroll **not yet moving** stays a candidate (the slop decides).

## The API: `on_long_press`

- `Widget::on_long_press()` (a hook, delegated by `Box`/`Keyed`/`Responsive`);
  the `Container::on_long_press(msg)` builder.
- `Ui::long_press_at(point)`: the topmost target (collected like the hits,
  bounded to what is visible).
- **Demo**: a long press on a task row deletes it (the mobile idiom), alongside
  the × button.

## Validation

- **245 tests**, all green — including 5 recogniser tests (fires exactly once at
  the deadline; a tap before the deadline is not swallowed; the slop rejects; an
  uninterested target stays inert; cancel gives up) and topmost collection.
  Existing behaviours intact (clicks, drags, the back gesture, demo 15).
- A warning-free build; the demo did not panic.

## What's next (Block B)

- **Stage 2**: the real arena (pure `Arena::resolve/close/sweep` returning the
  outcomes), `PointerRouter`, the full hit-test path, multi-pointer — when
  independently scrollable nested regions demand it.
- LSQ velocity, scale/pinch: stage 3 (deferred).

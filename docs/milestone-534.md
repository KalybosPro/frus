# Milestone 534 — A press a scroll took, let go

A segment a finger swiped a strip of filters by stayed grey. The swipe did everything else
right: the strip scrolled and no filter was chosen. What it left behind was the press.

## Seen

On a phone (a Huawei, 1080 × 2340 at a density of about 2.75, a release build of the demo), on
the owner's branch that puts the home screen's filters — a segmented button, All / Active /
Done — in a horizontal `SingleChildScrollView`. With the text scale raised until the row
overflowed, a swipe was injected from the "Done" segment 190 logical pixels to the left in half
a second.

- The row scrolled.
- The selected filter did not change.
- **"Done" stayed drawn in its grey highlight**, still there in a capture two seconds later with
  no further input. It went away only when something else was pressed — a menu opened, and the
  text scale changed.

## Reproduced

The shell's `App` cannot be built in a test: it needs a window and an event-loop proxy. So the
reproduction drives a **real frame** of a segmented button six segments wide in a 300-pixel
horizontal scroll through the pieces `App::pointer` is made of, in the order it calls them —
the hover, the press where the finger lands, the touch scroll `pointer_down` prepares, the moves
the drag handler turns into an offset, the release `pointer_up` takes — and then lets two
seconds of frames go by and compares **what is painted** with the same screen, scrolled the
same, that no pointer touched.

To see it fail, the tests were run once against a stand-in for the code before the fix — the
press cleared only by a tap's release or a cancel, which is what `pointer_up` and the cancel
path did — and the new call site otherwise in place. The swipe test failed for the phone's
reason: two seconds on, the frame painted **15 primitives where the untouched one paints 14**,
the extra one the segment's splash, held. The press a scroll takes past its slop, and a plain
button a page was scrolled from, failed the same way. The tap test passed, as a tap was never
broken. The two mouse tests failed too, but only because the stand-in cannot tell a mouse click
with no gesture from a gesture that moved; they are there for the new behaviour, not the
reproduction.

## The cause

`pointer_up` (`crates/frus-shell/src/app.rs`, the release of a gesture) ends a drag that moved
by returning early:

```rust
if ended.is_some() && !was_tap {
    self.request_redraw();
    return;
}
```

and the line that clears the press on a release, `self.runtime.input.pressed = None`, comes after
it, on the tap path. (A cancel clears it, and so does a release a long press swallowed; a
release that ends a gesture reaches neither.) So a scroll that had moved left `pressed` naming
the segment the finger landed on, after the finger had gone.

The hover was not the fault: the finger's lift clears it (milestone 505), and the press status a
widget paints needs both, so no press state layer showed. **The ink** is what showed. A splash
waits, full grown, for as long as its widget is pressed — `Runtime::advance_ink` sweeps one away
only when its id is no longer `input.pressed`, which is exactly how a finger that slid off is
told apart from one still down. With `pressed` never cleared, the sweep never came. The next
press anywhere replaced `pressed`, which is why opening a menu cleared it.

Frames were not the fault either: the release asks for one, and the ink was not animating — it
was holding, as it was told to.

The press was also wrong **during** the swipe. While a drag is under way the shell does not ask
the hover again — a move goes to the drag handler before `sync_hover`, and the frame's own
re-ask is skipped — so for the whole scroll the segment was both pressed and hovered, and drew
its press layer and its splash as the strip carried it away.

## What the reference does

A tap and a scroll compete in the gesture arena from the moment the finger lands. When the
finger passes the touch slop the drag recogniser claims the gesture, the tap recogniser is
rejected, and the ink well is told the tap was cancelled: its splash is cancelled and its
pressed highlight turns off **there and then**, with the finger still down. The same happens to
a tap that loses to any drag — a dismissible's, a sheet's. A finger has no hover in the
reference at all, so nothing stays highlighted when it lifts.

## The decision

**Let go of the press after each pointer event is routed, in one place.** `pointer` now asks,
before routing, whether the gesture under way could still end as a tap
(`gesture_was_a_tap`, the list that already decides whether a release clicks), and after routing
calls `settle_press`:

- **A move** that turned a gesture which could still have been a tap into one that cannot — a
  touch scroll past its slop, the scroll handed to a swipe, a sheet, a pan, a reorder or a
  carried item that moved — lets go of the press. This is the reference's rejection, at the
  moment of arbitration.
- **A release or a cancel** always lets go of it. The release has already read the press by
  then (a tap clicks from it), so this changes nothing for a tap and closes the early return for
  everything else.
- **A press** takes nothing.

Letting go clears `input.pressed`, which the next frame's `advance_ink` turns into a swept
splash (75 ms) and `advance` into a press that fades. **For a finger it also clears the hover**:
a finger hovers only so that its press can show (milestone 505), and once the press is taken
the hover has nothing to show but a highlight the reference never draws. The shell does not ask
the hover again during a drag, so it stays clear until the finger lifts. **A mouse keeps its
hover**: it is still over what it is over.

One place rather than a line at each drag's threshold and another in `pointer_up`: there are six
gestures that share a press with a tap, and forgetting one is silent — that is how milestone 327
lost a row's taps. `gesture_was_a_tap` is already that list, so the decision reads it rather
than restating it.

The press on landing moved into `press_at`, unchanged, so the tests land a finger through the
shell's own code rather than a copy of it.

**Not done:** a tap cancel message. The reference's ink well has an `onTapCancel`; nothing here
exposes one, and nothing asked for it.

## Verification

**Tests** — six, in `press_tests` in `crates/frus-shell/src/app.rs`. The first four drive a real
frame as described above and compare what is painted:

- `a_segment_a_swipe_started_on_is_not_left_pressed` — the phone's gesture: a finger lands on a
  segment of a strip wider than the screen, travels 190 to the left in nineteen moves and lifts.
  The gesture clicks nothing, and two seconds on the frame paints exactly what an untouched one
  at the same offset paints, no ink is held, and nothing is pressed or hovered.
- `the_press_ends_when_the_scroll_takes_it` — under the slop the segment is still pressed; past
  it, with the finger still down, the press and the finger's hover are gone, and half a second
  later nothing under the finger is highlighted.
- `a_button_a_page_scrolled_from_is_not_left_pressed` — the same down a vertical page, from a
  plain `Button`.
- `a_tap_still_presses_and_clicks` — a finger that wobbles inside the slop keeps its press and
  its hover; at the release it is still on the segment it pressed, which has a message to send;
  the gesture ends as a tap; and after the release the press is let go of.
- `a_mouse_keeps_its_press_while_it_moves_and_its_hover_after` — a mouse prepares no touch
  scroll, keeps its press while it moves across the strip, and still hovers the strip after its
  button comes up.
- `a_gesture_takes_a_mouse_s_press_and_leaves_its_hover` — `settle_press` alone: a sheet dragged
  past its slop takes the press from a mouse and a finger alike, the hover only from the finger;
  a gesture that was never a tap, a text selection, lets go of nothing.

What the tests cannot reach is the call site itself, one line in `App::pointer`, since `App`
cannot be built without an event loop. It is the line the tests stand in for, called with the
same arguments in the same place.

**Checks:**

- `cargo test -p frus-shell --lib` — 114 passed, the six above among them
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test -p frus-widgets --lib` — 1619 passed
- `cargo test -p frus-demo --lib` — 61 passed
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` — clean
- No `cfg(android)` code was touched, so the Android clippy was not needed.

**Mutations** — six, in `settle_press`, applied by a script that checked each target occurred
once, ran `press_tests`, wrote the file's original bytes back and compared the working tree's
diff hash with the one it started from. They ran one at a time, after every other build had
finished, and each was restored.

1. **A move past the slop keeps the press** — killed by `the_press_ends_when_the_scroll_takes_it`
   and the mouse gesture test. The swipe test alone would not have caught it: the release still
   lets go, and two seconds later the ink is gone. That is why the press is also checked with the
   finger still down.
2. **A release keeps the press** — killed by the tap test and the mouse test. The two scroll tests
   pass it, the move having let go already; it is the release of a tap, and of a mouse with no
   gesture, that this arm closes.
3. **A finger keeps its hover when its press is taken** — killed by
   `the_press_ends_when_the_scroll_takes_it` and the gesture test.
4. **A mouse loses its hover too** — killed by the gesture test alone. The mouse test passes it,
   and would in the shell too: a released mouse's hover is asked again straight after, by
   `sync_hover`. Where it would show is during a mouse's gesture, a sheet dragged by its panel,
   which only the unit test drives.
5. **Any move that is not a tap takes the press, shared or not** — killed by the gesture test and
   the mouse test: a mouse held down and moved, with no gesture at all, would lose its press.
6. **A landing takes the press** — killed by five of the six tests, every one that presses
   something first.

All six were killed on the first run, and the diff hash after the last matched the one before
the first.

## On the device — the Huawei STK-L21, a release build of the demo

The filter row overflowed at a raised text scale, and `adb shell input swipe 820 1210 290 1210
500`, starting on "Done", scrolled it to show "Done" in full without changing the filter. A
capture straight after showed **no segment grey**, and another two seconds later still showed
none — exactly steps 1 and 2 of the check this note asked for.

Not tried on the phone: holding the finger mid-gesture to see "Done" un-highlighted before the
release (step 3), a plain tap still pressing and selecting (step 4), and the same swipe started
on a task row's control (step 5). Left for whoever picks those up next.

The full check, for reference:

1. `adb shell input swipe 820 1210 290 1210 500`, starting on "Done". The row scrolls and the
   filter does not change, as before.
2. A capture straight after, and another two seconds later: **no segment is grey** in either.
3. Hold a finger on "Done" with `adb shell input motionevent DOWN 820 1210`, move it left past
   the slop with `MOVE 700 1210`, and capture **before** `UP`: "Done" is not highlighted while
   the finger is still down.
4. A plain tap on "Active": it still shows its splash and still selects the filter.
5. The same swipe down the task list, starting on a row's control: nothing stays highlighted.

## What is left

- **The call site is untested.** The pieces are, and the line joining them is not, for the
  reason above; a shell that could be driven without a window would close it, and would close it
  for every other gesture bug this file has had.
- **A finger hovers while it is down**, on a tap too, which the reference never does. The press
  needs it here (a press shows only while its widget is also hovered), so taking it away is a
  change to how a press is shown, not to this bug.
- **No tap-cancel message** for an application, as above.
- **The demo's scrollable filters** are the owner's branch, not this one; this milestone changed
  no demo code.

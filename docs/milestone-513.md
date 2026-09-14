# Milestone 513 — A letter a second late, and a caret that never blinked

Reported by the owner, typing into the demo on a phone: a letter pressed took a moment to
appear in the field, and the caret sat still while nothing was typed — "not like a real
field". Two faults, one of them a fault of the event loop rather than of the field.

## A letter a second late

### Measured, not guessed

A temporary trace timed three moments for each operation the keyboard sent: when the
bridge received it on the Java thread, when the shell applied it to the field, and when the
frame showing it was presented. On a Huawei STK-L21 with SwiftKey, four letters typed on
the keyboard's own keys:

| received → applied | received → on screen | the frame's render |
|---|---|---|
| 801 ms | 820 ms | 12.8 ms |
| 424 ms | 443 ms | 12.4 ms |
| 657 ms | 680 ms | 15.7 ms |
| 835 ms | 854 ms | 12.2 ms |

The frame was never the problem: under twenty milliseconds from applying a letter to
showing it, and the surface already presented in `Mailbox`, the lowest-latency mode the
device offers. Everything was spent **before** the shell even looked at the letter. And the
four letters were all applied at the same fraction of a second — `.819`, `.819`, `.820`,
`.821` — which is the demo's stopwatch, redrawing once a second.

### Why

The bridge queued each operation and woke the loop with the looper's own waker. winit's
Android backend takes a wake with **no redraw requested and no message of its own queued**
for a false one, and goes straight back to sleep (`platform_impl/android`, the
`PollEvent::Wake` arm) — sensibly, since false wakes would otherwise spin the loop. So the
letter waited in the queue for whatever woke the loop next. In the demo that was the
stopwatch, so up to a second; on a screen with nothing ticking it could have been until the
next touch. The autofill values of milestone 512 arrived through the same wake, and waited
the same way.

### The fix

The bridge now wakes the loop by **asking the window for a frame** — `Window::request_redraw`,
which winit allows from any thread and does honour — and falls back to the bare wake only
while there is no window. The frame is wanted anyway: it is what shows the letter. The shell
hands the bridge the window when the surface is created and takes it away when it is lost.

## A caret that never blinked

Nothing blinked it: a focused field painted its caret on every frame, and nothing asked
for a frame to take it away.

The rule is the reference's: **half a second shown, half a second hidden**
(`editable_text.dart:109`), and every change to the field — a letter typed, the caret moved,
a selection made — **starts it again from shown** (`:3718`, `:4552`), so a caret never
blinks out under a finger that is typing.

- **On the wall clock.** The animation clock is clamped per frame and stands still between
  frames, and a caret at rest is exactly when there are none; so the blink keeps an instant
  of its own (`crate::caret`, pure and tested).
- **A frame at each turn, and none in between.** The loop's idle policy now wakes at the
  nearest of the long press, the live-reload poll and the caret's next turn, and a wake
  that finds the turn due asks for a frame. A field at rest costs two frames a second, not
  sixty.
- **What restarts it** is observed rather than wired into every path that edits: each frame
  compares the focused field, its caret and selection and a digest of its value with the
  last frame's. Typing, the keyboard's composition, a handle dragged, an undo, a value the
  application changed — all of them restart the blink without any of them knowing it exists.
- **Only while the window is in front.** An application in the background, or behind
  another window, keeps a solid caret and wakes for nothing.
- **Tests and pictures are unchanged.** The flag is `caret_hidden`, false wherever nothing
  blinks it — every test harness and every golden — so a caret still shows in all of them.

## Verification

**On the device, after both.** The same phone, the same keyboard, the same four letters,
with the trace still in:

| received → applied | received → on screen | the frame's render |
|---|---|---|
| 0.0 ms | 9.0 ms | 5.9 ms |
| 0.3 ms | 10.6 ms | 4.9 ms |
| 0.3 ms | 10.9 ms | 5.4 ms |

The fourth letter was on screen 1.5 ms after its wake, in a frame the trace did not also
see it applied in; the field read `Rust`. The trace has been removed.

Then three captures taken back to back with the field at rest: the caret absent from the
first and back in the next two — the blink, on the device.

**Tests.** Four of the blink's rule, in `crate::caret`: shown then hidden each half second;
a change starts it again shown, and the same field looked at again unchanged does not; with
no caret nothing is hidden and nothing wakes; the loop wakes at the next turn. One of the
field: a hidden caret is not painted.

The wake itself has no test, and cannot have one here: what it corrects is winit's reading
of a looper wake, on Android. The measurement above is its verification.

**Mutations**, six, each failing the test meant for it: hidden in the first half instead of
the second; a quarter-second period; a change not restarting the blink; a wake scheduled
with no caret; the wake one turn late; the paint ignoring the flag.

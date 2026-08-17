# Milestone 327 — A swipe that never started is still a tap

The orange item on the roadmap, found on a device during milestone 324's pass: a task
row's avatar opens the task on paper, and tapping it dead centre — on two different rows —
navigated nowhere. A tap on the filter segment a few hundred pixels away in the same card
worked immediately, so presses were not broken in general.

## Ruling out the hit box

The roadmap offered two candidates, and one of them is cheap to settle. A probe swept the
whole window at the device's size, asking `Ui::hit` for every point and reading the message
back off it:

```
OpenTask reachable over x 58..86, y 480..508 (225 points)
```

Thirty pixels by thirty, exactly where the avatar is drawn. The target is registered, it is
topmost, and the point the finger landed on resolves to it. The hit box was never the
problem, and the second candidate — the gesture arena — was the whole of it.

## What it was

A press on a dismissible row prepares a swipe, in case the finger is about to slide
sideways. It engages only past `TOUCH_SLOP`, and the code that sets it up says so:

> It still waits for the threshold, or a tap on the row would start sliding it.

The release, though, tells a tap from a drag by matching the gesture that ended against a
list of the ones that engage at a threshold — and the list had four of the five:

```rust
let was_tap = matches!(
    ended,
    Some(Drag::Scroll { moved: false, .. })
        | Some(Drag::Pan { moved: false, .. })
        | Some(Drag::Reorder { moved: false, .. })
        | Some(Drag::Item { moved: false, .. })
);
```

`Drag::Dismiss` is missing. A press that never moved therefore counted as a drag that had
been dealt with, `pointer_up` returned before the click path, and the widget under the
finger was never told anything. The row stayed hittable, the press still recorded it, and
only the release quietly did nothing — which is why this reads as "that one control is
broken" rather than as a whole subsystem being off.

## Why only sometimes

Two branches can claim a press on a row like this, and which one runs decides whether the
bug shows:

- **A finger on a list that actually scrolls** takes `Drag::Scroll`, carrying the
  dismissible along as a candidate for the arbitration at the threshold. `Scroll` *is* in
  the list, so the tap goes through.
- **A finger on a list that does not scroll** — the content fits, so the area refuses the
  offset outright — falls to the dismissible branch, and the tap is swallowed.

The probe confirms both halves at the device's own size: with one task and with three,
`scroll_accepts=false`; with twelve, `scroll_accepts=true`. The device was carrying a short
list, which is why the bug arrived looking intermittent.

With a **pointer** the touch-scroll branch does not run at all — it is guarded on `touch` —
so a mouse always reaches the dismissible branch. Reading the routing, every click on a
dismissible row was swallowed on desktop, whatever the list was doing. That half is what
the code path says rather than something a device showed; the device confirms the touch
half.

## The fix

The list becomes a named function with the fifth variant in it, and a comment saying why it
is one list:

```rust
fn gesture_was_a_tap(ended: Option<&Drag>) -> bool
```

Naming it is most of the value. The bug was not a wrong decision, it was an enumeration
that had to be kept in step with an enum and was not, in a `matches!` buried in the middle
of a 250-line function. `a_swipe_that_never_started_is_still_a_tap` pins both directions —
a swipe under the threshold leaves the click alone, one that ran must not also click — and
keeps a gesture that captures on the press, `TextSelect`, on the other side of the line.
It fails without the fix.

## On the device

The reported gesture, on the reported hardware, in the state that reproduces it — a
release build on the XMJNW19B23011768, carrying two tasks, a list too short to scroll:

- **The avatar opens the task.** The tap that navigated nowhere now navigates, and the
  hero flies: the row's 30 px circle becomes the large centred one on the task's screen.
- **The checkbox works too.** It had been dead for the same reason, which nobody had
  reported because nobody had tried it — the bug was never about the avatar, it was about
  the row. That the reported symptom was the narrower one is worth remembering.
- **A swipe under the threshold springs back and does not navigate**, starting on the
  avatar. That is the other half of the guard, on hardware: a gesture that ran is not also
  a click.
- **A swipe past the threshold still dismisses.** The row leaves, and it does not navigate
  on the way out.

Milestone 321's centring, which this bug blocked, is verified in passing: on the task's
screen the avatar, the title and the state label are centred on the content box, and the
footer is pinned to the bottom.

### Found while doing it

Restoring the demo's tasks through the soft keyboard produced a very long label, and it
**pushed the row's delete button out of the card**. The row is `[avatar, checkbox, label,
spacer, ×]` with nothing bounding the label, so it grows past the spacer and evicts the
trailing action instead of ellipsising. Read one way that is the demo's to fix — the
reference gives such a title an expanding box. Read another, a row that silently drops its
trailing control rather than clipping is worth a framework answer. Recorded on the roadmap
rather than fixed here; this milestone is about the gesture.

## Left

- **Nothing drives a press end to end.** The predicate has a test; the routing that feeds it
  does not, because `App` needs a window. This bug lived in exactly that gap: every piece
  was individually right and the seam between them was not. A headless harness over
  `pointer_down`/`pointer_up` is the thing that would have caught it, and it is a real piece
  of work rather than a line.
- **The arbitration reads differently for a pointer and a finger.** That is deliberate —
  there is no touch scroll to lose to with a mouse — but it means the two paths reach the
  dismissible branch under different conditions, and only one of them is exercised by
  anything.
- **Milestone 321's device check of the task screen's centring**, which this bug blocked, is
  now reachable.

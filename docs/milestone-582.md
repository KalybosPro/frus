# Milestone 582 — Drags on a `GestureDetector`, and sharing a scroll

Milestone 581 gave `GestureDetector` taps. An application still could not follow a finger:
the drags that existed belonged to built-in widgets — a slider's value, a scrollbar, an item
lifted onto a target, an `InteractiveViewer`'s pan. Something the application draws itself —
a signature pad, a card flicked aside, a colour wheel, a split pane's divider — had no way to
hear a finger move.

## What it does

- **`on_pan_start(|local| …)`** is called when a press has moved far enough to be a drag. It
  receives where the pointer went down, in the detector's own coordinates.
- **`on_pan_update(|local, delta| …)`** is called for each movement: where the pointer is, and
  how far it moved since the last update.
- **`on_pan_end(|velocity| …)`** is called on the release, with the velocity in logical pixels
  per second, gated by the same slop a fling uses.
- **`pan_axis(PanAxis::Horizontal | Vertical | Free)`** keeps the drag to one direction: the
  reference's horizontal and vertical drags, as one family. The other component of each
  movement and of the velocity is zero. Free, a pan, is the default.
- A press that stays within the slop is still a tap, so `on_tap` and `on_pan_*` live together.

## Sharing a scroll

A detector inside a scroll competes with it for the finger. As with a dismissible row
(milestone 515), nothing is decided at the press. The first movement past the slop decides by
**direction**, and the loser never sees the gesture:

- a horizontal detector takes a movement across, and leaves a movement down to a vertical scroll;
- a vertical detector takes a movement down, from the scroll too: it asked for that direction and
  it is under the finger (in the reference's arena, the deeper recogniser is first in line);
- a free detector takes a movement that no scroll under the finger runs along. That is the
  reference's pan, which loses the scroll's own direction because its slop is twice the scroll's.

With no scroll under the pointer, or with a mouse, the detector takes the drag directly.

## How

- Two new hooks, `Widget::pan_axis` (whether and which way a widget drags) and `Widget::on_pan`
  (a `PanEvent`: `Start`, `Update`, `End`). They are forwarded by `Box`, every transparent wrapper
  and `Responsive`.
- A new registry of detectors that drag, `Ui::pan_at`. It is kept like every other registry, so
  it survives a repaint boundary replayed from the paint cache, a transform and a modal barrier.
  Each entry holds the clipped box, where a press can land, and the whole box, from which the
  coordinates are measured.
- A new drag in the shell, `Drag::Gesture`. A finger scroll carries the detector under it until
  the threshold; the arbitration hands the gesture over and gives the scroll back its offset
  untouched.

## A detector is its child

Milestone 581's detector was a box of its own around its child, and that was wrong in a way no
test caught: along a row's direction, a lone child with no width of its own is as wide as its
content. The demo's pad — a coloured box of no stated width inside a detector — came out zero
pixels wide on the phone and drew nothing. The reference's detector does not take part in
layout at all, and now neither does this one: it is a **transparent wrapper**, the child laid
out and painted exactly as it would be without it.

That needed the transparent-wrapper macro to let one wrapper answer the gestures instead of
forwarding them: `forward_transparent!(gestures …)` leaves the six gesture hooks (`on_click`,
`on_long_press`, `on_double_tap`, `on_secondary_tap`, `pan_axis`, `on_pan`) to the wrapper.
Being the child's node, the detector shares its identity, so **what the child answers itself
it keeps**: a detector around a button leaves the button its tap and answers only what the child
has no answer to — the innermost answer wins, as in the reference.

## Found on the way

- **`widget_rect` did not know the detector** (it resolves boxes from the registries, and only some
  are asked). A drag's coordinates were measured from the window's corner instead of
  the detector's. A detector at the top of the page hid it; the test moved it 30 px down and
  caught it. `widget_rect` now asks the drag registry too.
- **A hold on a detector that also drags never fired** (seen on the phone: a held finger counted
  a tap). A long press is only armed while nothing has claimed the pointer, and a detector's drag
  waiting for the finger to move counted as a claim; a scroll waiting the same way never had. It
  is now treated like the scroll's.
- **A drag's end velocity is capped** at 8000 logical pixels per second, the reference's
  `kMaxFlingVelocity`. Through the test driver, whose moves come microseconds apart, it had come
  out at over a million.
- The reference's pan waits for twice the slop, but its tap gives up at the first slop, so a
  movement in between is neither. Here one threshold keeps a press either a tap or a drag.

## Verification

- Through the shell:
  - a drag sends a start at the exact local point it went down at, updates that add up to the
    movement, and an end, and no tap;
  - a press that barely moves is a tap;
  - a mouse drags with its finer slop;
  - in a vertical scroll, a horizontal detector takes a movement across, with no vertical
    component and the page not moved, and a movement down scrolls the page and reaches no
    detector;
  - a free detector does the same;
  - a vertical detector takes a movement down from the scroll.
- In the widgets crate:
  - every moment of a drag reaches its builder through a `Keyed`, and only the moments told;
  - a tap alone is not a drag;
  - the innermost dragging detector is found under the pointer, also when its frame is replayed
    from the paint cache.
- Mutations, each failing a test:
  - no registration;
  - the registry not replayed;
  - `widget_rect` not asking it;
  - each of the three arbitration rules reversed;
  - a horizontal movement not masked;
  - no end;
  - an unmoved drag not a tap;
  - no detector carried by a scroll;
  - the start measured where the drag became one instead of where it went down;
  - no direct drag without a scroll.
- More mutations after the fixes, each failing a test: a waiting detector drag blocking the hold,
  no velocity cap, the detector's tap preferred over the child's.
- **On the phone** (Huawei STK-L21, Android 10, release APK), in the pad added to the demo's About
  page:
  - a tap counted one tap;
  - a double tap counted one double tap and no tap. It was injected with a `monkey` script, 100 ms
    apart: two `adb input` taps start a JVM each and landed more than 300 ms apart, which counted
    two taps, correctly;
  - a finger held 1.2 s counted a hold and no tap, after the fix above;
  - a swipe across the pad took the dot to within its radius of where the finger lifted.
- **Not seen on the phone**: sharing a scroll. The About page fits the screen, so nothing scrolls
  there; the arbitration is covered by the shell tests only.
- **Not explained**: in the first session on the phone, one swipe left the dot about halfway, and
  one set of counts came out higher than the taps sent. Neither was seen again after the app was
  restarted, including with the same swipe. Separately, once, the free-drag scroll test failed
  (the page did not scroll) in a run of the whole group; it passed alone and in 25 runs since.

## In the demo

The About page has a **Gestures** pad: a count of taps, double taps and holds, and a dot that
follows a finger dragged across it.

## Left

- Scaling (two fingers) and the reference's `onTapDown` family with positions.
- `MouseRegion`: hover entering and leaving, and the cursor.

# Milestone 583 — `MouseRegion`: entering, hovering, leaving, and the cursor

A widget could show that it was hovered — the framework knew which one was under the
pointer — but an application could not hear it. A card that raises itself under the mouse,
a toolbar that appears over an image, a preview on hover: none could be written. The cursor
was the widget's own business, and only for three shapes. The reference's `MouseRegion` does
both, and it was next on the list of missing basics after the gestures (milestones 581–582).

## What it does

`MouseRegion::new(child)`:

- **`on_enter(|local| …)`** is called when the pointer comes in, or when the region comes
  under a pointer that has not moved (a screen changed around it);
- **`on_hover(|local| …)`** is called when the pointer moves inside with no button held;
- **`on_exit(|local| …)`** is called when the pointer leaves, or the region goes from under it;
- **`cursor(Cursor::…)`** sets the cursor shown while the pointer is over it, unless something
  inside asks for its own;
- **`opaque(bool)`**, on by default as the reference's, sets whether the regions *behind* it —
  overlapping it without containing it — are left while the pointer is over it.

**Every region under the pointer is in**, not only the innermost: a region inside another is
entered with it, the outer one first, and left before it. A finger is over a region **while it
touches**, as the hover of milestone 505 already was: an enter at the press, an exit at the
lift, no moves.

`Cursor` has the shapes an application asks for, from the reference's system cursors:
- `None` (hidden), `Forbidden`, `Wait`, `Progress`, `Help`;
- `Precise`, `Move`, `Grab`, `Grabbing`;
- `ResizeColumn`, `ResizeRow`, `ZoomIn`, `ZoomOut`, `ContextMenu`.

**Breaking:** a `match` on `Cursor` must handle these.

**A field shows the text cursor.** It did not before: over a field's text the pointer stayed an
arrow, as the test that put a field inside a region asking for a hand showed — the hand won. The
text cursor now comes right after what the widget under the pointer asks for, and before any
region's, as a platform's field shows it.

## How

- Two new hooks, `Widget::hover_region` (whether and how a widget is a region) and
  `Widget::on_hover_event` (a `HoverEvent`: `Enter`, `Move`, `Exit`), forwarded everywhere.
- `MouseRegion` is a transparent wrapper, like `GestureDetector`. The transparent-wrapper macro
  now builds its body from two groups of forwarded hooks, the gestures and the hover, and a
  wrapper that answers one group states it (`forward_transparent!(gestures …)`,
  `forward_transparent!(hover …)`), keeping the other forwarded.
- A registry of regions, kept like the others through the paint cache, transforms and modal
  barriers. Each region records the region it is inside, which is what lets an opaque region
  hide the ones behind it and not the ones around it: `Ui::hover_regions_at`.
- The shell keeps the regions the pointer is in and compares them with the frame's:
  - after each pointer event;
  - when the mouse leaves the window;
  - after each frame, once the frame's messages have been sent.

  The exits go first, innermost first, then the enters, outermost first. A region that has gone
  from the tree sends no exit, as the reference's does not.

## Checked against the reference

- The reference's region is opaque by default and hears `onHover` only without a button held.
- Its mouse tracker re-checks after every frame.
- Its cursors include those named above.
- Its field sets the text cursor itself, from inside, so it wins over a region around it.

## Verification

- Through the shell:
  - into a region inside another, both are entered, outer first, each at its own local point;
  - a move is heard by both; out of the inner one only it is left, and out of both the outer
    one is too;
  - over an opaque region the one behind is not entered, and is once the pointer is off the
    front one; a region that is not opaque lets it in;
  - with a button held, regions are entered but hear no moves;
  - a finger enters at the press and leaves at the lift;
  - a region appearing under a still pointer is entered at the next frame;
  - the cursor is the inner region's, the outer region's, the default outside, and the text
    cursor over a field inside a region asking for a hand.
- In the widgets crate:
  - a region and a detector wrap each other either way round, each still answering;
  - a `Keyed` forwards a region;
  - the regions under a point come innermost first, also from a frame replayed from the cache.
- Mutations, each failing a test:
  - no registration;
  - no parent link;
  - opaque ignored;
  - the registry not replayed;
  - no exit;
  - moves while a button is held;
  - no check after a frame;
  - no text cursor over a field;
  - no region cursor.

- **On the phone** (Huawei STK-L21, Android 10, release APK): the demo's gestures pad is now also
  a region. A finger put down on it showed `· Pointer in` on the pad's line, and the mention went
  when the finger lifted. That hold also counted a long press: the two `adb input motionevent`
  commands landed more than half a second apart, which is one.

## In the demo

The gestures pad on the About page is a region: its line says `Pointer in` while the pointer is
over it, and the cursor over it is a hand ready to grab the dot.

## Left

- **The cursor on the web** follows the same path as the desktop's only where winit's web
  backend does. Not checked in a browser.
- **A mouse on the phone** was not tried; a finger's enter and exit were.

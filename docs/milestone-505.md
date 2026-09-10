# Milestone 505 — A tooltip nobody asked for

Found on the device while checking milestone 504: on the demo's task screen, a **"Back"
tooltip** stood over the status bar as soon as the screen opened, with nothing pressed and
nothing held.

## Why it showed

A tooltip shows while its anchor is **hovered**. On a phone nothing should be, and two
faults together made something so.

- **A finger hovered after it lifted.** Touch goes through the same normalised pointer
  path as the mouse, and the hover was set on every move — a finger's included — and
  cleared by nothing. Whatever a finger last touched stayed hovered.
- **The hover was an id kept from an earlier frame.** A widget id is a **position in the
  tree**, not a widget. The avatar tapped on the home screen and the back button of the
  screen that tap opened stand at the same position in their trees, so the id the finger
  left behind named the back button on the next frame — whose tooltip duly showed. With a
  mouse the same thing happened to whatever stood at the old position, until the mouse
  moved again.

## A place, asked of each frame

The shell now keeps the hovering pointer as a **place**, not an id — `Hover`, in the
shell — and asks each frame what is under it.

- **A mouse** is somewhere once it has moved, pressed or not; it is nowhere once it leaves
  the window, which the shell did not notice before.
- **A finger** is somewhere **only while it touches**. The reference has no hover for a
  finger at all; here a press only shows while its widget is also the hovered one, so a
  finger hovers from the moment it is down — without a move first, which a tap does not
  have — and nothing once it lifts or is cancelled.
- **The question is asked again** after every move, press and release, and after every
  rebuild — the moment the reference's mouse tracker asks too, since that is when the tree
  can change under a still pointer. A change found after a rebuild is shown on the next
  frame, as there. Not during a drag: a slider dragged past its end is still the one being
  pressed, and the move path already leaves it so.

## Verification

- Four tests on `Hover`, against real frames: a finger hovering what it touches from the
  moment it is down; hovering nothing once lifted or cancelled; a mouse still hovering after
  its button is released, and nothing once it leaves; the same still pointer over two
  frames answering for each frame, not for the first.
- Four mutations, each failing the test meant for it.
- **What the tests do not reach** is the wiring in the shell's event loop, whose types need
  a window; the device is what checked it. Huawei STK-L21, Android 10: the task screen
  opened from the home screen by the same tap as before, and no tooltip.

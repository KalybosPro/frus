# Milestone 586 — `Listener` and `OrientationBuilder`

Two more of the basics the audit after milestone 579 found missing.

- **`Listener`** hears the pointer's raw events. `GestureDetector` (milestones 581–582)
  recognises gestures, and a recogniser has to lose sometimes: a drag inside a scroll is the
  scroll's. A signature pad, a colour picker or a gesture of the application's own wants
  every event, whatever else is recognised, and that is what the reference's `Listener` gives.
- **`OrientationBuilder`** builds one layout for a wide box and another for a tall one.

## What it does

`Listener::new(child, |event| …)` is called for each raw event with a `ListenerEvent`: its kind,
where the pointer is in the listener's own coordinates, and whether it is a finger. The kinds:

- **`Down`**: a press inside the listener. Every listener under the pointer hears it, the
  innermost first.
- **`Move`**, **`Up`**, **`Cancel`**: the pointer that went down inside, **wherever it is now**,
  until it lifts. Each listener that heard the press keeps hearing that pointer, even outside
  its box.
- **`Hover`**: a mouse moving over the listener with no button held.

It competes with nothing. A scroll under the finger still scrolls, and a button inside still
takes its tap; the listener hears it all the same.

`OrientationBuilder::new(|orientation| …)` builds from the orientation of **its box**,
landscape when wider than tall. It decides on the box and not the window, as the reference's
decides on its constraints.

## How

- Two hooks, `Widget::pointer_listener` and `Widget::on_pointer_event`, which returns a list so
  that two listeners wrapping each other at one place both hear. `Listener` is a transparent
  wrapper. The macro's third group, `forward_transparent!(listener …)`, leaves these two hooks
  to it and forwards the gestures and the hover.
- A registry of listeners, `Ui::pointer_listeners_at`, kept like the others through the paint
  cache, transforms and modal barriers. (The frame already had a `listeners` registry, for
  keyboard shortcuts; the name was taken.)
- The shell hands each pointer event to the listeners **before** routing it anywhere else, so
  a press is heard against the frame it landed on, before anything it leads to rebuilds the
  tree. It keeps the listeners a press was heard by, `captured`, until the release.
- **`OrientationBuilder` is a `LayoutBuilder` that takes all the room on offer.** A
  `LayoutBuilder` hands its closure its final box, and without a size of its own that box is as
  big as what it built. The first version, in a wide box, built a 10-px square, measured it as
  tall and chose portrait; the test caught it. Filling the room, as an `Aligned` does, makes
  the box the room. Along a row's or a column's own direction, shared with other children, the
  room is only its content's, the same rule as `Aligned`'s. That is short of deciding on
  constraints, which needs a widget able to measure before it builds (#52).

## Verification

- Through the shell:
  - a press outside the inner listener is heard by the outer one alone, at its own point; a
    press inside both is heard by the inner one first, each at its own point;
  - a finger dragged up over the listener is heard all the way — a press, at least seven moves
    and a release — while the page under it scrolled;
  - a mouse pressed in the inner listener and carried out of both is heard outside them, the
    move and the release at points past their boxes;
  - a button inside still takes its tap, and the listeners hear a press and a release;
  - a mouse with nothing held is heard as a hover by the listener under it, and outside both
    by neither.
- In the widgets crate:
  - one builder in a wide box and in a tall one, in one window, builds each way;
  - two listeners at one place both hear, the inner first, through a `Keyed`;
  - the listeners under a point come innermost first, also from a replayed frame.
- Mutations, each failing a test:
  - no registration;
  - the registry not replayed;
  - the outermost first;
  - nothing captured at the press;
  - moves sent to whatever is under the pointer instead of to what was pressed;
  - no hover;
  - a release sent as a cancel;
  - the builder not filling;
  - the outer listener heard first.

  The moves one survived at first. The finger's test was meant to leave the box, and did not:
  the page scrolled with the finger, so the finger stayed over the same point of the content.
  The mouse's test, which scrolls nothing, carries the pointer right out, and catches it.
- Not run on a device: none needed a phone's own input, and the finger's path is the one
  milestones 581–583 checked on the phone.

## Left

- A **cancel** is routed, and not tested: the test driver has no way to send one.
- The reference's `behavior` (`deferToChild`, `opaque`, `translucent`): a listener here hears
  a press anywhere in its box, which is `opaque`'s rule without the blocking.

# Milestone 581 — `GestureDetector`: taps, double taps, long presses, secondary clicks

A widget answered a tap if it had one built in — a button, a tile, a `Container` given
`on_click` — and nothing else answered anything. There was no double tap anywhere, and no way
for an application to hear the right mouse button. The only right-click handler opened a
field's selection bar. The audit after milestone 579 put the reference's `GestureDetector`
first among the missing basics: it is how an application makes anything respond.

## What it does

`GestureDetector::new(child)` listens on its child, with four builders, each taking the
message to send:

- **`on_tap`**: a press and a release in the same place;
- **`on_double_tap`**: two taps within 300 ms and 100 px of each other (the reference's
  `kDoubleTapTimeout` and `kDoubleTapSlop`);
- **`on_long_press`**: a press held still for half a second, after which the release is not a
  tap;
- **`on_secondary_tap`**: a click of the secondary mouse button.

The detector draws nothing and is its child's size. The look — ink, hover — is left to
`InkWell` and the buttons.

**A tap waits while a double tap is possible.** With both set, a first tap is held for 300 ms:
a second tap in that time is the double tap, and only the double tap is sent. The wait ends
early, sending the tap, if the next press lands anywhere else. With only a tap set, it is sent
at once, as before. This is the reference's behaviour. Without it, the second tap of every
double tap would also be two taps.

## How

- Two new hooks, `Widget::on_double_tap` and `Widget::on_secondary_tap`, forwarded by `Box`,
  every transparent wrapper and `Responsive`.
- A widget that answers either one is now a **target for a press** even without a tap of its
  own. It is registered in the frame's hit list with no tap message, so a press lands on it and
  the shell asks it for the rest.
- The shell keeps the waiting first tap (`PendingTap`: what, when, where, and its own message).
  - A release on the same target, close and in time, sends the double tap instead.
  - Any other press sends the waiting tap first.
  - The loop's idle policy wakes when the wait is over, beside the long press's deadline, and
    sends it then.
- The right mouse button asks the target under the pointer for a secondary tap before anything
  else. Without one it opens a field's bar, as it did.

## Verification

- Through the shell:
  - a tap with a double tap possible sends nothing at the release and the tap after the wait;
  - two quick taps send only the double tap, and nothing is left waiting;
  - a tap alone answers at once, twice for two taps;
  - a press elsewhere sends the waiting tap before its own;
  - the secondary button sends its message, and nothing on a detector without one;
  - a long press sends its message and no tap;
  - a detector with only a double tap takes the press and hears the double tap;
  - the loop is scheduled to wake about 300 ms after a waiting tap, and not once it is sent.
- In the widgets crate: every gesture answers through a `Keyed`; a detector with only a double
  or a secondary tap is hit by a press, and one with nothing is not.
- Mutations, each failing a test:
  - only a tap registering a target;
  - a first tap not waiting;
  - a press elsewhere not ending the wait;
  - the wait never ending;
  - the secondary tap ignored;
  - the loop not woken;
  - the second tap sent as a tap;
  - the wrapper not forwarding the double tap.
- Not checked: the wake itself, which comes from the platform's event loop. The test reads the
  wake time the loop is given. Not run on a phone or a desktop window: no device was attached.

## Left

- **Drags**: a pan in two dimensions, and the horizontal and vertical drags, with the
  arbitration against a scroll the reference does in its gesture arena. That is milestone 582.
- Details on the gestures, such as where a tap landed or which button. The reference hands
  these to its `onTapDown` family; here a tap is a message, as a button's is.
- `MouseRegion`: hover entering and leaving, and the cursor.

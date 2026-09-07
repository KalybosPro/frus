# Milestone 441 — A press was a flag

Every state layer in the crate reached full the instant a finger landed, and vanished the
instant it left. Every held radius jumped. The reason was one line in the theme:

```rust
if status.interaction == Interaction::Pressed {
    overlay += 0.12;
}
```

`Status` carried `hover_progress` and `focus_progress` — animated, `0.0..=1.0`, driven by
the runtime — and carried the press as a **boolean**. So the one interaction a pointer
actually commits to was the only one that could not be seen arriving.

The reference does not merely animate it: it gives the press its **own, slower clock**.
`_HighlightType.pressed` fades over 200 ms where hover and focus get 50
(`ink_well.dart:995`), and `InkHighlight` runs that fade forward on activation and
*reverse* on deactivation (`ink_highlight.dart:62`, `:93`) — in and out, not on and off.

## `Status::press_progress`

The animated form of `Interaction::Pressed`, the way `hover_progress` is the animated form
of `Interaction::Hovered`. `Anim` gained a `press` field, `Runtime::advance` drives it, and
`build_ui` reads it back through `full_status` like the other two.

Three details are worth stating:

**Its own duration.** `PRESS_DURATION = 0.2` beside `ANIM_DURATION = 0.12`, and `advance`
computes a second step from it. Reusing the general step would have been simpler and would
have thrown away the thing the reference is saying — a press is slower than a hover.

**The target is `pressed` filtered by `hovered`**, which is the rule
`InputState::status_for` already applies to decide `Interaction::Pressed`. Without the
filter, a finger that slid off the widget while still down would keep it lit, and the flag
and the progression would disagree about what was happening.

**A press fading out is not at rest.** `advance` forgets animation entries that have
nothing left to do; the predicate had to learn about the press, or a release would drop the
entry on the first frame and the layer would vanish — the same jump, on the way back.

## What the theme does with it

```rust
let overlay = 0.08 * status.hover_progress.clamp(0.0, 1.0)
    + 0.10 * status.focus_progress.clamp(0.0, 1.0)
    + 0.12 * status.press_progress.clamp(0.0, 1.0);
```

Three progressions and no flag. Reading the flag **as well** — as a floor, say — would have
looked safer and would have defeated the whole change: the term would reach 12 % on the
first frame the finger was down, and the fade could never run. So the flag is not read
here at all.

That one line is every button, chip, list tile, menu item, destination, tab and card in the
crate: they all ask `Theme::state_layer`, and none of them had to change.

## The two widgets that read the flag themselves

**`Switch`**, whose held thumb (milestone 440) jumped to `pressedThumbRadius = 14`. It grows
into it now — and it grows from *wherever the thumb has got to*, so a switch pressed
mid-travel swells from where it is rather than snapping back to an end first.

**`Container::pressed_color`**, whose selecting line carried the comment `Pressed: instant.`
— the one step of the rest → hover → held path that was not a progression. It crosses over
from whatever the hover interpolation had reached, for the same reason: a pointer that
presses mid-fade must not jump backwards before it moves forwards.

The one case left discrete is a box given a `pressed_color` and no resting colour at all.
There is nothing to cross over *from*, and fading a fill in by its alpha would be the
colour-space mistake this project keeps finding.

## Isolated frames

`full_status` adopts the flag whole where the runtime has never heard of the widget:

```rust
status.press_progress = self.runtime.press_progress_or(
    id,
    if status.interaction == Interaction::Pressed { 1.0 } else { 0.0 },
);
```

This is the rule `value` follows on the line below it, for the same reason: a frame built
outside the loop — a test, a single render — must draw a widget that *is* being held as
held, not as untouched on its way there. In a running application the entry always exists
by the time a press is possible, because `advance` creates it for the hovered widget, and
a press is a widget that is hovered.

`hash_status` learned the field too. A repaint boundary that did not hash it would hold its
cached primitives still while the layer underneath faded — a fade that only happened where
nothing was cached.

## The tests

- `a_press_rises_then_falls_back` — part way in, full, part way back, zero.
- `a_press_arrives_more_slowly_than_a_hover` — the same `dt` leaves the press behind.
- `a_press_dragged_off_the_widget_comes_back_down`.
- `a_press_lights_by_degrees` — and that the flag alone no longer lights, which is the
  assertion that keeps the fade honest.
- `a_held_thumb_grows_into_it`.
- `a_held_box_crosses_over_to_its_pressed_colour`.

## Still open

`splashRadius`. The reference's reaction circle is **wider than its track**
(`constants.dart`, radius 20 against a 32-tall switch), and ours is bounded by the widget's
own box. Giving it the room needs the 48-pixel tap target this widget does not reserve —
which is `MaterialTapTargetSize`, and is its own milestone.

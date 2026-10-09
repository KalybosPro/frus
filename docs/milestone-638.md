# Milestone 638 — Dragging a switch's thumb

## Objective

The part of the switch milestone 637 left out. In the reference the thumb follows a
horizontal drag, and letting go decides (`material/switch.dart:864`):

- the drag moves the thumb by the finger's travel over the track's inner length (`:870`),
  the other way in right-to-left, with no curve (`:873`);
- let go past half way, the switch flips (`:885`); short of it, the thumb goes back and
  nothing is said;
- a tap still flips it, since a drag only begins past the slop.

frus's switch is **controlled**: it keeps no state, and its thumb is drawn from the animated
value the runtime steps towards `anim_target`. A thumb that follows a finger needs that value
to be held where the finger puts it, which nothing could do.

## Alternatives weighed

1. **Messages for every movement**, with the application keeping the thumb's position. That
   puts the switch's own business in every application that uses one.
2. **A widget-local state.** frus's widgets keep state in the runtime, keyed by id, not in
   themselves.
3. **The runtime holds the value.** This is what was done. A drag can move a widget's own
   animated value: the shell asks the widget where the drag puts it and the runtime holds it
   there, then lets it settle from there when the finger lifts. Any widget with an animated
   value can use it.

## What was done

- **Two widget hooks.** `pan_value(event, value, rtl)` says where a moment of a drag puts the
  widget's value. `on_value_release(value)` gives the message for letting go there. Both are
  forwarded by every transparent wrapper.
- **The runtime holds and places values.** `hold_value` keeps a value where the finger put
  it, and `release_value` lets it go. A value that was placed travels to its target in a
  straight line, in the share of the duration the distance left is, as a controller let go
  part way finishes. `value_placed` says whether it is still settling.
- **`Status::value_placed`** tells the paint, so a widget that curves its value paints it
  straight while it is placed.
- **The shell** sends a drag's moments to those hooks, holds the value, and on release
  dispatches the widget's answer. A cancelled drag lets the value settle back and says
  nothing.
- **The switch** takes a horizontal drag when it can be flipped and has someone to tell. Its
  thumb moves by the finger's travel over the 20 px inner length, mirrored in right-to-left.
  Let go at or past half way it flips; short of it nothing is said.
- **A fix in the test driver.** The shell's test `Driver` never stepped the widgets' animated
  values, which the window's frame does: a value was always its target there. A held value
  would have stayed held for ever, and no test could see a value animate. It now steps them
  as the window does.

## Tests

- Through the shell: `the_thumb_follows_the_finger`, `let_go_past_half_way_it_flips`,
  `let_go_short_of_half_way_it_goes_back` (out past the slop, then back), and
  `a_tap_still_flips_it`.
- Runtime: `a_held_value_stays_and_settles_from_where_it_was_left`.
- Switch: `a_live_switch_takes_a_horizontal_drag`, `the_drag_moves_the_thumb_by_the_finger`
  (including right-to-left and the ends), `let_go_past_half_way_it_flips`, and
  `a_placed_thumb_is_where_the_finger_put_it`.

Stepping the values in the driver showed one demo test that had only passed because animations
were skipped: `the_application_runs_through_the_shell` looked for a new task 0.1 s after
adding it, and the row arrives with a 0.2 s animation, as it does in the window. It now
waits 0.3 s.

Mutation testing: seven mutants, all killed. They cover the mirroring in right-to-left, the
half-way rule, which switches take a drag, the hold, the settle time and its end, and the
shell letting go of the value.

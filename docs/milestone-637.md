# Milestone 637 — The switch, as the reference's

## Objective

The third form control in the review against the reference (`docs/reference-review.md`).
frus's `Switch` already had the reference's track, thumb sizes and resting colours. It
differed in how it moves and answers, and in what a caller can change
(`material/switch.dart`):

| | reference | frus before |
|---|---|---|
| box | the 52 px track and 4 px either side (`:2304`, `:605`) | the track alone |
| halo | radius 20 round the thumb, past the track (`:1691`, `:2301`); `primary` round an on switch, `on_surface` round an off one, 8 % hover, 10 % focus or press (`:2264`) | a state layer clipped to the track |
| thumb under a pointer, focus or press | `primary_container` on, `on_surface_variant` off (`:2183`) | unchanged |
| flip | 300 ms (`:2386`); the thumb overshoots and settles (`easeOutBack`, `:800`); colours ease out on the way on and in on the way off (`:1153`) | the framework's duration, `ease_in_out` for everything |
| thumb size | through a 34×22 pill half way (`:1569`, `:2382`) | a circle growing linearly |
| icon | only an end with an icon has the larger thumb (`:1064`) | either icon grew both ends |
| settings | per-state `thumbColor`, `trackColor`, `trackOutlineColor`, `trackOutlineWidth`, `overlayColor`, `splashRadius`, `padding` | two colours per part |

## What was done

- **The box** is the track plus 4 px either side, so 60 × 48. The track is centred in it.
- **The halo** comes from the shared `toggleable` module, with the switch's own colour
  table. It is painted over the track and under the thumb, centred on the thumb, and
  spills into the room either side.
- **The thumb's state colours** follow the reference's table, and disabled colours stay
  opaque.
- **The flip**: the runtime's value is now the flip's linear progress over 300 ms. The paint
  curves it three ways, as the reference curves one controller: the thumb's place
  (`easeOutBack`, flipped on the way off), the colours (ease out, ease in), and the thumb's
  size (the reference's size sequence through the midway pill). Curves are pinned to their
  ends, so a settled switch lands exactly on its colours and its end.
- **Settings**:
  - on the switch: `thumb_colors`, `track_colors` and `track_outline_color` (each a
    `WidgetStateProperty<Color>`), `track_outline_width`, `overlay_color`, `splash_radius`
    and `padding`;
  - in `SwitchTheme`: the same settings, and the theme is no longer `Copy`.

  The plain `thumb_color`/`track_color` pairs stay and come after the per-state properties,
  as the reference's `activeThumbColor` comes after `thumbColor`.

## Not done here

- **Dragging the thumb.** The reference's thumb follows a horizontal drag and flips past
  half way (`:864`). frus's switch is controlled and keeps no state, so a thumb that follows
  the finger needs the runtime to hold a drag position for it. That is the next milestone.
- **The adaptive switch** (`Switch.adaptive`, iOS and macOS looks) is a later item.

## On screen

Three pictures were recorded again after a side-by-side look (`controls_toggles`,
`disabled_selection_controls`, `control_list_tiles`). The only change is the 4 px room
either side of each switch; colours and shapes at rest are unchanged.

## Tests

- `a_switch_answers_the_pointer_with_a_halo`: the halo's radius, centre, colours by state,
  and the caller's colour and radius.
- `the_thumb_answers_the_pointer`: the state colours, and the caller's per-state thumb.
- `the_track_and_its_rule_are_the_callers`: per-state track, outline colour and width.
- `the_flip_overshoots_and_settles`: 300 ms, linear runtime value, overshoot past either
  end, and the thumb sizes along the sequence in both directions.
- `the_box_has_room_either_side`: 60 by default, the caller's and the theme's padding.
- `a_thumb_that_carries_a_glyph_is_the_larger_one`, `the_thumb_grows_as_it_travels`, and
  the colour tests updated for the new curves.

Mutation testing: ten mutants, nine killed and one survivor. The run was interrupted
halfway, and the six mutants without a verdict were run again. Before the rerun, a check
was added for the default outline width, which the tests had only checked when the caller
gave one. The survivor was the first 11 % of the thumb's size sequence, where only the end
points were checked; the thumb is now checked to be stretching towards the pill there, and
that kills it.

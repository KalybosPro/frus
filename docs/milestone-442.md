# Milestone 442 — Nothing here reserved a tap target

A switch painted a track 32 pixels tall and asked the layout for 32. A checkbox painted a
box of 20 and asked for 20 — or for the height of its label, whichever was taller. An icon
button painted 40 and asked for 40. Every one of them handed the hit registry exactly the
rectangle it drew, so the area a finger could land in was the drawing.

Forty-eight is the number both mobile platforms' accessibility scanners check for, and the
reference reserves it by default for every one of these controls, painting the small thing
in the middle (`constants.dart:27`, `theme_data.dart:172`).

## `TapTarget`

A theme-wide setting with a per-widget override, which is how the reference has it: it is
the kind of decision an application makes once.

```rust
pub enum TapTarget { Padded, ShrinkWrap }   // 48, or the 40 the spec allows
```

`Theme::tap_target` is `Padded`. Each of the four controls resolves
`caller ?? widget theme ?? theme`, and `SwitchTheme`, `CheckboxTheme`, `RadioTheme` and
`IconButtonTheme` each gained the field so the middle rung exists.

The shrink-wrapped answer is 40, not zero: `kMinInteractiveDimension - 8` is what the
reference's own `shrinkWrap` resolves to for a checkbox (`checkbox.dart:522`) and for a
switch (`switch.dart:2090`). Shrink-wrapping is a smaller answer, not the absence of one.

**This is a layout answer, not a visual one.** Nothing any of the four paints changed. What
changed is the room around it, and therefore the rectangle a click may land in.

## What each one had to learn

- **`Switch`**: 52 × 48, and the track is centred in it (`switch.dart:605`). The thumb is
  measured from the track rather than from the box, so it stays where it was.
- **`Checkbox`** and **`Radio`**: a floor under both sides. The target is the **whole
  control**, label included, rather than a square around the box — the reference's checkbox
  carries no label and reaches the same guarantee by being a square with the words outside
  it. A labelled one is already wider than the minimum, so the width floor only ever binds
  on a bare box, and then the box is centred in what it was given.
- **`IconButton`**: the box is the target and the **face** is what it paints, centred in it.
  A caller who asks for a 32-pixel face gets a 32-pixel face in a 48-pixel box: the control
  shrinks, the area a finger may land in does not.

Both the checkbox's and the radio's labels were drawn at `bounds.y`. That was the same
sentence as "centred" while the bounds *were* the line; a floor under the height makes the
two different, so both now centre.

## Two things the change found

**The framework's own overflow check failed on all nine demo screens at once.** That is
milestone 335's instrument doing its job. Two causes:

- the demo pinned its task row at 62 pixels, and the row holds a checkbox and a delete
  button that now reserve 48 apiece inside 8 of padding and a 1-pixel rule. 66.
- **`NavigationBar` carried six pixels of vertical padding**, leaving 44 for the back
  button it holds. That one is the framework's: a toolbar is 56 tall and holds a 48-pixel
  button centred in it, and the four pixels either side come from the difference rather
  than from a rule. The padding is gone.

The failure message now names the box and the edge as well as the amount — "2 px" on nine
screens at once says a shared widget grew, and only the rectangle says which.

**A theme fade dropped every per-widget default it crossed.** `Theme::lerp` rebuilds from
the interpolated scheme and `from_scheme` starts from an empty `WidgetThemes`, so nothing
put them back: every override an application had written disappeared for the length of a
light/dark crossing and returned when it ended. Discrete now, like the direction beside it.
Found while adding `tap_target` to the same function.

## The tests

- `a_theme_reserves_a_tap_target_by_default`, `a_fade_keeps_what_it_cannot_interpolate`.
- `a_switch_reserves_room_for_a_finger` — including the three rungs of the resolution —
  and `the_track_is_centred_in_the_room`.
- `a_checkbox_reserves_room_for_a_finger`, `a_bare_box_is_centred_in_its_room`, and
  `a_click_below_the_box_still_lands`, which is the point of the whole milestone: a press
  under the box, inside the square, reaches the checkbox.
- `an_option_reserves_room_for_a_finger`, `the_label_is_centred_in_the_room`.
- `an_icon_button_reserves_room_for_a_finger`, `the_face_is_centred_in_the_room`.

Two existing icon-button tests asserted that the layout box *was* the face, which is
exactly what this changes; they measure the face now, and one of them gained the assertion
that a smaller face still gets the target's room.

Thirteen goldens moved, all read: the toggles, the disabled controls, the outlines, the
stepper, the navigation chrome and every calendar — the calendars because their month
chevrons are icon buttons, so the header row grew with them.

## Still open

Every other small control that sizes itself to what it paints. The snack bar's close cross
is its own widget at 40 (`toast.rs`), and it is not the only one. The four here are the
ones the reference names.

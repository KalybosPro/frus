# Milestone 635 — The radio, as the reference's

## Objective

The second form control in the review against the reference (`docs/reference-review.md`).
The reference's `Radio` (`material/radio.dart`) and frus's differed:

| | reference | frus before |
|---|---|---|
| ring | radius 8 to the middle of a 2 px stroke (`:31`), centred in a 48 px square | a 20 px ring at the leading edge |
| dot | radius 4.5 (`:32`), grown from nothing over the 200 ms animation (`:849`) | 10 px, at once |
| density | **the theme's** (`:1008`): compact on a desktop | none |
| halo | 20 px, the checkbox's colours per state (`:977`) | none |
| ring colour | moves from the unchosen colour to the chosen one with the animation | switched at once |

## What was done

- **The reference's geometry and animation**: the ring and the dot above, in a square of 48 px
  (40 shrink-wrapped) moved by the density. Unlike the checkbox, the density is the theme's,
  as the reference's radio takes it. The dot grows and the ring changes colour over 200 ms.
- **The halo**, shared with the checkbox: a new crate-private `toggleable` module holds what
  the reference's `ToggleableStateMixin` and `ToggleablePainter` share, which is the halo
  painting and its colour table per state. The checkbox now uses it too. Its error state
  keeps the reference's order: pressed, hovered, then focused.
- **Settings**: `Radio::visual_density`, `Radio::splash_radius`; `RadioTheme` gains
  `overlay_color`, `splash_radius` and `visual_density`, and is no longer `Copy`.
- The label stays frus's, after the square. `RadioGroup`, frus's group of labelled options,
  is built on the new radio. The reference's `RadioGroup` is only a registry.

## On screen

Three pictures recorded again after a side-by-side look (`controls_toggles`,
`disabled_selection_controls`, `control_list_tiles`). The ring is smaller with a smaller dot,
in a 40 px square on the pictures' Linux theme.

## Tests

- `the_ring_and_the_dot_are_the_reference_s`: the ring, the dot settled and half grown, and
  no dot before it is chosen.
- `an_option_reserves_room_for_a_finger` (on a phone's theme, 48; shrink-wrapped, 40), and
  the existing colour and behaviour tests, settled where the animation matters.
- `the_halo_colours_are_the_reference_s` (the shared table).

Mutation testing: four mutants. The theme's density turned compact was killed first. Three
survived, which showed the ring's size, the dot's growth and the halo's pressed colour
untested, and are now killed by the tests above.

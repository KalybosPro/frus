# Milestone 634 — The checkbox, as the reference's

## Objective

The first of the form controls in the review against the reference
(`docs/reference-review.md`). Compared with the reference's `Checkbox`
(`material/checkbox.dart`, Material 3 defaults at `:923`), frus's differed in most of what
it draws:

| | reference | frus before |
|---|---|---|
| box | 18 px, 2 px corner, centred in a 48 px square | 20 px, 5 px corner |
| density | **standard** unless the checkbox is told, on every platform (`:1043`) | — |
| tick | a stroke through three points of the box (`:750`), drawn as the box fills | a "✓" glyph from a font |
| partly ticked | a dash across the middle (`:773`) | a rounded bar |
| ticking | animated: the box fills and the tick draws itself (`:786`) | instant |
| halo | 20 px round the box, under a pointer 8 %, focused or pressed 10 %, in `on_surface` or `primary` depending on whether it is ticked (`:998`) | none |
| outline | `on_surface_variant` at rest, `on_surface` under a pointer, the keyboard or a finger, none once ticked | the same, 2 px |
| error | outline, fill and halo in `error`, tick in `on_error` | none |

## What was done

- **The reference's geometry**: an 18 px box with a 2 px stroke and a 2 px corner, centred
  in the room it reserves for a finger. That room is 48 px, or 40 shrink-wrapped, moved by
  the checkbox's own density, which is standard unless the checkbox or its theme says
  otherwise. Unlike a button, a checkbox does not take the theme's desktop density.
- **The tick drawn as a stroke** through the reference's three points, and the partly-ticked
  dash grown from the middle.
- **The animation**: the checkbox animates between empty and filled over 200 ms. On the way
  the box shrinks by a stroke and grows back, the fill fades in over the first quarter, the
  outline gives way to the fill, and the tick draws its short stroke and then its long one,
  as the reference's painter does.
- **The halo**: under a pointer and focus it fades in, and under a finger it grows. Its
  colours are the reference's per state, overridable with `overlay_color` (a per-state
  property) and `splash_radius`, on the checkbox or in its theme.
- **Error**: `error(true)`.
- **`semantic_label`** for a checkbox without a label.
- `CheckboxTheme` gains `overlay_color`, `splash_radius` and `visual_density`, and is no
  longer `Copy`.

## Not done

- **Ticked ↔ partly ticked is not animated.** frus's implicit animation carries one value,
  and that value says empty or filled. The reference also morphs the tick into the dash while
  the box stays filled, which needs the previous value as well. Going between these two the
  mark changes at once. Going to empty from partly ticked, the mark that shrinks is a tick.
- `Checkbox.adaptive` needs an Apple-style checkbox, which frus does not have yet.
- The label is frus's own. The reference puts words beside a checkbox with a list tile. Here
  the label follows the 48 px square.

## Tests

- `the_box_is_the_reference_s`: 18 px in a 48 px square on a desktop theme too; 40
  compact or shrink-wrapped.
- `the_tick_is_drawn_through_the_reference_s_points`, and the dash.
- `ticking_animates_the_box_and_the_tick`: half way and three quarters of the way.
- `the_halo_is_the_reference_s`, `an_error_takes_the_error_colours`,
  `a_disabled_box_is_inert_but_still_says_whether_it_is_ticked`.
- `click_toggles`, `a_tristate_box_cycles_through_the_third_answer`,
  `partly_ticked_is_announced_as_mixed`,
  `the_theme_answers_and_the_instance_overrules_it`, `the_label_follows_the_square`.

## On screen

Four pictures were recorded again after a side-by-side look (`controls_toggles`,
`disabled_selection_controls`, `light_outlines`, `control_list_tiles`). In each, the box is
18 px and centred in its square, the tick is a drawn stroke, the label starts after the
square, and a disabled ticked box shows its tick in `surface` on the opaque disabled fill.

Mutation testing: seven mutants, all killed. They are a 20 px box, the theme's desktop
density, a tick through the wrong point, a box that does not shrink half way, the halo in the
unticked colour, a tick that ignores the error, and a partly-ticked box animated as empty.
The last of these first survived, and the test now covers it.

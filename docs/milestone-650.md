# Milestone 650 — What a press and a slide may do to a slider

## Objective

The part of the reference's `Slider` that milestone 648 left out: `allowedInteraction`, the
`SliderInteraction` that says what a press and a slide may do (`material/slider.dart:1593`):

| | a press | a slide |
|---|---|---|
| `tapAndSlide` (default) | moves the thumb to it | carries it on |
| `tapOnly` | moves the thumb to it | nothing |
| `slideOnly` | nothing | moves the thumb by its own length, from where it was |
| `slideThumb` | nothing, unless on the thumb | as `slideOnly`, from a press on the thumb only |

The last two need what frus's drag did not keep: **where the press landed**, and where the
value was then. A slide that moves the value by its own length measures from both, for the
whole gesture, while the value itself changes under it with every message.

The same reading turned up a difference in the bracket: the reference's `onChangeStart`
reports the value **before** the press, so that a tap is a start at the old value and an end
at the new one. frus reported the value the press landed on.

## What was done

- **`SliderInteraction`**, the reference's four, and `Slider::allowed_interaction`; the
  theme's `SliderTheme::allowed_interaction`, read during layout (`build_themed`) since a
  drag is answered without a theme.
- **`DragMode`** on `DragTrack`, the shell's terms for the same four: `Follow`, `TapOnly`,
  `Slide { origin }` and `SlideThumb { origin, radius }`, where `origin` is the value's place
  along the travel when the press landed and `radius` the thumb's halo.
- **Each moment of a drag answered by the track** the widget named at the press, which the
  shell keeps for the whole gesture:
  - `engages` says whether the press starts anything at all — not for a press off the
    thumb of a `SlideThumb` slider, whose whole gesture then sends nothing;
  - `value_at` gives the value for the press and for each movement: the pointer's place,
    the press only, or the origin moved by the pointer's movement;
  - `released_at` gives the value the drag lets go of.

  A slide is measured before it is held to the travel, so one that starts or runs past an
  end still moves by its whole length, and right to left it runs the other way.
- **`on_change_start`** reports the value before the press, snapped to the divisions as the
  rest of the stream is.

**Changed**: `on_change_start`'s value; `DragTrack` has `mode`, and the shell's drag keeps
its press.

## Tests

- `frus-widgets`:
  - each interaction naming its drag mode, the theme's taken when the widget names none and
    the widget's over the theme's;
  - each mode on a track of 200 from 24: the press and a slide following the pointer; the
    press only; a slide from the origin, held at an end, started past an end, and right to
    left both ways; the thumb's halo engaging a press on it and at its edge, not off it, and
    on the right in a right-to-left box;
  - the bracket starting at the value before the press, in the caller's units, snapped.

Mutation testing: 10 mutants, all killed. They cover:

- tap only taking the press alone, and releasing where the press set it;
- a slide not jumping on the press, and measuring past the ends;
- the thumb's halo, its edge and its side right to left;
- tap only named as such, the halo's radius;
- the theme's interaction;
- the bracket's start snapped.

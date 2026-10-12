# Milestone 648 — `Slider`, the reference's 2023 design

## Objective

The first of the remaining form controls in the review against the reference (tracker
`docs/reference-review.md`). frus's slider was its own design: a 6-pixel track, a white
thumb with a ring, a tooltip on hover that made the control taller. The reference's
default slider — the 2023 design, `year2023` being `true` unless asked otherwise
(`material/slider.dart:834`) — is a different thing on every count:

| | reference | frus before |
|---|---|---|
| track | 4 high, the travelled part 6, inset by the halo's radius at each end | 6, the whole width |
| thumb | 20 across, the travelled colour | 18, white with a ring |
| halo | 48 across, on hover, focus and drag, its colour per state | none |
| ticks | one per division | none |
| value | a drop over the thumb, while moved or focused, by default only with divisions | a pill on hover or focus, room reserved above |
| secondary value | a lighter part of the track ahead of the value | none |
| colours | `primary`, `surface_container_highest`, ticks at 38 % | `primary`, `secondary_container`, white |
| width | the room on offer, at least 144 of track | 220 |
| right to left | runs from the right | did not |
| semantics | a formatter of its own | the label's |

## What was done

- **The geometry and the paint**, from the reference's shapes (`slider_parts.dart`):
  - the rounded track, its travelled part two pixels thicker, inset by the halo's radius;
  - the round thumb with its shadow rising while held;
  - the round halo, grown by hover, focus and a press, in the state's colour;
  - round ticks a quarter of the track high, kept off when they would crowd;
  - the drop indicator: a pill over a triangle pointing at the thumb, grown from the tip;
  - the thumb of a slider with divisions kept half a track inside each end, over the end
    ticks.
- **The value indicator's policy**, `ShowValueIndicator`:
  - `OnlyForDiscrete` (the default), `OnlyForContinuous` and `OnDrag` show while the
    slider is moved or focused;
  - `AlwaysVisible` always;
  - `Never` never.
  It is drawn over what is above the slider, as the reference's is, and takes no room.
- **Colours** resolve as the reference's do: the widget's, the theme's, then the 2023
  defaults. The thumb follows the travelled colour, and a halo follows a travelled colour
  given at 12 %. Disabled, the framework's disabled rule: content for the travelled part
  and the thumb, a container for the rest. A ring around the thumb, which the reference
  has no word for, stays available as `thumb_border_color` and is off unless asked for.
- **`secondary_track_value`** and `secondary_active_color`.
- **Right to left**: the travel, the track, the ticks and the drag run from the right.
- **Width**: the room on offer, as a progress bar's (milestone 590), with at least 144 of
  track; `width` still fixes one. Height: the halo's 48.
- **`semantic_formatter`**: what a screen reader is told, over the label.
- **`Widget::drag_track` and `DragTrack`**: where a drag lands along a widget, inset from
  its ends and mirrored or not. The shell measures every drag's start, move and end with
  it, in place of its own fraction of the whole width, so the thumb stays under the finger
  on an inset track and in either direction.
- **`SliderTheme`** gains the track height, the secondary, tick and disabled colours, the
  halo's per-state colour, the indicator's colour and the policy.

**Changed**: the 2023 colours; no ring unless asked; the label shown only with divisions
unless told otherwise; no room reserved for it; a slider takes the width on offer;
`SliderTheme` is not `Copy`, holding a per-state colour.

Two pictures moved with it, checked: `disabled_inputs` and `light_outlines`, each with a
slider now inset, solid-thumbed and 48 tall.

## Decisions

- **The halo shows over the whole slider.** The reference grows it on hover only while the
  pointer is over the thumb; frus's hover is the widget's, and a position-aware hover is
  not one this widget can ask for yet.
- **`allowedInteraction` is next.** Tap only and the default tap-and-slide follow from
  this; slide only and thumb only move the value by the drag's movement from where it
  started, and the drag hooks do not carry the start. That is the next step, with
  `RangeSlider`, which this milestone leaves as it was.

## Tests

- `frus-widgets`:
  - the inset track and the thumb's place at either end; the thicker travelled part;
  - right to left, from the right;
  - a tick per division, their two colours, none when crowded, the discrete thumb over the
    end ticks;
  - the secondary part ahead of the value, not behind it, in a colour given;
  - each indicator policy, the theme's, and none disabled;
  - the drop's place over the thumb, its fill and its point;
  - the halo per state, a colour given, a property given, none disabled;
  - the disabled rule;
  - the width on offer, and a width given;
  - a drag's fraction along the inset track, mirrored, held at the ends, and on no track;
  - the semantics formatter;
  - the 2023 defaults, the thumb following the travelled colour, the drop's type, the
    height with a label.
- `frus-test`: two new pictures, checked by eye:
  - `sliders_reference`: continuous; with divisions and the drop; with a secondary value;
    disabled with divisions; in colours of its own;
  - `sliders_right_to_left`: continuous and with divisions, from the right.

Mutation testing: 20 mutants, all killed. They cover:

- the track's inset and its thicker travelled part;
- the mirroring, and the discrete thumb's padding;
- the ticks' crowding rule and the secondary part's direction;
- the indicator's discrete and always-visible policies, and the drop's place;
- the halo's colour given, its hover tint and its disabled rule;
- the thumb following the travelled colour, the 2023 inactive colour and the disabled
  container;
- the semantics formatter, the minimum width;
- the drag's mirroring and inset, in both the slider and `DragTrack`.

# Milestone 651 — The range slider, against the reference

## Objective

Review `RangeSlider` against the reference's (`material/range_slider.dart`, its 2023
default `_RangeSliderDefaultsM2`, and the range parts of `slider_theme.dart`), as milestones
648 and 650 did for `Slider`.

frus's range slider was an early widget of its own: a fixed 220-pixel row of spacers and two
draggable 18-pixel thumbs, values only in `0..=1`, a tooltip whenever a label was set, and a
rail of 6. Next to the reference it differed in almost everything:

| | frus | the reference |
|---|---|---|
| width | 220, fixed | the room it is offered, at least three touch targets of track |
| travel | `0..=1` | `min..=max` |
| track | 6, one colour | 4, the span between the thumbs `primary` and 2 thicker, the rest `primary` at 24 % |
| thumbs | 18, dragged one by one | 20 round, a press takes the nearer, a press on both waits for the first movement |
| gap | the thumbs could sit on each other | 8 pixels apart without divisions, may meet with them |
| on top | always the end | the thumb last taken, with a ring in the surface colour when they overlap |
| elevation | both rise on a press | only the thumb held rises, 1 to 6 |
| halo | a focus ring | `primary` at 12 %, 48 across, on the held, hovered or focused thumb |
| labels | a tooltip, whenever labelled, reserving height | the rectangular indicator, while dragged or focused, by `showValueIndicator` |
| ticks | none | one per division, `onPrimary`/`primary` at 54 % |
| bracket | the value the press landed on | the interval as it was |

## What was done

- **The widget rewritten on the reference's model.** One box that takes the width it is
  offered (or the one given) and is as tall as the halo, painting the track, the ticks, the
  two thumbs with their shadows, the halo of the held thumb and both indicators while it is
  dragged. Over it, two **thumb stops** placed by a directional alignment
  (`2t − 1` puts a 48-pixel box's centre exactly where the inset track puts `t`). Each stop
  takes the focus, answers the arrows, Home and End, tells a reader its own value and travel,
  and shows its halo and its indicator while hovered or focused. A disabled slider takes no
  drag, no key and no focus, and still says where it sits.
- **Two thumbs on one drag.** `DragTrack` has a fifth mode, `DragMode::Thumbs { values,
  radius }`. `DragTrack::pick_thumb` is the reference's `_defaultRangeThumbSelector`: the
  nearer thumb, unless the press is within the touch radius of both, when the first movement
  decides (towards the start takes the start, mirrored right to left). The shell keeps the
  thumb it picked for the whole drag and reports each moment through the new hook
  `Widget::on_thumb_drag(thumb, ThumbDrag::{Start, Move, End}, fraction)`.
- **The thumb last taken** is kept by the runtime (`InputState::thumb`) and handed to the
  widget as `Status::thumb`, after the drag ends too, as the reference's last selection is:
  it is painted on top, it rises and fades with the press, and it wears the halo.
- **The gap.** A thumb is held off the other by the reference's eight pixels on a continuous
  slider and may meet it on one with divisions. Eight pixels is a fraction only a layout
  knows; the slider keeps the track's length its last paint found, which its thumbs' keys
  read too.
- **The rectangular indicator**: a box with corners of 4, sixteen wider and taller than its
  text, over an eight-pixel triangle whose point is fourteen above the thumb's centre; in the
  theme's indicator colour, else `onSurface` at 60 % over `surface` at 90 %, its text in
  `bodyLarge` and `onPrimary`.
- **The builders**: `range`, `divisions`, `width`, `value_label`, `semantic_formatter`,
  `show_value_indicator`, `active_color`, `inactive_color`, `thumb_color`, `overlay_color`,
  `thumb_elevation`, `pressed_thumb_elevation`, `enabled`, `on_change`, `on_change_start` and
  `on_change_end`. The theme's slider settings apply to it as to `Slider`.

**Changed**: `RangeSlider` is laid out, painted and dragged differently; its values are in
`min..=max`; `on_change_start` reports the interval before the press; a label no longer
shows by itself on a continuous slider (`show_value_indicator` says when); the slider itself
has no semantics node, its two thumbs do. `Status` and `InputState` have `thumb`.

## Alternatives weighed

- **Keeping draggable thumbs**, one gesture per thumb, as before. A press between the thumbs
  must move the nearer one and a press on both must wait for the movement, which only the
  whole track can decide: the drag belongs to the slider, the thumbs only take the focus.
- **The two labels as strings**, as the reference's `RangeLabels`. A formatter follows the
  values without the caller rebuilding the labels, and is what `Slider::value_label` already
  takes.
- **A shared "last thumb" kept in the widget.** Widgets are rebuilt every frame from the
  application's view: the selection has to live in the runtime, beside the press.

## Left

- The reference paints the halo of a hovered thumb under it; here a thumb stop paints over
  the slider, so its halo is a ring around the thumb instead.
- The reference's two `RangeLabels` strings and its `RangeSliderThumbShape` and friends;
  the shapes are not pluggable.

## Tests

- `frus-widgets`:
  - the drag track carrying both thumbs, in order, in the slider's units;
  - the press picking the nearer thumb, the middle dividing, right to left, a press on both
    undecided until it moves either way;
  - the bracket: the interval as it was, the moved thumb snapped, held to the travel;
    nothing when disabled or with no message;
  - the gap: meeting with divisions, eight pixels off without, from either side;
  - each thumb's focus, semantics in the slider's units and with a formatter, arrows, Home
    and End stopped by the other thumb;
  - a disabled slider's thumbs inert and still announced;
  - the colours: the span between the thumbs, the outer parts, right to left, the widget's
    colours over the theme's;
  - the thumb last taken on top, alone raised, with the halo; the ring where they overlap;
  - the indicators: both while dragged with divisions, none by default without, always
    when asked, the box's place, a focused thumb's own;
  - the width offered and the width given.
- `frus-shell`, a range slider driven through the shell: a press beside the start thumb
  takes it and the drag carries it, the bracket opening on the interval as it was; the
  start then painted on top; where the thumbs meet, leftwards takes the start and
  rightwards the end; the start dragged past the end stops at it.
- Goldens: `disabled_inputs`, `range_slider` and `range_slider_labels` re-recorded; the
  last now shows a focused start thumb's halo and its indicator over the slider.

Mutation testing: 15 mutants, all killed — two only after their tests were sharpened (the
gap on a slider with divisions measured on a painted slider with steps fine enough to show
it; the thumb taken reaching only its own widget). They cover:

- the thumb selector: the movement mirrored right to left, the middle dividing, the touch
  radius in pixels;
- the gap from either side, its length, none with divisions;
- the thumb on top, alone raised, the overlap ring;
- the thumb stop's place with divisions, the indicator out of the stack;
- the shell recording the thumb taken, the movement deciding a press on both, the status
  reaching only its widget.

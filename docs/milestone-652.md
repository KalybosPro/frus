# Milestone 652 — Painting over one's own children

## Objective

Close the first gap milestone 651 left in `RangeSlider`: a hovered or focused thumb's halo
was painted **over** its thumb. The reference paints the overlay first and the thumb on it.

The cause was structural. Each thumb's focus stop is a child of the slider, so it paints
after the slider. The slider drew the thumbs in its own `paint`, before its children, so
the stop's halo landed on top. Milestone 651 worked around it with a ring around the thumb
instead of a disc under it.

## What was done

- **`Widget::paint_over`**, with `Widget::paints_over` to opt in: a widget paints after its
  own subtree, in the same box and with the same status as `paint`. This is the reference's
  `foregroundPainter`, beside the `foreground` decoration that already took that place in the
  walk. The walk computes the status only for a widget that opts in. The hook is forwarded by
  `Box<dyn Widget>`, the transparent wrappers and `Responsive`.
- **The range slider paints in two passes.** The track, the ticks and the held thumb's halo
  go under the stops. The thumbs, their shadows, the overlap ring and the dragged values go
  over them. A stop's halo is a full disc again.

## Alternatives weighed

- **The stop repaints its thumb over its halo.** That breaks the order between the two
  thumbs: a hovered bottom thumb would come out on top of its neighbour.
- **The slider paints every halo itself**, learning which stop is hovered. It would need the
  child's hover in the parent's status, which nothing carries. The general hook is smaller,
  and other widgets need it too.

## Tests

- `frus-widgets`: built through `build_ui` with the start thumb's stop hovered, the scene
  holds the track, then the stop's halo round the start, then the thumbs. The slider's own
  tests paint both passes and check that it asks for the second.
- `frus-widgets`: a range slider behind a selector and behind a key still asks to paint
  over its children, and paints the same thing there.
- Goldens unchanged except `range_slider_labels`, where the focused thumb now sits on a
  full halo disc.

Mutation testing: 7 mutants, all killed. One was killed only after its test was sharpened:
the held halo's thumb before a drag has taken one. They cover:

- the walk asking for and calling the second pass;
- `Responsive` and the transparent wrappers forwarding both hooks;
- the held halo's thumb.

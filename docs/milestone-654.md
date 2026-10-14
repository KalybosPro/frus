# Milestone 654 — The slider's parts, replaceable

## Objective

Close the second gap milestone 651 left: a slider's shapes were fixed. In the reference,
each part of a `Slider` and a `RangeSlider` is a shape object held by `SliderThemeData`:
- the track (`SliderTrackShape`, `RangeSliderTrackShape`);
- the tick marks (`SliderTickMarkShape`, `RangeSliderTickMarkShape`);
- the thumb (`SliderComponentShape`, `RangeSliderThumbShape`);
- the halo (`overlayShape`);
- the value indicator (`SliderComponentShape`, `RangeSliderValueIndicatorShape`).

An application replaces any of them, or removes it with `noThumb`, `noOverlay` or
`noTickMark`. frus painted each part inline, so none could be replaced.

## What was done

- **Five traits**, one per part, shared by both sliders:
  - `SliderTrackShape` is given the track's rectangle and the thumbs' centres: one for a
    slider, two for a range slider. A slider also passes its secondary value's place.
  - `SliderTickMarkShape` gives its width, which the slider uses to leave out ticks that
    would crowd each other. It paints a mark on the active part or off it.
  - `SliderThumbShape` is given the thumb's elevation, and on a range slider whether it lies
    over the other thumb.
  - `SliderOverlayShape` is given the activation and the colour.
  - `SliderValueIndicatorShape` is given the text and the scale.

  Each part gets a `SliderPaint`: the theme, the resolved `SliderPalette`, whether the
  slider is enabled and discrete, and the opacity.
- **The reference's parts as the defaults**:
  - `RoundedRectSliderTrackShape` and `RoundedRectRangeSliderTrackShape`;
  - `RoundSliderTickMarkShape { radius }`;
  - `RoundSliderThumbShape { radius }`, which paints its own shadow and its ring;
  - `RoundSliderOverlayShape { radius }`;
  - `DropSliderValueIndicatorShape` and `RectangularSliderValueIndicatorShape`.

  `NoSliderShape` implements all five traits and paints nothing.
- **Where a part comes from**: the widget's own (`track_shape`, `tick_mark_shape`,
  `thumb_shape`, `overlay_shape` and `value_indicator_shape` on both sliders), then the
  theme's, then the default.
  - `SliderTheme` holds the reference's nine: `track_shape`, `range_track_shape`,
    `tick_mark_shape`, `range_tick_mark_shape`, `thumb_shape`, `range_thumb_shape`,
    `overlay_shape`, `value_indicator_shape` and `range_value_indicator_shape`.
  - Each is a `SliderShape`, a shared handle compared by identity, so the theme still
    clones, compares and prints as a value.

The defaults paint exactly what the sliders painted before: every golden passes unchanged.

## Alternatives weighed

- **Separate range traits**, as the reference has. Their only differences are the second
  thumb and the "on top" flag, which a slice of centres and a field carry. One set of traits
  lets a thumb shape serve both sliders.
- **Shapes deciding the geometry.** The reference asks the track shape for the track's
  rectangle and the thumb and halo for their sizes, which also size the slider. Here a drag
  is answered without a theme, so the travel must be known without one. The box, the inset
  and the travel stay the slider's: a larger custom thumb or halo overflows the box rather
  than growing it.

## Left

- Shapes do not size the slider (see above).
- The reference's other parts: the paddle indicators, the gapped track and the handle thumb
  of its 2024 design. Each is one implementation of a trait.

## Tests

- `frus-widgets`, with a part that writes down each call and paints nothing:
  - a slider with all five of its own parts paints nothing of the defaults. The parts are
    called in order: the track with the thumb and the secondary value, the halo, a tick per
    division and its side, the thumb with its pressed height, then the value;
  - the theme's parts, a slider's apart from a range slider's, and the widget's over the
    theme's;
  - a range slider's parts: both thumbs to the track, the ticks between them, the thumb on
    top told so and alone raised, both values, and a thumb stop's halo and value;
  - `NoSliderShape` paints nothing;
  - the reference's parts with other sizes: the thumb, the halo, the ticks, and the
    rectangle indicator on a slider;
  - a held part compares by identity, and a theme holding one compares with its clone;
  - a transparent halo paints nothing, and crowded ticks are left out.
- Goldens: all unchanged.

Mutation testing: 10 mutants, all killed. Three were killed only after tests were added:
- a transparent halo paints nothing;
- ticks that would crowd are left out by the mark's own width;
- the theme's range indicator is the range slider's, not the slider's.

They cover:
- the widget's part over the theme's;
- a range slider reading the theme's range parts: thumb, track and indicator;
- the overlap ring's width and the thumb border;
- the transparent halo;
- the tick density;
- the secondary value reaching the track;
- only the held thumb rising.

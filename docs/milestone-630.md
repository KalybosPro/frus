# Milestone 630 — The circular progress indicator, as the reference's

## Objective

The second widget of the functional review (milestone 629). frus's
`CircularProgressIndicator` was not the reference's at all: it drew **a ring of eight dots**
whose brightness turned. The reference (`material/progress_indicator.dart:863`) draws an
**arc**:

| | reference | frus before |
|---|---|---|
| shape | an arc stroked around a circle | eight dots on a circle |
| value | optional: a determinate arc sweeps clockwise from the top to it | none: always spinning |
| indeterminate motion | the arc's ends follow `fastOutSlowIn` over 1.333 s halves while the whole turns every 2.222 s (`:1060`) | a bright dot going round at 1.1 turns a second |
| size | 36 (2023 look) or 40 inside 4 px of padding (newer) (`:1552`, `:1599`) | 24 |
| stroke | 4, centred (2023) or inside (newer) | dots 12 % of the side |
| ends | flat when determinate, square when spinning (2023), round (newer) (`:754`) | — |
| track | none (2023); behind a determinate arc in the newer look, stopping a stroke and a gap short of each end (`:731`) | a dim ring of dots, if a colour was named |
| semantics | value 0–100, a label, no value when spinning | none |

## What was done

- **The reference's painter** (`:711`): a determinate arc from straight up, `value × (2π −
  0.001)` long; an indeterminate one starting at `−π/2 + tail·3π/2 + rotation·2π +
  offset·π/2` and `(head − tail)·3π/2` long, never less than 0.001. The track is drawn
  first, and its gap is measured along the circle.
- **`CircularProgressIndicator::new()` is indeterminate**, as the reference's default is;
  `determinate(value)` and `with_value(Option)` make the others. It is continuous only
  while indeterminate.
- **Both looks**: the 2023 one by default, as the reference's; `year2023(false)` or
  `ProgressTheme::year2023` (shared with the linear bar) for the newer one.
- **Every setting on three rungs** (caller, theme, reference): `size`, `padding`,
  `stroke_width`, `stroke_align`, `stroke_cap`, `track_color`, `track_gap`, `color`, and the
  semantics label and value. `ProgressTheme` gains `stroke_align`, `stroke_cap`,
  `circular_size` and `circular_track_padding`.
- **Line ends in the renderer.** `Stroke` gains a `cap` (`StrokeCap::Butt`, `Square`,
  `Round`; `Butt` by default, so every existing stroke is unchanged), and the GPU
  tessellator draws it.

## Not done

- `CircularProgressIndicator.adaptive`, which shows Apple's activity indicator on iOS and
  macOS: frus has no Apple-style activity indicator yet.
- The pull-to-refresh indicator still draws the ring of dots. In the reference it is a
  circular indicator with an arrowhead (`RefreshProgressIndicator`), and it will get its own
  review.
- `controller` and `valueColor`, as for the linear bar.

## Tests

- `the_default_look_is_the_reference_s_default`: 36 px, a quarter arc from the top, 4 px,
  flat ends, no track; square ends while spinning.
- `the_newer_look_is_the_reference_s`: 48 px in all, the stroke inside, round ends, the track
  and its two gaps; no track while spinning.
- `an_indeterminate_arc_runs_the_reference_s_timing`: the head, tail, offset and rotation a
  quarter of the way through a cycle, the arc they give, and the arc painted.
- `an_indicator_answers_to_its_theme_and_to_its_caller`, `a_reader_hears_the_reference_s_values`.
- `strokes_end_as_asked` (frus-gpu): an 8 px line drawn with each end, sampled just past its
  end and at its corner.
- The widget snapshot `small_indicators`, recorded again: the spinner is now the reference's
  arc. At time zero that is its shortest arc, a dot, so the snapshot also gained two
  determinate indicators at 60 %, one in the 2023 look and one in the newer look with its
  track and gaps. The 102 goldens are unchanged.

Mutation testing: seven mutants, all killed. They are the round end drawn flat by the GPU,
the newer look as default, flat ends while spinning, the 2023 stroke moved inside, one gap
instead of two, a rotation of half a turn, and the padding counted once.

# Milestone 629 — The linear progress indicator, as the reference's

## Objective

The user reported that many frus widgets do not do what the reference's widgets of the same
name do, and gave `LinearProgressIndicator` as the example. Compared with the reference
(`material/progress_indicator.dart`), frus's bar was a different widget under the same name:

| | reference | frus before |
|---|---|---|
| value | optional: none means **indeterminate**, two lines running along the track | required; no indeterminate bar at all |
| default look | the 2023 one (`year2023 ?? true`, `:595`): square ends, no gap, no dot | the newer one, chosen by frus |
| corners, newer look | 2 px, whatever the height (`:1627`) | half the height |
| a tiny value | drawn tiny | widened to a dot as wide as the bar is tall |
| the gap at the first percent | grows from nothing (`:256`) | appears whole |
| paint order | track, dot, then the fill over the dot | track, fill, dot |
| the dot's size | never more than half the bar's height (`:240`) | unbounded |
| right to left | mirrored (`:224`) | not mirrored |
| semantics | value 0–100, a label, no value when indeterminate (`:146`) | "50%" on a 0–1 range, no label |

## What was done

- **Indeterminate bars**: `LinearProgressIndicator::indeterminate()`, or
  `with_value(None)`. The reference's painter is ported line for line (`:211`): four
  curves, each an interval of the 1800 ms run on its own cubic (`:189`), give the two lines'
  heads and tails. The track is drawn around them, with the gap ramped at both ends. The bar
  declares itself continuous while indeterminate, so frames keep coming; a determinate one
  does not.
- **The reference's default look**: the 2023 one. `year2023(false)` (or
  `ProgressTheme::year2023`) selects the newer one. In the 2023 look the newer pieces (the
  gap and the dot) are ignored, as the reference ignores them.
- **The newer look's numbers**: 2 px corners; a dot of radius 2 in `primary`, capped at half
  the height and drawn under the fill; a 4 px gap that grows over the first percent.
- **No minimum fill**: a value of 0.1 % draws 0.1 %, as the reference does.
- **Right to left**: the bar fills from the right and its dot is at the left.
- **Semantics**: `semantics_label`, `semantics_value`; the value read out is the percentage
  without a sign, on a 0–100 range. An indeterminate bar has no value, which is how ARIA and
  the platforms' accessibility interfaces say "indeterminate".

## Not ported

- The reference's `controller` (an animation controller shared between indicators) and
  `valueColor` (an animated colour): frus has no animation objects to hand a widget. An
  indeterminate bar runs on the frame clock.
- The reference's clip of an indeterminate bar to its corners. Every segment is drawn with
  the corners already, and the fractions never leave the bar, so the clip draws nothing
  different.

## What it changes on screen

Every determinate bar that did not choose a look changes to the reference's default: square
ends, the track under the whole bar, no gap and no dot. One recorded picture holds such a bar
(the widget snapshot `small_indicators`); it was recorded again after a side-by-side look:
42 pixels, all at the bar's gap and its dot. The 102 goldens are unchanged.

## Tests

- `the_default_look_is_the_reference_s_default`, `the_newer_look_is_the_reference_s`.
- `right_to_left_mirrors_the_bar`.
- `an_indeterminate_bar_runs_the_reference_s_lines`: the curve values at 0.375 s, the lines
  on screen at 0.375 s and at 1.05 s, the period.
- `a_bar_answers_to_its_theme_and_to_its_caller`, `a_reader_hears_the_reference_s_values`,
  `value_is_clamped`.

Mutation testing: seven mutants, all killed. They are the newer look as the default, the gap
without its ramp, no mirroring, an unbounded dot, a wrong curve for line 1, an
indeterminate bar that stops asking for frames, and the read-out value on 0–1.

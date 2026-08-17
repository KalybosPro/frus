# Milestone 325 — Two tokens, and the assertion that was left out

Milestone 320 found that this framework's dark palette put a live outline and a disabled
one within a whisker of each other, wrote it up, and **deliberately left the assertion
out** rather than weaken it to something it would pass. Milestones 322, 323 and 324 each
repeated the note in their *Left* section. Milestone 324's device pass promoted it: on
hardware, a stepper at its lower bound was indistinguishable from one that could still be
pressed.

This fixes it. The assertion 320 left out is now in the test suite.

## The device measurement was too weak to act on

324's postscript recorded that taking `Quantity` from 0 to 1 moved the "−" button's mean
colour by about 1.4 of 255, *less* than the incidental shift on the "+" beside it. That was
enough to raise the alarm and not enough to justify moving a token every border depends on:
the mean was taken over a 140 × 135 button box in which the outline is a one-pixel hairline
and the glyph a few dozen pixels, so almost any real change would have come out small. The
direction was wrong too, which a sound measurement would not have been.

The arithmetic needs no phone. A disabled outline is `on_surface` at 12 % over the surface;
a live one is a token. Both resolve to a colour, and a colour has a tone:

| | `outline` ↔ disabled | `outline_variant` ↔ disabled |
|---|---|---|
| frus dark | 8.3 tones | **2.2** |
| frus light | 6.5 tones | **0.5** |
| the reference | 36–40 | ~10 |

`outline_variant` *was* the disabled colour, in both shipped palettes, to within rounding.
That is the finding, and it is checkable in a unit test rather than a screenshot.

## Where the tokens belong

`ColorScheme::from_seed` already had this right — it places the two roles at tones 60/30
(dark) and 50/80 (light), which is the reference's own tonal spec. Only the two
hand-written schemes had drifted, and they had drifted *down*: dark `outline` sat at tone
32 where it belongs at 60.

So the numbers were not invented. They are those tones taken from each palette's own
neutral-variant family, which lands them within a few units of the reference's baseline
schemes — dark `outline` (141, 145, 153) against its #938F99, light `outline_variant`
(195, 198, 207) against its #CAC4D0. Two independent routes to the same place is the best
evidence available that the tones are the right ones.

## Raising a token is not enough on its own

`theme.border` is `scheme.outline`, and about half its callers were not drawing an outline
at all — they were drawing a **separator**: a chart's gridlines, a navigation bar's hairline,
a drawer's edge, a kanban column's frame. At tone 32 the distinction did not show. At tone
60 those would all have shouted.

The reference splits them, and the split is legible from its source: `outlineVariant` for
dividers, card borders and unselected chips; `outline` for an outlined button's side, a
switch's unselected track, a selected chip. Thirteen call sites moved to `outline_variant`
accordingly. That is a correction in its own right, not damage control — a divider was
never supposed to be painted in the control-edge colour.

A slider's rail took neither. It is a **filled track**, not an edge, and the reference gives
it a container tone; ours was on `outline` and would have become a bright bar. It is on
`surface_container_high` now.

## What the pictures said

Sixty-two goldens moved and all sixty-two were read, before and after, at 1:1. Nothing
regressed. The changes fall into three groups:

- **Outlines that were invisible now exist.** An outlined text field, an outlined button, a
  checkbox's ring, a segmented control — all of them had edges at tone 32 on a tone-13
  surface. `outlined_field`, `decorated_form`, `password_field`, `form_wizard` and the whole
  table set are the clearest cases, and they were arguably a worse defect than the one this
  milestone set out to fix.
- **Disabled now reads as disabled.** `disabled_actions` is the picture 324's postscript was
  about: the stepper's "−" at its bound and the page strip's "‹" on page one are now plainly
  quieter than their live neighbours.
- **Separators got slightly clearer**, moving from tone 22 to tone 30 — a tree's guides, a
  timeline's connector, a chart's gridlines.

One contact-sheet reading was wrong and the zoom corrected it: a bar chart's baseline looked
*absent* after the change when downscaled, and at 4× is present and slightly brighter, as
the arithmetic said it had to be. Reading a picture at the wrong scale is its own way of
being confidently wrong.

## The light theme had no picture at all

Every golden in this repository was dark. That is how a palette bug that affects both
schemes shipped: the light theme's outlines were even closer together (0.5 tones) and
nothing was watching. `light_outlines` is the first light-theme golden — outlined controls,
live beside disabled — and it is what confirmed that the reference's tones dropped into this
palette give a light theme somebody would want to use rather than a wireframe.

## Verification

fmt clean, clippy silent in both profiles, 1101 tests, 83 goldens, and two assertions
that did not exist:

- `an_outline_is_never_the_colour_of_a_disabled_one` checks both shipped schemes and three
  seeded ones, so a future palette cannot reintroduce this.
- `disabled_is_never_louder_than_live` (in `chip.rs`) gained the outline half that milestone 320
  wrote a paragraph of comment explaining the absence of. The comment is now four lines and
  the assertion is two.

Verifying it turned up something else. `cargo test --workspace --release` has never been
able to pass: `ReloadWatcher::new` refuses outside a debug build, and its test asserted the
debug branch unconditionally, so the suite failed in `frus-shell` before reaching either the
widgets or the goldens. The routine command runs in debug, which is why nobody noticed —
the same shape as milestone 322's unformatted commit, where the check that would have caught
it was not the check being run.

## Left

- **The slider's live rail is fainter than its disabled one**, in both themes, by about four
  tones. Neither hand-written palette has a container darker than the 12 % disabled tone —
  the reference reaches for `surfaceContainerHighest`, a role this scheme does not have. In
  practice the disabled slider is unmistakable anyway, because its fill and thumb flatten
  too; the rail alone is not carrying the message. Adding the missing role is the fix.
- **A checkbox's unselected ring should be `on_surface`, not `outline`.** The reference is
  explicit and this palette is not. It is a louder change than it sounds and belongs in its
  own step.
- **The tone floors in the new guard are asymmetric** (24 for `outline`, 6 for
  `outline_variant`) because a chip's border is the only thing riding on the second one and
  a chip flattens its fill as well. If `outline_variant` ever governs a control on its own,
  that floor is too low.
- **`from_seed` was right all along**, which is worth remembering the next time a
  hand-written value disagrees with a generated one.

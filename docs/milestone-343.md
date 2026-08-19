# Milestone 343 — What text does about a box it does not fit

`Text` is the most used widget in the framework and it could answer three of the
reference's questions with a shrug: where do the lines sit inside their box, how many of
them are there, and what happens to the ones that do not fit. There was `wrap()` and there
was `ellipsis()`, and between them they covered two points of a space with a dozen.

This milestone is the space: `TextAlign`, `max_lines`, and the reference's four
`TextOverflow` modes.

## Alignment needs a box, and a text did not have one

A text that shrink-wraps has nowhere to align itself to. Centring a box exactly as wide as
its own words moves nothing, and the setting looks broken rather than inapplicable.

The reference lays a paragraph out at `constraints.constrain(its own size)` — its width is
its own until something narrower is imposed, and a tight box makes it the box's. Here a
`Text` declared `width: Length(its measurement)`, which no parent can take away.

So a text that has been given an alignment stops declaring a width and asks to **fill the
one it is offered** — through `main_axis_fill`, the hook milestone 342 added for `Row` and
`Column`, resolved by the same walk. Nothing else changes: a text with no alignment
shrink-wraps exactly as it did.

The alignment itself reaches the renderer on the primitive, along with the box width to
align inside and whether that width also wraps. Those three travel together because they
are one decision, and the width is handed over **only when something is going to use it**:
giving the shaper a width it did not have moves right-to-left text to the right-hand edge,
which is a bug this repository has already had once.

## `max_lines` is a height cap and a cut

Two separate things, and only one of them is layout. The cap is arithmetic — the measured
height, clamped to `max_lines` line heights, with the words still wrapping where they
wrapped. The cut is not: it has to fall on a break the *shaper* chose, or the words move.

`frus_text::visual_lines` returns the lines a text breaks into inside a box, at most so
many of them, and whether there was more. Each line comes back as its own string rather
than as an offset, because the caller is about to cut one of them and draw the rest.

It is reached **only by a text that asked for a limit**. That is not just about the shaping
saved: a paragraph handed to the renderer as lines is a paragraph *per line*, and rules
that span a paragraph stop working — a justified block leaves its last line ragged, and it
can only do that if it knows which line is the last.

## The four overflows

- **Clip** cuts at the edge of the box, and does it by intersecting the primitive's clip
  with the box — but only where the text genuinely does not fit. A clip around every text
  would put a hard edge through the antialiasing of every one that does.
- **Ellipsis** cuts the last kept line and ends it in one. It needed a second truncation
  routine beside the existing one: `truncate` asks whether a line fits and leaves it alone
  when it does, and a line being cut because the *next* one was dropped usually does fit
  and still has to say so.
- **Fade** wraps the text in a **masked group** — the mask machinery of milestone 339,
  reached from a widget's paint for the first time. It has to be a group: applied
  primitive by primitive, two overlapping glyphs would each fade against the background
  and the overlap would be neither. The fade runs over a fifth of the box and never more
  than three line heights of it — proportional alone would start halfway through perfectly
  legible words on a long line, absolute alone would swallow a short one — and it runs
  along the edge the text actually ran past: sideways for a line, downwards for a
  paragraph cut short.
- **Visible** draws past the box, which is what every text did before.

## Saying what to do is what makes a text squeezable

A flex item's automatic minimum size is its content, so a plain text refuses to shrink and
pushes its siblings out instead — the defect that cost milestones 333 and 334. `ellipsis()`
lifted that floor; now any overflow mode or line limit does, and with it the text is
clamped to its parent (`max_width: 100%`), which is the reference's
`constraints.constrain` in the vocabulary this layer has.

That clamp is the half that makes the modes fire at all. Without it a text declares the
width it wants, a narrower box does not take it away, and the words simply draw past the
edge — the very thing the mode was set to prevent.

A text that has said nothing is untouched, in all three respects. All 88 existing goldens
agree.

## Left

- **A limited paragraph is not justified.** Its lines are handed over as lines, so each is
  its own paragraph and none of them knows it is not the last. Justification and a line
  limit are rare together; the fix is a line limit the renderer understands rather than
  one resolved before it.
- **`visual_lines` is not cached**, where every measurement is. It shapes once per frame
  per limited text, which is the same shaping the renderer does again immediately
  afterwards.
- **`softWrap` still defaults to off.** In the reference a `Text` wraps unless told not to,
  and here it wraps only when told to. It is the one remaining difference in this widget,
  it is a default rather than a missing capability, and changing it moves every text in the
  framework — its own milestone.
- **No `textScaler`, no `strutStyle`, no `textHeightBehavior`.** Accessibility text scaling
  is the one of the three that will be missed.
- **`RichText` takes none of this**: no alignment, no limit, no overflow. The same three
  questions, one primitive along.

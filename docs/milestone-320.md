# Milestone 320 — The gap three milestones wrote down

Milestone 312 ended with it. So did 313, and so did 314:

> **Nothing is disabled.** Neither the control nor a single segment can be greyed out —
> the third widget in three milestones to be missing `enabled`, which is starting to look
> like a gap in the framework rather than in any one widget.

Milestone 317 gave it to `TextInput` because a form demanded it. `Chip` and
`SegmentedControl` still had nothing, and three notes saying the same thing is the repo's
own signal that something is overdue — the same signal that produced `IconButton` in 315.

## Greying out is the easy half

Both widgets now take `enabled(false)`, and what that turns off is deliberately more than
a colour:

- **the press goes nowhere** — `on_click` returns `None` rather than the caller's message;
- **no ink** — a splash is a promise that something is happening, and nothing is;
- **out of the tab order** — `focusable` is false, so Tab does not stop at a control that
  cannot be operated;
- **announced as disabled** rather than falling silent. A reader that simply stopped
  hearing about a filter chip would be told the filter had gone away, which is a different
  and worse fact;
- **still saying which segment is chosen.** A disabled control is read-only, not invisible:
  `toggled` survives, so the current answer is still legible to a reader who cannot change
  it.

## And the cross has to follow

A `Chip`'s delete cross is a **child widget** with its own `on_click`. Disabling the chip
without disabling the cross would have left the one live control sitting on an inert thing
— and it would have looked fine in a screenshot. The cross takes the chip's availability
now, and the test asserts it goes dead with its parent, because the failure is invisible
until someone taps it.

## Flatten, do not fade

The tempting implementation is to multiply everything by 0.38 and move on. That is not what
the reference does and not what `Button` already does here: it collapses every variant to
**`on_surface` at 12 %** under a label at **38 %**.

The difference matters most on a *selected* control. Fading the accent gives a pale accent,
which reads as *quietly selected*; flattening to grey reads as *unavailable*. A disabled
filter is not offering a dimmer version of its answer — it is not offering. Both widgets
have a test that the accent appears when live and **never** appears when disabled, which is
a stronger claim than any tolerance on a colour.

## Verification

1074 tests (4 new), clippy silent, and **one new golden**. `disabled_controls` puts each
pair side by side — live chip beside disabled chip, live control beside disabled control.
That picture is not optional here: milestone 312's delete cross was painted in a
transparent colour and survived because nothing drew it, and a state that only unit tests
have ever seen is the same bet again.

## What reading the picture turned up

The golden was worth having before it was even committed. Looking at the disabled chip
beside the live one raised the question of whether *disabled* was actually quieter, which
the rule does not guarantee on its own: a live unselected chip's label is
`on_surface_variant` and a disabled one's is `on_surface` at 38 % — two different tokens,
and which reads louder is the palette's business.

The label clears it in both shipped themes, and there is now a test that says so. Writing
that test got the measure wrong first: comparing raw **luminance** passes on a dark theme
and fails on a light one, because a quieter colour on a light ground is *brighter*, not
darker. The measure is contrast against the surface, and the first version was a test that
looked right and asserted the wrong thing.

The **outline** does not clear it. The rule matches the reference exactly —
`outline_variant` live, `on_surface` at 12 % disabled — but this dark palette puts the two
within a whisker: `outline_variant` is (48, 52, 62), and 12 % of `on_surface` over the
surface lands near 54, so the disabled hairline carries very slightly *more* contrast than
the live one. The reference's own palette separates them comfortably. What deviates is the
**token**, not the rule, and moving a token moves every outline in the framework — so it is
recorded rather than patched inside a milestone about `enabled`.

## Left

- **No single disabled segment.** The reference can disable one of several; here it is the
  whole control or none of it.
- **`enabled` is still per-widget.** `Button`, `TextInput`, `Chip` and `SegmentedControl`
  each carry their own flag and their own copy of the 12 %/38 % rule. The reference hangs
  it on the state machine every control shares.

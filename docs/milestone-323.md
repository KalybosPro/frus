# Milestone 323 — Finishing the sentence milestone 322 started

Milestone 322 opened by naming five controls:

> A form could not grey out a checkbox, a switch, a radio, a slider or a dropdown — the
> five controls a form is mostly made of.

It then did three of them. This does the other two, which were left last on purpose: the
slider is the one control that is neither tapped nor typed into, and the dropdown is the
one that owns something while it is disabled.

## The slider, which is why the guard has drag hooks

322 widened its source guard to cover `draggable`, `on_drag`, `on_drag_delta`, `on_key`
and `on_edit`, with the reasoning stated and no widget yet to prove it: *a disabled control
that was inert only to the gesture nobody was using on it is not disabled.* The slider is
that widget. It answers a drag on the track, a drag on each thumb, and four keys on a
focused thumb, and every one of those is a way to change a value the caller has frozen.

**The guard caught the omission it was written for.** Wiring `Slider`, `RangeThumb` and
`RangeSlider` by hand, `RangeSlider::semantics` was left ungated — a disabled range slider
that told a screen reader it was live. Two milestones, three catches, and each one a hook
that is invisible in a screenshot.

A slider also turns out to be the cleanest illustration of the container/content split that
322 arrived at: the part of the track **still to travel** is a container at 12 %, and the
part **already travelled**, plus the thumb, is content on it at 38 %. That is the
reference's split too, and it is the whole widget — there is nothing else in a slider.

## The dropdown, which owns a menu

A disabled dropdown is **never open**, whatever `options(open, …)` was told. That is not
tidying: the menu is an overlay, so leaving it built over a header that answers nothing
gives the screen a floating panel that traps a press and returns no message. The rule is
that a control which cannot be operated cannot be holding something open on the strength of
having been operated.

The rows also gained semantics they never had, the same hole `RadioOption` had in 322.
Announcing that a row is unavailable without ever announcing the row is announcing an
absence.

## A correction to 322's own documentation

322's `disabled_content` was documented as covering "its label, its glyph, its outline".
Writing the dropdown showed that to be wrong: the reference splits outlines the same way it
splits everything else. The outline **of a container** — a chip, an outlined button, a
field, a dropdown row — is part of that container and takes 12 %. A checkbox's box or a
radio's ring takes 38 % because it is not a container with a mark inside it; it **is** the
mark. The code was already right in both places; only the sentence was wrong, and a wrong
sentence in the one module that exists to stop people guessing is worth more than a typo.

## Verification

1097 tests (6 new), clippy silent, one new golden. `disabled_inputs` shows each control
beside its live twin — and shows the dropdown that was told to be open sitting closed,
which is the claim in this milestone least likely to be believed from a test name.

Reading it confirmed both greys stay apart on the slider. It also shows the disabled
dropdown's outline sitting very close to the live one, which is **not** new: it is the
`outline_variant` collision recorded as an open item in milestone 320, showing up in a
second widget exactly as that entry predicts.

## Left

- **Five controls still cannot be disabled**: `Rating`, `Stepper`, `Menu`, `Tabs`,
  `Pagination`. None of them is a form control, which is why they come after these.
- **No single disabled option**, still — in a radio group, a segmented control or now a
  dropdown. It is the whole control or none of it.
- **A disabled dropdown cannot be inspected.** Being never-open is right for a control that
  cannot be chosen from, but it also means an application cannot show the list read-only.
  The reference has the same behaviour, so this is a note rather than a complaint.
- **The 12 % outline is still hard to tell from the live one** in this dark palette —
  milestone 320's open item, now visible in two widgets rather than one.

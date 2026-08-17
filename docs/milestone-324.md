# Milestone 324 — The rest of the controls, and what the picture found

Milestones 322 and 323 gave `enabled` to the eight controls a form is made of. This gives
it to the five that were left — `Rating`, `Stepper`, `Menu`, `Tabs`, `Pagination` — after
which **every control in the framework that can be pressed can be told not to be**.

The mechanical half went as expected. The half worth writing down is what happened when
the golden was read.

## Flattening destroys the answer

Milestone 320 established the rule that a disabled control **flattens** rather than fading,
and 322 turned it into one shared module. The first version of this milestone applied it
faithfully to a rating: every star, lit or not, went to the same grey.

The picture refused it. Five identical stars say nothing, and how many are lit is the whole
of what a rating carries — the same claim 320 made about a segmented control still showing
which segment is chosen, and 322 about a checkbox still showing its tick. The fix is the
container/content split doing exactly what it is for: a lit star is the **mark** at 38 %, an
unlit one the **container** it sits in at 12 %.

The same picture showed a disabled page strip as six identical grey pills with no current
page, and that one was not the rating's mistake. It was `Button`'s.

## `Button` flattened too far, and the reference says so

`Button::palette` gave **every** variant a 12 % container when disabled, and dropped the
outlined variant's outline entirely. The reference is explicit that it should not: a text
button's `backgroundColor` is `transparent` in every state, disabled included, and only the
foreground moves to 38 %; an outlined button keeps its outline at 12 % and gains no fill.

So the rule is not "one disabled appearance for every variant". It is:

- the **label** goes to 38 % in every variant — that is what makes unavailable read as
  unavailable rather than as a quieter accent;
- the **shape** is kept. Variants that are a filled container flatten to one; a text button
  has none; an outlined one keeps its outline at the container opacity.

Milestone 320's note claimed the first sentence covered both, and a test named
`a_disabled_button_looks_the_same_in_every_variant` asserted it. Both are corrected. The
consequence is the one the picture showed: a group of buttons carrying a selection through
their variants — a page strip, a segmented row — keeps that selection when it is disabled.

`IconButton` had the second half of the same bug, and the golden was blunter about it: a
disabled stepper appeared as two bare glyphs with no buttons around them, which reads as
broken rather than as unavailable. Its outlined variant keeps its outline now too.

## A bound you can see

Two of these controls were quietly lying about what they could do.

`Stepper` clamped at its range ends, so at the top "+" stayed live and emitted the value
already showing. `Pagination` built its end arrows without an `on_press` and left them
otherwise untouched, so they were painted as full outlined buttons and Tab stopped on them —
its own comments said "disabled on the first page" and only the code did not. Both are
disabled at their bounds now, which is a change you can see before you press.

## Two hooks a `Box` was not forwarding

`Stepper` sets its value's colour, which needs the ambient theme, and it is assembled before
it has one — milestone 319 built `ThemeBuilder` for precisely this, and this is its second
consumer. Reaching for `Theme::default()` instead would paint a dark-theme grey into a
light-theme application, and nothing in a dark-themed golden would ever show it.

Writing the test for that found something else. `impl Widget for Box<dyn Widget>` — a
hand-written blanket impl with no macro behind it — was **two hooks short**: `build_themed`
and `repaint_boundary`. A boxed `ThemeBuilder` asked to build would quietly do nothing, and
a boxed repaint boundary would report that it was not one.

Neither was reachable: every walk in the framework takes `&dyn Widget` and dispatches
virtually, so both were waiting for the first caller to hold a `Box` instead — which is what
the new test did. That is the exact shape of bug `transparent.rs` exists to prevent, and the
blanket impl had no guard at all. It has one now: the same source-reflection instrument,
pointed at a second target.

## Verification

1110 tests (13 new), clippy silent, fmt clean, and one new golden.

`disabled_actions` moved four times before it was right — the rating's lost score, the
stepper's missing button shapes, a cropped bottom row, and a **disabled tab bar still
painting its indicator in the accent** while its labels had already flattened. Not one of
those four was caught by a test. All four are covered by one now.

One existing picture moved with it: `data_table_paginated`, whose back arrow now reads as
unavailable on the first page instead of as a full outlined button. That is the change, seen
somewhere it was not put on purpose, which is the best kind of confirmation a golden gives.

## Postscript: what the device said about "a bound you can see"

The section above is wrong as shipped, and the phone is what said so.

On the demo's Settings screen, `Quantity` sits at 0, so its "−" is disabled and its "+" is
not. Pressing "+" takes it to 1 and the "−" becomes live. Measuring that region across the
two screenshots, the "−" button's mean colour moves from `(39.0, 42.9, 50.7)` to
`(37.6, 41.7, 49.9)` — about **1.4 of 255**, and *smaller* than the incidental shift on the
"+" button beside it, which never changed state at all. The disabled one is also, very
slightly, the **brighter** of the two.

So the behaviour is right — no message, out of the tab order, and the tests prove both — and
the appearance conveys nothing. On this palette a disabled outlined icon button and a live
one are the same button.

This is not a new problem. It is milestone 320's open item, `outline_variant` sitting on top
of `on_surface` at 12 %, showing up for the third time and now on hardware, on a control a
person actually presses. What that entry needed was evidence that it was worth the cost of
moving a token that every outline in the framework depends on. It has it.

## Left

- **No single disabled item**, still: it is the whole rating, group, control or strip.
- **`enabled` is a flag per widget.** Twelve controls now carry their own, all resolving
  through one module, which is as far as this shape goes. The reference hangs it on a
  resolver every control shares — `WidgetStateProperty` — which would also let a theme say
  *this colour, except when pressed*. That is the next shape.
- **`Rating`, `Stepper`, `Pagination`, `Menu` and `Tabs` have no theme entry**, so their
  colours are not overridable the way a chip's are.
- **The 12 % outline is still hard to tell from the live one** in this dark palette
  (milestone 320's open item), and it now governs more widgets than before.

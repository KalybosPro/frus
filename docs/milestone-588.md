# Milestone 588 — A left inset stays on the left in a right-to-left script

frus lays a frame out left to right and mirrors it as a whole for a right-to-left script. Every
padding went through that mirror untouched, so a padding given as *left* ended up on the right
in Arabic: `Container::padding_each`, `Flex::padding_each`, `Container::margin_each`, and
`SafeArea`. The reference keeps its physical insets physical; its directional ones
(`EdgeInsetsDirectional`) are the ones that follow the reading direction. Milestone 580 made
`Padding` follow that rule. This milestone does it everywhere else, at the owner's go-ahead.

## Measured before changing

A test put a 20-px square after a 30-px left padding. In right to left it came out with the
padding on the right. The same probe showed that `Positioned::left` already stays on the left:
a stack's layers are placed on their own, and nothing there changed.

## Not only a matter of matching the reference

`SafeArea` pads by the **device's** insets, which are physical by nature: a cutout on the left
of a phone held sideways is on the left whatever language the application is in. In right to
left the safe area padded the right and left the content under the cutout. The floating action
button had the same fault: at the start, which is the right in Arabic, it cleared the
**left-hand** inset.

## What it does

- `padding_each`, `margin_each` and a `SafeArea`'s insets are **physical**: a left inset is on
  the left in either direction.
- **`Container::padding_insets`**, **`Container::margin_insets`** and **`Flex::padding_insets`**
  take any insets (`InsetsGeometry`): one number, physical `Insets`, or `InsetsDirectional`,
  whose start is the right in a right-to-left script.
- The framework's own **directional** paddings now say so, where the reference's are
  directional or the code's intent was the start or the end:
  - a `Banner`'s default padding and the gap after its leading slot;
  - the gap after a drawer header's account pictures;
  - the floating button's margin, now kept apart from the safe area's physical insets;
  - the demo's licence header.

  Paddings that are equal on both sides do not change, and nearly all of them are.
- `InsetsGeometry::laid_out(direction)`, in `frus-core`, is the one rule, shared by `Padding`,
  `Container`, `Flex` and `SafeArea`: a directional inset is resolved as if left to right, and
  the mirror moves it; a physical one is swapped before the mirror, which puts it back.

**Breaking, in a right-to-left script only.** An application that wrote `padding_each` with
different left and right values, meaning start and end, and relied on the mirror to move them,
now has them on the sides it named. It should use `padding_insets(InsetsDirectional::…)`. In
left to right nothing changes.

## Verification

- A square is centred in what the padding leaves, so where it lands says which side the padding
  is on: at 55 with 30 on the left, at 25 with 30 on the right.
  - a container's left padding: 55 in both directions; its start padding: 55, then 25 in right
    to left;
  - a container's left margin: 55 in both;
  - a column's left padding: 55 in right to left; its start padding: 25;
  - a safe area with a 30-px cutout on the left: 55 in both directions.
- The floating button with a 40-px cutout on the left:
  - in right to left, at the start, its margin from the right edge;
  - in right to left, at the end, clear of the cutout and its margin;
  - in left to right, at the start, the same.
- The whole widget suite passes unchanged: no existing test relied on the mirrored padding.
  The 102 golden images pass too, the right-to-left ones included.
- Mutations, each failing a test: a container, a column or a safe area laying its padding out
  as if left to right; no swap of a physical inset; the button's row without the safe area's
  insets.
- Not run on the phone: the device reads left to right, and the demo has no right-to-left
  switch that shows these paddings.

## Left

- Other widgets that build an asymmetric padding from numbers of their own were read one by one
  and found symmetric. A widget added later has to choose physical or directional, and the doc
  of `padding_each` now says which it is.

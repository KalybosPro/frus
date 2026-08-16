# Milestone 313 — Five buttons, one of which had a shadow

`Button` had three variants — `Primary`, `Secondary`, `Danger` — a hardcoded 18 px label,
20 px of padding either side, 12 above and below, the theme's corner radius, and **a shadow
under every enabled button whatever its variant**.

The reference has five: filled, tonal, elevated, outlined, text. They are one box with one
label and differ in **emphasis** — which is the whole point of having five. Drawing a shadow
under all of them is drawing the elevated one five times over, and an emphasis order in
which nothing is quiet.

## What the numbers are

Read out of `filled_button.dart` and its siblings: minimum size **64 × 40**, a
**`StadiumBorder`** shape, `label_large` for the type, 24 px of horizontal padding (12 for a
text button, whose label has no box to fill), vertical padding **none** — the height comes
from the minimum, not from the padding — and elevation 0 for four of the five variants,
1 for the elevated one. Disabled is the same in all five: `on_surface` at 12 % under a label
at 38 %.

The old numbers were not versions of these. A 46 px tall button with soft corners and a
shadow is a different component.

## The names now say what they are

`Primary` and `Secondary` became `Filled` and `Outlined` — 83 call sites, all in this
repository — with `Tonal`, `Elevated` and `Text` added. `Secondary` had meant "a surface
with a border", which is an outlined button described rather than named.

`Danger` stays, and is the one variant that is not the reference's. There, a destructive
button is a filled button handed the error colours at the call site. It is a name here
because *this action destroys something* is worth saying once rather than as two colour
overrides in every application that needs it.

Ten more settings per call — `color`, `label_color`, `label_style`, `border_color`,
`border_width`, `radius`, `padding`, `height`, `min_width`, `elevation` — and a `ButtonTheme`
carrying the same ten.

## A stadium is a rule, not a number

The corner radius defaults to **half the button's height**, computed at paint time from the
box it was actually given. A button told to be 30 px tall stays a lozenge instead of becoming
a rounded box, and `radius(4.0)` still turns it into one for a caller who wants that.

## What the change exposed

Four of the framework's own buttons hold **one glyph**: the date picker's two month arrows,
the navigation bar's back arrow, the stepper's plus and minus. With a 64 px minimum width and
24 px of padding they came out as wide lozenges around a single character.

That is not the button being wrong — it is `Button` being used where the reference would use
an icon button, which this framework does not have. They now ask for the shape they want
(`min_width(40).padding(8)`, giving a 40 px circle), and the missing widget is written down
below rather than papered over with a "compact" flag that the reference has no equivalent of.

Two tests failed for a better reason than they looked:

- the app bar counted **shadows** to work out how many actions were inline. With four
  variants flat, the count is zero. A proxy that stops standing for the thing it stood for
  is worse than no test, so it counts `Role::Button` semantics now — what the buttons *are*
  rather than what they paint.
- the segmented control asserted three **opaque** fills. Its unselected segments are
  outlined, and an outlined button is now an outline over nothing — which is also what the
  reference's segmented button does. The assertion looks at the boxes, not their alpha.

## Verification

1041 tests, clippy silent, rustdoc clean, and **22 goldens re-blessed**. They fall into one
diff: boxy shadowed buttons became flat stadiums, outlined ones took the accent for their
label, and the filled ones lost their shadow. Five were read closely — a disabled pair, the
form wizard, a date picker, the bulk-action bar, a kanban board — and the rest were checked
to be the same change in the same widget before being accepted.

## Left

- **No icon button.** The reference's `IconButton` is 40 × 40, circular, with no minimum
  width and an icon rather than a label. Four call sites in this repository want one, and
  every application that puts a glyph in a button will want one too.
- **No leading icon.** `FilledButton.icon` puts an icon before the label with 8 px between;
  `Button` takes a `String` and nothing else.
- **The elevated button does not lift on hover.** The reference goes from 1 to 3 while the
  pointer is over it, and back on press. Here the elevation is fixed.
- **`SegmentedControl` is next.** It builds itself out of buttons, and the reference's M3
  segmented button is `secondary_container` with a checkmark on the selected segment — not a
  filled primary button. The same finding as `Tabs`, in the widget beside it.

# Milestone 425 — The two the family was missing

Milestone 424 left two names on the roadmap: `SimpleDialog`, which the reference builds on
the same surface, and `MaterialBanner`, which is the widget `Alert` resembles without being.

## The three ways of saying something

Naming them together is the point, because each is a different promise about how long the
message stays:

| | how long | where |
|---|---|---|
| `SnackBar` | a few seconds | over the bottom |
| **`MaterialBanner`** | until an action is taken | in the flow, above the content |
| `AlertDialog` | until it is answered | over everything, with a barrier |

frus had the first and the third. The middle one is the one an application reaches for when
something is wrong and the reader has to decide, but not right now.

## `MaterialBanner`

Its actions are **required** in the reference (`banner.dart:109`), and the reason is the
promise in that table: a message that stays until it is dismissed and offers no way to
dismiss it stays for ever.

Two things about it are conditional, and both are the reference's:

- **One action rides on the message's line; two take a line of their own**
  (`banner.dart:348`). That is also why its padding has two shapes — a tucked-in bar needs
  almost no top padding, one below needs the message to breathe above it.
- **A rule only when the banner is flat.** Off the page it casts a shadow instead
  (`banner.dart:425`). Height or a line, never both — the same either-or `Card` already
  makes between a shadow and a hairline.

The elevation decides a third thing quietly: a banner off the page keeps ten pixels of
margin under it, so its shadow has somewhere to fall.

## `SimpleDialog` and `SimpleDialogOption`

The difference from `AlertDialog` is what it is *for*: an alert dialog asks a question and
puts the answers in a row of buttons; this one lists them, and **each row is an answer**.

Which is why the list has no side padding (`dialog.dart:1171`) while each option has 24 of
its own (`:1082`). The row is the tappable thing, and its ink has to reach both edges of the
dialog; padding the list instead would inset the ripple. There is a test for exactly that
arithmetic — the option's text starts one option padding in from the surface, not one plus a
list one.

The title drops its bottom padding when options follow (`:1316`), the list having a top
padding of its own. Two of them stacked would be a gap nobody asked for.

## Three things the tests found

**`Divider::new().height(0.0)` draws nothing here.** The reference's `Divider(height: 0)`
means the line with no air around it; in frus the height *is* the room the separator takes,
so zero takes none. The flush hairline is `height == thickness`, which this crate's own
`Divider` documentation says in as many words — and which the first draft of the banner did
not read.

**A dialog's panel is its overlay, not its child.** The first version of the simple dialog's
test walked `Dialog::children()` looking for the options' messages and found none, because a
modal is drawn *over* the screen rather than in it: `children()` is `[body]`, and the panel
comes back from `overlay()`. The test asserts on the right tree now, and the failure was the
widget being correct rather than the test.

**And a weak assertion was hiding a real bug.** The first version of
`the_message_takes_what_is_left_of_the_line` only asserted that the action's text was drawn
to the *right* of the message's — which "Save" after "A message" satisfies whether or not
anything is laid out correctly. Strengthened to "the action reaches the banner's trailing
edge", it failed at `x = 101` on a 400-wide banner: the padding around the message row was a
`Flex::row`, and a row's child sits on its main axis at the width of its own content, so the
message row hugged and the `Expanded` in it had nothing to expand into. A column stretches
its child across instead. The bug was real, the widget was wrong, and the first assertion
would never have said so.

No golden moved (91 + 13 + 27).

## Still open in the family

On `AlertDialog`: `scrollable`, the actions' overflow family, `buttonPadding`,
`semanticLabel` / `namesRoute`, `alignment`, and `paddingScaleFactor`. On `Dialog`:
`Dialog.fullscreen`. On `MaterialBanner`: `overflowAlignment` (the actions do not wrap yet),
`animation` and `onVisible`, and the reference's clamp on the content's text scaling.

`MaterialBanner`'s Material 3 background is `surfaceContainerLow`, a tone lower than this
scheme carries; `surface_container` is the nearest it has, which is the same substitution
`Card` records for its elevated variant. The scheme wanting one more container tone is worth
a step of its own.

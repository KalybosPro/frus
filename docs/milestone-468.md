# Milestone 468 — The top of the panel, a rule that goes the other way, and a gap that is not a guess

Milestone 467 gave `NavigationDrawer` a `header` slot and there was nothing to put in it.
This is the four things that were missing around it — two the reference puts in that slot,
and two the framework had been doing without for 467 milestones without anybody writing it
down.

## `DrawerHeader`

A block of a fixed height at the top of a panel, with a rule under it. The height is the
reference's `160.0 + 1.0` (`drawer_header.dart:16`), and the `+ 1` is worth keeping
visible: **the rule is the header's last pixel**, not a line under it. A header and its
rule are 161 together and the thing below starts at 161, which is why a panel never has a
one-pixel seam in it.

### The notch lands here

A panel runs to the top of the screen, so the first thing in it is the thing under the
status bar. The intrusion goes on the **height and on the padding** — both, not either
(`drawer_header.dart:86`, `:90`):

- on the height, so the block's background runs up behind the status bar;
- on the padding, so the content stays the same 160 pixels tall underneath it.

Doing one and not the other is the mistake, and it looks perfect on a desktop.
`the_notch_is_added_to_the_height_and_to_the_padding` holds both halves; breaking either
one fails it.

## `UserAccountsDrawerHeader`

The same block laid out for the account the application is signed in to: a picture in one
corner, up to three others in the opposite one, a name, an address, and a control for
switching.

### The address is level with the control, and the name is above it

The obvious reading of "a name over an address in a 56-pixel row" is *centre the pair*. The
reference does something else: it places the **bottom line's** centre at the row's centre
and puts the name above it (`user_accounts_drawer_header.dart:262`), overflowing upward
into the pictures.

That is about ten pixels different, and the reason is the case where there is no name at
all — then the address *is* the only line, and it has to be in the same place. Anchoring
the bottom line rather than the pair is what makes those two arrangements agree.
`the_address_is_level_with_the_control` pins it in both.

### Three struts, and one that had to go

The row is 56 tall in the reference and the name simply overflows it; a hand-written
layout does not mind. Here the boxes are real, so the row has to grow and let the pictures
give up the difference.

The first attempt computed the height: `max(56, name + email + lift)`, with the line
heights from the type scale. It overflowed by **0.600 pixels** and said so in a yellow band
across the header, because the type scale's `line_height` and what the text actually
measures are not the same number. Measuring the strings instead got closer and still
overflowed by the same 0.600 — the laid-out box is not the measurement either.

The fix is not a better prediction. A **zero-width child of the right height** floors the
row, and flexbox does the `max` without anybody predicting anything:

```rust
Flex::row()
    .align(Align::End)
    .child(Flex::column().width(0.0).height(DETAILS_HEIGHT))   // ← the strut
    .child(Expanded::new(lines))
```

Two arithmetics that have to agree are one arithmetic too many. This is the second time in
two milestones that a number from the reference could not be copied as a number — 467's
indicator width was a ceiling, and this row's height is a floor.

### `arrowColor` is not white

The reference defaults it to `Colors.white` (`user_accounts_drawer_header.dart:301`), which
is right for the dark primary a 2014 palette had and wrong on a light one. This framework
has a role that means *what goes on primary*, so the default is `on_primary`. Naming a
colour still wins.

### The whole block is one thing to a reader

`Semantics` around it with the localizations' `signed_in_label`
(`user_accounts_drawer_header.dart:359`) — otherwise a name, an address and an arrow
arrive as three unrelated nodes at the top of a panel and none of them says what it is the
top of.

Three entries join the table: `signed_in_label`, `show_accounts_label`,
`hide_accounts_label`. **Two for the control rather than one that flips**, as the reference
has it: a control is named for what pressing it *will do*, and the two sentences are not
each other's negation in every language.

The reference rotates one filled triangle through half a turn. There is no such glyph here
and there is no rotation in a static build, so it is `ChevronDown` and `ChevronUp` — the
two ends of that turn, both already drawn on the grid.

## `VerticalDivider`

Every separator in the framework ran across a column. A row had nothing.

It is the same widget turned ninety degrees, and the box is where that shows: one declares
a height and lets the column stretch its width, the other declares a width and lets the row
stretch its height. They read the **same theme field** — `DividerThemeData.space` there,
`DividerTheme::height` here — so an application that wants its rules tighter says it once.
That is also the only reason the field is not called `width` on one of them.

### It stretches on its own say-so

The first picture of it was a row with no line in it. A row that centres its children gives
this one the height of its content, and its content is nothing — so a rule in a centred row
is a rule nobody can see, which is the first thing anybody hits.

It now asks for the cross axis itself (`align_self: Stretch`), the way the reference's
`Center` inside a `SizedBox` fills whatever height it is offered. The horizontal one has no
such problem: a column stretches it by default, and a column that does not is a column with
a reason.

## `Spacer`

A gap that **takes what is left**. A fixed box between two things is a guess about the
parent's width; this is not a guess, and two of them place a child anywhere along an axis
without measuring anything.

It is `Expanded` around nothing (`spacer.dart:58`) — the same three properties, written out
rather than composed, because a wrapper with no content is a wrapper whose whole
implementation is its box. The three go together: a basis of zero, the grow factor, and
**no automatic minimum**, which is the one that is always forgotten and without which
`flex: 1` is a no-op.

The reference's `flex` is an `int` asserted to be at least one. This takes an `f32` and
does not assert: a spacer at `0.5` beside one at `1.0` is a two-thirds mark, which is a
thing to want and which whole numbers can only say by getting larger.

The account header is its first caller — the current account's picture, a spacer, the
others — which puts the two groups in the corners and lets the direction decide which
corner is which, where the reference stacks them at `top: 0` and `end: 0`.

## What the pictures found

Both goldens were wrong the first time and both said so.

`drawer_account_header` came out with the overflow band described above, and with the
avatars invisible: `CircleAvatar` defaults its fill to `theme.primary`, which is exactly
the colour the account header paints behind it. That is not a bug in either — it is what a
caller has to decide — but it is only visible in a picture.

`row_rules_and_space` came out as `Drafts Sent12` with no rule and no gap: the row was as
wide as its content, so the spacer had nothing to take, and the divider had collapsed for
the reason above. One of those was the test's fault and one was the widget's.

## What this turned up

**A box cannot tell its child how big to be.** The reference sizes the account pictures
with `SizedBox.fromSize`, which passes *tight* constraints down, and a `CircleAvatar` under
them is 72 across whatever it thinks. Here every widget sizes itself from its own `style()`
and a parent cannot overrule it: `current_picture_size` reserves the corner's room and the
picture keeps whatever size it was built with. It is on the roadmap.

## Verification

`cargo fmt`, clippy across the workspace with all targets and all features: silent.
`RUSTDOCFLAGS='-D warnings' cargo doc`: silent. **1271 unit tests**, all green — fifteen of
them new, three of which were checked by breaking the thing they guard and watching them
fail. Goldens **91 + 32 + 14**, two pictures added and none moved.

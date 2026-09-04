# Milestone 465 — A row whose whole width works one control

Three of the reference's widgets, none of which existed here:
`CheckboxListTile`, `RadioListTile`, `SwitchListTile`
(`checkbox_list_tile.dart` and its two neighbours).

Every part was already in the crate. `ListTile` takes two slots and a tap; `Checkbox`,
`Radio` and `Switch` are all there. What was missing is the thing that says *this row and
this control are one control*.

That is not a convenience. **A settings screen where only the twenty-pixel box answers,
and the label beside it does nothing, is a screen where most of the taps go nowhere** —
and the person tapping does not conclude that they missed, they conclude the app is
broken. It is the single most common row in any application with settings in it.

## A radio could only exist inside a group

`RadioListTile` needed a radio, and there was none: `RadioOption` was **private**, built
by `RadioGroup` for each of its labels, and unreachable from anywhere else. The
reference's `Radio` is a widget in its own right — which is what lets one sit in a list
row, a table cell, or anywhere the group's fixed column of labels is the wrong shape.

`Radio` is now public, with its own builders for the four colours, the label, the tap
target and `on_select`. `RadioGroup` still builds it by struct literal from inside the
module, so nothing about the group changed.

It reports **being pressed** and says nothing about what the answer becomes — this
framework's message model already has the application holding the choice, so a value and
a group value would be two copies of the same fact.

## What the three share

Nearly everything: the title, the subtitle, the `secondary` slot, the affinity, dense,
three-line, selected, enabled, the two surfaces, the shape, the padding and the two type
styles. Written once as a private `Row` plus a `row_builders!` macro, so the three faces
differ only in their control and its colours.

**`control_affinity` is trailing by default**, as the reference's is: a column of labels
reads down the leading edge and a column of controls down the other. `secondary` takes
whichever slot the control did not.

## Two decisions

**The control answers with the row's message, ignoring the value it computed.** A
`Checkbox` can say `on_change(|next| …)`; here it says `on_change(move |_| message)`. The
caller already said what a change means, and a control tile's control is not the thing
deciding what the next value is — the application is.

**Disabled, neither answers.** There are two guards for this, one on the row and one on
the control, and it takes removing *both* to change the behaviour — which is what
`a_disabled_row_answers_nowhere` actually holds, and worth saying plainly rather than
claiming the test proves each guard separately. A row that still reported a tap while its
control refused one would be two controls in one row disagreeing about what they are.

## The crate's own guard found the macro

The three `Widget` implementations are near-identical, so they were written once as a
`macro_rules!`. The suite went red on a test in another module:

```
these look available to the tap, the tab or the reader while disabled:
["controltile.rs::fn on_click( (byte 9249)"]
```

`every_control_with_an_enabled_flag_honours_all_four` (milestone 322) reads the **source**
of every module carrying an `enabled` flag and checks that each of the four hooks —
`on_click`, `ink`, `focusable`, `semantics` — consults it. The code was correct: the
wrapper's `on_click` returns a bare `None` because the row answers and not it, which the
guard normally recognises. What it could not recognise is a hook inside a macro body,
where its rule for finding the end of a function does not hold.

So the macro had put this module in the one place the net does not reach. **A safety net
a widget can hide from is a net with a hole in it**, and three copies of twenty lines is
the cheaper of the two prices. Written out, with the reason in a comment above them so
the next person does not helpfully fold them back up.

## The tests

Six. Four of them fail when the milestone is undone, checked by taking the row's wiring
away, ignoring the affinity, and removing both disabled guards.

`a_tap_anywhere_on_the_row_works_the_control` is the milestone: it taps the **words**, 220
pixels from the box, and expects the message.

## The picture

**A new golden**, `control_list_tiles`: a switch tile with a subtitle, a checkbox tile, a
tristate "select all", and two radios — leading and trailing affinity both in the frame,
which is the only way to see that the choice is a choice.

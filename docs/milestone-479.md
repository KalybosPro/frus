# Milestone 479 — A dropdown's choices, and the control that was missing beside it

Closes [#35](https://github.com/KalybosPro/frus/issues/35).

Two pieces, and the issue is right that the first is much the smaller.

## Choices that are not words

`DropdownButton::options` took `&[&str]`, which rules out most of what a dropdown is
actually asked for: a colour swatch beside a name, a flag, a two-line entry with a
subtitle, a choice that is there and **cannot be picked**.

`DropdownOption` carries all four, `options_widgets` takes them, and `options` is unchanged
— it now builds the same values from its strings, so there is one path and not two.

**A choice carries no message**, unlike a menu item. Both `options` and `options_widgets`
map the chosen *index*, which is what a list built from an application's own vector wants:
the alternative asks the caller to write the same message once per row and to keep the two
orders in step by hand.

**A choice that cannot be picked is drawn rather than left out**, greyed and announced as
unavailable, and still ticked when it is the selected one. That last part is the reason: a
list that hides what it cannot offer cannot show what was already chosen, and a set can
stop being available *after* something in it was picked.

The tick's column is kept clear on **every** row rather than on the ticked one, so a choice
the caller drew does not change width the moment it becomes the selection.

## `DropdownMenu`

The Material 3 control that looks like a text field, filters as you type, and drops its
choices underneath. It did not exist here. `Autocomplete` is the nearest-looking thing and
is a different control — it *suggests* over free text, where this has a **closed set** and
a selected member of it.

### One rule shapes the whole widget

**A shut field shows the choice that is selected, and nothing else can appear there.** The
query is displayed only while the menu is open.

The issue names the hard part exactly: the filtering "must never leave the field showing
something that is not a member of the set". The obvious arrangement — the field shows the
query at all times, and the application puts the real value back when the menu closes —
leaves that rule as something a caller can forget, in the one place where forgetting it
means the control has lied about its own value.

So the display is **derived**, not passed in. A reader who types three letters, thinks
better of it and clicks away is looking at their actual choice again the moment the menu
shuts: no message sent, no state restored, nothing for an application to remember. The rule
is not enforced by a check; there is no way to express the broken state.

It costs one thing, and it is the right price: a choice the caller *drew* has no words, so a
shut field showing one falls back to empty rather than to the placeholder — something is
selected and "choose one" would be false.

### Filtering

A case-insensitive **substring**, not a prefix. `"Dark green"`, `"Light green"` and
`"Sea green"` filtered by `green` on a prefix rule answer nothing at all, which reads as a
broken control rather than as a rule. An empty query is not a filter yet and matches
everything.

**`on_select` is given the index into the caller's own list**, never into what the filter
left showing. That is the classic bug in this control, and it is invisible: both indices
are valid, so it works perfectly until somebody types, and then quietly picks the wrong
thing. The test names both rows for that reason — an assertion about the second row alone
passes just as well when it is hitting nothing.

A choice with no words is **not** filtered out. It has nothing to match against, and hiding
it would make the swatches vanish as soon as anybody typed; a filter that removes what it
cannot read is worse than one that keeps it.

A query matching nothing floats **no panel**. An empty surface hanging under a field is a
control that looks broken.

### Filtering is on by default, which the reference's is not

`DropdownMenu.enableFilter` is off in the reference. Here it is on, because filtering is the
reason to reach for this control rather than `DropdownButton` — the same closed set behind a
button — and a caller who does not want it already has that widget. Off, the field is
**read-only**: one that accepts typing which changes nothing looks broken.

### What it does not own

The application holds the query, the open flag and the selection, like every other
controlled widget here. The chevron sends `on_toggle` and turns over with the menu, which
on a shut field is the only thing saying there is more of it. Typing sends `on_input`; an
application will normally keep the menu open there, which is what the demo does.

The rows are `dropdown.rs`'s own, through one shared constructor. Two lists that look nearly
alike are worse than one that looks the same, and a tick that moved in one and not the other
is exactly the drift that gets shipped.

## In the demo

The Settings screen has both, one under the other: the same eight cities are a closed set
whichever way you reach them. Three of them share a word that none of them starts with, so
the substring rule is visible rather than asserted. The query is cleared when the menu
**opens**, not when it closes — a list that opens already filtered by what was typed last
time looks broken — and there is nothing to reset on the way out, which is the derived
display paying for itself in an application rather than in a test.

## Not here

- **`DropdownButtonFormField`.** The issue says it follows from `Form` once the above
  exists, and it does; it is a different piece of work and belongs with whoever is next in
  `Form`.
- **A group heading in the list.** `DropdownOption::widget` will draw one, but it would
  still be focusable and clickable, which a heading must not be. Doing it properly is the
  same shape as `MenuItem::divider` from milestone 478 and should be that, not a widget
  pretending.
- **Opening on a press anywhere in the field.** The chevron opens it. A press on the text
  places the caret, which is what a field is for, and there is no hook here for a widget to
  claim both.

## The pictures

`dropdown_widget_options`: a swatch, a two-line entry, a greyed choice and a plain one, with
the tick column lining up down all four. `dropdown_menu_filtering`: the field open with
`gre` in it and the two greens under it — which is also what says the rule is a substring.

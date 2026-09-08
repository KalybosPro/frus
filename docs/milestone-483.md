# Milestone 483 — A second language, and the words that could not reach one

Closes [#26](https://github.com/KalybosPro/frus/issues/26).

Milestone 449 built the machinery: a `Localizations` table, installed for the thread, read
by `of()`, resolved against the platform's preferred locale and installed by the shell
every frame. Everything worked except the part a reader can see. There was **one table**,
and it was English.

## Two halves, and the second one was not in the issue

The issue asks for a language. Writing one found the other half: **five widgets said their
own English out loud** where no table could reach it.

| where | what it said |
|---|---|
| `datepicker.rs` | "Previous month", "Next month" — the labels on the arrows either side of the month |
| `stepper.rs` | "Less", "More" — the only thing a minus glyph and a plus glyph say to a screen reader |
| `table.rs` | "Row selected", "Row deselected", "All rows selected", "All rows deselected" |
| `datatable.rs` | "Search" in the search field, "No results" when there is nothing to show |
| `timepicker.rs` | "Hour", "Minute", "AM", "PM", "Start", "End" |

Sixteen entries, none of them reachable, several of them the first thing a screen reader
says about the widget. Shipping a French table without them would have produced exactly the
half-measure the issue warns about — a French interface with an English calendar in it —
and the golden would have shown it.

The trait declares **29** entries now. Every one of them has an English body, so nothing
already written changed: an application that says nothing is exactly where it was.

## `French`

Written rather than generated, and by someone who speaks it. A machine-translated table is
worse than none at all: it silences the question for that language and leaves a native
reader with something subtly wrong and nobody looking at it.

Three things in it are **not translation**, and they are the three a translation of the
strings alone leaves behind:

- **The week starts on Monday.** `first_day_of_week_index` is `1`. A calendar that always
  began on Sunday was not untranslated — it put every day in the wrong column.
- **The months are lower case.** "janvier 2026" is correct French and "Janvier 2026" is
  not. It looks like a bug to an English eye, which is why it is worth saying here.
- **The four selection sentences are four sentences.** *Ligne* is feminine, so the
  participle takes an `e`, and an `s` as well once there are several of them. A formula
  that glued *row* to *selected* would have been right in English and wrong here — the same
  reason `show_accounts_label` and `hide_accounts_label` were already two entries rather
  than one that flips.

And three entries are deliberately identical to the English ones: **"OK"**, **"Minute"**,
and **"AM"/"PM"** — French tells the time on a twenty-four hour clock and has no everyday
words for the halves of a twelve-hour one, so the reference's own French table keeps the
Latin abbreviations too. They are **written out** with the reason beside them rather than
left to the trait's default, because a default is not agreement: silence and "the English
answer is right here" look identical in the source and mean opposite things.

## Two tripwires

The rule this module needs is *no entry left silent*, and no test that checks the entries
someone remembered can enforce it. So one test **reads this file**, takes the method names
the trait declares and the ones the `French` block answers, and fails on the difference —
the same shape as the macro tripwire in `transparent.rs`. The other compares every French
string against the English one and fails on a match outside the three above, which is what
catches an entry "answered" by copying.

Both were checked by breaking them: deleting `end_label` from the French table fails both,
for two different reasons.

A third test walks the five rewired widgets and looks for French on the far side — through
the scene and the accessibility tree, not through the table it has already tested.

## When the words are read

A widget composes its children **when it is constructed**, so it says the words in force at
that moment, not the ones in force when it is painted. In an application that distinction
does not arise: the shell installs the table every frame, before the view is built. In a
test it does, and the first version of this milestone's golden proved it by coming out in
English — the picker was built outside the scope and rendered inside it. Both the golden
and the routing test now build inside.

## In the demo

The language menu already cycled English, Français and العربية, switching the
application's own Fluent resources. It now hands the framework a table as well, so the
calendar's month, its columns and its first day switch with everything else.

**Arabic gets English**, deliberately and not by omission — there is no Arabic table yet,
and this milestone will not invent one by machine. Its layout still mirrors.

## The picture

`date_picker_french`: January 2026 as **janvier 2026**, over columns reading L M M J V S D,
with the 1st under J. The first day of the week is the half of a translation that a unit
test on an index cannot show has actually moved the grid.

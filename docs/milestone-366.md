# Milestone 366 — A field with a limit, and one that is read but not written

Two of `TextField`'s gaps, both of them things an application reaches for on its first
form.

## `max_length`

Enforced, which is the reference's default and the only behaviour that keeps a field and
its counter telling the same story.

The interesting half is what happens to a **paste** that crosses the limit: the part that
fits lands, and the rest does not. Refusing the whole paste is the tidier rule and the
worse one — it loses work the user can see they had, for the sake of a boundary they can
see too. Refusing the keystroke that crosses the limit is the same rule seen one character
at a time.

It counts **characters**, not bytes. "é" is one, and a name written in an alphabet that is
not Latin is counted the way its writer would count it.

A value the **caller** supplied over the limit is left alone. It is the application's
state, not something typed, and quietly shortening it would be editing a value nobody
edited — the counter reads `7/5` and says so.

The counter sits at the far end of the line below the box, where the reference puts one,
and reserves that line even when there is no helper text to share it with. It takes the
helper's colour rather than the error's even while an error is showing: it is a fact about
length, not a second complaint.

## `read_only`

Not `enabled(false)`, and the difference is the point.

A **disabled** field is greyed out and inert: out of the tab order, no caret, nothing to
select. That says *unavailable*.

A **read-only** field looks and behaves like any other field except that typing does
nothing. Everything that only *moves* still happens — the caret, the word jumps, the
selection — so a reference number the application generated can be focused, selected and
copied. That says *fixed*, which is the truth about an identifier nobody is meant to edit.

The implementation is one line in the right place: the refusal happens **after** the edit
walk has moved the caret and **before** the change is emitted. Refusing earlier would have
frozen the caret too, which is the disabled field again under another name.

## Left on the field

`text_align`, `input_formatters`, and the mobile keyboard hints — `keyboardType` and
`textInputAction`, which decide whether a phone shows a number pad and whether its Enter
key says "next" or "done". Those reach the shell rather than the widget, and want a step
with the Android side in it.

# Milestone 389 — What a field lets through, and what the keyboard offers

A quantity field took letters. A code field took lower case. A field that refused
everything but digits still opened a full QWERTY keyboard, because nothing connected the
two.

`input_filter`, `digits_only`, `capitalization`.

## One character at a time, and that is the decision

The reference's `inputFormatters` reshape the **whole value** after every keystroke. That
buys grouping — spaces every four digits of a card number — and costs a caret: the
formatter has to say where the cursor went, and getting it wrong puts the cursor in the
middle of a group the reader never touched.

`input_filter` is `Fn(char) -> Option<char>`: `None` drops the character, `Some(c)` puts
`c` in instead of what was typed. It cannot group. It also cannot lose the caret — a
dropped character never arrives, and a substituted one takes exactly the place of the one
typed, so the caret arithmetic in `on_edit` is the arithmetic it always was.

That covers digits-only, letters-only, no-spaces and forced case, which is nearly every
field that filters at all. Grouping is left, and it is left knowingly.

## A refused keystroke is not an edit

Typing a letter into a digits-only field used to be handled by the `Key::Text` arm setting
`changed = true` unconditionally. With a filter that would emit `on_input` with a value
identical to the one already there, rebuilding the whole tree for a key that did nothing.

So the arm now asks whether anything happened. A **selection** counts even when what
replaces it is empty — the reference filters the value *after* the replacement, so the
selection really is gone — and a bare refused keystroke does not.

Two tests, one for each half.

## The caller's value is not typed

A filter applies to what is typed or pasted, never to the value the application supplied.

Same rule `max_length` already followed, for the same reason: it is the application's
state, and rewriting it would be editing a value nobody edited. A test pastes a digit into
`"a1b2"` and gets `"a1b23"` back.

A paste keeps whatever it can rather than being refused whole — again `max_length`'s rule,
because losing work the reader can see they had is worse than keeping some of it.

## Digits ask for a keypad

`digits_only()` also names `KeyboardType::Number`, but only when no keyboard was named.

A field that refuses everything but digits and then opens a QWERTY keyboard is a field
whose keys mostly do nothing. Filling in an unset choice rather than overwriting one means
the two builders can be written in either order, which is what a reader of the call site
would assume, and a test says so both ways round.

## Capitalisation replaces, never adds

`Capitalization::{Auto, None, Sentences, Words, Characters}`, and `Auto` is the default.

The keyboard types already carry capitalisation — ordinary text capitalises sentences, a
name capitalises words, an email address capitalises nothing, and that last one is a bug
avoided rather than a detail. So an explicit choice **clears** the type's bits before
setting its own: two capitalisation bits at once is a keyboard being told two things.

`Auto` leaves them alone, which is why every existing type answers exactly what it always
did — a test walks all of them and asserts that.

Only a text class capitalises at all. `0x1000` is *signed* on a number class, so asking a
phone field for capitals would quietly turn on a minus key. A test walks the keypads too.

The composition moved from `KeyboardType::android_input_type` to `Ime::android_input_type`,
since it is now a question about the pair rather than about the type. The checked-in dex
still receives two integers and needs no rebuild, which was the point of computing these
in Rust in the first place (milestone 380).

## Left

Grouping formatters, and `TextField.inputFormatters` as a list. `autocorrect` and
`enableSuggestions` are still only reachable through the keyboard type.

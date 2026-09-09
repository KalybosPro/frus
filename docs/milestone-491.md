# Milestone 491 — Ctrl+Z in a text field

Closes [#28](https://github.com/KalybosPro/frus/issues/28).

A text field had no undo. Typing, a backspace held down, a paste, a cut, a whole selection
replaced by one character — all of it was one-way, and Ctrl+Z was not bound to anything.
This is the shortcut people press without deciding to, and its absence reads as the field
being broken rather than as a feature being missing.

## The stack is fifteen lines; the rule is the milestone

What makes undo useful rather than merely present is the answer to **what counts as one
step**, because a history that steps back one character at a time is nobody's idea of undo.
The rule here is a **run** of the same kind of change, ended by anything that is not more of
the same:

- **Typing** joins the run, so a word is one step. A character that is not a word character
  — a space, a full stop — joins the run *and closes it behind itself*, which is what makes
  undo step back word by word rather than sentence by sentence.
- **Deleting** is a run of its own. A held backspace is one step, and it does not join the
  typing it is taking back by hand.
- **A pause of half a second** breaks a run, whatever it was. That number is the
  reference's, chosen there as a best fit for what Windows, macOS and Linux each do; there
  is no better argument for it than that, and no worse one.
- **A chunk** — a paste, a cut, a selection replaced, a line break — is always its own step
  and joins nothing on either side. That is the issue's own list, and the four have one
  thing in common: each is a single deliberate act whose whole result the user can see.
- **A composition** — the provisional text an input method underlines while a word is being
  chosen — is one step for the whole word, and **the clock does not apply to it**. The
  rhythm of an IME's revisions is the IME's, not the typist's, and a slow suggestion list is
  not a pause in the writing. This is the issue's third question, answered.
- **Moving the caret closes the run.** Nothing is recorded, because nothing changed, but
  what is typed next is a step of its own: typing here, then there, then Ctrl+Z should not
  take back both visits at once.

A step is the value **and** the caret. An undo that restores the text and leaves the caret
at the end has done half the job, since the point of undoing is to carry on from where you
were.

## Recorded on the evidence, not on the intent

The obvious implementation snapshots when a key that changes the value arrives. That is
wrong four ways over, and each of them is a real field in this framework: an input filter
may refuse the character, a length limit may swallow it, a read-only field ignores it, and
an **Enter submits instead of typing**. Recording on intent gives a history with entries
that undo to the state they are already in — which looks, from the outside, exactly like
Ctrl+Z being broken.

So the shell reads the field's value on both sides of the keystroke and records only when
the two differ. That is also how the reference does it: its `UndoHistory` listens to the
*value*, not to the keyboard.

It has a consequence worth stating. The demo clears its draft field when Enter adds the
task, and that change happens *during* a keystroke in the field — so it is recorded, and
Ctrl+Z brings the typed text back. Pressing Enter too soon is undoable. A value the
application changes on its **own**, outside any keystroke, is not recorded at all; the
reference would record that too, and here an undo that reverted something the application
did would be a surprise rather than a mercy.

## The value is the application's, so an undo is a message

A field owns its caret; the application owns its text. An undo therefore cannot reach in
and set what is on screen — the next frame would paint the value the application still
holds. It goes back through a message like any other edit, which needed one new hook:

```rust
fn replace_value(&self, value: String) -> Option<Msg>
```

*The message that puts `value` in this field: the one typing would have produced, for a
value the framework restores rather than the user typing it.* A field makes the same two
refusals it makes for a keystroke — disabled, read-only — and **not** the other two: the
value being restored passed the filter and the length limit on its way in, and running it
through them again would let an undo land somewhere the field has never been.

## Where it lives

The history and the rule are in `frus-widgets` (`undo.rs`), beside `Edit` and for the same
reason: it is state the runtime keeps for a widget that is rebuilt every frame. The shell
drives it, because the shell is where a keystroke is a keystroke — it owns the clock the
pause is measured on, and it is where Ctrl+Z is bound.

Both spellings of redo are bound: **Ctrl+Y** and **Ctrl+Shift+Z**. Both are in use, and
people bring the one their hands already know.

## What is covered, and what is not

Nine tests on the rule, and five that drive it through a **real field** the way the shell
drives it — the same four steps `apply_key` takes, with an application holding the value in
between. That seam is where everything that decides whether undo is right actually happens:
a rule tested only on its own side of it is a rule that works in a test.

What is not covered is the shell's own plumbing — that Ctrl+Z reaches this at all, and that
the restored value is dispatched. Nothing in this repository can drive a frame; that is the
roadmap entry this keeps running into, and it is the same gap milestone 416 dented and did
not close.

There is no golden. Undo has no picture: a field after an undo looks exactly like a field
that was typed into, which is the whole point of it.

# Milestone 510 — A word written twice, and the pixel that did it

Found on the device during milestone 509: typing `frusclip` into the demo's field with
SwiftKey left **`FFFrusclip`**, while the keyboard's own suggestion strip read `FFrusclip`
— the field and the keyboard disagreeing about what had been written, by a letter. A
second try gave `FFrFFrsclip`.

## What the keyboard sent

A temporary trace of every operation the bridge received, on the device:

```
Composing("F")
Composing("Fr")        ← the field already held "F": written beside it, "FFr"
FinishComposing
Composing("Fru")       ← after the keyboard reclaimed "Fr", written beside it again
…
```

A keyboard that predicts writes a word as a **composition** — provisional text it replaces
on every keystroke until it finishes the word. The field had it right up to the second
keystroke and then wrote the second composition *next to* the first.

## Three faults, one after another

1. **The composition was a count the shell kept.** It was erased with that many
   backspaces from the caret, and zeroed by the paths that ask for the keyboard again —
   which had nothing to do with the composition. Once zeroed, the next update erased
   nothing.
2. **What zeroed it was a question asked of the wrong thing.** Whether the focused widget
   takes typing was answered by a **caret hit test at the corner of a field one pixel
   wide**. The demo's field shows a clickable × — a clear button — once it holds text, and
   a click on that suffix places no caret. In a field one pixel wide the suffix covers the
   pixel. So the first letter made the ×, the × made the answer *no*, the shell closed the
   keyboard and forgot the composition, and the keyboard's next update was written beside
   the first.
3. **The keyboard was never told.** Its `setComposingRegion` — used to reclaim a word it
   had finished, to go on predicting it — was dropped by the bridge, and nothing reported
   the field's changes back: no `updateSelection`, so a keyboard whose model had slipped
   stayed slipped.

## The fixes

- **The composition is the field's own** — `Edit::composing`, the range the field already
  underlines — and every operation is planned against it: select the range, type over it.
  `crate::ime::plan` turns one operation into steps the shell carries out through the
  field's ordinary editing, and the range after a composition is taken from where the
  caret ended, so a character the field's filter refused is not counted as composed. The
  count is gone.
- **The keyboard is wanted by a widget that takes typing**: `wants_keyboard` asks the
  widget — a text value, and focusable — instead of a hit test with invented geometry.
- **The bridge forwards `setComposingRegion`**, converted from Java's UTF-16 units to the
  field's characters, and **reports every change with `updateSelection`**, as an Android
  editor does. A keyboard restarted for a new field finds no composition left over from
  the last one.

## On the device

Huawei STK-L21, Android 10, SwiftKey: `frusclip` typed into the same field. The trace shows
eight compositions, each replacing the one before (`(0, 1)` up to `(0, 7)`), and no restart
of the keyboard; the field reads **`Frusclip`**, underlined as it is composed, and the
keyboard's suggestions are for the same word. The temporary trace was removed afterwards.

## Verification

- Ten tests of the planning, replaying the device's sequences against a model that edits
  the way `TextField` does: each composition replaces the one before; a composition after
  written text starts at the caret; a commit replaces the composition; a word the keyboard
  reclaims is typed over; finishing keeps the text; an empty composition takes it away; a
  `\n` commit submits; a refused character is not counted as composed; the keys and
  actions a keyboard sends are the field's keys; UTF-16 positions land on the right
  characters, an emoji included.
- One test of the trigger, on the device's own field: text and a clickable ×.
- Six mutations, each failing the test meant for it: the composition not selected before
  it is typed over; a reclaimed region ignored; the composition counted from the text sent;
  Java's positions taken as character positions; the keyboard asked for through the caret
  hit test again; a disabled field wanting the keyboard.

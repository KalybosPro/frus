# Milestone 570 — The arrows move the caret in a field that has text

Reported as "I cannot move the cursor in an input field with ← and →". On every platform, in any
text field with something in it.

## What was wrong

Left and right are offered to the focused widget first; if it does not take them, the shell moves
the **focus** to the next control in that direction. Whether the widget takes them as a text field
was decided by a caret probe — `cursor_at(0, 0, 1.0, 0)`, a hit test at the corner of a field
one pixel wide. A field that holds text shows a **clear button**, and the button covers that
corner, so the probe answered "no caret goes here". The field was not a text field, and the arrow
moved the focus out of it to the neighbouring control. Traced in a browser: the first Left took
the focus from the field to the *Terminées* filter, the second to *Actives*, the third to
*Toutes*.

In an **empty** field, which has no clear button, the probe finds a caret and the arrows worked —
which is how it went unseen, and why it is not a web bug: it is the shell's, on the desktop and
with a keyboard on Android too. Milestone 510 had already met this exact trap and got the
soft keyboard out of it (`wants_keyboard` asks the widget), and this was the one other place that
asked the probe.

## The fix

`is_text_field(widget)` — `text_value().is_some()` — is the one question, asked of the widget, and
`wants_keyboard` and the arrow keys both read it. The probe is gone from the shell.

## Verification

- `a_field_with_a_clear_button_is_a_text_field_and_keeps_its_arrows` asserts the trap itself — the
  probe finds no caret at that corner of a field with a clear button — and that the field is a text
  field all the same, empty or not, and that a container is not.
- `scripts/web-caret-keys-check.py`, in headless Edge with the demo built for the web: type
  `hello world`; Left three times and a letter gives `hello woXrld`; Right twice and a letter
  gives `hello woXrlYd`; Home and End reach the two ends (`<hello woXrlYd>`). Before the fix the
  same keys left the value untouched and moved the focus to a filter button.
- clippy `-D warnings` on the shell passes.

## Left

- The other places that ask a widget a probe question of this shape were looked for (`cursor_at`
  at the origin) and this was the only one in the shell. Widgets that override `on_key` for the
  arrows (a slider, a reorderable header) are unaffected: they are offered the key first, as
  before.

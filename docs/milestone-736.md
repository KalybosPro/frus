# Milestone 736 — Cmd is the key of the shortcuts on a Mac

## Objective

The shell read the editing shortcuts — Copy, Cut, Paste, Select all, Undo, Redo — off **Ctrl**
alone. On a Mac the key of those shortcuts is **Cmd**, which winit reports as the "super"
modifier, held in the shell's `meta`; the shell never looked at it. So Cmd+C, Cmd+X, Cmd+V,
Cmd+A and Cmd+Z did nothing in a native macOS build, nor in a web build read in a browser on a
Mac. The same was true of the caret's movements: Option+arrow, which jumps a word on a Mac, moved
one character, and Cmd+arrow, which goes to the start or the end of the line, moved one too.

## What was done

**One question, asked once: is this an Apple keyboard?** The shell holds the answer in
`apple_keys`, filled at start-up by `apple_keyboard()`:

- natively, from the build: macOS and iOS (an iPad with a keyboard uses Cmd too);
- on the web, from the browser, through the same `TargetPlatform::from_browser` the shell uses
  for the target platform at start-up — `navigator.platform`, the user agent and the touch
  points, so an iPad that says it is a Mac is still Apple's.

It is asked of the **machine**, and deliberately not of `default_target_platform()`. That one
answers which design language to follow, and a debug switch or a simulated device changes it: a
window standing in for an iPhone on a Windows machine is still typed on a Windows keyboard, and
Ctrl+C must keep copying there. The idea is the global shortcuts' `Primary` (milestone 709) —
Cmd on macOS, Ctrl elsewhere — with the browser's operating system standing in for the build's.

From that answer, two small functions decide every editing key:

- `primary_held(apple, ctrl, meta)`: the platform's own modifier — Cmd on an Apple keyboard,
  Ctrl elsewhere. It now drives the clipboard (`clipboard_command`, for a field and for a
  selection area), Select all (a field and an area), Undo and Redo, and the field-wide Home and
  End.
- `word_held(apple, ctrl, alt)`: what makes an arrow jump a word — Option on an Apple keyboard,
  Ctrl elsewhere.

And on an Apple keyboard, Cmd with an arrow in a text field goes to a bound
(`apple_arrow_jump`): left and right to the start and the end of the line, up and down to those
of the field, Shift extending the selection — what Home, End, Ctrl+Home and Ctrl+End do
elsewhere, and spelled as those keys to the field.

**Decision: on a Mac, Ctrl+C does not copy.** Only the platform's own modifier counts. On a Mac,
Ctrl belongs to the terminal and to the system fields' own editing keys (Ctrl+A goes to the
start of the line there), and a Mac application that copied on Ctrl+C would be the odd one out.
The other way round holds as well: elsewhere the logo key with C is not Copy (Win+V is the
clipboard's history on Windows).

**Redo on a Mac is Cmd+Shift+Z only.** Cmd+Y is not Redo there — a browser opens its history on
it. Ctrl+Y and Ctrl+Shift+Z both stay Redo elsewhere.

**Android's keys are untouched.** A keyboard's own Copy, Cut and Paste keys, which Android
reports by their physical code alone, need no modifier and are the clipboard on every keyboard.

**On the web**, a focused field is edited through a real `<input>` (milestone 711), and the
browser already handled Cmd+C, Cmd+X and Cmd+V in it; Cmd+A and Cmd+Z are handed to the canvas,
where the shell ignored them until now. A selection area, which has no `<input>`, now copies
with Cmd+C.

**`KeyStroke`'s documentation said** that the shell folded Cmd into `ctrl` on macOS. It never
did: an application's shortcut bound to Ctrl+S answers Ctrl+S on a Mac too, and Cmd+S arrives
with `meta`. The documentation now says so, and that a shortcut meant for both is bound under
both strokes.

## Tests

- `the_clipboard_answers_cmd_on_an_apple_keyboard_and_ctrl_elsewhere`: the primary modifier's
  table, Cmd+C/X/V on an Apple keyboard, Ctrl+C/X/V elsewhere, Ctrl+C on a Mac and the logo key
  elsewhere refused, and a keyboard's own Copy key with no modifier.
- `an_apple_keyboard_jumps_words_with_option_and_bounds_with_cmd`: the word modifier on both,
  and Cmd with each arrow.
- `a_native_keyboard_is_the_builds`: the native answer is the build's operating system.
- End to end, through the test driver, which now has `set_apple_keyboard` (a driver starts with
  everyone else's keyboard, whatever the machine running it, so that a test reads the same on a
  Mac): `cmd_c_copies_on_an_apple_keyboard_and_ctrl_c_elsewhere` selects a line of a selection
  area with the mouse and presses the keys with the modifiers held as a keyboard holds them, then
  reads the driver's clipboard; `cmd_a_selects_the_whole_area_on_an_apple_keyboard` does the
  same for Select all.
- The existing tests of the clipboard keys and of Android's physical keys are unchanged and pass.

## Not done

- **Not tried on a Mac.** No Mac was available: neither the native build nor a browser on a Mac
  has been driven with a real Cmd key. The macOS code was compiled and linted for
  `x86_64-apple-darwin` only, and the web code for `wasm32-unknown-unknown`.
- The web's answer (`apple_keyboard` under `cfg(web)`) has no native test host; it is the same
  `from_browser` reading that milestone 614 tested.
- Deleting a word — Option+Backspace on a Mac, Ctrl+Backspace elsewhere — is not a key the
  shell's own editing has at all; on the web the browser does it in its `<input>`.
- An application's own shortcuts (`Shortcuts`) still match the stroke as pressed: there is no
  `Primary` in a `KeyStroke`, so one meant for Cmd on a Mac and Ctrl elsewhere is bound twice.
- Cmd+Up and Cmd+Down outside a text field, and Home and End's scrolling on a Mac, are left as
  they were.

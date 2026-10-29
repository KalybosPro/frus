# Milestone 711 — A text field on the Web is edited through a real one

## Objective

Milestone 710 left a question about the web: whether a browser's assistive technology can set a
field's value. Looking at it found a larger gap: on the web, frus had **no editable element at
all**. The canvas is pixels, and a field took its text from winit's key events alone:

- no software keyboard on a phone — nothing editable ever had the focus, so none came up;
- no composition — no Chinese, Japanese or Korean, and no accented letter made with an input
  method;
- no dictation, autocorrect or password manager;
- nothing for an assistive technology to set a value in, or to read a caret from.

Milestone 380 had already written it down: "iOS and the Web, neither of which has a text field
yet". winit's web backend ignores every IME call (`set_ime_allowed` and the rest do nothing).

## What was tried first

**The accessibility bridge's node as the editing element.** The bridge (milestone 564) already
builds one DOM element per node, as the canvas's fallback content; making a field's node a real
`<input>` would have given one element for the reader and the keyboard both — and the reference
does exactly that when its semantics are on. A probe in headless Edge settled it: an `<input>`
inside a `<canvas>` takes the focus and its keys, and **no text** — none typed, none composed, not
even a selection set from a script. Fallback content is never laid out, and a browser edits only
what is. The element has to be a sibling of the canvas.

## What was done

- **An editing element over the field with the focus** (`frus-shell/src/web_editing.rs`): an
  `<input>`, or a `<textarea>` for a field of several lines, placed over the field, transparent
  and out of the pointer's way (`pointer-events: none`, so taps and drags still reach the
  canvas). It holds the field's text, its selection and its name, and gets the browser's focus.
  It goes when the field loses the focus, and the canvas gets the focus back.
- **What the browser does comes back.** After every `input`, composition and selection change,
  the element's value, selection and open composition are read and put in the field: the text
  through the message typing would produce — so a field that refuses typing refuses this too, and
  the next frame puts the field's own text back in the element — and the caret, the selection
  and the composition's range as the browser left them, converted from UTF-16 offsets.
- **The keyboard a field asks for** (`Ime::web`, in `frus-widgets`, beside Android's numbers):
  the input `type` (`password` for a secret), `inputmode`, `enterkeyhint`, `autocapitalize`, and
  `autocorrect`/`spellcheck` off for whatever is not prose.
- **Which keys are whose.** The element is outside the canvas, so winit hears nothing typed
  into it. The browser keeps characters (dead keys, AltGr and composition included), deletion
  and the clipboard's keys — a paste the browser makes itself asks for no permission. Every other
  key is cancelled on the element and handed on to the canvas, where the shell treats it as it
  always has: the arrows, Home, End, the page keys and Enter move and edit by the application's
  own layout, which is what is on the screen; Tab moves the focus; Escape, undo and the
  application's shortcuts stay the application's (`browser_edits`).
- **The window's focus on the web** is the page's. The canvas loses the focus to the page's own
  elements — the editing element, the accessibility bridge's nodes — and winit reported each time
  as the window going inactive. The shell now asks the document whether the page still has it.

## Two races, found by the check

- **An arrow straight after letters.** A letter the browser types waits for the end of the turn
  to reach the field; a key handed to the canvas is handled at once. Typed fast, "1", "2", ←
  moved a caret that had not had the letters yet, and the letters' arrival undid the move. The
  shell now reads what the browser did before it handles a key.
- **A letter straight after an arrow.** The arrow moved the field's caret, and the element's
  only at the next frame, so the letter typed just after it went in at the old place. The
  element is now told where a key left the field as soon as the key is handled.

Both were caught by the browser check typing with no pause, and failed there before the fix
(2 runs in 4 for the first) and never after it (12 runs in 12).

## And one focus rule

The element took the browser's focus back on every frame. An assistive technology that moved to
a button of the selection bar was pulled back to the field, and the Enter meant for the button
submitted the field. The element now takes the focus when the field takes the shell's, and again
only when the focus has gone nowhere in particular — the canvas (winit focuses it on every press,
so a tap in the field would otherwise leave it), the page, or the field's own node in the
accessibility tree.

## Checked in a browser

`scripts/web-text-editing-check.py`, headless Edge driven over the DevTools protocol, on the
demo's new-task field:

| | |
|---|---|
| Before the field has the focus | no element |
| Focused | an `<input>` with the browser's focus, outside the canvas, laid out, named *New task*, `autocapitalize="sentences"` |
| Typed "hello world" | in the field and in the element |
| ← three times, then X | *hello woXrld* — the element's caret followed the shell's |
| Home, <, End, > | *<hello woXrld>* |
| A composition "ni", then 你 committed | *<hello woXrld>ni* while open, *<hello woXrld>你* after |
| "1", "2", ←, "3" with no pause | *…你132* |
| Ctrl+A (the shell's), then Z | *Z* — the selection reached the element |
| Backspace | empty |
| The value set as an assistive technology does, then Enter | the field holds it, a task named after it appears, the field empties |
| Tab | the element goes; the focus is on the Add button |

Twelve runs in a row pass. Of the four checks of earlier milestones, on the same build, the
accessibility bridge's, the caret keys' and the browser shortcuts' pass. The selection bar's
(milestone 568) fails two lines — after **Select all** is activated through the accessibility
tree, the bar does not come back offering Cut and Copy — and **fails them the same way on
master**, three runs in three: an older break, left for its own milestone. A third line it
failed on this branch at first, the Enter that pressed the button submitting the field, was this
milestone's (the focus rule above) and passes now.

**Headless Edge sometimes puts the page in the background.** It is hidden, its frames stop, and
a key the shell handles shows nothing until a frame that never comes: that was the check failing
one run in four before anything else was wrong. The check now starts the browser with
background throttling off and focus emulation on, and fails if the page was not visible at the
end.

## Not checked

- **A phone.** The emulator's Chrome has WebGPU but no adapter (not with SwiftShader either), so
  frus draws nothing there, and no phone was connected. Whether a phone's keyboard comes up on a
  tap — the element is focused in the frame after the tap, not in the tap's own event — is the
  first thing to look at on a device.
- **A screen reader heard it.** As for milestone 564, the computed accessibility tree was read,
  not listened to.
- **Firefox and Safari.**

## Found on the way

- **The selection bar after Select all**, on the web: see above. Broken on master too.

## Left

- **Autofill of a whole form.** The browser sees one field at a time; the reference builds a
  `<form>` of every field in the group so that a password manager can fill them all. `autocomplete`
  is `off` for now.
- **The field's two nodes.** While the field is edited, a reader's browse mode finds it twice:
  the bridge's node in the canvas, and the element.
- **The page's keyboard insets.** A phone's keyboard covers the bottom of the page and nothing
  scrolls the field above it.
- **The composition's underline and candidate window.** The composing range reaches the field's
  `Edit`; the element is the size of the field, not of its caret, so an input method's candidates
  come up at the field's corner rather than at the caret.

## Tests

- `web_editing.rs`: UTF-16 offsets become characters (an emoji, a surrogate pair split); the
  selection comes back with its direction; a composition ends at the caret; the field's selection
  goes to the element, with what is past the end of a refused text brought back; which keys are
  the browser's (typing, deleting, the clipboard, composing, AltGr, Option on a Mac) and which
  are the shell's (moving, submitting, Tab, Escape, shortcuts).
- `ime.rs`: the default is a line of prose; a secret is a password input; each keyboard is its
  `inputmode`; several lines are a textarea, and a capitalisation asked for wins.
- `scripts/web-text-editing-check.py`, in a browser, above.

## Mutation testing

Twelve mutants. Ten were killed outright. One — a capitalisation asked for ignored when it is
*none* — survived and found a missing test (a prose field told not to capitalise is `off`), which
now kills it. One — the caret not brought back from past the end of a refused text — was
equivalent: counting characters with `take` already stops at the end. The clamps went instead.

| Mutant | Killed by |
|---|---|
| Typing handed to the shell | `typing_deleting_and_the_clipboard_are_the_browsers` |
| AltGr read as a shortcut | `alt_gr_types_and_alt_is_a_shortcut_but_on_a_mac` |
| A composing key handed to the shell | `typing_deleting_and_the_clipboard_are_the_browsers` |
| Ctrl+V handed to the shell | `typing_deleting_and_the_clipboard_are_the_browsers` |
| Alt types everywhere | `alt_gr_types_and_alt_is_a_shortcut_but_on_a_mac` |
| A backward selection read forwards | `the_selection_comes_back_with_its_direction` |
| No composition read | `a_composition_ends_at_the_caret` |
| UTF-16 counted as characters | `utf16_offsets_become_characters` |
| A secret not a password input | `a_secret_is_a_password_input_on_the_web` |
| The phone keypad as text | `each_keyboard_is_its_inputmode` |
| No capitalisation ignored | `several_lines_are_a_textarea` (after the added line) |
| Past the end not clamped | equivalent — the clamp was removed |

What only a browser can show — the element, the keys handed on, the two races, the focus rule —
is the browser check's.

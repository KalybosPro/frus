# Milestone 526 — Copy and paste in the browser

Answers [#17](https://github.com/KalybosPro/frus/issues/17).

The shell's clipboard was `arboard` on the desktop, the platform's `ClipboardManager` on Android
since milestone 509, and on the web the same empty struct iOS has. A field on a web build took
Ctrl+C and found nothing to copy into; it took Ctrl+V and found nothing to paste. Nothing failed
and nothing logged. The platform table in the roadmap said ❌, which was at least true.

## What was in the way

Not the key: the shortcuts are decided once, by `clipboard_command`, on every platform. The
issue names the real difficulty. The browser's clipboard is **asynchronous and permissioned**,
and the other two are neither. A read is a promise; it may prompt the reader, it may be refused,
a browser may allow it only during a user gesture, and it answers after the key press that asked
for it has been handled. The shell's clipboard had one shape for a paste — `get_text()` returning
the text now — and a promise cannot satisfy it.

The shape the issue asks for has two conditions: the desktop must satisfy it **without
pretending to be async**, and a widget must use it **without branching on the platform**.

## What the reference does

On the web its clipboard is the same API: `navigator.clipboard.writeText` and `readText`, each a
future. When `navigator.clipboard` is missing — an old browser, or a page that is not a secure
context — it raises an error the caller turns into a failed result rather than a crash. There is
no fallback to the older `execCommand` path.

The field's paste is an `async` method: it awaits the clipboard, returns if there is no answer,
and applies the text to **whatever the field's selection is when the answer arrives**, checking
again that the field still accepts a paste. The platforms whose clipboard is immediate go through
the same await, which completes at once.

## The decision

**No widget holds the clipboard, so no widget changed.** The text field never touched it: the
shell reads the key, reads the clipboard and types the text into the focused field as a key. So
the second of the issue's conditions was already met, and the interface to agree on is the
shell's own, private one.

**A paste is asked for on behalf of a field, and answered.** `Clipboard::paste(into)` names the
field that asked. On the desktop and on Android it reads the clipboard and answers there and
then, `Some(Pasted { into, text })`: nothing waits, nothing is spawned. On the web it starts the
read and answers `None`; when the browser has the text, the answer is queued and the window asked
for a frame, and the frame drains the queue through `Clipboard::take_answered` before it builds —
which elsewhere finds nothing. A promise could not be handed to the driver without every platform
becoming a future; a queue drained per frame costs the synchronous platforms one empty check.

**Both answers land through one rule**, `Pasted::lands`: in the field that asked, **only while
it still has the focus**, and **only when there is text**.

- The focus, because on the web the reader can move between the key press and the answer — Tab
  to another field, click away — and a paste arriving in a field nobody pasted into is worse than
  a paste lost. The reference instead pastes into its own field whatever has happened since, since
  its paste belongs to that field's state; here the paste belongs to the key press, and the key
  press was made in a field that had the focus.
- The text, because the browser answers a clipboard holding no text — empty, or an image — with
  `""`, and typed as a key an empty string replaces the selection with nothing. The reference
  returns on a null answer only; Android here already reads an empty clip as nothing, and the
  desktop now does the same.

**The copy keeps the desktop's shape**: `set_text` hands the text over and returns. On the web
the write finishes later, and a refusal is a log line. A cut deletes the selection at once, as
it does on the desktop, whether or not the write is later refused.

**The read is started inside the key press.** `readText` is called before the first `await`, so
the promise exists while the browser is still handling the keydown that asked; a read started
from a later task is no longer part of that gesture, and a browser that ties the read to one
would refuse it. winit's canvas calls `preventDefault` on the keydown, so the browser's own paste
event does not also fire and nothing is pasted twice.

**Nothing on the path can panic.** `navigator.clipboard` is `undefined` outside a secure context,
and a browser may have the object without a `readText` — the bindings would call straight through
and the browser would throw, which in wasm is not a `None`. So the object and the method are both
asked for before either is used; missing, the copy or the paste logs a warning and does nothing.
A refused promise, and a read that answers something other than a string, are log lines too.

The shortcuts are the desktop's, through the same `clipboard_command`: Ctrl+C, X and V, and the
named Copy, Cut and Paste keys.

## Verification

**Tests.** Everything that decides something is outside the web's `cfg`, so it runs natively. A
paste lands in the field that asked while it has the focus, and not once the focus is on another
field or on none, and an empty answer lands nowhere. The web's queue, `clip::Answers`, is held by
the web's clipboard and tested on its own: two pastes asked in two fields and answered in the other
order come out in the order the answers came, each naming the field that asked; each answer wakes
the window once; nothing is taken before it is answered, and nothing is taken twice.

**Mutations** — nine, five killed: the focus ignored, an empty clipboard pasted, an answer queued
at the front, an answer that does not wake, a take that leaves the answer in the queue. The four in
the shell's wiring survived — one answer drained per frame instead of all of them, answers drained
and dropped, `land_paste` ignoring the rule, and the immediate platforms' paste answering nothing.
No test drives the shell's event loop, so nothing can kill them there, as in milestone 521. The
browser half, `web_clipboard.rs`, is JavaScript calls and nothing else; it is not mutated natively.

**Checks.** `frus-shell` checked for `wasm32-unknown-unknown` with no extra flags, as CI builds it:
`readText` and `writeText` are stable bindings, and only the formats variant of `read` needs the
unstable ones. The keydown's `preventDefault` was read in winit's web backend, where it is on
unless an application turns it off, and frus does not. `cargo test -p frus-shell` (106 in the
library), clippy with warnings denied, `cargo fmt --check` and strict `cargo doc` pass. Clippy for
wasm32 flags one line, which is not this milestone's — see below.

## What is left

- **Seen in a real browser**: nothing here has been. What only a browser can show is the
  permission prompt and what follows a refusal; whether a read started from winit's keydown
  handling counts as the gesture everywhere (Safari is the strict one, and Firefox shows its own
  Paste button before answering); and a copy in the page pasted into another application and
  back.
- **Cmd on a Mac.** The shortcuts are Ctrl on every platform, the desktop's macOS build included;
  in a browser on a Mac Cmd+C and Cmd+V do nothing. That is the desktop's gap as much as the
  web's, and one fix for both.
- **Paste from the browser's own menu.** With `preventDefault` on the canvas the page never sees
  a `paste` event, so Edit → Paste and a context-menu paste reach nothing. A text field on the web
  that took those would need the hidden input the soft keyboard work also needs.
- **Whether the clipboard holds text**, for the selection toolbar's Paste item (#23): on the web
  that question is itself a permissioned read.
- **Clippy on wasm32 is not clean**, and CI does not run it: one `needless_return` at the end of
  the web block in `resumed`, older than this milestone and left alone here. The new web code
  has nothing flagged.

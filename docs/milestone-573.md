# Milestone 573 — Text that can be selected and copied

The first half of #24: a `Text` outside a field could not be selected or copied. An order number,
an address, a code or an error message on the screen could be read and not taken.

## What it does

- **`Text::selectable()`** opts one text in. A press puts a caret in it, a drag, a double click or a
  long press selects words, Ctrl+A selects all, and Ctrl+C — or the bar that opens over the
  selection — copies. The selection is highlighted in the theme's selection colour, and one made
  with a finger has the two handles a field's has.
- **`Text::selection_toolbar(build)`** is the application's say over the bar, as
  `TextField::selection_toolbar` is over a field's. The default list is Copy (when something is
  selected) and Select all (unless everything is): a text cannot be cut from or pasted into.
- A selectable text **takes focus** — Tab reaches it, and a keyboard user sees the ordinary focus
  ring — and the arrows, Shift+arrows, Ctrl+arrows, Home and End move its caret. Up and Down still
  move focus.
- **It does not raise the software keyboard.** A new widget hook, `takes_typing`, says whether a
  text field can be changed; the shell asks it before opening the keyboard. This fixes a bug the
  read-only field already had: focusing a `read_only` `TextField` raised the keyboard over the
  words it was there to be read from.

## Decisions

**A read-only field, in effect, and not a new selection system.** A field already has everything a
selection needs: a caret and an anchor kept by the runtime, a drag, handles, a bar, a clipboard
path. A selectable text answers the same hooks (`cursor_at`, `word_at`, `selected_text`,
`selection_handles`, `selection_anchor`, `selection_toolbar`, `on_edit`), so the shell needed one
new question — `takes_typing` — and no new machinery. The pieces the field owned privately
(handle geometry and painting, the bar's anchor box, the word under an index, the caret keys) moved
to shared functions in `textinput.rs`, and `TextField` uses them too.

**The layout follows what is drawn.** `frus-text` gained `TextLayout::resolved(text, &style,
width)`, which shapes with the style's line height and family. The field's `wrapped` knows the
default leading only; a paragraph set at twice the line height and hit-tested against the default
would find its third line where the second is drawn.

**The text writes down what it resolved.** The hooks are asked outside a frame, with no theme and
no reader's font setting in force, and a `Text`'s size may be handed down by a subtree (an app bar)
and scaled by that setting — neither visible from the hook. Layout (`style_themed`) and paint
record the resolved style and wrap in a `Cell`; the hooks read it back. Before any layout, the
fallback is the text's own style under the reader's setting *as it was when the text was built*.
Two tests pin the two paths (a handed-down size, a font setting), and mutating either away fails
them.

**Selection state is boxed and absent by default.** A `Text` is built by the thousand; the
selection's state (the shaping cell, eight lazily built bars, the application's list) lives behind
one `Option<Box<…>>`, eight bytes for a text that is only read. The bars are kept as `dyn Any`
because `Text` is not generic over the application's message; `Widget<Msg> for Text` now requires
`Msg: Clone + 'static`, which every message already satisfies to be boxed into a tree.

## Verification

- Tests: an ordinary text is untouched (no focus, no hit, no bar); a selectable one takes focus and
  no typing; presses land between the letters, line by line in a wrapped paragraph, at the leading
  the paragraph is drawn with, at a size handed down by a subtree, and at the reader's font
  setting; the selected text is by character and in either direction; the caret keys move the
  caret and no other key changes anything; the highlight is under the words and only while
  focused; the handles are painted where the hook says they are; the bar's default lists and the
  application's override; in a built tree a selectable text is a focus target and an ordinary one is
  not; in the shell, neither a read-only field nor a selectable text wants the keyboard, and an
  editable field still does.
- **Mutations**, each of which the named test fails: the layout not remembering what it resolved,
  the reader's setting forgotten at build time, the paragraph's leading ignored (also in
  `frus-text`), the highlight not painted, the caret keys not moving the caret, and a read-only
  field taking typing.
- fmt, clippy `-D warnings` (workspace, `frus-demo --features shots`, `aarch64-linux-android` for
  the shell and widgets), rustdoc `-D warnings`, a `wasm32` check and `cargo test --workspace` pass.
- **On the phone** (Huawei STK-L21, Android 10, a release APK of the demo): the task screen's title
  is selectable. A long press selected `Write` of `Write code`, with the highlight exactly over the
  word, both handles hanging below it and the bar — **Copy**, **Select all** — over it, and no
  keyboard came up. Copy, then a long press in the list's input and **Paste**, put `Write` in the
  field, so the clipboard had the word and only the word.

## Left

- **Alignment, line limits and ellipsis.** A selection is laid out from the start of each line, so
  a centred or end-aligned text, or one cut by `max_lines` or an ellipsis, is highlighted as if it
  were not. Right-to-left text has the field's own limits.
- **Up and Down** move focus, not the caret, in a selectable paragraph.
- **Inside a tap target.** A selectable text inside a button or a list tile takes the press. Make
  selectable the texts that are not the button's label.
- **Selecting across several texts** (a `SelectionArea`) — the second half of #24 — is not done: a
  selection lives in one text.
- **Not run:** a browser, and a desktop (a mouse drag, Ctrl+C, the context-menu key). The shell
  paths they use are the field's, which were run in milestone 568.
- The demo's task screen centres its title and state at the left of the screen rather than the
  middle; it looks like the column shrink-wrapping to its widest child at the start of its box. It
  is not laid out by anything this change touches, so it is very likely older than it, but the
  screen was not run without the change to prove it. Left alone.

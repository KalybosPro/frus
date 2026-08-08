# Milestone 143 — Word jump (Ctrl+Arrows) & field bounds (Ctrl+Home/End)

## Analysis

Keyboard navigation in the text field stopped at the character (Left/Right) and the whole
field (Home/End). The expected editor shortcuts were missing:

- **Ctrl+Left / Ctrl+Right**: jump one **word** at a time.
- **Ctrl+Home / Ctrl+End**: go to the **start / end of the whole field** — and, as a
  corollary in multi-line mode, plain **Home/End** should target the **current line**, not
  the whole field.

## Technical decisions

- **The modifier travels with the key.** Rather than adding `Key` variants, we enrich the
  existing ones: `Key::Left/Right { shift, word }` and `Key::Home/End { shift, doc }`. The
  shell fills `word`/`doc` from `self.ctrl` (already tracked through `ModifiersChanged`).
  The widget remains the sole judge of what those flags **mean**.

- **Editor-style word boundaries.** A "word" character = alphanumeric or `_`. Going left we
  first skip separators then the word (stopping **at the start** of the previous word);
  going right, separators then the word (stopping **after** the next word). Two pure helpers
  over `&[char]`, character indices like the rest of the editing.

- **Home/End become line-relative.** `line_start`/`line_end` scan for the `\n` bracketing
  the cursor. In a **single-line** field, the line bounds are the field bounds: the previous
  behaviour is preserved with no special case. `doc` (Ctrl) short-circuits to `0` / `len`.

- **Shift selection unchanged.** All these moves go through `move_cursor`, so `Shift`
  extends the selection (a word jump / line leap selects), with no extra code.

## Implementation

- `interaction.rs`: `Key::Left/Right` gain `word`, `Home/End` gain `doc`.
- `textinput.rs`: the `is_word`, `word_boundary_left/right`, `line_start/line_end` helpers;
  the corresponding `on_edit` branches.
- `app.rs`: maps Ctrl → `word`/`doc` when building the `Key`s.

## Verification

- **Unit**: `"foo bar baz"` — Ctrl+Left stops at each word's start, Ctrl+Right after each
  word; `"ab\ncd\nef"` — plain Home/End bound the **2nd line** (3 / 5), Ctrl+Home/End bound
  the **field** (0 / 8). The existing Shift+Arrow and Home/End tests stay green after the
  flags were added.
- **No regression**: `cargo test --workspace` green, no golden moved.

## What's left

- **Ctrl+Backspace / Ctrl+Delete**: delete the previous / next word (would reuse
  `word_boundary_*`).
- **Double/triple click**: a double click already selects the word (shell); a triple click
  for the line is still to do.

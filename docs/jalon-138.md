# Jalon 138 — Automatic text wrapping (word-wrap)

## Analysis

Milestone 137 shipped the multi-line field with **explicit newlines**; **automatic
wrapping** was left out (a long line with no `\n` folding back at the field's width),
because character indexing across wraps looked like it drifted. This milestone fixes it at
the source, in the text layer, then wires it to the field.

## The trap (probed, not assumed)

Probing cosmic-text established precisely how it segments a wrapped line:

- A `LayoutRun`'s `run.text` is the **entire hard line** (`"aaaa bbbb cccc"`) — repeated
  identically for **every** visual line. Counting characters from `run.text` therefore
  attributed ~19 characters to *each* wrap: the drift.
- The truth is in the **glyphs**: run 0 covers bytes `0..4` (`aaaa`), run 1 `5..9`
  (`bbbb`), and so on. The **break space** (byte 4, 9…) is **removed** from the glyphs.
- `glyph.x` is **local to the visual line** (restarting from 0 at each wrap), and
  `glyph.start` is the byte **relative to the hard line**.

## Technical decisions

- **Delimit by glyphs, index by bytes.** `TextLayout::wrapped(max_width)` replaces `new`'s
  loop: each visual line carries the byte segment
  `[first glyph, first glyph of the next wrap)` of its hard line (which **includes** the
  break space, at the end of the preceding line), its `offsets` come from the `glyph.x`
  (already local), and its `start_char` from the **hard line's byte offset** + the segment
  — exact indexing, with no phantom character.

- **`new` = `wrapped(None)`.** The general algorithm handles the unwrapped case
  identically (one hard line = one run, the segment = the whole line): no regression across
  all the framework's text (labels, buttons…), validated by the full suite.

- **Rendering and measurement wrapped by the SAME width.** The multi-line field shapes its
  measurement (caret/hit-test) *and* emits its text (`scene.text_wrapped`) with the **same**
  `max_width` = the content width. cosmic-text then produces the same break points on both
  sides → the caret and the selection land exactly on the displayed text.

## Implementation

- `frus-text/src/lib.rs`: `TextLayout::wrapped(text, size, weight, italic, max_width)`;
  `new` delegates with `None`. The `soft_wrap_indexes_chars_correctly_across_lines` test
  (word starts at x≈0 on increasing lines, a round trip in the middle of a wrap, the last
  boundary = the exact character count).
- `frus-widgets/src/textinput.rs`: `layout(wrap_width)`; in multi-line mode, `paint` and
  `cursor_at` wrap at the content width, and rendering goes through `text_wrapped`. The
  `multiline_wraps_long_lines_to_the_width` test.
- `frus-test/tests/goldens/multiline_field.png`: regenerated — a long sentence **without**
  `\n` wrapped over three visual lines.

## Verification

- **Rendered and looked at**: the message wraps softly at the field's width (the
  `multiline_field` golden).
- **Unit**: exact indexing across wraps (frus-text); a click on a wrapped line places the
  caret further into the text (widgets).
- **Total non-regression**: the new layout algorithm underpins **all** text;
  `cargo test --workspace` stays green, no text golden moved.

## What's left

- **Wheel / touch scrolling** in a multi-line field taller than `rows`.
- Breaking **within a very long word** (one exceeding the width): depends on cosmic-text's
  policy; to be checked if a real case demands it.

# Jalon 137 — Multi-line field

## Analysis

The input field was single-line: Enter submitted, the box was one line high, the hit-test
was 1D. A real form needs a **multi-line** field (a message, a comment, notes). The good
news: the text layer is already multi-line — `TextLayout` shapes `\n`, and
`caret_rect`/`hit_test`/`selection_rects` are **2D** (the `y` picks the line). So the work
is mostly in the widget and in click routing.

## Technical decisions

- **A mode on `TextInput`, not a separate widget.** `multiline()` / `rows(n)` reuse all
  the editing, decoration and rendering logic (the established `max_lines` shape). In
  multi-line mode: Enter **inserts a `\n`** (instead of submitting), the box is `rows`
  lines high, and the drawing — already written in 2D (`text_top + r.y`) — naturally shows
  every line.

- **Explicit newlines first, no soft wrapping.** This milestone handles **explicit** `\n`
  (which the layout already counts correctly). **Automatic wrapping** (word-wrap) was
  deliberately set aside: in cosmic-text it requires mapping each visual line to its byte
  range in the source text (the per-run text may omit the break space) — subtle indexing,
  to be done properly in a dedicated milestone rather than approximated here.

- **Minimal vertical scrolling, mirroring the horizontal.** In multi-line mode, a
  `vscroll` keeps the **caret's line** visible, recomputed from the cursor exactly like
  the horizontal scroll (`(caret.y + h − content_h).max(0)`) — so rendering and clicking
  share the same geometry.

- **A 2D hit-test: `cursor_at` gains `local_y`.** Placing the caret on the right line at
  click time needs the vertical coordinate. The trait signature becomes
  `cursor_at(local_x, local_y, width, scroll_cursor)`; the field subtracts the label band
  and the padding, adds the `vscroll`, and delegates to the layout's 2D `hit_test`. In
  single-line mode, `local_y` has no effect (one line only). Every forwarder (`Box`,
  `Keyed`, `Responsive`) and every shell call site (placement + drag selection, "is this
  editable?" probes) is updated.

## Implementation

- `frus-widgets/src/textinput.rs`: the `multiline`/`rows` fields + builders; a multi-line
  `field_height`; Enter inserts `\n` in multi-line mode; `paint` computes the `vscroll` and
  draws at `text_top`; a 2D `cursor_at(local_x, local_y, …)`. Tests: newline vs submit, the
  `rows` height, the per-line hit-test.
- `frus-widgets`: the `Widget::cursor_at` trait + the `Box`/`Keyed`/`Responsive`
  forwarders.
- `frus-shell/src/app.rs`: the `cursor_at` call sites now pass `local_y` (and `0.0` for the
  probes).
- `frus-test/tests/goldens.rs`: the `multiline_field` golden (a floating label + 3 lines in
  a 4-line box).

## Verification

- **Rendered and looked at**: three lines of text in a tall box, the label floated — the
  `multiline_field.png` golden.
- **Unit**: Enter inserts `\n` in multi-line mode (submits in single-line); `rows(4)`
  reserves the height; a click one line lower places the caret on the 2nd line.
- **No regression**: the extended `cursor_at` signature does not alter the single-line
  hit-test (the existing scrolling tests green); `cargo test --workspace` green.

## What's left

- **Automatic wrapping** (word-wrap): `TextLayout::wrapped(max_width)` with correct visual
  line indexing (bytes → characters) — the natural complement.
- **Wheel / touch scrolling** in a multi-line field taller than `rows`.
- Enter key repeat (holding the key) to insert several `\n` in a row.

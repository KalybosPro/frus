# Jalon 63 — `TextLayout`: caret, hit-testing and selection on cosmic-text

The last brick of the basic typography thread (§5): the **geometry of editing**.
`TextInput` computed its positions by hand — `prefix_width` re-measured a
**substring per boundary** (losing the kerning at the cut), and `cursor_at`
measured *every* boundary on each click (O(n²)).

## `TextLayout` (frus-text)

The text is shaped **once** (cosmic-text, unconstrained); from that we extract
the `x` offsets of each **character boundary** from the real glyphs (kerning and
clusters/ligatures included — boundaries inside a cluster are interpolated).
Indices in **characters** (frus's editing convention), coordinates local to the
text, multi-line handled (a `\n` counts as one character).

- **`caret_rect(index)`** — the caret's position at a boundary (zero width: it is
  up to the widget to choose the stroke thickness); clamped to the text.
- **`hit_test(point)`** — the **nearest** boundary (the `y` picks the line, the
  `x` the boundary); that is the semantics of caret placement.
- **`selection_rects(start, end)`** — one rectangle per line crossed.
- **`size()`** — the natural size. Empty text → a synthetic line (caret at
  `x = 0`, fallback height), never undefined.

## `TextInput` migrated

`prefix_width` is gone. The field shapes its value **once** per paint
(`self.layout()`): scrolling (keeping the caret visible), the selection
rectangles, the caret and `cursor_at` (click → boundary) all go through the same
geometry — **consistent** (the same shaped glyphs) where prefix measurements
could drift from the rendering at the kerning level. `cursor_at` goes from O(n²)
to one shape plus one scan.

The behaviour is preserved: the field's pinned tests
(`cursor_at_accounts_for_scroll`, `text_is_clipped_to_content_box`, and all the
editing logic) pass unchanged.

## Validation

- `frus-text`: **9 tests** (+5) — **monotonic** offsets whose last one reaches
  the natural width, a **round trip** `caret_rect ↔ hit_test` on every boundary,
  selection rectangles flush with the carets, multi-line mapping (`ab\ncd`),
  empty text.
- `frus-widgets` **138** (TextInput migrated, its pinned behaviour intact),
  **231 tests** in total, all green; a warning-free build; the demo did not
  panic.

## Not covered (accepted, documented)

- **RTL/bidi**: the offsets assume left-to-right reading (as the old code did);
  bidi will come with the RTL work (§14).
- **min/max intrinsics → taffy**: deferred to the integration of *measure
  closures* in `frus-layout` (the wrapping paragraph) — so as not to ship a dead
  API.

## What's next

The §5 "text" foundation is complete (styles, scale, rich text, editing
geometry). The next candidates: the wrapping paragraph (measurement under taffy
constraint), decorations (underline/strikethrough), or a return to the colour
side (a complete `ColorScheme`, the remaining state layers).

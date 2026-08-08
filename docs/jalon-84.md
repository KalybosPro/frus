# Jalon 84 — RTL: reading direction and layout mirroring (§14, opening)

## Analysis

§14 (i18n/l10n/RTL) starts with **direction**. The goal: correctly displaying a
right-to-left interface (Arabic, Hebrew) — rows, alignment and directional
padding flip, while the text stays legible inside its box (the *internal* bidi of
a paragraph is cosmic-text's business).

## Architecture

- **`frus-core`**: `TextDirection { Ltr, Rtl }` and `InsetsDirectional
  { start, end, top, bottom }` with `.resolve(dir) -> Insets` (in RTL, `start` →
  the right). Carried in the zero-dependency base.
- **Propagation**: `Theme.direction` (the ambient context threaded down to paint,
  pending a §2 `Env`), `Theme::rtl()`. `Theme::lerp` keeps the target's direction
  (a discrete attribute, not faded).
- **Layout mirroring** (the heart of it): taffy 0.7 has no `direction: rtl`.
  Rather than rewriting every widget, the driver **flips the rectangles** of
  *each layout root* around its width when the direction is RTL:

  ```
  r.x  ->  root.x + (root.width - (r.x - root.x) - r.width)
  ```

  taffy computes in LTR (canonical, and cached), and the mirroring is applied
  after retrieval in `Builder::cached_rects`. The result: rows reverse, alignment
  and padding flip, and hit-testing and clips stay consistent (the same
  rectangles) — **without touching the ~60 widgets**. The LTR path is unchanged
  bit for bit (`mirror` short-circuited).

## Decisions

- Mirroring **per layout root** (window, screen, scrolling content, list item)
  around the 1st rect (the root): it composes correctly through nested
  translations.
- Text is not flipped glyph by glyph: its box moves to the correct side and it
  draws normally there; **intra-paragraph bidi** (digits, Latin words inside
  Arabic text) is delegated to cosmic-text.
- `InsetsDirectional` is provided for padding that has to follow the direction;
  the widgets will adopt it progressively.

## Tests (283 → 287)

- `directional_insets_flip_start_end` (core).
- `rtl_mirrors_row_horizontally` (widgets, hit-test): a fixed button moves from
  the left (LTR) to the right (RTL), and the flexible one takes the other edge.
- `rtl_mirrors_the_row` (frus-test, golden + pixels): the row [red][green][blue]
  becomes [blue][green][red] in RTL (red on the right) — visual proof,
  independent of the font.
- The 21 LTR suites stay green (an unchanged path).

## Demo

An "RTL"/"LTR" action in the AppBar's menu: it mirrors the whole application (top
bar, cards, lists, navigation).

## Limits (the rest of §14)

- **Overlay placement** (the Left/Right drawer, menu anchoring) is not flipped
  yet — a "Left" drawer stays on the left in RTL.
- The **back gesture** stays on the left edge (it should be on the right in RTL).
- **Font coverage**: rendering Arabic depends on the font; the bundled font
  (DejaVu) has limited Arabic coverage. On Android, the system font takes over.
- **Localisation** (Fluent) and **accessibility** (AccessKit): separate pieces of
  work, still to come.

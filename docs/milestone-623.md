# Milestone 623 — Letter spacing

## Objective

A text style in frus could not say how far apart its letters sit. The reference's can
(`TextStyle.letterSpacing`, logical pixels after every character), and it uses it all the
time: every step of its Material 3 type scale names one — 0.1 for `labelLarge`, 0.25 for
`bodyMedium`, 0.5 for `bodySmall` and the small labels — and widgets name their own, like
the desktop selection menu's −0.15 (milestone 618). Without it frus's text is set a little
tighter than the reference's everywhere. This milestone gives styles the setting and carries
it through measuring, breaking, hit-testing and drawing; the type scale's values are the next
milestone, because they move text everywhere.

## What was done

- **`TextStyle::letter_spacing`** (`Option<f32>`, logical pixels; negative draws letters
  closer) and the builder `.letter_spacing(px)`. It cascades like every field (`merge`),
  travels in an animated style (`lerp`, as sizes do), and resolves to `0` when unset.
  **It is not grown by the reader's font setting**: the reference scales a style's font size
  and leaves its letter spacing as written. (The reference's painting library is not in the
  archive this repository searches; this is its documented behaviour.)
- **Carried everywhere text is shaped**: `ResolvedTextStyle`, `TextRun` and the scene's text
  primitive hold it; one helper, `frus_text::spaced`, turns it into the shaper's em-based
  setting, and every shaping site goes through it — measurement (`measure_at`, whose cache
  now keys on it), line breaking (`line_spans_resolved`, new), caret and selection geometry
  (`TextLayout::resolved`), rich text, and the renderer's own shaping.
- **`Text` measures under its whole style.** It measured by size, weight and slant alone, so
  a text with a family or a line height of its own was measured in another face, at another
  height, than it was drawn; it now measures, breaks its lines and fits its ellipsis under the
  resolved style. `measure_at` takes the resolved style rather than eight of its parts.

Nothing sets a spacing yet, so nothing moves: every golden is unchanged.

## Tests

- `letter_spacing_cascades_travels_and_stays_written` (frus-core).
- `letter_spacing_widens_a_word_by_its_spacing`: eight letters spaced 2 px are about 16 px
  wider, negative spacing tightens, the caret layout agrees, and the cache tells spaced from
  plain. `letter_spacing_moves_the_line_breaks`.
- `letter_spacing_reaches_the_pixels` (frus-gpu): four letters drawn 6 px apart reach about
  18 px further right than unspaced — the renderer spaces them as the measurement did.

Mutation testing: four mutants, all killed — half the spacing passed to the shaper, the
measurement cache blind to it, the spacing grown with the reader's setting, and the
renderer not spacing at all.

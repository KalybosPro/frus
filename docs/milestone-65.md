# Milestone 65 — `RichText::wrap()`: the wrapped rich paragraph

Closing the "paragraph" story: milestone 64's measurement-under-constraint
mechanics (taffy `MeasureFn`, the `measure_key` that stops the cache going
stale), applied to **rich** text (milestone 62).

## What changes

- **`Primitive::RichText` carries `max_width: Option<f32>`** (+
  `Scene::rich_text_wrapped`): GPU rendering wraps the runs at the **same width
  as the layout**; DPI scaling applies to the wrapping width.
- **`frus_text::measure_runs_wrapped(runs, max_width)`** — rich measurement under
  a width constraint (`measure_runs` delegates to it, unconstrained).
- **`RichText::wrap()`**: free dimensions, `measure()` = an owned closure over
  the flattened runs, `paint()` = `rich_text_wrapped(bounds.width)`.
- **`TextSpan::measure_hash`** (frus-core): the tree's measurement fingerprint —
  texts, sizes, weights, italics — **without flattening** the tree each frame,
  and **without the colours**: recolouring a span must not invalidate the layout
  (a test pins this explicitly).

## Validation

- `wrapped_rich_text_measures_and_keys_by_content`: wrapping bounded in width and
  taller than when free; the measure key **follows the content** but **ignores
  the colour**; without `.wrap()`, neither measure nor key (the hooks' contract).
- **237 tests** in total, all green; a warning-free build; the demo did not
  panic. The About screen's rich tagline now wraps to its card's width.

## What's left (remaining §5)

Text decorations (underline/strikethrough), `letter_spacing`/`line_height`,
consolidating `ColorScheme` (+ HCT `from_seed`), `content_padding` → taffy,
per-corner radii (SDF shader), `Alignment`, RTL.

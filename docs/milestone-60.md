# Milestone 60 — Typography: `TextStyle` + `TextTheme` (weight and italic rendered)

A continuation of the design system (§5). frus had **no typographic model**: a
text primitive reduced to `(size, colour)`, hard-coded sizes in every widget, and
neither weight nor italic. This milestone lays down the vocabulary (`TextStyle`),
the named scale (`TextTheme`, 15 Material steps) and — above all — **genuinely
renders** weight and italic, from measurement all the way to the GPU.

## `TextStyle` (frus-core, pure, `Copy`)

`TextStyle { size, weight: FontWeight, italic, color: Option<Color> }` with
`const` builders and **`merge`** (the cascade: the overlay's typographic
attributes win, and the colour **inherits** when absent — `None` = resolved
against the theme at paint time). `FontWeight` = Regular/Medium/SemiBold/Bold,
mapped onto the OpenType weights.

## The pipeline renders the style, end to end

- **`Primitive::Text`** now carries `weight` + `italic`; `Scene::text_styled`
  emits them (`Scene::text` is unchanged, regular weight).
- **`frus-gpu`** passes weight and italic on to cosmic-text through glyphon's
  `Attrs` — the matching face of the family is chosen (with a graceful fallback
  otherwise).
- **`frus-text::measure_styled`** measures **styled** text — bold is wider, and
  the layout has to know. `measure` delegates to it (regular weight).
- **A bundled bold face**: `DejaVuSans-Bold.ttf` joins the bundled fonts. Without
  it, `Bold` would have **silently** fallen back to the regular face everywhere
  only the bundled fonts exist (Android) — betraying the promise of deterministic
  rendering. (~700 KB; the oblique is not bundled: italic falls back cleanly
  where the system does not provide one.)

## `TextTheme`: the named type scale

`theme.text` = the **15 Material 3 steps** (`display_large 57 … label_small 11`,
with the title/label steps in medium weight). Widgets pick a step
(`Text::styled("Title", theme.text.title_large)`) rather than a hard-coded size.
Typography takes no part in the theme fade (it is identical in light and dark).

## Adoption

- **`Text`**: `.weight()` / `.italic()` builders + a `Text::styled(…)`
  constructor; **styled** measurement (bold lays out correctly); the style's
  colour inherited from the theme when absent.
- **`NavBar`** and **`AppBar`**: titles in **medium** weight (a bar title is a
  "title", not body text), with centring/budget measurement aligned.
- **Demo**: the dialogue title in medium, the empty-state message in *italic*.

## Validation

- **Proof that the bold is real**: `bold_measures_wider_than_regular`
  (frus-text) would fail if `Bold` fell back to the regular face (equal widths) —
  that is the end-to-end validation that the bundled face resolves. Doubled on
  the widget side by `bold_text_lays_out_wider`.
- `frus-core` **49** (+3: builders, OpenType weights, the `merge` cascade),
  `frus-widgets` **132** (+2), `frus-text` **3** (+1); demo/gpu/shell/layout all
  green — **214 tests** in total. A warning-free build; the demo ran without
  panicking.

## What's next (§5, text)

- `TextSpan` (a rich `{text, style, children}` tree flattened into *runs* for
  cosmic-text) — `merge` is already ready for its cascade.
- `TextLayout` on cosmic-text (intrinsics → taffy, `hit_test`, caret, selection)
  — which will also unify multi-run measurement and rendering.
- `letter_spacing`/`line_height`/decorations in `TextStyle` once rendering
  supports them; a bundled oblique face if deterministic italic becomes required.

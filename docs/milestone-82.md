# Milestone 82 — IME input, stage 3: styled composition + suggestion context

## Analysis

Stage 2 (J81) provided a real `InputConnection` but with two limits: the text
being composed was not distinguished (no underline), and `getTextBeforeCursor`
returned an empty local `Editable`, so IMEs **disabled** composition and
suggestions (SwiftKey committed character by character). The goal: a composing
region that is underlined, plus relevant suggestions and predictions.

## Architecture

### The composing region (rendering)
- `Edit.composing: Option<(usize, usize)>` (a range in characters) →
  `Status.composing` (propagated in `full_status`) → `TextInput` **underlines**
  that range (thin 1.5 px rectangles under the text, reusing `selection_rects`
  for the extent).
- Shell (`drain_ime`): on `Composing(text)`, it replaces the previous region and
  records the new range `[caret_before, caret_before+n]` in the `Edit`;
  `FinishComposing`/`Commit`/`clear_composing` set it back to `None`.

### The input context (suggestions)
- `Widget::text_value() -> Option<&str>` (implemented by `TextInput`, delegated
  by `Box`/`Keyed`/`Responsive`): the field's value.
- `android_ime`: a shared `EditorState` (`Mutex`) = text + caret + selection; the
  shell pushes it (`push_ime_context`) after each IME edit and when the keyboard
  opens, and clears it on blur.
- Three natives (`nativeTextBeforeCursor`/`After`/`SelectedText`) read that
  state; the Java `Connection` overrides `getTextBeforeCursor`,
  `getTextAfterCursor`, `getSelectedText` — and above all **`getExtractedText`**
  (the field's complete state): that is what makes SwiftKey **compose** rather
  than commit per character. EditorInfo: `TYPE_CLASS_TEXT | CAP_SENTENCES` (no
  more `NO_SUGGESTIONS`).

## Validated on the device (STK-L21, SwiftKey)

With the IME log to back it up: `Composing("H") → ("He") → ("Hel")` (composition
active; before, it was `Commit` per character). On screen:
- **"Hel" underlined** in the field (the composing region);
- a **suggestion bar** "Hel | Help | Hello" (the context feeds the predictions);
- tapping "Hello" → `Commit("Hello")` + `Commit(" ")`, the underline cleared,
  then **next-word predictions** "how | bro | chef" (a continuous context);
- auto-capitalisation of the first character (the "start of sentence" context
  does come through).

## Tests (281 → 283)

- `composing_region_draws_an_underline` (rendering: the composed range adds thin
  rectangles that are otherwise absent); `text_value_exposes_the_field_content`
  (the context). The model: a `composing` field on `Edit`/`Status` (`Copy`
  preserved).

## Limits (beyond)

- Composition is materialised **in the value** (the controlled field has no
  separate composition buffer): correct for the IME, but a `setComposingRegion`
  over already-committed text would fall back to a plain replacement.
- `getExtractedText` returns a snapshot (no incremental `partial`) — sufficient
  for a single-line field.

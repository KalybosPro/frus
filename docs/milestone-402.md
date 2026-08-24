# Milestone 402 — A style that can say nothing

From a question about milestone 400: *if the reference's field is nullable, can ours not be
`None`?*

It can, it should have been, and the answer is worth more than the change. Milestone 400
gave `Text` a `Chosen` record — one boolean per question the caller could answer — because
`TextStyle::size` was an `f32` and `Text::new` had to put *something* in it. That works and
carries the same information twice. Worse, it leaves something **unsayable**.

## What was unsayable

```rust
Text::styled("Title", TextStyle::new(20.0))
```

That names a size. It also names a weight and a slant, because the type has no way to
withhold them — `TextStyle::new` fills in `Regular` and `false` whether or not the caller
meant to. So *size 20, inherit the weight* could not be written, however anybody wrote it.
The reference writes it `TextStyle(fontSize: 20)` and always could: every field of its
`TextStyle` is nullable, and "unset" is a value the type holds.

That is not a tidiness argument. A caller who wanted one step of the type scale at a
different size had to restate the weight, and a section that set a weight for its labels
could not reach a single one of them.

## Three copies of the same idea

Once the fields are `Option`s, three separate workarounds collapse:

| Where | What it was |
|---|---|
| `frus-core/src/text_style.rs` | a private `Overrides` struct — `TextStyle` with every field wrapped, for rich-text spans |
| `frus-widgets/src/text.rs` | milestone 400's `Chosen` — eight booleans beside the style |
| `frus-widgets/src/widgettheme.rs` | half of `DefaultTextStyle` — every typographic field, again, as an `Option` |

All three existed because `TextStyle::merge` could only replace the typography wholesale,
and each caller needed the other behaviour. `merge` is now field-by-field `or`, and it is
one operation instead of three.

`Text::resolved` went from fourteen lines of bookkeeping to two:

```rust
style: handed.style.merge(self.style).resolved(),
```

`DefaultTextStyle` is now the reference's shape exactly — a `TextStyle`, plus the four
questions that are about the **box** rather than the type (`align`, `soft_wrap`,
`overflow`, `max_lines`).

## Where the chain stops

A shaper needs a number. So `TextStyle::resolved()` returns a `ResolvedTextStyle` with
concrete `size`, `weight`, `italic` and `decoration`, and that is the type the scene and
the measurement functions take now. The type system enforces what a convention used to:
you cannot hand an unresolved style to something that draws.

`DEFAULT_TEXT_SIZE` is 16.0 — the framework's own last word, where nothing in the chain
said anything.

**The colour stays optional even when resolved**, and deliberately. Size, weight and slant
have a framework default that is right everywhere. A colour's last word belongs to the
*theme*, which `frus-core` cannot see, so a widget resolves it at paint against
`on_surface` as it always did.

`Text::new` now starts from `TextStyle::NONE` rather than a 16 px style. Those read the
same and mean different things: the old one *answered* 16, so an app bar or a section could
not dress it.

## Two behaviour changes that fell out, both in the right direction

- **`TextSpan::style(s)` no longer forces every field.** It used to build an `Overrides`
  with `Some(...)` on size, weight, slant and decoration, because a `TextStyle` could not
  say otherwise. A style naming only a size now overrides only the size, and the weight
  goes on being inherited from the parent span.
- **A merge no longer erases a decoration nobody replaced.** `over.decoration` was a value
  the type could not withhold, so merging a plain style over an underlined one silently
  removed the underline. There is a test for it now.

## What the tests said

The whole routine suite passed on the first run after the conversion, and all 91 goldens
are unchanged. That is the expected result and worth stating: `TextStyle::new(20.0)` still
resolves to the same three numbers it used to produce; what changed is that it now *says*
one of them instead of three.

## Left

- **`frus_text::measure_style(text, style)`** replaced the three-argument
  `measure_styled(text, size, weight, italic)` at most call sites. Three arguments is three
  chances to pass a size from one style and a weight from another, which draws text the
  layout never measured. A few sites still take the three, where they genuinely have loose
  numbers.
- The reference's `TextStyle` has far more than six fields — `height`, `letterSpacing`,
  `wordSpacing`, `fontFamily`, `fontFeatures`, `shadows`, `background`. None of them are
  here yet, and the shape they would need is now the shape that exists.

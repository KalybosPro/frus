# Milestone 410 — A style can name its font

`frus_text::set_default_family` and `set_monospace_family` register faces for the whole
program. `monospace_family()` exists and returns one. **No widget could use it.** A frus
application chose its fonts once, globally, or not at all — it could not say *this text, in
that face*, which is an ordinary thing to want and the reference's `TextStyle.fontFamily`.

## The type

```rust
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(&'static str),
}
```

`Copy`, and `Named` borrows, so [`TextStyle`] stays `Copy` and travels down a subtree by
value like every other field. Family names come from `add_font`, which registers them for
the life of the program, so a `&'static str` is the normal case rather than a restriction.

## The rule that is not obvious

A named family **does not always win**. A run containing Arabic keeps the registered Arabic
face whatever was asked for.

That is not timidity. cosmic-text does not fall back across families on Android, where the
platform fallback lists are empty, so a family without Arabic coverage renders **nothing at
all** — not a substituted glyph, not a box, nothing. Text in an unexpected face is a smaller
failure than a blank screen, and a caller who wants an Arabic family names it and gets it.

The proper answer is **coverage**: ask the face whether it has the characters. fontdb does
not offer that cheaply. That is a real limit, and it is written into the function's own
documentation rather than left for someone to find on a device.

## Measure and paint call the same function

```
TextStyle::family  →  Primitive::Text.family
                          ├─ measure  : family_for_style  (+ the cache key)
                          └─ paint    : family_for_style   ← literally the same call
```

Not two rules that have to agree — one function, called twice. Milestones 407, 408 and 409
were each spent on a version of two things that had to agree and eventually did not; doing
it this way from the start was the least this milestone could do.

The measurement cache is keyed on the family for the same reason it gained the line height:
different faces set the same words to different widths, and that difference is the entire
point of naming one.

## The tests

- `a_named_family_reaches_the_measurement` — a monospaced face sets the same words to a
  different width. Were it not so, the field would be decoration.
- `arabic_keeps_its_face_whatever_was_named` — the script wins over the name, and a Latin
  run *in the same style* still takes the name.
- `two_families_do_not_share_a_cached_answer` — asked in both orders, so a cache answering
  from the wrong entry fails as loudly as one that never filled.

## Left

- **Coverage is not consulted**, per above. A named family that does cover Arabic is
  overruled anyway.
- **No widget defaults to `Monospace` yet.** `Kbd`, and anything showing code, should — they
  currently take the default sans. The mechanism is now there; using it is a separate step,
  and a visible one, since it moves goldens.
- `letterSpacing`, `wordSpacing`, `fontFeatures`, `shadows` and `background` remain out of
  reach for the reasons milestone 409 recorded.

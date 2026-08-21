# Milestone 378 — A badge that is more than one colour

`Badge` had **one** builder.

```rust
Badge::new(text)
```

No colour, no text colour, no size, no padding, no type. A pill in `primary`, a label at a
hardcoded 13 px, 8 px of padding either side, and no entry in `WidgetThemes` — so neither
the caller nor the theme could change any of it.

That is the same breach milestone 368 found on `ExpansionTile`: themed defaults are fine,
hardcoded-only never. `Badge` was the next one down the list, and the shortest file in the
catalogue was hiding it.

## A dot is a shape, not an empty label

The reference draws two badges. With a label it is a pill carrying a number; without one it
is a **dot**, and a dot is what most notification marks actually are — *something happened*
is the whole message, and a count nobody reads is a number taking up room on a bell.

Ours could not draw one at all. `Badge::dot()` is it, and `label_visible(false)` falls back
to it rather than to an empty pill, which would be a wide blank mark saying nothing. That
is what a count of zero wants, and deciding it at the call site means an `if` around a
widget instead of a value inside one.

## `error`, not `primary`

The untold fill is the scheme's `error` now, as the reference has it. A badge is an
**alert**, not an accent: it says *look here*, and painting it in the same colour as every
selected tab and pressed button makes it one more thing in the accent colour.

This changes the default appearance. It is a correction rather than a preference — the
previous colour was not a decision, it was the first role to hand.

## Round below a certain width

A one-character badge is never narrower than it is tall. A lone digit sitting in a wide pill
reads as a mistake, and the reference rounds a single character to a circle for the same
reason. The floor is a minimum and not a size: `1024` still widens it.

## What is not here, and why it is not a builder

The reference's `Badge` takes a **child** and pins itself to that child's corner — which is
what a badge usually is, a mark *on* something. Ours is standalone.

It was in this milestone until it was not. A badge with a child has to become a stack
holding both, and a stack cannot be assembled from `&self`: the child is a
`Box<dyn Widget<Msg>>` that cannot be cloned or moved out on the way down. The routes that
remain are real but each is its own decision — build the pair in `child()` and layer them
through the existing `stack()` hook, or give the walk a way to paint a widget *over* a
subtree, which `foreground()` does for decorations and cannot do for text.

Half a stack shipped now would be worse than none. The mark itself needed the colours
either way, and it has them.

## One golden moved

`small_indicators.png`. The badge went from a wide green pill to a round red one — both
changes at once, and both intended: `error` rather than `primary`, and a single digit
rounded rather than stretched. It was read before it was accepted, which is the point of
the rule.

## Also

A comment in `paint` was in **French** — *"Pastille (pilule) d'accent."* The repository is
English throughout except one file, and that one is `README.fr.md`.

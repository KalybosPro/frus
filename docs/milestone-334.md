# Milestone 334 — The child that takes what is left

Milestone 333 fixed half of a device finding and named the other half: a long task label
pushes the row's delete button off the card, out of the window, and out of the hit
registry, so the task cannot be deleted at all. `Text::ellipsis()` taught the label to be
*cut*; the note said the button now needed `flex-shrink: 0`, and that
`frus_layout::Style` had no such field.

Reading the reference before writing that field changed the answer.

## The reference has no shrinking at all

`RenderFlex._computeSizes` lays out its inflexible children first, and it hands them
`_constraintsForNonFlexChild` — which, in a row, constrains the **cross** axis only:

```dart
Axis.horizontal =>
  fillCrossAxis
      ? BoxConstraints.tightFor(height: constraints.maxHeight)
      : BoxConstraints(maxHeight: constraints.maxHeight),
```

No maximum width. An inflexible child is never squeezed; it is asked how big it wants to
be and believed. Only then is what is left divided among the flexible ones, each given
exactly `spacePerFlex * flex`.

So the reference's answer to this row is not "let the button refuse to shrink". It is
**the label should never have been asked how wide it wants to be**. That is what
`Expanded` does, and it is the textbook fix for exactly this overflow.

## `Expanded`

A transparent wrapper that changes one thing — the flex item its child is. Three
properties at once, and it is worth saying why none of them works alone:

| | | |
|---|---|---|
| `flex_basis: 0` | the child stops telling the row how wide it wants to be | without it the row is over budget before anything is shared |
| `flex_grow: n` | it then takes the spare room, or a share of it | without it, a basis of zero means a child of zero width |
| `min_width: 0` | it is *allowed* to be narrower than its content | without it, `flex: 1` grows and still never yields |

The third is the one everybody forgets, on the web as here: a flex item's automatic
minimum size is its own content, so `flex: 1` on a long label produces exactly the
overflow it was reached for to prevent. All three, or none.

```rust
row![
    Checkbox::new(todo.done),
    Expanded::new(label.ellipsis()),   // takes the rest, cut to fit
    IconButton::new(IconName::Close),  // keeps its 40 px
]
```

The `spacer()` that used to sit before the button is gone: an expanding label is what
pushes the button to the right edge, and two things both claiming the spare room would
have split it.

## `flex_shrink` as well, because the other case is real

`Expanded` handles the row whose deficit comes from *one* greedy child. It does nothing
for a row that is over budget with every child at its natural size — there, flexbox's
default still applies and everything squashes in proportion, the smallest fixed thing
included. So `Style` gains `flex_shrink` too (default `1.0`, flexbox's and taffy's, so
nothing moves), with `shrink()` / `no_shrink()` on `Flex` and `Container`.

Two fields, because the flex item model has three properties and this framework shipped
with one.

## Two things the wrapper found on the way

**The demo's row never filled its card.** With `Expanded` in place and every unit test green,
the device drew the × *on top of the label*. The tree says why: the row measured 126 px inside
a 323 px card. A flex container is sized by its content on its own main axis — stretch is a
cross-axis rule — so the row was as wide as its children and the expanding label had nothing to
expand into. It had always been that way; the old overflowing label merely hid it. The row now
asks to fill (`.flex(1.0)`), and the fact that every caller has to know this is on the roadmap:
the reference's `Row` defaults to `MainAxisSize.max`.

**Clearing the cross axis cost a height.** The first version of `restyle` blanked `width` and
`height` on the grounds that a fixed size would overrule a basis of zero. Half true: a basis
already overrules the size on the main axis, so nothing needed clearing — and on the cross axis
the child's own size is exactly the one wanted. `Text` reports its measured size *as its style*
rather than through a measure function, so an expanded label came out 0 px tall, invisible in a
centred row. Both facts now have a test.

## Read from the registry, not the picture

The regression test drives the demo's own `view` at a phone's width, sweeps every other
pixel of the window, and asks the hit registry what a tap there would send. It is green
now and it fails on the previous row with the exact words of the report:

```
no tap anywhere in the window deletes the long task
```

That is the assertion worth having. A screenshot would have shown a button crushed to a
sliver and called it cosmetic; the button was not there at all. A second assertion — that the
target sits on the right-hand side of the window — is what caught the row not filling its card,
which the first one happily passed.

On the device: a task label longer than the phone is wide shows an ellipsis, its × sits on the
right edge in line with the other rows', and tapping it deletes the task.

## The wrapper macro grew a third hook

`Expanded` is transparent in every respect but the box, and `forward_transparent!`
hardcoded `style`. Rather than hand-write the other seventy hooks — the specific mistake
that macro exists to prevent — both `style` and `style_themed` now go through `restyle`,
an inherent method each wrapper writes. `Keyed` and `Themed` state that they return the
child's box unchanged, in the same spirit as the two hooks already handled that way: what
a wrapper does not claim, it visibly forwards.

The test that checked wrappers state those hooks used a hand-kept list of two files. It
now finds them by looking for the macro, so a wrapper is covered the day it is written.

## Left

- **`Flexible`.** The reference's loose fit — *at most* your share, less if you want it —
  has no flexbox spelling and is not here. `Expanded` is the tight one.
- **`shrink()` is on `Flex` and `Container` only.** Nine other widgets carry `flex()` and
  should carry its counterpart; none has a case yet that needs it.
- **`maxLines`,** still. `Text` is one line or a paragraph.
- **A row that overflows does so silently.** The reference paints stripes across it and
  writes to the console. Here it simply draws outside its parent, which is how this bug
  survived to reach a device.

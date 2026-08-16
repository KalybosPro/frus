# Milestone 317 — The field everybody types into

`TextInput` is the widget an application cannot avoid, and it had nine hardcoded numbers,
one variant that was really two half-variants, and no way to change any of it. The sweep
that rewrote `Tabs`, `Chip`, `Button`, `SegmentedControl` and added `IconButton` arrives
here, and the findings are the same kind — with one that is not.

## The numbers, none of which were right

Read out of `input_decorator.dart`'s Material 3 defaults rather than remembered:

| | was | reference |
|---|---|---|
| content padding, filled | 8 × 6 | `(12, 8, 12, 8)` |
| content padding, outlined | 8 × 6 | `(12, 20, 12, 12)` |
| value type | 18 | `body_large`, 16 |
| floating label | fixed 13 | **× 0.75** of the field's own type |
| helper / error | 12 | `body_small`, 12 ✓ |
| icon | 20 | 24 |
| corner radius | the theme's | 4 |
| border, rest → focus | 1 → 2 ✓ | 1 → 2 |

Two of those are more than a number.

The outlined field's **asymmetric** padding — 20 above, 12 below — is not a typo in the
specification. An outlined field floats its label *onto* the top border, so the top has to
give it room; a filled one floats it inside the box and does not. A single symmetric
padding cannot express that, which is why the old one looked wrong in both variants at once.

And the label **scales** rather than taking a second fixed size. A fixed 13 is invisible
next to the mistake it makes: a field told `size(32)` got a 13 px label. `0.75 ×` keeps the
relationship at any size, and the reference writes `0.75 * labelStyle.fontSize` for exactly
that reason.

## One variant was two half-variants

`outlined()` was a modifier on a default that was neither of the reference's two fields:
both drew the same stroked box, differing only in where the label went. The reference has
**filled** and **outlined**, and they are not the same widget tinted differently.

Filled is a container with its **top two corners** rounded and a single line under it. A
fill inside a four-sided stroke is the outlined variant wearing a tint — it says *box* when
the whole point of the filled field is that it says *surface*. There is a test that counts
stroked rectangles, because this is the sort of thing that drifts back one careless commit
at a time.

The filled field is also **taller when it has a label**, by `4 + 0.75 × size`, since the
label floats inside the box and something has to make room for it. The reference calls that
`floatingLabelHeight` and adds it above the content; so does this.

## `enabled`, at the fourth time of asking

Milestones 312, 313 and 314 each ended with the same line in *Left*: the widget cannot be
disabled. Three in a row is not three oversights.

A disabled field here keeps 38% of its colours — the reference's figure — and that is the
easy half. The rest: it shows **no focus** (it cannot be focused, so a ring left over from
before would be a lie about what Tab will do), it leaves the tab order, it refuses a caret,
it ignores a key arriving from a focus set before it was disabled, and it tells a screen
reader it is disabled. It still **displays its value**, because the reference dims a
disabled field rather than hiding it: it is usually the answer to why the rest of a form
looks the way it does.

## What the bigger field broke, and what it uncovered

Two things, and only one of them was this milestone's doing.

**A cell editor no longer fitted its row.** A field is 56 px tall now, which is right for a
form and wrong inside a table. The reference has an answer and frus did not have it:
`isDense`, `(12, 4, 12, 4)` filled and `(12, 16, 12, 8)` outlined — the room given back,
the shape, label and border unchanged. `dense(true)` is here, and the editable table and
the demo's grid use it.

**Scrolled multi-line text was painted over its own label.** The content was clipped to the
**box** rather than to the box *inside its padding*, so the third line of a scrolled field
rode up onto the top border — which is exactly where an outlined field's floating label
lives. This was not introduced here: it has been wrong since the clip was written, and six
pixels of padding hid it. Twenty pixels did not. It is the useful kind of regression, and
it is the golden that caught it — no unit test was looking at the clip rectangle, and there
is one now.

## Nothing hardcoded

`TextInputStyle` carries fourteen settings and resolves `caller ?? theme ?? framework`
through a new `TextInputTheme` — the same chain milestone 309 established and every widget
since has followed.

## Verification

1064 tests (8 new), clippy silent. The new ones pin the reference's measurements, that a
filled field strokes **nothing** and is underlined at its foot, that focus widens *and*
recolours, that a disabled field is dimmed and inert on all five counts, and the resolution
order. One new golden, `filled_field`, holds the three states side by side. **Eighteen** goldens
moved — every scene with a field in it, and nothing else — and all eighteen were read, which
is how the clip defect was found.

## Left

- **Padding and icon size resolve `caller ?? framework` only.** Both are needed by
  `style` and `cursor_at`, which run without a theme in hand. Closing it means routing them
  through `style_themed`, which milestone 310 built for exactly this and which this
  milestone did not take up.
- **No bare underline field.** The reference's *default* is an underline with no fill;
  here the two on offer are filled and outlined, and the default is outlined.
- **The label does not animate its own weight or letter spacing**, only position and size.
- **No character counter, no `maxLength`, no prefix/suffix *text*** — only icons.

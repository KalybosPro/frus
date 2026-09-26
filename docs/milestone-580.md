# Milestone 580 — The single-purpose boxes: `Padding`, `ColoredBox`, `DecoratedBox`, `ClipRect`

`Container` could inset, fill, decorate and clip, and nothing else could. That meant a tree
said `Container` where it meant "a padding", and an application following the reference's
vocabulary had no `Padding`, the most common wrapper in that vocabulary. The audit that found
`Center` missing (milestone 579) listed these four next. Each is one thing a `Container` does,
given a name.

## What it does

- **`Padding::new(padding, child)`** insets the child. The padding is:
  - one number for all four sides;
  - `Insets`, now with `Insets::symmetric(horizontal, vertical)`;
  - `InsetsDirectional`, now with `only_start` and `only_end`.

  A new `InsetsGeometry` takes any of the three through `Into`, as `AlignmentGeometry` does for
  anchors.
- **`ColoredBox::new(color, child)`** fills its box in one colour behind the child.
- **`DecoratedBox::new(decoration, child)`** paints a `BoxDecoration` behind the child;
  **`DecoratedBox::foreground(decoration, child)`** paints it in front.
- **`ClipRect::new().child(…)`** cuts whatever the child paints past its box, alongside
  `ClipRRect`, `ClipOval` and `ClipPath`.

## Right to left

The frame is laid out left to right and mirrored as a whole in a right-to-left script. A
`Padding` has to account for that:

- a directional inset is resolved **as if left to right**, and the mirror puts its start on the
  right;
- a physical inset is **swapped before layout**, so the mirror puts it back on the side it names.

That is the reference's meaning of the two: a left inset is on the left in Arabic too. The first
version resolved both against the theme's direction. The test caught it: the directional inset
was flipped twice and came back on the left.

## How

- `Padding` and `ClipRect` are boxes of their own, like the other clips.
- `ColoredBox` and `DecoratedBox` are a `Container` with one setting, forwarded through the
  macro the named animated widgets already use. That macro now forwards the foreground
  decoration too, and is shared with this module.

## Sizes

Like every box here, these are **as big as their child**, stretched across a flex line, and fill
the room only when something inside asks to. A `ColoredBox` around a centred child fills the
whole box it is in; around a 20-px square it is 20 px wide. The reference instead hands a lone
child tight constraints and a fill would always cover the box. The framework's model of when a
box fills is the same for all widgets (milestones 333 and 405). These do not change it.

## Verification

- Frame tests:
  - ten all round and a symmetric padding put the child at its insets;
  - a start inset leaves the room on the right of the start in left to right, and on the left
    in right to left;
  - a left inset stays on the left in both;
  - a coloured box is its child's width around a square, and the whole box, painted before the
    child, around a centred one;
  - a decoration is painted before the child, or after it as a foreground;
  - a clipped 200-px square is painted in a layer clipped to its 50-px box;
  - asked for without a theme, a padding's style carries the padding, resolved left to right.
- Mutations, each failing a test:
  - the directional inset resolved against the direction (the double flip);
  - no swap of a physical inset;
  - the theme's direction ignored;
  - no colour;
  - the foreground painted behind;
  - the macro not forwarding the foreground;
  - no clip;
  - no padding.

  The last one survived at first. The walk only asks a widget for its style with a theme, so
  the style asked without one went unread. It is still public, and a test now reads it
  directly.
- Not run on the phone: no device was attached. Nothing here is platform code.

## Left

- A `Container`'s own `padding_each` goes through the same mirror untouched, so by the reasoning
  above its left inset would end up on the right in right to left. This milestone does not check
  or change that.
- Of the reference's other basic wrappers, `MouseRegion`, `Listener`, `GestureDetector`,
  `Builder`, `StatefulBuilder` and the listenable, future and stream builders are still missing.

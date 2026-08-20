# Milestone 357 — A stack that can place a layer

Milestone 336 counted the catalogue: 391 of the reference's widgets against ours, 22
missing, in four groups. All four groups are closed — focus, shortcuts, filters, and the
boxes. The catalogue is done.

Which leaves what that milestone said was larger than any of it and did not measure:
**depth**. A widget's presence is not the same as its every property.

## The audit, and what it is worth

The same shape of script: for two dozen widgets applications actually reach for, the
reference's constructor parameters and declared fields against our builder methods.

It is a **pointer, not a count**. Without a synonym map `textAlign` reads as missing when
we spell it `align`, and `onChanged` when we spell it `on_toggle`; `TextField` came back
with 67 "gaps" of which most are a `focusNode`, a `mouseCursor`, a `dragStartBehavior` or
a semantics label — things this framework answers elsewhere or does not have the concept
for. Reading it is the work; the script only says where to look.

Where it looked worst was not `TextField`. It was `Stack`, whose entire surface here was

```
new  width  height  flex  layer
```

against the reference's `alignment`, `fit`, and — the one that matters — `Positioned`,
which **did not exist at all**. A badge on a corner, a caption over an image, a bar across
the bottom of a photo: all of them are a stack with one layer pinned somewhere, and none
of them could be written.

## `Positioned`

Each edge optional, and what is given decides the size as well as the place, as in the
reference:

- `left` **and** `right` → the width is what is between them;
- one of them plus `width` → that width, at that edge;
- neither → the child's own width, placed by the stack's alignment.

```rust
Stack::new()
    .layer(photo)
    .layer(Positioned::new(badge).top(8.0).right(8.0))
```

It is a transparent wrapper that claims one hook, `Widget::positioned`, which the stack
reads. Not a style: an offset from an edge is not something the layout engine's box model
has a field for, and the number it resolves to depends on how big the stack came out.

## The fit, and a deviation kept on purpose

The reference's `StackFit` defaults to `loose` — an unpinned child is *asked* what size it
would like. Ours defaults to `Expand`, where it is *given* the stack's box, and that stays.

It is not an oversight. In the reference a loosely-constrained box with no size of its own
still fills, because a childless box there takes the biggest size it is allowed. Under
this framework's layout engine the same widget hugs and comes out at nothing — invisibly,
since a stack draws no box of its own — and a scrim, a barrier and every internal overlay
here are exactly that widget. `fit(StackFit::Loose)` asks for the other behaviour, and is
what a badge or a caption wants. Both are documented where the choice is made.

## Three things the tests caught

**`stack_loose` was not forwarded.** `the_macro_forwards_every_hook_the_trait_declares`
failed the moment the hook was added — a wrapped stack would have reported the default
fit. That test is two years of this bug class in one assertion.

**A pin has to overrule a child's own size.** `Positioned(left: 10, right: 30)` on a 20 px
badge left it 20 px wide: `Constraints::filled` fills a dimension the content left `Auto`
and defers to one it chose. That deference is right for a layer merely *handed* the box —
a badge sitting in a filled stack has always relied on it — and wrong for a pin, where two
opposite edges are not a suggestion and a width of its own is a contradiction rather than
a second opinion. `Layout::compute_tight` is the third answer, and the distinction is now
three named things instead of a boolean: asked, filled, forced.

**A one-layer stack would have shifted twice.** `Stack::alignment` is answered through the
same `alignment_geometry` hook `Container::alignment` uses, and the walk applies that hook
to any single-child widget. A stack means something else by it — where each *layer* sits,
which the stack branch applies per layer — so the general path now excludes stacks.

## Left, from the same audit

- **`Scroll`** has no `padding` and no `reverse`. A chat list that grows from the bottom
  is the second of those, and it is a real gap.
- **`Image`** has `fit` and `tint` and no `width`/`height`/`alignment`/`repeat`.
- **`Container`** covers the reference's decoration surface but not `transform` or
  `foregroundDecoration`; `constraints` is reachable through `ConstrainedBox`.
- The audit itself is not committed. It reads a source archive that is not in the
  repository, and a script that cannot be run by whoever comes next is a liability rather
  than an artifact — the list above is what it produced, and that is the part worth
  keeping.

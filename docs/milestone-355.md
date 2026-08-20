# Milestone 355 — A `LayoutBuilder` is as big as what it built

The reference's `LayoutBuilder` sizes itself to its child. Its render object says so in
four lines:

```dart
void performLayout() {
  final BoxConstraints constraints = this.constraints;
  runLayoutCallback();
  if (child != null) {
    child!.layout(constraints, parentUsesSize: true);
    size = constraints.constrain(child!.size);   // ← as big as what it built
  } else {
    size = constraints.biggest;
  }
}
```

Ours was a **layout leaf**: its size came from its style, and its content had no say. Its
own module documentation said as much, under the heading "the sizing contract", and told
you to go and find a height for it — which is the shape of an admission rather than a
contract. Dropped into a column with nothing set, it laid out 0 px tall and its content
drew through whatever was below it.

## Why it was a leaf

The obvious reason, and it is a real one: to build the content you need the box, and to
know the box you need the content. The framework builds its layout tree, hands it to the
engine, and *then* has boxes. There is no point in that sequence where a widget can ask
what it is about to be given.

Except there is, and the framework was already using it for text. `Layout::measured_leaf`
registers a closure the engine calls **during** the computation, with the space actually
available — a paragraph is measured that way, because how tall a paragraph is depends on
how wide it is allowed to be. That is the same moment, and the same question, as
`runLayoutCallback()`.

So a `LayoutBuilder` is a measured leaf now. The closure builds the subtree, lays it out
in a tree of its own, and returns its size.

## What it cost

**A lifetime on `Layout`.** The measurement has to borrow: it calls back into the widget
layer with the widget being measured and the frame's runtime. `MeasureFn` was
`Box<dyn Fn(…) -> Size>`, which is `'static`, and a `'static` closure cannot hold a
reference to the tree it measures. It is `MeasureFn<'a>` now, on `Layout<'a, T>`, and
`build_layout` threads the lifetime through. Every existing measurement — text, rich
text, the alert — owns what it needs and is unaffected.

The one thing that could **not** be borrowed is the theme, because a themed subtree's
theme is a local: it is resolved inside the walk, from the widget, and there is nothing
for it to outlive. `Theme` is `Copy`, so the closure takes one.

**A hole in the relayout cache.** The cache reuses a root's geometry when a fingerprint
of its styles and structure has not moved, and its contract is strict: a hit is
bit-for-bit what the full computation would have produced. A closure has no fingerprint.
Two frames whose styles and structure are identical can want different boxes, because the
application changed what it builds — that is the entire purpose of the widget.

So a root holding a `LayoutBuilder` is **volatile**: it is recomputed every frame and no
entry is stored for it. The alternative — hashing the subtree built at some canonical
size — is a fingerprint that is right most of the time, and a cache that is right most of
the time is worse than no cache, because the failures are invisible and intermittent.

## What is unchanged

An axis the style pins is still the style's. The engine does not ask about a dimension it
already knows, so `width`, `height` and `flex` win exactly as before and every existing
use behaves identically. `a_pinned_axis_is_still_the_style_s` says so with a sibling that
sits at 120 rather than at the content's 60.

## What to know

The closure runs **more than once a frame** — once for the measurement, once for the
paint, and again for each intrinsic question the engine asks. It must be cheap and free
of side effects, which it had to be anyway: it is called with no retained state.

And an intrinsic question has no honest answer here. Asked "how big would you be with no
limit", finding out means running the application's callback speculatively; the reference
refuses outright (`computeDryLayout` asserts and returns `Size.zero`). We build at
whatever numbers were offered, substituting zero for the ones that were not, and leave
the unoffered axis free in the nested computation so the content's own size comes back.

## Nothing moved

All 91 goldens are unchanged, and that is the expected result rather than a lucky one:
the change is additive. A `LayoutBuilder` with both axes settled behaves exactly as it
did, and the demo's only one — the task summary that shortens its label when the footer
is narrow — pins both (`flex(1.0)` across, `height(20.0)` down). What changed is what
happens when an axis is left open, which used to be nothing and is now the reference's
answer.

## What it unblocks

The grid delegate milestone 353 could not write: a column count derived from a maximum
tile width. It needs the grid's real width before the layout and a height that follows
what it produced, which is now exactly what this widget provides.

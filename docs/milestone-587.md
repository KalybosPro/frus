# Milestone 587 — A proposal for #52: measuring children

Issue #52 asks for the shape to be argued before any code, and names two:

- **lazy children**, a child being a closure the parent may call;
- **a resolution pass** between build and layout.

This proposes a third, which the framework has already half built. No code is changed by this
milestone. It is here to be argued with, in the issue.

## What is already there

Three kinds of node in the layout build (`build_layout_scoped` in `ui.rs`) size themselves
from a child, and none of them is new:

- **`ConstraintsTransformBox`**, with `UnconstrainedBox`, is a *measured leaf*. Taffy calls
  its measure closure with the space on offer. The closure lays the **already built** child
  out in a `Layout` of its own, under constraints derived from the offer
  (`AxisConstraint::offered`), and answers with the size that came back. The walk then lays
  the child out again at the box it produced and places it by an alignment.
- **`LayoutBuilder`** is the same, except that its child is *built* in the closure, from the
  offer. That is where its "no retained state" comes from.
- **`Intrinsic`** measures its child once, unconstrained (`natural_size`), and writes the answer
  into its own style before layout.

So a widget *can* measure its child. It just cannot say what to do with the measurement: the
one rule a `ConstraintsTransformBox` knows is "as big as the child, held to the offer".

## The proposal: measured containers

One hook, in place of the fixed rule:

```rust
/// This widget sizes itself from its child's size, laid out separately under
/// `offer_to_child(offer)`, and places the child in the box it answers with.
fn measured(&self) -> Option<&dyn MeasuredLayout>;

trait MeasuredLayout {
    /// The constraints the child is laid out under, from the ones offered.
    fn child_constraints(&self, offered: Offer) -> Offer;
    /// This widget's size, from the offer and the child's size.
    fn size(&self, offered: Offer, child: Size) -> Size;
    /// Where the child goes in that box.
    fn position(&self, size: Size, child: Size) -> Point;
}
```

`Offer` is the pair taffy already hands a measure closure: per axis, a number or "asked".
That pair says whether the incoming constraint was unbounded, which is the information
`LimitedBox` needs. `ConstraintsTransformBox` becomes the first implementation of the trait,
with its present rule, and nothing about it changes.

**A multi-child form** (`MultiMeasuredLayout`) takes the children's sizes in order and answers
a position for each. It is the reference's `CustomMultiChildLayout` delegate in the only
shape this framework can offer without lazy children: every child is measured first, then
positioned. The reference allows a delegate to lay out child B with constraints derived from
child A's size; that ordering is lost. The multi-child form has to be argued separately.

## Against the six symptoms and the wart

| symptom | fixed? | how |
|---|---|---|
| `AnimatedSize` (#30) | **yes** | `size` answers the animated size; the runtime keeps the last target per identity, as it keeps an animated size today |
| `SizeTransition` (#32) | **yes** | `size` is the child's times the factor, on one axis |
| `Aligned` width/height factors (579) | **yes** | `size` is the child's times the factor |
| `LimitedBox` (#37) | **yes** | `child_constraints` limits an axis that was asked rather than given |
| a layout delegate (#38) | **single child: yes; several: partly** | see the multi-child form above |
| the FAB's declared height and its `*Top` placements | **partly** | a scaffold that measures its slots is the multi-child form, and a large refactor of `Scaffold` on top of it |
| an extended body told what it runs under | **partly** | the same refactor |
| `AppBar` being a builder | **no** | it decides *what to build* from its width. That is `LayoutBuilder` territory, which keeps no state; only lazy children fix it properly |
| `Intrinsic` being partial | **yes** | `Intrinsic` becomes a `MeasuredLayout` whose `child_constraints` asks for the natural size |

## What this costs, and what has to be fixed first

- **The child is laid out twice**, once in the measure closure and once in the walk, as a
  `ConstraintsTransformBox`'s is today. Each is a separate taffy tree. Text is not reshaped,
  because shaping is cached apart from layout, but the tree is rebuilt. The walk's pass
  already goes through the relayout cache (`cached_rects`); the measure closure's does not.
  It should, keyed on the child's identity and the offer. Without that, a measured container
  deep in a list costs one extra layout per frame per container.
- **The child is painted by the reduced walk** (`render_item`), the one a virtualised list item
  goes through. By its own comment ("it cannot defer an overlay") that walk has no deferred
  overlays, nor every special branch of the main one, so a dropdown's menu declared inside an
  `UnconstrainedBox` should not show. That follows from the code; the case was not run for this
  proposal. Before measured containers are the general tool, their child has to go through the
  main walk, given the rects its separate layout produced. This is the biggest piece of work in
  the proposal, and it pays for itself at once: an `OverflowBox` paints its child through the
  same reduced walk today.
- **Animated sizes read a target the layout found.** A measured `AnimatedSize` learns its new
  target during layout. The runtime keeps the target the last frame found and animates towards
  it, so a change shows one frame late. The reference's is the same frame; one frame at 60 Hz is
  judged acceptable here, and the proposal says so rather than hiding it.

## Why not the other two

- **Lazy children** change what a child *is*, the most load-bearing type in the crate, for
  every widget. They are the only way to fix `AppBar`. But every symptom except `AppBar` is
  fixed without them, and `AppBar` works today as a builder.
- **A resolution pass** adds a walk to every frame for the few widgets that need it. The
  measured leaf is already the resolution, done by taffy at the moment it needs the answer
  and only for the nodes that ask.

## If this is accepted, in order

1. The child of a measured leaf goes through the main walk. Its first test is the case above,
   run: a dropdown inside an `UnconstrainedBox` opening its menu.
2. The measure closure's layout goes through the relayout cache.
3. `MeasuredLayout`, with `ConstraintsTransformBox` and `Intrinsic` moved onto it; then
   `LimitedBox`, the `Aligned` factors, `SizeTransition`, and `AnimatedSize`.
4. The multi-child form, argued on its own.
